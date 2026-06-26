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
    assert!(!dest_dir.join("users/1/index.json").exists());

    let discovery = fs::read_to_string(dest_dir.join("index.json")).unwrap();
    assert!(!discovery.contains("\"/users\""));
    assert!(!discovery.contains("\"/users/1\""));
}

#[test]
fn test_template_subpath_expansion_with_filter_override() {
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = tmp.path().join("data");
        let dest_dir = tmp.path().join("dist");
        let config_file = tmp.path().join("prest.json");

        fs::create_dir(&data_dir).unwrap();
        fs::write(
                data_dir.join("activities.json"),
                r#"[
    {"id": 1, "from": "2024-01-01", "public": false, "label": "private-2024"},
    {"id": 2, "from": "2024-05-10", "public": true, "label": "public-2024"},
    {"id": 3, "from": "2025-03-03", "public": true, "label": "public-2025"}
]"#,
        )
        .unwrap();

        let config_json = format!(
                r#"{{
    "serializers": [{{"serializer": "json", "layout": "index", "dest": "{}"}}],
    "activities": {{
        "$filter": [{{"field": "public", "op": "eq", "value": true}}],
        "${{year}}": {{
            "$values": ["2024", "2025"],
            "$filter": [{{"field": "from", "op": "contains", "value": "{{year}}"}}]
        }}
    }}
}}"#,
                dest_dir.display()
        );
        fs::write(&config_file, &config_json).unwrap();

        let config: Config = Config::load(Path::new(&config_file)).unwrap();
        assert!(prest::run(config, data_dir).is_ok());

        assert_file(&dest_dir, "activities/index.json");
        assert_file(&dest_dir, "activities/2024/index.json");
        assert_file(&dest_dir, "activities/2025/index.json");

        // Root applies parent filter.
        let root = fs::read_to_string(dest_dir.join("activities/index.json")).unwrap();
        assert!(!root.contains("private-2024"));
        assert!(root.contains("public-2024"));
        assert!(root.contains("public-2025"));

        // Child filter overrides parent filter and includes non-public entries matching year.
        let by_2024 = fs::read_to_string(dest_dir.join("activities/2024/index.json")).unwrap();
        assert!(by_2024.contains("private-2024"));
        assert!(by_2024.contains("public-2024"));

        let discovery = fs::read_to_string(dest_dir.join("index.json")).unwrap();
        assert!(discovery.contains("\"/activities/2024\""));
        assert!(discovery.contains("\"/activities/2025\""));
}

#[test]
fn test_invalid_template_without_values_fails_to_load() {
        let tmp = tempfile::tempdir().unwrap();
        let config_file = tmp.path().join("prest.json");
        let config_json = r#"{
    "serializers": [{"serializer": "json", "layout": "index", "dest": "dist"}],
    "activities": {
        "${year}": {
            "$filter": [{"field": "from", "op": "contains", "value": "{year}"}]
        }
    }
}"#;
        fs::write(&config_file, config_json).unwrap();

        let err = match Config::load(Path::new(&config_file)) {
            Ok(_) => panic!("config should be rejected"),
            Err(e) => e,
        };
        assert!(format!("{}", err).contains("template sub-path requires $values"));
}

#[test]
fn test_pick_directive_keeps_only_specified_fields() {
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = tmp.path().join("data");
        let dest_dir = tmp.path().join("dist");
        let config_file = tmp.path().join("prest.json");

        fs::create_dir(&data_dir).unwrap();
        fs::write(
                data_dir.join("users.json"),
                r#"[
    {"id": 1, "name": "Alice", "email": "a@example.com", "password": "secret"}
]"#,
        )
        .unwrap();
        let config_json = format!(
                r#"{{
    "serializers": [{{"serializer": "json", "layout": "index", "dest": "{}"}}],
    "users": {{
        "$pick": ["id", "name"]
    }}
}}"#,
                dest_dir.display()
        );
        fs::write(&config_file, &config_json).unwrap();

        let config: Config = Config::load(Path::new(&config_file)).unwrap();
        assert!(prest::run(config, data_dir).is_ok());

        let content = fs::read_to_string(dest_dir.join("users/index.json")).unwrap();
        assert!(content.contains("\"id\""));
        assert!(content.contains("\"name\""));
        assert!(!content.contains("\"email\""));
        assert!(!content.contains("\"password\""));
}

