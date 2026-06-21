//! Compilation orchestrator module
//!
//! Orchestrates the multi-serializer execution loop based on configuration,
//! reading raw JSON and generating static files according to specified layouts.

use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use serde_json::Value;
use crate::{Serializer, Layout, JSONSerializer, TypescriptSerializer, SqliteSerializer, IndexLayout, FileLayout, ExtensionLayout, Error};

/// Serializer configuration
#[derive(Deserialize)]
pub struct SerializerConfig {
    /// Serializer type (json, typescript, sqlite)
    pub serializer: String,
    /// Delivery layout (index, file, extension)
    pub layout: Option<String>,
    /// Destination directory
    pub dest: String,
}

/// Global configuration
#[derive(Deserialize)]
pub struct Config {
    /// List of serializer configurations
    pub serializers: Vec<SerializerConfig>,
}

impl Config {
    /// Loads configuration from a file
    pub fn load(path: &Path) -> Result<Self, Error> {
        let content = fs::read_to_string(path).map_err(Error::Io)?;
        serde_json::from_str(&content).map_err(Error::SerdeJson)
    }
}

/// Executes the API build process
pub fn run(config: Config, data_dir: PathBuf) -> Result<(), Error> {
    for s_conf in config.serializers {
        run_serializer(&s_conf, &data_dir)?;
    }
    Ok(())
}

fn run_serializer(conf: &SerializerConfig, data_dir: &Path) -> Result<(), Error> {
    let serializer = get_serializer(&conf.serializer);
    let layout = get_layout(conf.layout.as_deref());
    let data_dir = fs::read_dir(data_dir).map_err(Error::Io)?;

    for entry in data_dir {
        let entry = entry.map_err(Error::Io)?;
        process_entry(&entry, serializer.as_ref(), layout.as_ref(), &conf.dest)?;
    }
    Ok(())
}

fn process_entry(
    entry: &fs::DirEntry,
    s: &dyn Serializer,
    l: &dyn Layout,
    dest: &str
) -> Result<(), Error> {
    let path = entry.path();
    if path.extension().and_then(|s| s.to_str()) != Some("json") { return Ok(()); }
    
    let content = fs::read_to_string(path).map_err(Error::Io)?;
    let json: Value = serde_json::from_str(&content).map_err(Error::SerdeJson)?;
    write_data(entry.file_name().to_str().unwrap(), &json, s, l, dest)
}

fn write_data(
    name: &str,
    data: &Value,
    s: &dyn Serializer,
    l: &dyn Layout,
    dest: &str
) -> Result<(), Error> {
    let is_coll = data.is_array();
    let endpoint = name.strip_suffix(".json").unwrap_or(name);
    let path = l.determine_path(endpoint, s.extension(), is_coll);
    let full_path = Path::new(dest).join(path);
    
    fs::create_dir_all(full_path.parent().unwrap()).map_err(Error::Io)?;
    fs::write(full_path, s.serialize(data)?).map_err(Error::Io)?;
    Ok(())
}

fn get_serializer(s: &str) -> Box<dyn Serializer> {
    match s {
        "typescript" | "js" => Box::new(TypescriptSerializer),
        "sqlite" => Box::new(SqliteSerializer),
        _ => Box::new(JSONSerializer),
    }
}

fn get_layout(l: Option<&str>) -> Box<dyn Layout> {
    match l {
        Some("file") => Box::new(FileLayout),
        Some("extension") => Box::new(ExtensionLayout),
        _ => Box::new(IndexLayout),
    }
}
