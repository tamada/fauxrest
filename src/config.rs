//! Configuration model for `fauxrest`.
//!
//! This module defines the `_config.json` schema: the top-level [`Config`]
//! (which lists output [`SerializerConfig`]s and an optional routing
//! overlay), the [`ApiNode`](crate::config::ApiNode) overlay tree that
//! drives `$filter`, `$aggregate`, `$pick`, `$omit`, `$emit`, `$values`,
//! and `$derive` directives, and the validation logic that rejects
//! malformed overlays before the orchestrator runs.

pub use crate::filter::{FilterCondition, FilterOp};
use crate::{Error, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::{fs, io};

/// Selects which physical outputs are produced for a collection endpoint via
/// the `$emit` directive.
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EmitTarget {
    /// Emit the collection itself (e.g. `/endpoint`).
    List,
    /// Emit one file per item, keyed by its `id` field (e.g. `/endpoint/{id}`).
    Ids,
}

/// The value of a `$aggregate` directive: either a plain list of source
/// paths (flat mode) or a full [`AggregateConfig`] with an explicit mode and
/// per-source key mapping.
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum AggregateSpec {
    /// Shorthand form: `"$aggregate": ["a", "b"]`, equivalent to flat mode
    /// with each entry as an unkeyed source.
    Paths(Vec<String>),
    /// Full form: `"$aggregate": { "mode": ..., "sources": [...] }`.
    Config(AggregateConfig),
}

/// Full form of an `$aggregate` directive, specifying the merge [`AggregateMode`]
/// and the list of [`AggregateSource`]s to combine.
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct AggregateConfig {
    /// How the sources are combined (defaults to [`AggregateMode::Flat`]).
    #[serde(default)]
    pub mode: AggregateMode,
    /// The datasets/endpoints to combine, in order.
    pub sources: Vec<AggregateSource>,
}

/// How multiple datasets are combined by an `$aggregate` directive.
#[derive(Deserialize, Debug, Clone, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum AggregateMode {
    /// Concatenate all source arrays (or push scalar/object sources as a
    /// single element) into one flat array.
    #[default]
    Flat,
    /// Merge sources into a single object keyed by source name (or an
    /// explicit `as` alias); duplicate keys are a configuration error.
    Keyed,
}

/// A single entry of an `$aggregate.sources` list: either a bare path string
/// or a [`AggregateSourceMapping`] that renames the key used in keyed mode.
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum AggregateSource {
    /// A bare source path, e.g. `"job-histories"`.
    Path(String),
    /// A source path with an explicit `as` key alias.
    Mapping(AggregateSourceMapping),
}

/// Maps a source dataset/endpoint path to an alternate key name, used in
/// [`AggregateMode::Keyed`] aggregation (`{ "from": "...", "as": "..." }`).
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct AggregateSourceMapping {
    /// The source dataset or endpoint path to read from.
    pub from: String,
    /// Optional alias used as the output key instead of `from` (serialized
    /// as `"as"` in JSON).
    #[serde(rename = "as")]
    pub as_key: Option<String>,
}

/// A normalized `$aggregate` source entry, produced by [`AggregateSpec::entries`],
/// combining a source path with its resolved output key (if any).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregateEntry {
    /// The source dataset or endpoint path to read from.
    pub from: String,
    /// The resolved output key to use in keyed mode, if explicitly set.
    pub key: Option<String>,
}

impl AggregateSpec {
    /// Returns the effective [`AggregateMode`] for this spec.
    ///
    /// The shorthand [`AggregateSpec::Paths`] form is always flat mode; the
    /// full [`AggregateSpec::Config`] form uses its configured `mode`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fauxrest::config::{AggregateMode, AggregateSpec};
    ///
    /// let spec = AggregateSpec::Paths(vec!["a".to_string(), "b".to_string()]);
    /// assert_eq!(spec.mode(), AggregateMode::Flat);
    /// ```
    pub fn mode(&self) -> AggregateMode {
        match self {
            AggregateSpec::Paths(_) => AggregateMode::Flat,
            AggregateSpec::Config(cfg) => cfg.mode.clone(),
        }
    }

