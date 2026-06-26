use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde_json::Value;
use crate::{Error, Result};

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FilterOp {
    Eq, Neq, Gt, Gte, Lt, Lte, Contains, Exists, Regeq, Regneq,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct FilterCondition {
    pub field: String,
    pub op: FilterOp,
    pub value: Value,
}

#[derive(Deserialize, Debug, Clone, Default, PartialEq)]
pub struct ApiNode {
    #[serde(rename = "$filter")]
    pub filter: Option<Vec<FilterCondition>>,

    #[serde(rename = "$aggregate")]
    pub aggregate: Option<Vec<String>>,

    #[serde(rename = "$pick")]
    pub pick: Option<Vec<String>>,

    #[serde(rename = "$omit")]
    pub omit: Option<Vec<String>>,

    #[serde(flatten)]
    pub sub_paths: HashMap<String, ApiNode>,
}

/// Layout configuration
#[derive(Deserialize, Serialize, Debug, Clone, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Layout {
    Index,
    File,
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
}

/// Global configuration
#[derive(Deserialize)]
pub struct Config {
    /// List of serializer configurations
    #[serde(default)]
    pub serializers: Vec<SerializerConfig>,

    /// Advanced routing overlay
    #[serde(flatten)]
    pub api: HashMap<String, ApiNode>,
}

impl Default for Config {
    fn default() -> Self {
        Self { 
            serializers: vec![
                SerializerConfig { 
                    serializer: "json".into(),
                    layout: Layout::Index,
                    dest: "dist".into()
                }
            ],
            api: HashMap::new(),
        }
    }
}

impl Config {
    pub fn new<P: AsRef<Path>>(serializer: String, layout: Layout, dest: P) -> Self {
        let dest = dest.as_ref().to_path_buf();
        Self { 
            serializers: vec![
                SerializerConfig{
                    serializer, layout, dest
                }
            ],
            api: HashMap::new(),
        }
    }

    /// Loads configuration from a specific file path
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path).map_err(Error::Io)?;
        serde_json::from_str(&content).map_err(Error::SerdeJson)
    }

    /// Discovers and loads a configuration file from a directory.
    /// It searches for '_config.json', '_prest.json', '.config.json', and '.prest.json' in order.
    pub fn discover(dir: &Path) -> Option<PathBuf> {
        let configs = [ "_config.json", "_prest.json", ".config.json", ".prest.json" ];
        configs.iter()
            .map(|c| dir.join(c))
            .find(|path| path.exists())
    }

    /// Loads the configuration based on provided options.
    /// If an explicit config path is provided, it attempts to load it.
    /// If an explicit layout is provided, it creates a new Config.
    /// Otherwise, it attempts to discover the config in the inputs directory,
    /// or falls back to the default configuration.
    pub fn load_or_default(
        explicit_config: Option<&PathBuf>,
        inputs_dir: &Path,
        explicit_layout: Option<&Layout>,
        serializer: String,
        dest: PathBuf,
    ) -> Result<Self> {
        if let Some(config_path) = explicit_config {
            Self::load(config_path)
        } else if let Some(discovered_path) = Self::discover(inputs_dir) {
            Self::load(&discovered_path)
        } else if let Some(layout) = explicit_layout {
            Ok(Self::new(serializer, layout.clone(), dest))
        } else {
            Ok(Self::default())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_advanced_routing_config() {
        let config_path = Path::new("testdata/tamada/_config.json");
        let config = Config::load(config_path).expect("Failed to load complex configuration");

        // Verify parsing of job-histories/current/$filter
        let job_hist = config.api.get("job-histories").expect("Missing job-histories node");
        let current = job_hist.sub_paths.get("current").expect("Missing current sub-path");
        let filter = current.filter.as_ref().expect("Missing filter array");
        assert_eq!(filter.len(), 1);
        assert_eq!(filter[0].field, "to");
        assert_eq!(filter[0].op, FilterOp::Eq);
        assert_eq!(filter[0].value, Value::String("Present".to_string()));

        // Verify parsing of activities/$filter
        let activities = config.api.get("activities").expect("Missing activities node");
        let filter2 = activities.filter.as_ref().expect("Missing filter array for activities");
        assert_eq!(filter2[0].field, "public");
        assert_eq!(filter2[0].op, FilterOp::Eq);
        assert_eq!(filter2[0].value, Value::Bool(true));

        // Verify parsing of profile/$aggregate
        let profile = config.api.get("profile").expect("Missing profile node");
        let agg = profile.aggregate.as_ref().expect("Missing aggregate array");
        assert_eq!(agg, &vec!["job-histories".to_string(), "activities".to_string(), "degrees".to_string(), "skills".to_string()]);
    }
}