#[test]
fn test_omit_directive_removes_specified_fields() {
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = tmp.path().join("data");
        let dest_dir = tmp.path().join("dist");
        let config_file = tmp.path().join("prest.json");

        fs::create_dir(&data_dir).unwrap();
        fs::write(
                data_dir.join("users.json"),
                r#"[
    {"id": 1, "name": "Alice", "email": "a@example.com", "password": "secret"}
]"#,
        )
        .unwrap();
        let config_json = format!(
                r#"{{
    "serializers": [{{"serializer": "json", "layout": "index", "dest": "{}"}}],
    "users": {{
        "$omit": ["password", "email"]
    }}
}}"#,
                dest_dir.display()
        );
        fs::write(&config_file, &config_json).unwrap();

        let config: Config = Config::load(Path::new(&config_file)).unwrap();
        assert!(prest::run(config, data_dir).is_ok());

        let content = fs::read_to_string(dest_dir.join("users/index.json")).unwrap();
        assert!(content.contains("\"id\""));
        assert!(content.contains("\"name\""));
        assert!(!content.contains("\"email\""));
        assert!(!content.contains("\"password\""));
}

    #[test]
    fn test_json_minify_flag_compacts_output() {
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = tmp.path().join("data");
        let pretty_dir = tmp.path().join("dist_pretty");
        let minify_dir = tmp.path().join("dist_minify");

        fs::create_dir(&data_dir).unwrap();
        fs::write(
            data_dir.join("profile.json"),
            r#"{"name":"Alice","roles":["admin","editor"]}"#,
        )
        .unwrap();

        let pretty = Config {
            serializers: vec![prest::SerializerConfig {
                serializer: "json".into(),
                layout: Layout::Index,
                dest: pretty_dir.clone(),
                minify: false,
            }],
            api: std::collections::HashMap::new(),
        };
        let minified = Config {
            serializers: vec![prest::SerializerConfig {
                serializer: "json".into(),
                layout: Layout::Index,
                dest: minify_dir.clone(),
                minify: true,
            }],
            api: std::collections::HashMap::new(),
        };

        assert!(prest::run(pretty, &data_dir).is_ok());
        assert!(prest::run(minified, &data_dir).is_ok());

        let pretty_text = fs::read_to_string(pretty_dir.join("profile/index.json")).unwrap();
        let minified_text = fs::read_to_string(minify_dir.join("profile/index.json")).unwrap();
        assert!(pretty_text.contains("\n  \"name\""));
        assert!(!minified_text.contains("\n  \"name\""));
        assert!(minified_text.len() < pretty_text.len());
    }

    #[test]
    fn test_typescript_minify_flag_compacts_embedded_json() {
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = tmp.path().join("data");
        let pretty_dir = tmp.path().join("dist_pretty");
        let minify_dir = tmp.path().join("dist_minify");

        fs::create_dir(&data_dir).unwrap();
        fs::write(
            data_dir.join("users.json"),
            r#"[{"id":1,"name":"Bob","team":"R&D"}]"#,
        )
        .unwrap();

        let pretty = Config {
            serializers: vec![prest::SerializerConfig {
                serializer: "typescript".into(),
                layout: Layout::Index,
                dest: pretty_dir.clone(),
                minify: false,
            }],
            api: std::collections::HashMap::new(),
        };
        let minified = Config {
            serializers: vec![prest::SerializerConfig {
                serializer: "typescript".into(),
                layout: Layout::Index,
                dest: minify_dir.clone(),
                minify: true,
            }],
            api: std::collections::HashMap::new(),
        };

        assert!(prest::run(pretty, &data_dir).is_ok());
        assert!(prest::run(minified, &data_dir).is_ok());

        let pretty_text = fs::read_to_string(pretty_dir.join("users/index.ts")).unwrap();
        let minified_text = fs::read_to_string(minify_dir.join("users/index.ts")).unwrap();
        assert!(pretty_text.contains("\n  {"));
        assert!(!minified_text.contains("\n  {"));
        assert!(minified_text.contains("export const data = [{\"id\":1,\"name\":\"Bob\",\"team\":\"R&D\"}];"));
    }