    /// Normalizes this spec into a flat list of [`AggregateEntry`] values,
    /// resolving `Path` and `Mapping` sources (and the shorthand `Paths`
    /// form) into a uniform `(from, key)` representation.
    ///
    /// # Examples
    ///
    /// ```
    /// use fauxrest::config::AggregateSpec;
    ///
    /// let spec = AggregateSpec::Paths(vec!["skills".to_string()]);
    /// let entries = spec.entries();
    /// assert_eq!(entries.len(), 1);
    /// assert_eq!(entries[0].from, "skills");
    /// assert_eq!(entries[0].key, None);
    /// ```
    pub fn entries(&self) -> Vec<AggregateEntry> {
        match self {
            AggregateSpec::Paths(paths) => paths
                .iter()
                .map(|p| AggregateEntry {
                    from: p.clone(),
                    key: None,
                })
                .collect(),
            AggregateSpec::Config(cfg) => cfg
                .sources
                .iter()
                .map(|s| match s {
                    AggregateSource::Path(p) => AggregateEntry {
                        from: p.clone(),
                        key: None,
                    },
                    AggregateSource::Mapping(m) => AggregateEntry {
                        from: m.from.clone(),
                        key: m.as_key.clone(),
                    },
                })
                .collect(),
        }
    }
}

/// A single node of the advanced routing overlay tree.
///
/// Each key in [`Config::api`] (and recursively, each key of `sub_paths`)
/// maps to an `ApiNode` that may carry directives (`$filter`, `$aggregate`,
/// `$pick`, `$omit`, `$emit`, `$values`, `$derive`) describing how the
/// corresponding endpoint's data should be transformed, plus nested
/// `sub_paths` for deeper routes.
#[derive(Deserialize, Debug, Clone, Default, PartialEq)]
pub struct ApiNode {
    /// `$filter`: conditions that items must satisfy to be included.
    #[serde(rename = "$filter")]
    pub filter: Option<Vec<FilterCondition>>,

    /// `$aggregate`: combines one or more other datasets/endpoints into this
    /// endpoint's data.
    #[serde(rename = "$aggregate")]
    pub aggregate: Option<AggregateSpec>,

    /// `$pick`: restricts object fields to only those listed.
    #[serde(rename = "$pick")]
    pub pick: Option<Vec<String>>,

    /// `$omit`: removes the listed object fields.
    #[serde(rename = "$omit")]
    pub omit: Option<Vec<String>>,

    /// `$emit`: selects which physical outputs (list and/or per-id files)
    /// are produced for this node. `None` means "emit everything" (the
    /// default); an empty list means "emit nothing".
    #[serde(rename = "$emit")]
    pub emit: Option<Vec<EmitTarget>>,

    /// `$values`: explicit list of scalar values used to expand a
    /// `${name}` template sub-path.
    #[serde(rename = "$values")]
    pub values: Option<Vec<Value>>,

    /// `$derive`: derives the list of values used to expand a `${name}`
    /// template sub-path from a field of the parent dataset.
    #[serde(rename = "$derive")]
    pub derive: Option<DeriveSource>,

    /// All other (non-`$`-prefixed) keys, forming the nested routing tree.
    #[serde(flatten)]
    pub sub_paths: HashMap<String, ApiNode>,
    // #[serde(rename = "$private")]
    // pub private: Option<bool>,
    // #[serde(rename = "$emit_list")]
    // pub emit_list: Option<bool>,

    // #[serde(rename = "$emit_id")]
    // pub emit_id: Option<bool>,

    // // Backward-compatible alias of $emit_id.
    // #[serde(rename = "$emit_items")]
    // pub emit_items: Option<bool>,
}

/// The value of a `$derive` directive: either a bare field-name shorthand or
/// a full [`DeriveConfig`] with an extraction pattern.
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum DeriveSource {
    /// Shorthand form: `"$derive": "field"`, deriving template values
    /// directly from the named field's raw value.
    Field(String),
    /// Full form: `"$derive": { "field": ..., "pattern": ... }`.
    Config(DeriveConfig),
}

/// Full form of a `$derive` directive: the source `field` to read from each
/// item, an optional regex `pattern` whose first capture group (or, absent a
/// group, the whole match) is used as the derived value, and an optional
/// `type` that converts the extracted value to another JSON scalar kind.
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct DeriveConfig {
    /// Name of the field to read from each item in the parent dataset.
    pub field: String,
    /// Optional regular expression applied to the field's stringified value
    /// to extract the derived scalar.
    pub pattern: Option<String>,
    /// Optional target type for the derived value (serialized as `"type"`).
    ///
    /// A `pattern` always extracts a string, so without this the derived
    /// value can never match a numeric or boolean field under `$filter`.
    /// `None` keeps the extracted value as-is, which is the behavior of
    /// releases before 0.0.4.
    #[serde(default, rename = "type")]
    pub value_type: Option<DeriveType>,
}

