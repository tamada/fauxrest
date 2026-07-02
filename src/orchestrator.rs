//! Compilation orchestrator module
//!
//! Orchestrates the multi-serializer execution loop based on configuration,
//! reading raw JSON and generating static files according to specified layouts.

use regex::Regex;
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use crate::Result;
use crate::config::{
    AggregateMode, AggregateSpec, ApiNode, DeriveConfig, EmitTarget, FilterCondition,
};
use crate::context::SerializerContext;
use crate::{Config, Error, JSONSerializer, Serializer, SqliteSerializer, TypescriptSerializer};

const DERIVE_CARDINALITY_WARN_THRESHOLD: usize = 1000;

/// Executes the API build process
pub fn run<P: AsRef<Path>>(config: Config, data_dir: P) -> Result<()> {
    let mut endpoints = Vec::new();
    let dataset = DataSource::new(data_dir)?;
    for s_conf in &config.serializers {
        let context: SerializerContext = s_conf.try_into()?;
        endpoints.extend(run_serializer(context, &dataset, &config.api)?);
    }
    generate_discovery(&config, &endpoints)?;
    Ok(())
}

struct DataSource {
    entries: HashMap<String, Value>,
}

impl DataSource {
    pub fn new<P: AsRef<Path>>(dir: P) -> Result<Self> {
        collect_datasets(dir.as_ref()).map(|entries| Self { entries })
    }

    pub fn names(&self) -> Vec<String> {
        let mut names = self.entries.keys().cloned().collect::<Vec<_>>();
        names.sort();
        names
    }

    pub fn overlay_names<'a>(&self, api_overlay: &'a HashMap<String, ApiNode>) -> Vec<&'a String> {
        let mut result = api_overlay
            .keys()
            .filter(|&k| !self.entries.contains_key(k))
            .collect::<Vec<_>>();
        result.sort();
        result
    }

    pub fn get(&self, name: &str) -> Result<&Value> {
        self.entries
            .get(name)
            .ok_or_else(|| Error::Config(format!("missing dataset for endpoint '{}'", name)))
    }
}

struct Target<'a> {
    endpoint: &'a str,
    data: &'a Value,
    node: Option<&'a ApiNode>,
}

impl<'a> Target<'a> {
    pub fn new(endpoint: &'a str, data: &'a Value, node: Option<&'a ApiNode>) -> Self {
        Target {
            endpoint,
            data,
            node,
        }
    }

    pub fn build(
        endpoint: &'a str,
        source: &'a DataSource,
        api_overlay: &'a HashMap<String, ApiNode>,
    ) -> Result<Self> {
        let data = source.get(endpoint)?;
        let node = api_overlay.get(endpoint);
        Ok(Target::new(endpoint, data, node))
    }

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

