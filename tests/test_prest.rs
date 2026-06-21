//! Integration tests for the prest library

use std::fs;
use std::path::Path;
use prest::{Config};

/// Helper to assert that a file exists and has content containing a substring.
fn assert_contains(dest: &Path, rel_path: &str, expected: &str) {
    let file = dest.join(rel_path);
    assert!(file.exists(), "File {:?} should exist", file);
    let content = fs::read_to_string(file).expect("Failed to read file");
    assert!(content.contains(expected), "Content of {:?} should contain {:?}", rel_path, expected);
}

#[test]
fn test_integration_json_json_layout() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().join("data");
    let dest_dir = tmp.path().join("dist");
    let config_file = tmp.path().join("prest.json");

    fs::create_dir(&data_dir).unwrap();
    fs::write(data_dir.join("profile.json"), r#"{"name": "Alice"}"#).unwrap();
    let config_json = format!(r#"{{"serializers": [{{"serializer": "json", "layout": "index", "dest": "{}"}}]}}"#, dest_dir.display());
    fs::write(&config_file, &config_json).unwrap();

    let config: Config = Config::load(Path::new(&config_file)).unwrap();
    assert!(prest::run(config, data_dir).is_ok());
    assert_contains(&dest_dir, "profile/index.json", "Alice");
}

#[test]
fn test_integration_typescript_file_layout() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().join("data");
    let dest_dir = tmp.path().join("dist");
    let config_file = tmp.path().join("prest.json");

    fs::create_dir(&data_dir).unwrap();
    fs::write(data_dir.join("users.json"), r#"[{"id": 1, "name": "Bob"}]"#).unwrap();
    let config_json = format!(r#"{{"serializers": [{{"serializer": "typescript", "layout": "file", "dest": "{}"}}]}}"#, dest_dir.display());
    fs::write(&config_file, &config_json).unwrap();

    let config: Config = Config::load(Path::new(&config_file)).unwrap();
    assert!(prest::run(config, data_dir).is_ok());
    assert_contains(&dest_dir, "users/index.ts", "export const data");
}