/// Target scalar type of a `$derive` directive's `type` field, applied to the
/// derived value before it is deduplicated, turned into a path segment, and
/// substituted into `$filter` conditions.
///
/// The conversion runs on the value's stringified form, so it applies equally
/// to values extracted by a `pattern` and to raw field values.
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DeriveType {
    /// Stringify the value (e.g. the number `2024` becomes `"2024"`).
    String,
    /// Parse the value as a 64-bit integer. Values that do not parse
    /// exactly (including floats such as `2024.5`) are skipped.
    Int,
    /// Parse the value as a floating point number.
    Float,
    /// Parse the literals `true` and `false`; anything else is skipped.
    Bool,
    /// Infer the type conservatively: `true`/`false` become booleans, and a
    /// string becomes an integer only when the conversion round-trips
    /// losslessly (so `"007"` and `"+7"` stay strings). Floats are never
    /// inferred — request [`DeriveType::Float`] explicitly for those.
    Auto,
}

/// Declares which non-JSON static files (images, CSS, …) found in the input
/// data directory should be copied verbatim into each serializer destination.
///
/// Two shapes are accepted from configuration under the top-level `$static` key:
///
/// - Shorthand (include only): `"$static": ["*.png", "css/**"]`
/// - Full form: `"$static": {"include": ["..."], "exclude": ["..."]}`
///
/// # Priority
///
/// Copying is **deny by default**: without an `include` glob (or the
/// `--copy-static` command line flag) nothing is copied. `exclude` (deny)
/// always wins: a file that matches an `exclude` glob is never copied, even
/// when copying is forced on for every file via `--copy-static`.
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum StaticSpec {
    /// Shorthand form: a list of include (allow) globs.
    Include(Vec<String>),
    /// Full form with explicit `include` (allow) and `exclude` (deny) globs.
    Detailed(StaticConfig),
}

/// Full static-copy configuration with explicit include/exclude glob lists.
#[derive(Deserialize, Debug, Clone, PartialEq, Default)]
pub struct StaticConfig {
    /// Glob patterns that allow a static file to be copied.
    #[serde(default)]
    pub include: Vec<String>,
    /// Glob patterns that deny a static file from being copied. `exclude`
    /// always wins over any allow decision, including `--copy-static`.
    #[serde(default)]
    pub exclude: Vec<String>,
}

impl StaticSpec {
    /// Returns the configured include (allow) glob patterns.
    ///
    /// # Examples
    ///
    /// ```
    /// use fauxrest::StaticSpec;
    ///
    /// let spec = StaticSpec::Include(vec!["*.png".to_string()]);
    /// assert_eq!(spec.include(), ["*.png".to_string()]);
    /// ```
    pub fn include(&self) -> &[String] {
        match self {
            StaticSpec::Include(patterns) => patterns,
            StaticSpec::Detailed(cfg) => &cfg.include,
        }
    }

    /// Returns the configured exclude (deny) glob patterns.
    ///
    /// The shorthand [`StaticSpec::Include`] form has no deny globs, so it
    /// always returns an empty slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use fauxrest::{StaticConfig, StaticSpec};
    ///
    /// let spec = StaticSpec::Detailed(StaticConfig {
    ///     include: vec!["**/*.css".to_string()],
    ///     exclude: vec!["secret/**".to_string()],
    /// });
    /// assert_eq!(spec.exclude(), ["secret/**".to_string()]);
    /// ```
    pub fn exclude(&self) -> &[String] {
        match self {
            StaticSpec::Include(_) => &[],
            StaticSpec::Detailed(cfg) => &cfg.exclude,
        }
    }
}

/// Represents the relationship between endpoint URL resolution and
/// physical file placement.
#[derive(Deserialize, Serialize, Debug, Clone, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Layout {
    /// Outputs endpoints as `/endpoint/index.[ext]`. Highly compatible with all static web servers, maintaining clean URLs.
    Index,
    /// Outputs endpoints as extensionless files (`/endpoint`).
    /// **Smart Fallback Specification**: To avoid physical file-directory collisions,
    /// collections that contain sub-paths are automatically replaced (fallback) by `.../index.[ext]` files during compilation.
    File,
    /// Outputs endpoints with explicit extensions (`/endpoint.[ext]`). 100% web server compatible.
    Extension,
}