    pub fn map_node<F, R>(&'a self, mapper: F) -> Option<&'a R>
    where
        F: FnOnce(&'a ApiNode) -> Option<&'a R>,
    {
        self.node.and_then(mapper)
    }

    pub fn build_endpoint_data(
        &self,
        filters: &Option<&Vec<FilterCondition>>,
        sources: &DataSource,
    ) -> Result<(Value, Value)> {
        let mut endpoint_base = self.data.clone();
        if let Some(agg) = self.map_node(|n| n.aggregate.as_ref()) {
            endpoint_base = aggregate_values2(agg, sources)?;
        }

        let endpoint_data = if let Some(filters) = filters {
            apply_filters(&endpoint_base, filters)?
        } else {
            endpoint_base.clone()
        };

        let result = if let Some(n) = self.node {
            apply_pick_omit(endpoint_data, n)
        } else {
            endpoint_data
        };
        Ok((endpoint_base, result))
    }

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

fn materialize_node(
    target: &Target,
    filter: Option<&Vec<FilterCondition>>,
    sources: &DataSource,
    context: &SerializerContext,
) -> Result<Vec<String>> {
    let effective_filter = target.map_node(|n| n.filter.as_ref()).or(filter);

    let (base, endpoint_data) = target.build_endpoint_data(&effective_filter, sources)?;
    // let is_private = *target.map_node(|n| n.private.as_ref()).unwrap_or(&false);
    // if is_private {
    //     return Ok(vec![]);
    // }
    let mut endpoints = Vec::new();

    let (emit_list, emit_id) = resolve_emit_flags(target.node);
    if emit_list {
        write_data2(target.endpoint, &endpoint_data, context)?;
        endpoints.push(format!("/{}", target.endpoint));
    }
    if emit_id {
        endpoints.extend(target.emmit_ids(&endpoint_data, context)?);
    }

    for (key, child) in target.subpaths() {
        if let Some(var) = template_var_from_key(key) {
            let values = resolve_template_values(target.endpoint, key, child, &base)?;
            for value in &values {
                let segment = scalar_to_path_segment(value)?;
                let child_endpoint = format!("{}/{}", target.endpoint, segment);
                let expanded_child = expand_template_node(child, var, value);
                let child = Target {
                    endpoint: &child_endpoint,
                    data: &base,
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
                data: &base,
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

fn matches_all_conditions(item: &Value, filters: &[FilterCondition]) -> Result<bool> {
    for cond in filters {
        if let Ok(false) = cond.apply(item) {
            return Ok(false);
        }
    }
    Ok(true)
}

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

fn apply_pick_map(mut obj: Map<String, Value>, pick: &[String]) -> Map<String, Value> {
    obj.retain(|k, _| pick.contains(k));
    obj
}

fn apply_omit_map(mut obj: Map<String, Value>, omit: &[String]) -> Map<String, Value> {
    for key in omit {
        obj.remove(key);
    }
    obj
}

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

fn derive_values_from_data(
    source_data: &Value,
    cfg: &DeriveConfig,
    context: &str,
) -> Result<Vec<Value>> {
    let mut unique: BTreeMap<String, Value> = BTreeMap::new();
    let mut skipped = 0usize;

    match source_data {
        Value::Array(arr) => {
            for item in arr {
                if let Some(v) = item.get(&cfg.field) {
                    if let Some(extracted) = derive_scalar_value(v, cfg)? {
                        let key = scalar_deterministic_key(&extracted);
                        unique.entry(key).or_insert(extracted);
                    } else {
                        skipped += 1;
                    }
                }
            }
        }
        Value::Object(obj) => {
            if let Some(v) = obj.get(&cfg.field) {
                if let Some(extracted) = derive_scalar_value(v, cfg)? {
                    let key = scalar_deterministic_key(&extracted);
                    unique.entry(key).or_insert(extracted);
                } else {
                    skipped += 1;
                }
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

fn derive_scalar_value(value: &Value, cfg: &DeriveConfig) -> Result<Option<Value>> {
    let extracted = if let Some(pattern) = cfg.pattern.as_ref() {
        let s = match value {
            Value::String(v) => v.clone(),
            Value::Number(v) => v.to_string(),
            Value::Bool(v) => v.to_string(),
            _ => return Ok(None),
        };
        let re = Regex::new(pattern)
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
    if let Value::String(ref s) = extracted
        && (s.is_empty() || s.contains('/'))
    {
        return Ok(None);
    }
    Ok(Some(extracted))
}

fn scalar_deterministic_key(value: &Value) -> String {
    match value {
        Value::String(s) => format!("s:{}", s),
        Value::Number(n) => format!("n:{}", n),
        Value::Bool(b) => format!("b:{}", b),
        _ => format!("x:{}", value),
    }
}

fn template_var_from_key(key: &str) -> Option<&str> {
    if key.starts_with("${") && key.ends_with('}') && key.len() > 3 {
        Some(&key[2..key.len() - 1])
    } else {
        None
    }
}

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

fn scalar_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => value.to_string(),
    }
}

fn write_data2(name: &str, data: &Value, context: &SerializerContext) -> Result<()> {
    let is_coll = data.is_array();
    let full_path = context.full_path(name, is_coll);

    fs::create_dir_all(full_path.parent().unwrap()).map_err(Error::Io)?;
    fs::write(full_path, context.serialize(data)?).map_err(Error::Io)?;
    Ok(())
}

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

fn get_serializer(s: &str, minify: bool) -> Result<Box<dyn Serializer>> {
    match s {
        "typescript" | "javascript" | "ts" | "js" => Ok(Box::new(TypescriptSerializer { minify })),
        "sqlite" | "sql" => Ok(Box::new(SqliteSerializer)),
        "json" => Ok(Box::new(JSONSerializer { minify })),
        _ => Err(Error::UnknownSerializer(s.into())),
    }
}

/// Trait for physical data serialization
pub(crate) trait LayoutTrait {
    fn determine_path(&self, endpoint: &str, file_ext: &str, is_coll: bool) -> PathBuf;
}

/// Layout that places files in `index.[ext]`
pub(crate) struct IndexLayout;
impl LayoutTrait for IndexLayout {
    fn determine_path(&self, endpoint: &str, ext: &str, _: bool) -> PathBuf {
        Path::new(endpoint).join(format!("index.{}", ext))
    }
}

/// Layout that appends the extension directly
pub(crate) struct ExtensionLayout;
impl LayoutTrait for ExtensionLayout {
    fn determine_path(&self, endpoint: &str, ext: &str, _: bool) -> PathBuf {
        PathBuf::from(format!("{}.{}", endpoint, ext))
    }
}

/// Layout that avoids extensions where possible (supports smart fallback)
pub(crate) struct FileLayout;
impl FileLayout {
    fn is_coll_path(&self, endpoint: &str, is_coll: bool) -> bool {
        is_coll && !endpoint.is_empty()
    }
}
impl LayoutTrait for FileLayout {
    fn determine_path(&self, endpoint: &str, ext: &str, is_coll: bool) -> PathBuf {
        if self.is_coll_path(endpoint, is_coll) {
            Path::new(endpoint).join(format!("index.{}", ext))
        } else {
            PathBuf::from(endpoint)
        }
    }
}
