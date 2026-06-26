//! Compilation orchestrator module
//!
//! Orchestrates the multi-serializer execution loop based on configuration,
//! reading raw JSON and generating static files according to specified layouts.

use std::fs;
use std::path::{Path, PathBuf};
use serde_json::{Value, json};
use crate::{Serializer, JSONSerializer, TypescriptSerializer, SqliteSerializer, Error, Layout, Result, Config, SerializerConfig};

/// Executes the API build process
pub fn run<P: AsRef<Path>>(config: Config, data_dir: P) -> Result<()> {
    let mut endpoints = Vec::new();
    for s_conf in &config.serializers {
        endpoints.extend(run_serializer(s_conf, data_dir.as_ref())?);
    }
    generate_discovery(&config, &endpoints)?;
    Ok(())
}

fn run_serializer(conf: &SerializerConfig, data_dir: &Path) -> Result<Vec<String>> {
    let serializer = get_serializer(&conf.serializer)?;
    let layout = get_layout(&conf.layout);
    let data_dir = fs::read_dir(data_dir).map_err(Error::Io)?;
    let mut endpoints = Vec::new();

    for entry in data_dir {
        let entry = entry.map_err(Error::Io)?;
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();
        if file_name_str.starts_with('_') || file_name_str.starts_with('.') {
            continue;
        }
        endpoints.extend(process_entry(&entry, serializer.as_ref(), layout.as_ref(), &conf.dest)?);
    }
    Ok(endpoints)
}


fn process_entry(
    entry: &fs::DirEntry,
    s: &dyn Serializer,
    l: &dyn LayoutTrait,
    dest: &Path
) -> Result<Vec<String>> {
    let path = entry.path();
    if path.extension().and_then(|s| s.to_str()) != Some("json") { return Ok(vec![]); }
    
    let content = fs::read_to_string(path).map_err(Error::Io)?;
    let json: Value = serde_json::from_str(&content).map_err(Error::SerdeJson)?;
    let name = entry.file_name().to_str().unwrap().strip_suffix(".json").unwrap().to_string();
    write_data(&name, &json, s, l, dest)?;
    
    let mut endpoints = vec![format!("/{}", name)];
    if let Value::Array(arr) = json {
        for item in arr {
            if let Some(id) = item.get("id") {
                let id_str = id.as_str().map(|s| s.to_string()).unwrap_or_else(|| id.to_string());
                write_data(&format!("{}/{}", name, id_str), &item, s, l, dest)?;
                endpoints.push(format!("/{}/{}", name, id_str));
            }
        }
    }
    Ok(endpoints)
}

fn write_data(
    name: &str,
    data: &Value,
    s: &dyn Serializer,
    l: &dyn LayoutTrait,
    dest: &Path
) -> Result<()> {
    let is_coll = data.is_array();
    let endpoint = name.strip_suffix(".json").unwrap_or(name);
    let path = l.determine_path(endpoint, s.extension(), is_coll);
    let full_path = dest.join(path);
    
    fs::create_dir_all(full_path.parent().unwrap()).map_err(Error::Io)?;
    fs::write(full_path, s.serialize(data)?).map_err(Error::Io)?;
    Ok(())
}

fn generate_discovery(config: &Config, endpoints: &[String]) -> Result<()> {
    let discovery = json!({ "endpoints": endpoints });
    for s_conf in &config.serializers {
        let s = get_serializer(&s_conf.serializer)?;
        let path = s_conf.dest.join(format!("index.{}", s.extension()));
        fs::create_dir_all(path.parent().unwrap()).map_err(Error::Io)?;
        fs::write(path, s.serialize(&discovery)?).map_err(Error::Io)?;
    }
    Ok(())
}

fn get_serializer(s: &str) -> Result<Box<dyn Serializer>> {
    match s {
        "typescript" | "javascript" | "ts" | "js" => Ok(Box::new(TypescriptSerializer)),
        "sqlite" | "sql" => Ok(Box::new(SqliteSerializer)),
        "json" => Ok(Box::new(JSONSerializer)),
        _ => Err(Error::UnknownSerializer(s.into())),
    }
}

fn get_layout(l: &Layout) -> Box<dyn LayoutTrait> {
    match l {
        Layout::File => Box::new(FileLayout),
        Layout::Extension => Box::new(ExtensionLayout),
        Layout::Index => Box::new(IndexLayout),
    }
}

/// Trait for physical data serialization
trait LayoutTrait {
    fn determine_path(&self, endpoint: &str, file_ext: &str, is_coll: bool) -> PathBuf;
}

/// Layout that places files in `index.[ext]`
struct IndexLayout;
impl LayoutTrait for IndexLayout {
    fn determine_path(&self, endpoint: &str, ext: &str, _: bool) -> PathBuf {
        Path::new(endpoint).join(format!("index.{}", ext))
    }
}

/// Layout that appends the extension directly
struct ExtensionLayout;
impl LayoutTrait for ExtensionLayout {
    fn determine_path(&self, endpoint: &str, ext: &str, _: bool) -> PathBuf {
        PathBuf::from(format!("{}.{}", endpoint, ext))
    }
}

/// Layout that avoids extensions where possible (supports smart fallback)
struct FileLayout;
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