/// Serializer configuration
#[derive(Deserialize)]
pub struct SerializerConfig {
    /// Serializer type (json, typescript, sqlite)
    pub serializer: String,
    /// Delivery layout (index, file, extension)
    pub layout: Layout,
    /// Destination directory
    pub dest: PathBuf,
    /// Whether output should be compact (minified)
    #[serde(default)]
    pub minify: bool,
    /// Whether existing files in `dest` may be overwritten.
    /// When false (the default) and `dest` already contains files,
    /// the build aborts with an error.
    #[serde(default)]
    pub overwrite: bool,
}

/// Global configuration
#[derive(Deserialize)]
pub struct Config {
    /// List of serializer configurations
    #[serde(default, rename = "$config")]
    pub serializers: Vec<SerializerConfig>,

    /// Static-file copy policy (allow/deny globs). `None` means the feature is
    /// unconfigured; combined with the default deny policy nothing is copied
    /// unless `copy_static_all` is set from the command line.
    #[serde(default, rename = "$static")]
    pub static_files: Option<StaticSpec>,

    /// When `true`, every static file is treated as allowed regardless of the
    /// `include` globs. Set from the `--copy-static` command line flag and never
    /// deserialized from the configuration file. `exclude` globs still win.
    #[serde(skip)]
    pub copy_static_all: bool,

    /// Advanced routing overlay
    #[serde(flatten)]
    pub api: HashMap<String, ApiNode>,
}

impl Default for Config {
    /// Returns the fallback configuration used when no `_config.json` is
    /// found: a single JSON serializer with [`Layout::Index`] writing to
    /// `dist/`, and no routing overlay.
    fn default() -> Self {
        Self {
            serializers: vec![SerializerConfig {
                serializer: "json".into(),
                layout: Layout::Index,
                dest: "dist".into(),
                minify: false,
                overwrite: false,
            }],
            static_files: None,
            copy_static_all: false,
            api: HashMap::new(),
        }
    }
}

impl Config {
    /// Builds a `Config` with a single serializer entry and an empty routing
    /// overlay, without needing a `_config.json` file.
    ///
    /// # Examples
    ///
    /// ```
    /// use fauxrest::{Config, Layout};
    ///
    /// let config = Config::new("json".to_string(), Layout::Index, "dist");
    /// assert_eq!(config.serializers.len(), 1);
    /// assert_eq!(config.serializers[0].serializer, "json");
    /// ```
    pub fn new<P: AsRef<Path>>(serializer: String, layout: Layout, dest: P) -> Self {
        let dest = dest.as_ref().to_path_buf();
        Self {
            serializers: vec![SerializerConfig {
                serializer,
                layout,
                dest,
                minify: false,
                overwrite: false,
            }],
            static_files: None,
            copy_static_all: false,
            api: HashMap::new(),
        }
    }

    /// Loads configuration from a given string.
    ///
    /// # Examples
    ///
    /// ```
    /// use fauxrest::Config;
    ///
    /// let json = r#"{
    ///     "$config": [{"serializer": "json", "layout": "index", "dest": "dist"}]
    /// }"#;
    /// let config = Config::load_from_str(json).expect("valid config");
    /// assert_eq!(config.serializers.len(), 1);
    /// ```
    pub fn load_from_str<S: AsRef<str>>(s: S) -> Result<Self> {
        let content = s.as_ref();
        let config: Self = serde_json::from_str(content).map_err(Error::SerdeJson)?;
        config.validate()?;
        Ok(config)
    }

    /// Loads configuration by reading and parsing the full contents of a
    /// [`std::io::Read`] implementor (e.g. a file handle or in-memory buffer).
    ///
    /// # Examples
    ///
    /// ```
    /// use fauxrest::Config;
    /// use std::io::Cursor;
    ///
    /// let json = r#"{
    ///     "$config": [{"serializer": "json", "layout": "index", "dest": "dist"}]
    /// }"#;
    /// let mut reader = Cursor::new(json);
    /// let config = Config::load_from_reader(&mut reader).expect("valid config");
    /// assert_eq!(config.serializers.len(), 1);
    /// ```
    pub fn load_from_reader(reader: &mut impl std::io::Read) -> Result<Self> {
        let mut reader = std::io::BufReader::new(reader);
        let content = io::read_to_string(&mut reader)?;
        Self::load_from_str(content)
    }

