//! Integration tests for the prest library

use std::fs;
use std::path::Path;
use prest::{Config, Layout};

/// Helper to assert that a file exists and has content containing a substring.
fn assert_contains(dest: &Path, rel_path: &str, expected: &str) {
    let file = dest.join(rel_path);
    assert!(file.exists(), "File {:?} should exist", file);
    let content = fs::read_to_string(file).expect("Failed to read file");
    assert!(content.contains(expected), "Content of {:?} should contain {:?}", rel_path, expected);
}

fn assert_file(dest: &Path, rel_path: &str) {
    let file = dest.join(rel_path);
    assert!(file.exists(), "File {:?} should exist", file);
    let metadata = file.metadata()
        .expect("{:?} failed to read file info");
    assert!(metadata.is_file(), "{:?} should be regular file", file);
}

#[test]
fn test_json_ser_file_layout() {
    let tmp = tempfile::tempdir().unwrap();
    let dist_dir = tmp.path().join("dist");
    let config = Config::new("json".into(), Layout::File, &dist_dir);

    fs::create_dir(&dist_dir).unwrap();
    prest::run(config, "testdata/example1")
        .expect("Failed to run prest");
    assert_file(&dist_dir, "index.json");
    assert_file(&dist_dir, "profile");
    assert_file(&dist_dir, "users/1");
    assert_file(&dist_dir, "users/2");
    assert_file(&dist_dir, "users/index.json");
}

#[test]
fn test_json_ser_index_layout() {
    let tmp = tempfile::tempdir().unwrap();
    let dist_dir = tmp.path().join("dist");
    let config = Config::new("json".into(), Layout::Index, &dist_dir);

    fs::create_dir(&dist_dir).unwrap();
    prest::run(config, "testdata/example1")
        .expect("Failed to run prest");
    assert_file(&dist_dir, "index.json");
    assert_file(&dist_dir, "profile/index.json");
    assert_file(&dist_dir, "users/1/index.json");
    assert_file(&dist_dir, "users/2/index.json");
    assert_file(&dist_dir, "users/index.json");
}

#[test]
fn test_json_ser_extension_layout() {
    let tmp = tempfile::tempdir().unwrap();
    let dist_dir = tmp.path().join("dist");
    let config = Config::new("json".into(), Layout::Extension, &dist_dir);

    fs::create_dir(&dist_dir).unwrap();
    prest::run(config, "testdata/example1")
        .expect("Failed to run prest");
    assert_file(&dist_dir, "index.json");
    assert_file(&dist_dir, "profile.json");
    assert_file(&dist_dir, "users/1.json");
    assert_file(&dist_dir, "users/2.json");
    assert_file(&dist_dir, "users.json");
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

#[test]
fn test_private_directive_hides_collection_endpoint() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().join("data");
    let dest_dir = tmp.path().join("dist");
    let config_file = tmp.path().join("prest.json");

    fs::create_dir(&data_dir).unwrap();
    fs::write(data_dir.join("users.json"), r#"[{"id": 1, "name": "Bob"}]"#).unwrap();

    let config_json = format!(
        r#"{{
  "serializers": [{{"serializer": "json", "layout": "index", "dest": "{}"}}],
  "users": {{"$private": true}}
}}"#,
        dest_dir.display()
    );
    fs::write(&config_file, &config_json).unwrap();

    let config: Config = Config::load(Path::new(&config_file)).unwrap();
    assert!(prest::run(config, data_dir).is_ok());

    assert!(!dest_dir.join("users/index.json").exists());
    assert_file(&dest_dir, "users/1/index.json");

    let discovery = fs::read_to_string(dest_dir.join("index.json")).unwrap();
    assert!(!discovery.contains("\"/users\""));
    assert!(discovery.contains("\"/users/1\""));
}
