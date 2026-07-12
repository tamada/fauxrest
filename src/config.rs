pub use crate::filter::{FilterCondition, FilterOp};
use crate::{Error, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::{fs, io};

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EmitTarget {
    List,
    Ids,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum AggregateSpec {
    Paths(Vec<String>),
    Config(AggregateConfig),
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct AggregateConfig {
    #[serde(default)]
    pub mode: AggregateMode,
    pub sources: Vec<AggregateSource>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum AggregateMode {
    #[default]
    Flat,
    Keyed,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum AggregateSource {
    Path(String),
    Mapping(AggregateSourceMapping),
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct AggregateSourceMapping {
    pub from: String,
    #[serde(rename = "as")]
    pub as_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregateEntry {
    pub from: String,
    pub key: Option<String>,
}

impl AggregateSpec {
    pub fn mode(&self) -> AggregateMode {
        match self {
            AggregateSpec::Paths(_) => AggregateMode::Flat,
            AggregateSpec::Config(cfg) => cfg.mode.clone(),
        }
    }

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

#[derive(Deserialize, Debug, Clone, Default, PartialEq)]
pub struct ApiNode {
    #[serde(rename = "$filter")]
    pub filter: Option<Vec<FilterCondition>>,

    #[serde(rename = "$aggregate")]
    pub aggregate: Option<AggregateSpec>,

    #[serde(rename = "$pick")]
    pub pick: Option<Vec<String>>,

    #[serde(rename = "$omit")]
    pub omit: Option<Vec<String>>,

    #[serde(rename = "$emit")]
    pub emit: Option<Vec<EmitTarget>>,

    #[serde(rename = "$values")]
    pub values: Option<Vec<Value>>,

    #[serde(rename = "$derive")]
    pub derive: Option<DeriveSource>,

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

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum DeriveSource {
    Field(String),
    Config(DeriveConfig),
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct DeriveConfig {
    pub field: String,
    pub pattern: Option<String>,
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
    pub fn include(&self) -> &[String] {
        match self {
            StaticSpec::Include(patterns) => patterns,
            StaticSpec::Detailed(cfg) => &cfg.include,
        }
    }

    /// Returns the configured exclude (deny) glob patterns.
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
    pub fn load_from_str<S: AsRef<str>>(s: S) -> Result<Self> {
        let content = s.as_ref();
        let config: Self = serde_json::from_str(content).map_err(Error::SerdeJson)?;
        config.validate()?;
        Ok(config)
    }

    pub fn load_from_reader(reader: &mut impl std::io::Read) -> Result<Self> {
        let mut reader = std::io::BufReader::new(reader);
        let content = io::read_to_string(&mut reader)?;
        Self::load_from_str(content)
    }

    /// Loads configuration from a specific file path
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path).map_err(Error::Io)?;
        Self::load_from_str(content)
    }
}

impl Config {
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

fn template_var_from_key(key: &str) -> Option<&str> {
    if key.starts_with("${") && key.ends_with('}') && key.len() > 3 {
        Some(&key[2..key.len() - 1])
    } else {
        None
    }
}

fn is_scalar(value: &Value) -> bool {
    matches!(value, Value::String(_) | Value::Number(_) | Value::Bool(_))
}

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
    pub fn to_config(&self) -> DeriveConfig {
        match self {
            DeriveSource::Field(field) => DeriveConfig {
                field: field.clone(),
                pattern: None,
            },
            DeriveSource::Config(c) => c.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

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