    /// Loads configuration from a specific file path
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use fauxrest::Config;
    ///
    /// let config = Config::load_from_file("_config.json").expect("valid config file");
    /// assert!(!config.serializers.is_empty());
    /// ```
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path).map_err(Error::Io)?;
        Self::load_from_str(content)
    }
}

impl Config {
    /// Recursively validates every node of the routing overlay, checking
    /// `$aggregate` well-formedness and the `${name}` template sub-path
    /// rules (see [`validate_node`]).
    fn validate(&self) -> Result<()> {
        if let Some(spec) = self.static_files.as_ref() {
            validate_static(spec)?;
        }
        let mut keys = self.api.keys().cloned().collect::<Vec<_>>();
        keys.sort();
        for key in keys {
            if let Some(node) = self.api.get(&key) {
                validate_node(&key, node)?;
            }
        }
        Ok(())
    }
}

/// Validates every `$static` glob pattern, reporting malformed patterns as a
/// [`Error::Config`] with the offending pattern and the underlying reason.
fn validate_static(spec: &StaticSpec) -> Result<()> {
    for pattern in spec.include().iter().chain(spec.exclude().iter()) {
        globset::Glob::new(pattern).map_err(|e| {
            Error::Config(format!("invalid $static glob pattern '{}': {}", pattern, e))
        })?;
    }
    Ok(())
}

/// Validates a single overlay node and recurses into its `sub_paths`.
///
/// Checks that any `$aggregate` directive is well-formed (via
/// [`validate_aggregate`]), and that `${name}` template sub-path keys have
/// exactly one of `$values`/`$derive` set (and that non-template keys have
/// neither).
fn validate_node(path: &str, node: &ApiNode) -> Result<()> {
    if let Some(aggregate) = node.aggregate.as_ref() {
        validate_aggregate(path, aggregate)?;
    }

    let mut keys = node.sub_paths.keys().cloned().collect::<Vec<_>>();
    keys.sort();
    for key in keys {
        let child = node
            .sub_paths
            .get(&key)
            .ok_or_else(|| Error::Config(format!("{}: missing child node {}", path, key)))?;
        let child_path = format!("{}/{}", path, key);
        if template_var_from_key(&key).is_some() {
            if child.values.is_some() && child.derive.is_some() {
                return Err(Error::Config(format!(
                    "{}: $values and $derive cannot be used together",
                    child_path
                )));
            }
            if child.values.is_none() && child.derive.is_none() {
                return Err(Error::Config(format!(
                    "{}: template sub-path requires $values or $derive",
                    child_path
                )));
            }

            if let Some(values) = child.values.as_ref() {
                if values.is_empty() {
                    return Err(Error::Config(format!(
                        "{}: $values must not be empty",
                        child_path
                    )));
                }
                for value in values {
                    if !is_scalar(value) {
                        return Err(Error::Config(format!(
                            "{}: $values entries must be scalar (string/number/bool)",
                            child_path
                        )));
                    }
                    if let Value::String(s) = value
                        && s.contains('/')
                    {
                        return Err(Error::Config(format!(
                            "{}: $values string must not contain '/'",
                            child_path
                        )));
                    }
                }
            }

            if let Some(derive) = child.derive.as_ref() {
                validate_derive(&child_path, derive)?;
            }
        } else if child.values.is_some() || child.derive.is_some() {
            return Err(Error::Config(format!(
                "{}: $values/$derive are only allowed for template sub-path keys like ${{name}}",
                child_path
            )));
        }
        validate_node(&child_path, child)?;
    }
    Ok(())
}

