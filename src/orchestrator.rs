//! Compilation orchestrator module
//!
//! Orchestrates the multi-serializer execution loop based on configuration,
//! reading raw JSON and generating static files according to specified layouts.

use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

use crate::Result;
use crate::config::{
    AggregateMode, AggregateSpec, ApiNode, DeriveConfig, DeriveType, EmitTarget, FilterCondition,
};
use crate::context::SerializerContext;
use crate::{Config, Error, JSONSerializer, Serializer, SqliteSerializer, TypescriptSerializer};

/// Number of unique `$derive`d values above which [`derive_values_from_data`]
/// prints a stderr warning, as a guard against accidentally generating an
/// enormous number of template sub-paths.
const DERIVE_CARDINALITY_WARN_THRESHOLD: usize = 1000;

/// Executes the API build process.
///
/// First verifies that every serializer destination is empty or allowed to
/// be overwritten (returning [`Error::DestNotEmpty`] otherwise). Then reads
/// every `*.json`
/// dataset from `data_dir`, and for each serializer listed in
/// `config.serializers` materializes the full routing tree (applying
/// `$filter`/`$aggregate`/`$pick`/`$omit`/`$emit`/template expansion from
/// `config.api`) and writes the resulting static files. Finally, a
/// discovery index listing every generated endpoint is written, and any
/// static assets allowed by the `$static` policy are copied into each
/// destination.
///
/// The build writes into [`Staging`] directories throughout and is published
/// to the configured destinations only once every step has succeeded, so a
/// build that fails leaves them as they were.
///
/// # Examples
///
/// ```no_run
/// use fauxrest::{Config, run};
///
/// let config = Config::load_from_file("_config.json").expect("valid config");
/// run(config, "data").expect("build should succeed");
/// ```
pub fn run<P: AsRef<Path>>(mut config: Config, data_dir: P) -> Result<()> {
    for s_conf in &config.serializers {
        ensure_dest_writable(s_conf)?;
    }
    let staging = Staging::redirect(&mut config)?;
    build(&config, data_dir.as_ref())?;
    staging.publish()
}

/// Runs the build itself, writing through whatever destinations `config`
/// currently names. Split out of [`run`] so every fallible step happens while
/// those destinations are still [`Staging`] directories.
fn build(config: &Config, data_dir: &Path) -> Result<()> {
    let mut endpoints = Vec::new();
    let dataset = DataSource::new(data_dir)?;
    for s_conf in &config.serializers {
        let context: SerializerContext = s_conf.try_into()?;
        endpoints.extend(run_serializer(context, &dataset, &config.api)?);
    }
    generate_discovery(config, &endpoints)?;
    crate::static_files::copy_static_files(config, data_dir)
}

/// Prefix for staging directory names, so one left behind by a killed process
/// is recognizable.
const STAGING_PREFIX: &str = ".fauxrest-staging-";

/// Points every serializer at a private directory for the duration of a
/// build, keeping the real destinations untouched until it succeeds.
///
/// A build writes through three paths — endpoints, the discovery index, and
/// copied static files — and all of them resolve against a serializer's
/// `dest`, so redirecting `dest` covers every one of them.
///
/// Dropping this discards the staged output: a failed build leaves nothing
/// behind, which is the whole point. Publishing is deliberately the last step,
/// after every step that can fail on the data has already run.
struct Staging {
    /// Each staging directory paired with the destination its contents belong
    /// in. Dropping the [`TempDir`] deletes the directory.
    dirs: Vec<(TempDir, PathBuf)>,
}

impl Staging {
    /// Replaces each serializer's `dest` with a fresh staging directory,
    /// remembering where its contents are eventually to be published.
    ///
    /// The staging directory is created beside its destination rather than in
    /// the system temporary directory, so publishing renames within one
    /// filesystem instead of copying the whole output.
    fn redirect(config: &mut Config) -> Result<Self> {
        let mut dirs = Vec::new();
        for s_conf in &mut config.serializers {
            let parent = staging_parent(&s_conf.dest);
            fs::create_dir_all(&parent).map_err(Error::Io)?;
            let dir = tempfile::Builder::new()
                .prefix(STAGING_PREFIX)
                .tempdir_in(&parent)
                .map_err(Error::Io)?;
            let dest = std::mem::replace(&mut s_conf.dest, dir.path().to_path_buf());
            dirs.push((dir, dest));
        }
        Ok(Staging { dirs })
    }

    /// Moves the staged output into the real destinations.
    ///
    /// Files are merged into what is already there rather than replacing the
    /// destination wholesale: these outputs are published to static hosts, and
    /// a `CNAME` or `.nojekyll` sitting next to them is not this build's to
    /// delete. That makes publishing non-atomic, but it moves already-written
    /// files and computes nothing, so the only failures left are the kind that
    /// would break any write.
    fn publish(self) -> Result<()> {
        for (dir, dest) in &self.dirs {
            publish_tree(dir.path(), dest)?;
        }
        Ok(())
    }
}