/// Validates a `$aggregate` directive: it must have at least one entry, no
/// entry's source path may be blank, and in [`AggregateMode::Keyed`] mode
/// every resolved key must be non-empty and unique.
fn validate_aggregate(path: &str, aggregate: &AggregateSpec) -> Result<()> {
    let entries = aggregate.entries();
    if entries.is_empty() {
        return Err(Error::Config(format!(
            "{}: $aggregate must not be empty",
            path
        )));
    }

    let mode = aggregate.mode();
    let mut keyed_names = BTreeSet::new();
    for entry in entries {
        if entry.from.trim().is_empty() {
            return Err(Error::Config(format!(
                "{}: $aggregate source must not be empty",
                path
            )));
        }

        if mode == AggregateMode::Keyed {
            let key = entry.key.unwrap_or(entry.from);
            if key.trim().is_empty() {
                return Err(Error::Config(format!(
                    "{}: $aggregate keyed source alias must not be empty",
                    path
                )));
            }
            if !keyed_names.insert(key.clone()) {
                return Err(Error::Config(format!(
                    "{}: duplicate keyed aggregate key '{}'",
                    path, key
                )));
            }
        }
    }
    Ok(())
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

/// Returns `true` if `value` is a JSON scalar (string, number, or bool) as
/// opposed to null, an array, or an object.
fn is_scalar(value: &Value) -> bool {
    matches!(value, Value::String(_) | Value::Number(_) | Value::Bool(_))
}

/// Validates a `$derive` directive: the target `field` must be non-empty,
/// and if a `pattern` is given it must be a valid regular expression.
fn validate_derive(path: &str, derive: &DeriveSource) -> Result<()> {
    let cfg = derive.to_config();
    if cfg.field.trim().is_empty() {
        return Err(Error::Config(format!(
            "{}: $derive.field must not be empty",
            path
        )));
    }
    if let Some(pattern) = cfg.pattern.as_ref() {
        Regex::new(pattern).map_err(|e| {
            Error::Config(format!(
                "{}: invalid $derive.pattern '{}': {}",
                path, pattern, e
            ))
        })?;
    }
    Ok(())
}

impl DeriveSource {
    /// Normalizes this `$derive` value into a full [`DeriveConfig`].
    ///
    /// The shorthand [`DeriveSource::Field`] form is expanded to a
    /// `DeriveConfig` with no pattern; the full [`DeriveSource::Config`]
    /// form is returned as-is.
    ///
    /// # Examples
    ///
    /// ```
    /// use fauxrest::config::DeriveSource;
    ///
    /// let derive = DeriveSource::Field("from".to_string());
    /// let cfg = derive.to_config();
    /// assert_eq!(cfg.field, "from");
    /// assert_eq!(cfg.pattern, None);
    /// assert_eq!(cfg.value_type, None);
    /// ```
    pub fn to_config(&self) -> DeriveConfig {
        match self {
            DeriveSource::Field(field) => DeriveConfig {
                field: field.clone(),
                pattern: None,
                value_type: None,
            },
            DeriveSource::Config(c) => c.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Loads `testdata/tamada/_config.json` and checks that `$filter`,
    /// `$derive`, `$aggregate` (keyed), and `$emit` directives across
    /// several overlay nodes are parsed as expected.
    #[test]
    fn test_parse_advanced_routing_config() {
        let config_path = Path::new("testdata/tamada/_config.json");
        let config =
            Config::load_from_file(config_path).expect("Failed to load complex configuration");

        // Verify parsing of job-histories/current/$filter
        let job_hist = config
            .api
            .get("job-histories")
            .expect("Missing job-histories node");
        let current = job_hist
            .sub_paths
            .get("current")
            .expect("Missing current sub-path");
        let filter = current.filter.as_ref().expect("Missing filter array");
        assert_eq!(filter.len(), 1);
        assert_eq!(filter[0].field, "to");
        assert_eq!(filter[0].op, FilterOp::Eq);
        assert_eq!(filter[0].value, Value::String("Present".to_string()));

        // Verify parsing of activities template and $emit
        let activities = config
            .api
            .get("activities")
            .expect("Missing activities node");
        assert_eq!(activities.filter, None);
        assert_eq!(activities.emit, None);
        let by_year = activities
            .sub_paths
            .get("${year}")
            .expect("Missing ${year} sub-path");
        let derive_config = by_year
            .derive
            .as_ref()
            .expect("Missing $derive")
            .to_config();
        assert_eq!(derive_config.field, "from");
        assert_eq!(derive_config.pattern, Some("^(\\d{4}).*".to_string()));

        // Verify parsing of profile/$aggregate
        let profile = config.api.get("profile").expect("Missing profile node");
        assert_eq!(profile.emit, None);
        let agg = profile.aggregate.as_ref().expect("Missing aggregate array");
        assert_eq!(agg.mode(), AggregateMode::Keyed);
        let entries = agg.entries();
        assert_eq!(entries[0].from, "job-histories");
        assert_eq!(entries[1].from, "activities");
        assert_eq!(entries[2].from, "degrees");
        assert_eq!(entries[3].from, "skills");

        // Verify parsing of secret/$private
        let secret = config.api.get("secret").expect("Missing secret node");
        assert_eq!(secret.emit, Some(vec![]));
        // assert_eq!(secret.private, Some(true));

        // Verify optional parsing of $emit_items
        // assert_eq!(profile.emit_items, None);
        // assert_eq!(profile.emit_list, None);
        // assert_eq!(profile.emit_id, None);
    }

    /// Checks that a full `$derive` object (`{ "field": ..., "pattern": ... }`)
    /// on a `${year}` template sub-path is parsed correctly.
    #[test]
    fn test_parse_template_derive_config() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(
            tmp,
            r#"{{
    "$config": [{{"serializer":"json","layout":"index","dest":"dist"}}],
    "activities": {{
        "${{year}}": {{
            "$derive": {{"field":"from", "pattern":"^(\\d{{4}})"}}
        }}
    }}
}}"#
        )
        .unwrap();

        let config =
            Config::load_from_file(tmp.path()).expect("Failed to load derive configuration");
        let activities = config
            .api
            .get("activities")
            .expect("Missing activities node");
        let by_year = activities
            .sub_paths
            .get("${year}")
            .expect("Missing template node");
        let derive = by_year.derive.as_ref().expect("Missing derive").to_config();
        assert_eq!(derive.field, "from");
        assert_eq!(derive.pattern, Some("^(\\d{4})".to_string()));
    }

    /// Checks that a `$derive` object carrying an explicit `"type"` parses
    /// into the matching [`DeriveType`], and that omitting `"type"` leaves
    /// it unset (the pre-0.0.4 behavior).
    #[test]
    fn test_parse_derive_value_type() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(
            tmp,
            r#"{{
    "$config": [{{"serializer":"json","layout":"index","dest":"dist"}}],
    "papers": {{
        "${{year}}": {{
            "$derive": {{"field":"published", "pattern":"^(\\d{{4}})", "type":"int"}}
        }},
        "${{tag}}": {{
            "$derive": {{"field":"tag"}}
        }}
    }}
}}"#
        )
        .unwrap();

        let config = Config::load_from_file(tmp.path()).expect("Failed to load derive type config");
        let papers = config.api.get("papers").expect("Missing papers node");
        let by_year = papers
            .sub_paths
            .get("${year}")
            .expect("Missing ${year} sub-path");
        let derive = by_year.derive.as_ref().expect("Missing derive").to_config();
        assert_eq!(derive.field, "published");
        assert_eq!(derive.value_type, Some(DeriveType::Int));

        let by_tag = papers
            .sub_paths
            .get("${tag}")
            .expect("Missing ${tag} sub-path");
        let derive = by_tag.derive.as_ref().expect("Missing derive").to_config();
        assert_eq!(derive.value_type, None);
    }

    /// Checks that an unknown `$derive.type` name is rejected at load time
    /// rather than silently ignored.
    #[test]
    fn test_unknown_derive_value_type_is_rejected() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(
            tmp,
            r#"{{
    "$config": [{{"serializer":"json","layout":"index","dest":"dist"}}],
    "papers": {{
        "${{year}}": {{
            "$derive": {{"field":"published", "type":"integer"}}
        }}
    }}
}}"#
        )
        .unwrap();

        assert!(Config::load_from_file(tmp.path()).is_err());
    }

    /// Checks that `$derive` on a non-template (plain) sub-path key is
    /// rejected by validation with the expected error message.
    #[test]
    fn test_non_template_derive_is_rejected() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(
            tmp,
            r#"{{
    "$config": [{{"serializer":"json","layout":"index","dest":"dist"}}],
    "activities": {{
        "by-year": {{
            "$derive": "from"
        }}
    }}
}}"#
        )
        .unwrap();

        let err = match Config::load_from_file(tmp.path()) {
            Ok(_) => panic!("config should be rejected"),
            Err(e) => e,
        };
        assert!(format!("{}", err).contains("$values/$derive are only allowed"));
    }
}