/// Returns the directory to create a staging directory in, given a
/// destination. A relative single-component `dest` such as `dist` has an empty
/// parent, which is the current directory.
fn staging_parent(dest: &Path) -> PathBuf {
    match dest.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

/// Recursively moves everything in `staging` into `dest`, creating
/// directories as needed and leaving unrelated entries in `dest` alone.
fn publish_tree(staging: &Path, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest).map_err(Error::Io)?;
    for entry in fs::read_dir(staging).map_err(Error::Io)? {
        let entry = entry.map_err(Error::Io)?;
        let target = dest.join(entry.file_name());
        if entry.file_type().map_err(Error::Io)?.is_dir() {
            publish_tree(&entry.path(), &target)?;
        } else {
            move_file(&entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Moves one file, falling back to a copy when it cannot be renamed.
///
/// A rename is what makes publishing cheap, but it only works within one
/// filesystem; `dest` may be a mount point or a bind mount even though the
/// staging directory sits beside it. The rename error is dropped in that case
/// because the copy attempt that follows reports the real obstacle — a
/// permission or space problem surfaces there just the same.
fn move_file(from: &Path, to: &Path) -> Result<()> {
    if fs::rename(from, to).is_ok() {
        return Ok(());
    }
    fs::copy(from, to).map_err(Error::Io)?;
    fs::remove_file(from).map_err(Error::Io)?;
    Ok(())
}

/// Aborts the build with [`Error::DestNotEmpty`] when the serializer
/// destination already contains files and overwriting has not been
/// explicitly permitted (via the `overwrite` config field or the
/// `--overwrite` CLI flag). A missing or empty destination is always fine.
fn ensure_dest_writable(s_conf: &crate::SerializerConfig) -> Result<()> {
    if s_conf.overwrite {
        return Ok(());
    }
    let dest = &s_conf.dest;
    if dest.is_dir() {
        let is_empty = fs::read_dir(dest).map_err(Error::Io)?.next().is_none();
        if !is_empty {
            return Err(Error::DestNotEmpty(dest.display().to_string()));
        }
    }
    Ok(())
}

/// In-memory collection of raw JSON datasets loaded from the input data
/// directory, keyed by dataset/endpoint name (the file name without the
/// `.json` extension).
struct DataSource {
    entries: HashMap<String, Value>,
}

impl DataSource {
    /// Loads all top-level `*.json` files from `dir` into a new `DataSource`
    /// (see [`collect_datasets`] for the file selection rules).
    pub fn new<P: AsRef<Path>>(dir: P) -> Result<Self> {
        collect_datasets(dir.as_ref()).map(|entries| Self { entries })
    }

    /// Returns the names of all loaded datasets, sorted alphabetically.
    pub fn names(&self) -> Vec<String> {
        let mut names = self.entries.keys().cloned().collect::<Vec<_>>();
        names.sort();
        names
    }

    /// Returns the keys of `api_overlay` that do not correspond to a loaded
    /// dataset (i.e. virtual endpoints defined only via the routing
    /// overlay, such as `$aggregate`-only endpoints), sorted alphabetically.
    pub fn overlay_names<'a>(&self, api_overlay: &'a HashMap<String, ApiNode>) -> Vec<&'a String> {
        let mut result = api_overlay
            .keys()
            .filter(|&k| !self.entries.contains_key(k))
            .collect::<Vec<_>>();
        result.sort();
        result
    }

    /// Looks up the raw data for dataset/endpoint `name`.
    ///
    /// Returns [`Error::Config`] if no such dataset was loaded.
    pub fn get(&self, name: &str) -> Result<&Value> {
        self.entries
            .get(name)
            .ok_or_else(|| Error::Config(format!("missing dataset for endpoint '{}'", name)))
    }
}

/// One target's data at the stages its callers need separately.
///
/// The three differ in what has been applied, and using the wrong one is how
/// endpoints go missing or appear when they should not:
///
/// - `base` is handed to children, so a child `$filter` replaces its parent's
///   rather than narrowing what the parent already removed.
/// - `filtered` is what `$derive` enumerates, so a record the filter dropped
///   cannot bring an endpoint into existence. `$pick`/`$omit` are deliberately
///   not applied yet: omitting a field must not silently empty the set of
///   values derived from it.
/// - `written()` is what reaches the output.
struct EndpointData {
    base: Value,
    filtered: Value,
    /// The `$pick`/`$omit` form, present only when the node has either.
    /// `None` means `filtered` is already what gets written, which keeps the
    /// common node from paying for a copy of its dataset.
    picked: Option<Value>,
}

impl EndpointData {
    /// The value to serialize for this endpoint.
    fn written(&self) -> &Value {
        self.picked.as_ref().unwrap_or(&self.filtered)
    }
}

/// A single endpoint being materialized: its URL path (`endpoint`), the
/// underlying dataset (`data`), and the optional overlay [`ApiNode`]
/// (`node`) carrying directives that apply to it.
struct Target<'a> {
    endpoint: &'a str,
    data: &'a Value,
    node: Option<&'a ApiNode>,
}

impl<'a> Target<'a> {
    /// Builds a `Target` directly from its parts.
    pub fn new(endpoint: &'a str, data: &'a Value, node: Option<&'a ApiNode>) -> Self {
        Target {
            endpoint,
            data,
            node,
        }
    }

    /// Builds a top-level `Target` for `endpoint`, looking up its raw data
    /// in `source` and its overlay node (if any) in `api_overlay`.
    pub fn build(
        endpoint: &'a str,
        source: &'a DataSource,
        api_overlay: &'a HashMap<String, ApiNode>,
    ) -> Result<Self> {
        let data = source.get(endpoint)?;
        let node = api_overlay.get(endpoint);
        Ok(Target::new(endpoint, data, node))
    }

    /// Returns this target's overlay sub-paths as `(key, node)` pairs,
    /// sorted by key. Returns an empty list if there is no overlay node.
    pub fn subpaths(&self) -> Vec<(&str, &ApiNode)> {
        if let Some(current) = self.node {
            let mut items = current
                .sub_paths
                .iter()
                .map(|(k, v)| (k.as_str(), v))
                .collect::<Vec<_>>();
            items.sort_by(|a, b| a.0.cmp(b.0));
            items
        } else {
            vec![]
        }
    }

    /// Applies `mapper` to this target's overlay node, if present, flattening
    /// the result. Used to read optional per-node directives (e.g.
    /// `n.filter.as_ref()`) without repeating the `Option` dance at each
    /// call site.
    pub fn map_node<F, R>(&'a self, mapper: F) -> Option<&'a R>
    where
        F: FnOnce(&'a ApiNode) -> Option<&'a R>,
    {
        self.node.and_then(mapper)
    }

    /// Computes this target's endpoint data in the stages its callers need to
    /// keep apart. See [`EndpointData`] for what each is used for.
    pub fn build_endpoint_data(
        &self,
        filters: &Option<&Vec<FilterCondition>>,
        sources: &DataSource,
    ) -> Result<EndpointData> {
        let mut base = self.data.clone();
        if let Some(agg) = self.map_node(|n| n.aggregate.as_ref()) {
            base = aggregate_values2(agg, sources)?;
        }

        let filtered = if let Some(filters) = filters {
            apply_filters(&base, filters)?
        } else {
            base.clone()
        };

        let picked = match self.node {
            Some(n) if n.pick.is_some() || n.omit.is_some() => {
                Some(apply_pick_omit(filtered.clone(), n))
            }
            _ => None,
        };
        Ok(EndpointData {
            base,
            filtered,
            picked,
        })
    }

    /// If `data` is an array, writes one file per item under
    /// `{endpoint}/{id}` (using each item's `id` field), returning the
    /// list of written endpoint paths. Items without an `id` field are
    /// skipped. Returns an empty list if `data` is not an array.
    pub fn emmit_ids(&self, data: &Value, context: &SerializerContext) -> Result<Vec<String>> {
        let mut results = Vec::new();
        if let Value::Array(arr) = data {
            for item in arr {
                if let Some(id) = item.get("id") {
                    let id_str = id
                        .as_str()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| id.to_string());
                    let item_path = format!("{}/{}", self.endpoint, id_str);
                    write_data2(&item_path, item, context)?;
                    results.push(format!("/{}", item_path));
                }
            }
        }
        Ok(results)
    }
}

/// Materializes every endpoint for a single [`SerializerContext`]: all
/// loaded datasets, plus any overlay-only `$aggregate` endpoints that have
/// no backing dataset file. Returns the sorted, deduplicated list of
/// written endpoint paths.
fn run_serializer(
    context: SerializerContext,
    source: &DataSource,
    api_overlay: &HashMap<String, ApiNode>,
) -> Result<Vec<String>> {
    let mut endpoints = Vec::new();
    for name in source.names() {
        let target = Target::build(&name, source, api_overlay)?;
        endpoints.extend(materialize_node(&target, None, source, &context)?);
    }
    for name in source.overlay_names(api_overlay) {
        let Some(node) = api_overlay.get(name) else {
            continue;
        };
        if node.aggregate.is_none() {
            continue;
        };
        let target = Target::new(name, &Value::Null, Some(node));
        endpoints.extend(materialize_node(&target, None, source, &context)?);
    }
    endpoints.sort();
    endpoints.dedup();
    Ok(endpoints)
}

/// Loads every top-level `*.json` file in `data_dir` into a map of dataset
/// name (file stem) to parsed [`Value`].
///
/// Files and directories whose name starts with `_` or `.` are skipped
/// (allowing config files like `_config.json` to live alongside data), as
/// are any non-`.json` entries.
fn collect_datasets(data_dir: &Path) -> Result<HashMap<String, Value>> {
    let mut datasets = HashMap::new();
    let entries = fs::read_dir(data_dir).map_err(Error::Io)?;
    for entry in entries {
        let entry = entry.map_err(Error::Io)?;
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();
        if file_name_str.starts_with('_') || file_name_str.starts_with('.') {
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let content = fs::read_to_string(&path).map_err(Error::Io)?;
        let json: Value = serde_json::from_str(&content).map_err(Error::SerdeJson)?;
        let name = file_name_str
            .strip_suffix(".json")
            .unwrap_or(&file_name_str)
            .to_string();
        datasets.insert(name, json);
    }
    Ok(datasets)
}

/// Recursively writes output files for `target` and all of its overlay
/// sub-paths.
///
/// A `$skip` node returns immediately, before reading data and before looking
/// at its sub-paths. Nothing below it is visited, so no sub-path can publish
/// through a node that was skipped — including one added long after the
/// `$skip` was written, which is the guarantee `$emit: []` cannot make.
///
/// Otherwise applies the effective `$filter` (the target's own filter, or the
/// inherited `filter` from its parent if it has none), then writes the
/// list/id outputs selected by `$emit` (see [`resolve_emit_flags`]). For
/// each sub-path, expands `${name}` template keys into one child target per
/// resolved value (see [`resolve_template_values`]) or descends into a
/// plain named child, recursing in both cases. Returns the combined list of
/// endpoint paths written by this call and all of its descendants.
fn materialize_node(
    target: &Target,
    filter: Option<&Vec<FilterCondition>>,
    sources: &DataSource,
    context: &SerializerContext,
) -> Result<Vec<String>> {
    if target.map_node(|n| n.skip.as_ref()) == Some(&true) {
        return Ok(vec![]);
    }

    let effective_filter = target.map_node(|n| n.filter.as_ref()).or(filter);

    let data = target.build_endpoint_data(&effective_filter, sources)?;
    let mut endpoints = Vec::new();

    let (emit_list, emit_id) = resolve_emit_flags(target.node);
    if emit_list {
        write_data2(target.endpoint, data.written(), context)?;
        endpoints.push(format!("/{}", target.endpoint));
    }
    if emit_id {
        endpoints.extend(target.emmit_ids(data.written(), context)?);
    }

    let base = &data.base;
    for (key, child) in target.subpaths() {
        if let Some(var) = template_var_from_key(key) {
            let values = resolve_template_values(target.endpoint, key, child, &data.filtered)?;
            for value in &values {
                let segment = scalar_to_path_segment(value)?;
                let child_endpoint = format!("{}/{}", target.endpoint, segment);
                let expanded_child = expand_template_node(child, var, value);
                let child = Target {
                    endpoint: &child_endpoint,
                    data: base,
                    node: Some(&expanded_child),
                };
                endpoints.extend(materialize_node(
                    &child,
                    effective_filter,
                    sources,
                    context,
                )?)
            }
        } else {
            let child_endpoint = format!("{}/{}", target.endpoint, key);
            let child = Target {
                endpoint: &child_endpoint,
                data: base,
                node: Some(child),
            };
            endpoints.extend(materialize_node(
                &child,
                effective_filter,
                sources,
                context,
            )?);
        }
    }
    Ok(endpoints)
}

/// Resolves the `$emit` directive of `node` into `(emit_list, emit_id)`
/// flags. Absent a node or an explicit `$emit`, both default to `true`
/// (emit everything); an explicit `$emit` list enables exactly the targets
/// it names.
fn resolve_emit_flags(node: Option<&ApiNode>) -> (bool, bool) {
    let Some(node) = node else {
        return (true, true);
    };

    if let Some(targets) = node.emit.as_ref() {
        let emit_list = targets.iter().any(|t| matches!(t, EmitTarget::List));
        let emit_id = targets.iter().any(|t| matches!(t, EmitTarget::Ids));
        (emit_list, emit_id)
    } else {
        (true, true)
    }
}

/// Executes a `$aggregate` directive: reads each source dataset from
/// `source` and combines them according to `aggregate`'s mode.
///
/// In [`AggregateMode::Flat`] mode, array sources are concatenated and
/// non-array sources are appended as single elements, producing a
/// [`Value::Array`]. In [`AggregateMode::Keyed`] mode, each source is
/// inserted whole under its resolved key, producing a [`Value::Object`];
/// returns [`Error::Config`] if two sources resolve to the same key.
fn aggregate_values2(aggregate: &AggregateSpec, source: &DataSource) -> Result<Value> {
    match aggregate.mode() {
        AggregateMode::Flat => {
            let mut merged = Vec::new();
            for entry in aggregate.entries() {
                let data = source.get(&entry.from)?;
                if let Value::Array(arr) = data {
                    merged.extend(arr.iter().cloned());
                } else {
                    merged.push(data.clone());
                }
            }
            Ok(Value::Array(merged))
        }
        AggregateMode::Keyed => {
            let mut merged = Map::new();
            for entry in aggregate.entries() {
                let data = source.get(&entry.from)?;
                let key = entry.key.unwrap_or(entry.from.clone());
                if merged.contains_key(&key) {
                    return Err(Error::Config(format!(
                        "$aggregate keyed output contains duplicate key '{}'",
                        key
                    )));
                }
                merged.insert(key, data.clone());
            }
            Ok(Value::Object(merged))
        }
    }
}

/// Applies `filters` to `data`: for an array, keeps only the items that
/// satisfy every condition (see [`matches_all_conditions`]); for a scalar
/// or object, returns it unchanged if it matches, or [`Value::Null`]
/// otherwise.
fn apply_filters(data: &Value, filters: &[FilterCondition]) -> Result<Value> {
    match data {
        Value::Array(arr) => {
            let mut out = Vec::new();
            for item in arr {
                if matches_all_conditions(item, filters)? {
                    out.push(item.clone());
                }
            }
            Ok(Value::Array(out))
        }
        _ => {
            if matches_all_conditions(data, filters)? {
                Ok(data.clone())
            } else {
                Ok(Value::Null)
            }
        }
    }
}

/// Returns whether `item` satisfies every condition in `filters` (a
/// logical AND).
///
/// A condition that cannot be evaluated is propagated as an error and aborts
/// the build. `$filter` is what keeps records out of the generated API, so
/// treating an unevaluable condition as a match would publish the very
/// records it was meant to withhold — silently, and with a successful exit
/// code. `Config` rejects unusable regex patterns when it loads, so a
/// configuration that validated should never reach this path.
fn matches_all_conditions(item: &Value, filters: &[FilterCondition]) -> Result<bool> {
    for cond in filters {
        if !cond.apply(item)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Applies `node`'s `$pick` (keep only listed fields) and then `$omit`
/// (remove listed fields) directives to `value`. Works on both a single
/// object and an array of objects (each element is transformed
/// independently); non-object values pass through unchanged.
fn apply_pick_omit(mut value: Value, node: &ApiNode) -> Value {
    if let Some(pick) = &node.pick {
        value = match value {
            Value::Object(obj) => Value::Object(apply_pick_map(obj, pick)),
            Value::Array(arr) => Value::Array(
                arr.into_iter()
                    .map(|item| match item {
                        Value::Object(obj) => Value::Object(apply_pick_map(obj, pick)),
                        _ => item,
                    })
                    .collect(),
            ),
            other => other,
        };
    }
    if let Some(omit) = &node.omit {
        value = match value {
            Value::Object(obj) => Value::Object(apply_omit_map(obj, omit)),
            Value::Array(arr) => Value::Array(
                arr.into_iter()
                    .map(|item| match item {
                        Value::Object(obj) => Value::Object(apply_omit_map(obj, omit)),
                        _ => item,
                    })
                    .collect(),
            ),
            other => other,
        };
    }
    value
}

/// Retains only the keys of `obj` that appear in `pick`.
fn apply_pick_map(mut obj: Map<String, Value>, pick: &[String]) -> Map<String, Value> {
    obj.retain(|k, _| pick.contains(k));
    obj
}

/// Removes each key in `omit` from `obj`.
fn apply_omit_map(mut obj: Map<String, Value>, omit: &[String]) -> Map<String, Value> {
    for key in omit {
        obj.remove(key);
    }
    obj
}

/// Resolves the set of values used to expand a `${var}` template sub-path
/// named `key` under `endpoint`: either `child`'s literal `$values` list,
/// or values derived from `source_data` via `$derive` (see
/// [`derive_values_from_data`]). Config validation guarantees one of the
/// two is present, but this is re-checked defensively.
fn resolve_template_values(
    endpoint: &str,
    key: &str,
    child: &ApiNode,
    source_data: &Value,
) -> Result<Vec<Value>> {
    if let Some(values) = child.values.as_ref() {
        return Ok(values.clone());
    }

    if let Some(derive) = child.derive.as_ref() {
        let cfg = derive.to_config();
        return derive_values_from_data(source_data, &cfg, &format!("{}/{}", endpoint, key));
    }

    Err(Error::Config(format!(
        "{}/{}: template key requires $values or $derive",
        endpoint, key
    )))
}

/// Extracts the deduplicated set of `$derive` values from `source_data`
/// (an array of items, or a single object), reading `cfg.field` from each
/// item and applying `cfg.pattern` if set (see [`derive_scalar_value`]).
///
/// Values named by `cfg.exclude` are dropped (see [`is_excluded`]); unlike a
/// value that could not be derived, an excluded one is deliberate and is not
/// counted as skipped.
///
/// Values are deduplicated by their [`scalar_deterministic_key`] and
/// returned in a stable (sorted) order. Prints a warning to stderr,
/// tagged with `context`, if the number of unique values exceeds
/// [`DERIVE_CARDINALITY_WARN_THRESHOLD`] or if any items were skipped
/// because their value could not be derived.
/// Returns whether `value` is named by `cfg.exclude`.
///
/// Comparison is by `serde_json::Value` equality, so it distinguishes kinds:
/// an `exclude` of `0` leaves the string `"0"` in place. The check runs on the
/// already-converted value, so an entry names what would have become the path
/// segment.
fn is_excluded(value: &Value, cfg: &DeriveConfig) -> bool {
    cfg.exclude
        .as_ref()
        .is_some_and(|excluded| excluded.contains(value))
}

fn derive_values_from_data(
    source_data: &Value,
    cfg: &DeriveConfig,
    context: &str,
) -> Result<Vec<Value>> {
    let mut unique: BTreeMap<String, Value> = BTreeMap::new();
    let mut skipped = 0usize;

    let collect = |v: &Value, unique: &mut BTreeMap<String, Value>| -> Result<bool> {
        let Some(extracted) = derive_scalar_value(v, cfg)? else {
            return Ok(false);
        };
        if is_excluded(&extracted, cfg) {
            return Ok(true);
        }
        let key = scalar_deterministic_key(&extracted);
        unique.entry(key).or_insert(extracted);
        Ok(true)
    };

    match source_data {
        Value::Array(arr) => {
            for item in arr {
                if let Some(v) = item.get(&cfg.field)
                    && !collect(v, &mut unique)?
                {
                    skipped += 1;
                }
            }
        }
        Value::Object(obj) => {
            if let Some(v) = obj.get(&cfg.field)
                && !collect(v, &mut unique)?
            {
                skipped += 1;
            }
        }
        _ => {
            return Ok(vec![]);
        }
    }

    if unique.len() > DERIVE_CARDINALITY_WARN_THRESHOLD {
        eprintln!(
            "warning: {} derived {} values (threshold {})",
            context,
            unique.len(),
            DERIVE_CARDINALITY_WARN_THRESHOLD
        );
    }
    if skipped > 0 {
        eprintln!(
            "warning: {} skipped {} non-derivable values while processing $derive",
            context, skipped
        );
    }

    Ok(unique.into_values().collect())
}

/// Extracts a single derived scalar from `value` according to `cfg`.
///
/// If `cfg.pattern` is set, `value` is stringified (string/number/bool
/// only) and matched against the compiled pattern; the first capture group
/// (or, absent one, the whole match) becomes the result. Without a
/// pattern, `value` itself is used. The extracted value is then converted
/// to `cfg.value_type` (see [`cast_derived_value`]), which is what allows a
/// pattern-extracted string to compare against a numeric or boolean field.
/// Returns `Ok(None)` (meaning: skip this item) if `value` is not a scalar,
/// the pattern does not match, the requested conversion fails, or the
/// resulting string is empty or contains `/` (which would make it unusable
/// as a path segment). Returns `Err` only if `cfg.pattern` fails to
/// compile.
fn derive_scalar_value(value: &Value, cfg: &DeriveConfig) -> Result<Option<Value>> {
    let extracted = if let Some(pattern) = cfg.pattern.as_ref() {
        let s = match value {
            Value::String(v) => v.clone(),
            Value::Number(v) => v.to_string(),
            Value::Bool(v) => v.to_string(),
            _ => return Ok(None),
        };
        let re = crate::compile_regex(pattern)
            .map_err(|e| Error::Config(format!("invalid $derive.pattern '{}': {}", pattern, e)))?;
        if let Some(caps) = re.captures(&s) {
            if let Some(group1) = caps.get(1) {
                Value::String(group1.as_str().to_string())
            } else if let Some(full) = caps.get(0) {
                Value::String(full.as_str().to_string())
            } else {
                return Ok(None);
            }
        } else {
            return Ok(None);
        }
    } else {
        value.clone()
    };

    if !matches!(
        extracted,
        Value::String(_) | Value::Number(_) | Value::Bool(_)
    ) {
        return Ok(None);
    }
    let Some(extracted) = cast_derived_value(extracted, cfg.value_type.as_ref()) else {
        return Ok(None);
    };
    if let Value::String(ref s) = extracted
        && (s.is_empty() || s.contains('/'))
    {
        return Ok(None);
    }
    Ok(Some(extracted))
}

/// Converts a derived scalar to the type requested by `$derive.type`.
///
/// The conversion always runs on the value's [`scalar_to_string`] form, so a
/// string extracted by a `$derive.pattern` and a raw field value are treated
/// identically. Returns `None` when the value cannot be represented in the
/// requested type (a non-numeric string for `int`), which makes the caller
/// skip that item and count it as non-derivable. `value_type` of `None` keeps
/// the value unchanged.
///
/// The converted value is what gets deduplicated, rendered as a path
/// segment, and substituted into `$filter` conditions, so `"$type": "int"`
/// on the value `"007"` yields both the endpoint `/7` and the numeric
/// comparison value `7`.
fn cast_derived_value(value: Value, value_type: Option<&DeriveType>) -> Option<Value> {
    let Some(value_type) = value_type else {
        return Some(value);
    };
    let text = scalar_to_string(&value);
    match value_type {
        DeriveType::String => Some(Value::String(text)),
        DeriveType::Int => text.parse::<i64>().ok().map(|n| Value::Number(n.into())),
    }
}

/// Builds a type-tagged string key for `value` (e.g. `"s:foo"`, `"n:1"`)
/// suitable for use in a `BTreeMap` to deduplicate scalars while keeping
/// values of different kinds (a string `"1"` vs. the number `1`) distinct.
fn scalar_deterministic_key(value: &Value) -> String {
    match value {
        Value::String(s) => format!("s:{}", s),
        Value::Number(n) => format!("n:{}", n),
        Value::Bool(b) => format!("b:{}", b),
        _ => format!("x:{}", value),
    }
}

/// If `key` has the `${name}` template sub-path syntax, returns `name`;
/// otherwise returns `None`.
fn template_var_from_key(key: &str) -> Option<&str> {
    if key.starts_with("${") && key.ends_with('}') && key.len() > 3 {
        Some(&key[2..key.len() - 1])
    } else {
        None
    }
}

/// Produces the per-value overlay node used when expanding a `${var}`
/// template sub-path: clones `node`, clears its `$values`/`$derive` (they
/// don't apply to the expanded child), and substitutes `{var}` tokens in
/// its `$filter` conditions with `value` (see [`replace_template_token`]).
fn expand_template_node(node: &ApiNode, var: &str, value: &Value) -> ApiNode {
    let mut out = node.clone();
    out.values = None;
    out.derive = None;
    if let Some(filters) = &node.filter {
        let token = format!("{{{}}}", var);
        out.filter = Some(
            filters
                .iter()
                .map(|cond| FilterCondition {
                    field: cond.field.clone(),
                    op: cond.op.clone(),
                    value: replace_template_token(&cond.value, &token, value),
                })
                .collect(),
        );
    }
    out
}

/// Recursively substitutes occurrences of `token` (e.g. `"{year}"`) inside
/// `input` with `replacement`. A string that equals `token` exactly is
/// replaced with `replacement`'s own value (preserving its JSON type, e.g.
/// a number); a string that merely contains `token` has that substring
/// replaced with `replacement`'s stringified form. Arrays and objects are
/// walked recursively; other value kinds pass through unchanged.
fn replace_template_token(input: &Value, token: &str, replacement: &Value) -> Value {
    match input {
        Value::String(s) => {
            if s == token {
                replacement.clone()
            } else if s.contains(token) {
                Value::String(s.replace(token, &scalar_to_string(replacement)))
            } else {
                Value::String(s.clone())
            }
        }
        Value::Array(arr) => Value::Array(
            arr.iter()
                .map(|v| replace_template_token(v, token, replacement))
                .collect(),
        ),
        Value::Object(map) => {
            let mut out = Map::new();
            for (k, v) in map {
                out.insert(k.clone(), replace_template_token(v, token, replacement));
            }
            Value::Object(out)
        }
        other => other.clone(),
    }
}

/// Converts a resolved template `value` into a URL path segment, rejecting
/// values that stringify to an empty string or contain `/`.
fn scalar_to_path_segment(value: &Value) -> Result<String> {
    let segment = scalar_to_string(value);
    if segment.is_empty() || segment.contains('/') {
        return Err(Error::Config(format!(
            "template value '{}' cannot be used as a path segment",
            segment
        )));
    }
    Ok(segment)
}

/// Stringifies a scalar JSON value without surrounding quotes (strings are
/// returned as-is; numbers and bools use their `Display` form). Non-scalar
/// values fall back to their JSON representation.
fn scalar_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => value.to_string(),
    }
}

/// Serializes `data` with `context` and writes it to the output path for
/// endpoint `name` (per [`SerializerContext::full_path`]), creating any
/// necessary parent directories first.
fn write_data2(name: &str, data: &Value, context: &SerializerContext) -> Result<()> {
    let is_coll = data.is_array();
    let full_path = context.full_path(name, is_coll);

    fs::create_dir_all(full_path.parent().unwrap()).map_err(Error::Io)?;
    fs::write(full_path, context.serialize(data)?).map_err(Error::Io)?;
    Ok(())
}

/// Writes a discovery index (`{ "endpoints": [...] }`) listing every
/// generated endpoint path, once per configured serializer, as
/// `index.[ext]` directly under each serializer's `dest` directory.
fn generate_discovery(config: &Config, endpoints: &[String]) -> Result<()> {
    let discovery = json!({ "endpoints": endpoints });
    for s_conf in &config.serializers {
        let s = get_serializer(&s_conf.serializer, s_conf.minify)?;
        let path = s_conf.dest.join(format!("index.{}", s.extension()));
        fs::create_dir_all(path.parent().unwrap()).map_err(Error::Io)?;
        fs::write(path, s.serialize(&discovery)?).map_err(Error::Io)?;
    }
    Ok(())
}

/// Looks up the [`Serializer`] implementation matching the given name, used
/// by [`generate_discovery`] to serialize the discovery index with each
/// configured serializer.
///
/// Returns [`Error::UnknownSerializer`] for any unrecognized name.
fn get_serializer(s: &str, minify: bool) -> Result<Box<dyn Serializer>> {
    match s {
        "typescript" | "javascript" | "ts" | "js" => Ok(Box::new(TypescriptSerializer { minify })),
        "sqlite" | "sql" => Ok(Box::new(SqliteSerializer)),
        "json" => Ok(Box::new(JSONSerializer { minify })),
        _ => Err(Error::UnknownSerializer(s.into())),
    }
}

/// Strategy for mapping an endpoint name to a physical output file path,
/// selected by the [`crate::Layout`] configuration value.
pub(crate) trait LayoutTrait {
    /// Computes the output path for `endpoint`, given the serializer's file
    /// extension and whether the data being written is a collection
    /// (array).
    fn determine_path(&self, endpoint: &str, file_ext: &str, is_coll: bool) -> PathBuf;
}

/// Layout that places files in `index.[ext]`
pub(crate) struct IndexLayout;
impl LayoutTrait for IndexLayout {
    /// Always writes to `{endpoint}/index.{ext}`.
    fn determine_path(&self, endpoint: &str, ext: &str, _: bool) -> PathBuf {
        Path::new(endpoint).join(format!("index.{}", ext))
    }
}

/// Layout that appends the extension directly
pub(crate) struct ExtensionLayout;
impl LayoutTrait for ExtensionLayout {
    /// Always writes to `{endpoint}.{ext}`.
    fn determine_path(&self, endpoint: &str, ext: &str, _: bool) -> PathBuf {
        PathBuf::from(format!("{}.{}", endpoint, ext))
    }
}

/// Layout that avoids extensions where possible (supports smart fallback)
pub(crate) struct FileLayout;
impl FileLayout {
    /// Returns `true` when `endpoint` needs the `index.[ext]` fallback:
    /// it's a non-root collection, which would otherwise collide with its
    /// own sub-path directory.
    fn is_coll_path(&self, endpoint: &str, is_coll: bool) -> bool {
        is_coll && !endpoint.is_empty()
    }
}
impl LayoutTrait for FileLayout {
    /// Writes non-root collections to `{endpoint}/index.{ext}` (the smart
    /// fallback, avoiding a file/directory name collision with sub-paths)
    /// and everything else to the extensionless `{endpoint}`.
    fn determine_path(&self, endpoint: &str, ext: &str, is_coll: bool) -> PathBuf {
        if self.is_coll_path(endpoint, is_coll) {
            Path::new(endpoint).join(format!("index.{}", ext))
        } else {
            PathBuf::from(endpoint)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::FilterOp;

    /// An unevaluable condition must abort rather than count as a match.
    ///
    /// [`Config`] rejects unusable regex patterns when it loads, so this
    /// exercises the second layer on its own: a `FilterCondition` built
    /// directly, bypassing validation, still must not fail open.
    #[test]
    fn test_unevaluable_condition_errors_instead_of_matching() {
        let broken = FilterCondition {
            field: "name".to_string(),
            op: FilterOp::RegEq,
            value: json!("([unclosed"),
        };
        let item = json!({"name": "alpha"});
        assert!(matches_all_conditions(&item, &[broken]).is_err());
    }

    /// A non-string `value` for a regex operator is the same class of
    /// problem and must not be swallowed either.
    #[test]
    fn test_non_string_regex_value_errors_instead_of_matching() {
        let broken = FilterCondition {
            field: "name".to_string(),
            op: FilterOp::RegNeq,
            value: json!(42),
        };
        let item = json!({"name": "alpha"});
        assert!(matches_all_conditions(&item, &[broken]).is_err());
    }

    /// Failing closed must not swallow working conditions: evaluable filters
    /// keep reporting matches and non-matches as before.
    #[test]
    fn test_evaluable_conditions_are_unaffected() {
        let matching = FilterCondition {
            field: "name".to_string(),
            op: FilterOp::RegEq,
            value: json!("^al"),
        };
        let item = json!({"name": "alpha"});
        assert!(matches_all_conditions(&item, std::slice::from_ref(&matching)).unwrap());

        let other = json!({"name": "beta"});
        assert!(!matches_all_conditions(&other, &[matching]).unwrap());
    }
}
