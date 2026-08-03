//! Integration tests for the fauxrest library

use fauxrest::{Config, Layout};
use std::fs;
use std::path::{Path, PathBuf};

/// Helper to assert that a file exists and has content containing a substring.
fn assert_contains(dest: &Path, rel_path: &str, expected: &str) {
    let file = dest.join(rel_path);
    assert!(file.exists(), "File {:?} should exist", file);
    let content = fs::read_to_string(file).expect("Failed to read file");
    assert!(
        content.contains(expected),
        "Content of {:?} should contain {:?}",
        rel_path,
        expected
    );
}

fn assert_file(dest: &Path, rel_path: &str) {
    let file = dest.join(rel_path);
    assert!(file.exists(), "File {:?} should exist", file);
    let metadata = file.metadata().expect("{:?} failed to read file info");
    assert!(metadata.is_file(), "{:?} should be regular file", file);
}

/// Reads a generated JSON endpoint for exact value assertions.
fn read_json(dest: &Path, rel_path: &str) -> serde_json::Value {
    let content = fs::read_to_string(dest.join(rel_path)).expect("Failed to read JSON endpoint");
    serde_json::from_str(&content).expect("Failed to parse JSON endpoint")
}

#[test]
fn test_json_ser_file_layout() {
    let tmp = tempfile::tempdir().unwrap();
    let dist_dir = tmp.path().join("dist");
    let config = Config::new("json".into(), Layout::File, &dist_dir);

    fs::create_dir(&dist_dir).unwrap();
    fauxrest::run(config, "testdata/example1").expect("Failed to run fauxrest");
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
    fauxrest::run(config, "testdata/example1").expect("Failed to run fauxrest");
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
    fauxrest::run(config, "testdata/example1").expect("Failed to run fauxrest");
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
    let config_file = tmp.path().join("fauxrest.json");

    fs::create_dir(&data_dir).unwrap();
    fs::write(data_dir.join("profile.json"), r#"{"name": "Alice"}"#).unwrap();
    let config_json = format!(
        r#"{{"$config": [{{"serializer": "json", "layout": "index", "dest": "{}"}}]}}"#,
        dest_dir.display()
    );
    fs::write(&config_file, &config_json).unwrap();

    let config: Config = Config::load_from_file(Path::new(&config_file)).unwrap();
    assert!(fauxrest::run(config, data_dir).is_ok());
    assert_contains(&dest_dir, "profile/index.json", "Alice");
}

#[test]
fn test_integration_typescript_file_layout() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().join("data");
    let dest_dir = tmp.path().join("dist");
    let config_file = tmp.path().join("fauxrest.json");

    fs::create_dir(&data_dir).unwrap();
    fs::write(data_dir.join("users.json"), r#"[{"id": 1, "name": "Bob"}]"#).unwrap();
    let config_json = format!(
        r#"{{"$config": [{{"serializer": "typescript", "layout": "file", "dest": "{}"}}]}}"#,
        dest_dir.display()
    );
    fs::write(&config_file, &config_json).unwrap();

    let config: Config = Config::load_from_file(Path::new(&config_file)).unwrap();
    assert!(fauxrest::run(config, data_dir).is_ok());
    assert_contains(&dest_dir, "users/index.ts", "export const data");
}

#[test]
fn test_private_directive_hides_collection_endpoint() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().join("data");
    let dest_dir = tmp.path().join("dist");
    let config_file = tmp.path().join("fauxrest.json");

    fs::create_dir(&data_dir).unwrap();
    fs::write(data_dir.join("users.json"), r#"[{"id": 1, "name": "Bob"}]"#).unwrap();

    let config_json = format!(
        r#"{{
  "$config": [{{"serializer": "json", "layout": "index", "dest": "{}"}}],
    "users": {{"$emit": []}}
}}"#,
        dest_dir.display()
    );
    fs::write(&config_file, &config_json).unwrap();

    let config: Config = Config::load_from_file(Path::new(&config_file)).unwrap();
    assert!(fauxrest::run(config, data_dir).is_ok());

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
    let config_file = tmp.path().join("fauxrest.json");

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
    "$config": [{{"serializer": "json", "layout": "index", "dest": "{}"}}],
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

    let config: Config = Config::load_from_file(Path::new(&config_file)).unwrap();
    assert!(fauxrest::run(config, data_dir).is_ok());

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
fn test_invalid_template_without_values_or_derive_fails_to_load() {
    let tmp = tempfile::tempdir().unwrap();
    let config_file = tmp.path().join("fauxrest.json");
    let config_json = r#"{
    "$config": [{"serializer": "json", "layout": "index", "dest": "dist"}],
    "activities": {
        "${year}": {
            "$filter": [{"field": "from", "op": "contains", "value": "{year}"}]
        }
    }
}"#;
    fs::write(&config_file, config_json).unwrap();

    let err = match Config::load_from_file(Path::new(&config_file)) {
        Ok(_) => panic!("config should be rejected"),
        Err(e) => e,
    };
    assert!(format!("{}", err).contains("template sub-path requires $values or $derive"));
}

#[test]
fn test_template_subpath_expansion_with_derive() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().join("data");
    let dest_dir = tmp.path().join("dist");
    let config_file = tmp.path().join("fauxrest.json");

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
    "$config": [{{"serializer": "json", "layout": "index", "dest": "{}"}}],
    "activities": {{
        "${{year}}": {{
            "$derive": {{ "field": "from", "pattern": "^(\\d{{4}})" }},
            "$filter": [{{"field": "from", "op": "contains", "value": "{{year}}"}}]
        }}
    }}
}}"#,
        dest_dir.display()
    );
    fs::write(&config_file, &config_json).unwrap();

    let config: Config = Config::load_from_file(Path::new(&config_file)).unwrap();
    assert!(fauxrest::run(config, data_dir).is_ok());

    assert_file(&dest_dir, "activities/2024/index.json");
    assert_file(&dest_dir, "activities/2025/index.json");

    let discovery = fs::read_to_string(dest_dir.join("index.json")).unwrap();
    assert!(discovery.contains("\"/activities/2024\""));
    assert!(discovery.contains("\"/activities/2025\""));
}

#[test]
fn test_emit_id_false_suppresses_item_files_but_keeps_id_field() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().join("data");
    let dest_dir = tmp.path().join("dist");
    let config_file = tmp.path().join("fauxrest.json");

    fs::create_dir(&data_dir).unwrap();
    fs::write(
        data_dir.join("activities.json"),
        r#"[
        {"id": 1, "from": "2024-01-01", "public": true, "label": "a"},
        {"id": 2, "from": "2025-02-01", "public": true, "label": "b"}
    ]"#,
    )
    .unwrap();

    let config_json = format!(
        r#"{{
        "$config": [{{"serializer": "json", "layout": "index", "dest": "{}"}}],
        "activities": {{
        "${{year}}": {{
            "$derive": {{ "field": "from", "pattern": "^(\\d{{4}})" }},
            "$filter": [{{"field": "from", "op": "contains", "value": "{{year}}"}}],
            "$emit": ["list"]
        }}
        }}
    }}"#,
        dest_dir.display()
    );
    fs::write(&config_file, &config_json).unwrap();

    let config: Config = Config::load_from_file(Path::new(&config_file)).unwrap();
    assert!(fauxrest::run(config, data_dir).is_ok());

    let by_2024 = dest_dir.join("activities/2024/index.json");
    assert!(by_2024.exists());
    let content = fs::read_to_string(by_2024).unwrap();
    assert!(content.contains("\"id\": 1"));
    assert!(!dest_dir.join("activities/2024/1/index.json").exists());
}

#[test]
fn test_emit_list_false_emit_id_true_emits_only_item_endpoints() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().join("data");
    let dest_dir = tmp.path().join("dist");
    let config_file = tmp.path().join("fauxrest.json");

    fs::create_dir(&data_dir).unwrap();
    fs::write(
        data_dir.join("users.json"),
        r#"[
        {"id": 1, "name": "Alice"},
        {"id": 2, "name": "Bob"}
    ]"#,
    )
    .unwrap();

    let config_json = format!(
        r#"{{
        "$config": [{{"serializer": "json", "layout": "index", "dest": "{}"}}],
        "users": {{
        "$emit": ["ids"]
        }}
    }}"#,
        dest_dir.display()
    );
    fs::write(&config_file, &config_json).unwrap();

    let config: Config = Config::load_from_file(Path::new(&config_file)).unwrap();
    assert!(fauxrest::run(config, data_dir).is_ok());

    assert!(!dest_dir.join("users/index.json").exists());
    assert!(dest_dir.join("users/1/index.json").exists());
    assert!(dest_dir.join("users/2/index.json").exists());

    let discovery = fs::read_to_string(dest_dir.join("index.json")).unwrap();
    assert!(!discovery.contains("\"/users\""));
    assert!(discovery.contains("\"/users/1\""));
    assert!(discovery.contains("\"/users/2\""));
}

#[test]
fn test_emit_empty_set_is_allowed_and_emits_nothing_at_node() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().join("data");
    let dest_dir = tmp.path().join("dist");
    let config_file = tmp.path().join("fauxrest.json");

    fs::create_dir(&data_dir).unwrap();
    fs::write(
        data_dir.join("users.json"),
        r#"[
        {"id": 1, "name": "Alice"}
    ]"#,
    )
    .unwrap();

    let config_json = format!(
        r#"{{
        "$config": [{{"serializer": "json", "layout": "index", "dest": "{}"}}],
        "users": {{
        "$emit": []
        }}
    }}"#,
        dest_dir.display()
    );
    fs::write(&config_file, &config_json).unwrap();

    let config: Config = Config::load_from_file(Path::new(&config_file)).unwrap();
    assert!(fauxrest::run(config, data_dir).is_ok());

    assert!(!dest_dir.join("users/index.json").exists());
    assert!(!dest_dir.join("users/1/index.json").exists());

    let discovery = fs::read_to_string(dest_dir.join("index.json")).unwrap();
    assert!(!discovery.contains("\"/users\""));
    assert!(!discovery.contains("\"/users/1\""));
}

/// Writes `papers.json` holding two numeric-`year` records, plus a config
/// deriving `${year}` through `derive` and filtering the same field with a
/// strict `eq`. Shared by the two halves of the no-op-pattern regression.
fn numeric_derive_fixture(tmp: &Path, derive: &str) -> (PathBuf, PathBuf, PathBuf) {
    let data_dir = tmp.join("data");
    let dest_dir = tmp.join("dist");
    let config_file = tmp.join("fauxrest.json");

    fs::create_dir(&data_dir).unwrap();
    fs::write(
        data_dir.join("papers.json"),
        r#"[
    {"id": 1, "year": 2024, "title": "p1"},
    {"id": 2, "year": 2025, "title": "p2"}
]"#,
    )
    .unwrap();

    let config_json = format!(
        r#"{{
    "$config": [{{"serializer": "json", "layout": "index", "dest": "{}"}}],
    "papers": {{
        "${{year}}": {{
            "$derive": {},
            "$filter": [{{"field": "year", "op": "eq", "value": "{{year}}"}}],
            "$emit": ["list"]
        }}
    }}
}}"#,
        dest_dir.display(),
        derive
    );
    fs::write(&config_file, &config_json).unwrap();

    (data_dir, dest_dir, config_file)
}

/// A no-op pattern stringifies a numeric value, so the derived `"2024"` can
/// never equal the number `2024` the records hold. That comparison is now
/// rejected outright: before, it produced an empty collection per year and
/// left the mistake to be noticed in the output.
#[test]
fn test_no_op_pattern_on_numeric_field_is_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let (data_dir, _dest_dir, config_file) =
        numeric_derive_fixture(tmp.path(), r#"{ "field": "year", "pattern": ".*" }"#);

    let config: Config = Config::load_from_file(Path::new(&config_file)).unwrap();
    let err = fauxrest::run(config, data_dir).expect_err("stringified derive must be rejected");
    let message = err.to_string();
    assert!(
        message.contains("'year'") && message.contains("number") && message.contains("string"),
        "error should name the field and both kinds, got: {}",
        message
    );
}

/// Without a pattern the derived value keeps its original JSON number, so the
/// same strict `eq` selects the expected record for each year.
#[test]
fn test_numeric_derive_preserves_type_without_pattern() {
    let tmp = tempfile::tempdir().unwrap();
    let (data_dir, dest_dir, config_file) = numeric_derive_fixture(tmp.path(), r#""year""#);

    let config: Config = Config::load_from_file(Path::new(&config_file)).unwrap();
    assert!(fauxrest::run(config, data_dir).is_ok());

    assert_eq!(
        read_json(&dest_dir, "papers/2024/index.json"),
        serde_json::json!([{"id": 1, "year": 2024, "title": "p1"}])
    );
    assert_eq!(
        read_json(&dest_dir, "papers/2025/index.json"),
        serde_json::json!([{"id": 2, "year": 2025, "title": "p2"}])
    );
}

/// `$derive.type: "int"` lets a pattern-extracted value be compared against
/// a genuinely numeric field with `eq`. Without the conversion the derived
/// value would be the string `"2024"`, which never equals the number `2024`,
/// so every generated endpoint would be an empty collection.
#[test]
fn test_derive_int_type_matches_numeric_field() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().join("data");
    let dest_dir = tmp.path().join("dist");
    let config_file = tmp.path().join("fauxrest.json");

    fs::create_dir(&data_dir).unwrap();
    fs::write(
        data_dir.join("papers.json"),
        r#"[
    {"id": 1, "year": 2024, "title": "p1"},
    {"id": 2, "year": 2025, "title": "p2"},
    {"id": 3, "year": 2024, "title": "p3"}
]"#,
    )
    .unwrap();

    let config_json = format!(
        r#"{{
    "$config": [{{"serializer": "json", "layout": "index", "dest": "{}"}}],
    "papers": {{
        "${{year}}": {{
            "$derive": {{ "field": "year", "pattern": "^(\\d{{4}})", "type": "int" }},
            "$filter": [{{"field": "year", "op": "eq", "value": "{{year}}"}}],
            "$emit": ["list"]
        }}
    }}
}}"#,
        dest_dir.display()
    );
    fs::write(&config_file, &config_json).unwrap();

    let config: Config = Config::load_from_file(Path::new(&config_file)).unwrap();
    assert!(fauxrest::run(config, data_dir).is_ok());

    assert_contains(&dest_dir, "papers/2024/index.json", "p1");
    assert_contains(&dest_dir, "papers/2024/index.json", "p3");
    assert_contains(&dest_dir, "papers/2025/index.json", "p2");

    let y2024 = fs::read_to_string(dest_dir.join("papers/2024/index.json")).unwrap();
    assert!(
        !y2024.contains("p2"),
        "2024 must not include the 2025 paper"
    );
    let y2025 = fs::read_to_string(dest_dir.join("papers/2025/index.json")).unwrap();
    assert!(
        !y2025.contains("p1"),
        "2025 must not include the 2024 paper"
    );
}

/// A numeric `$derive.type` also makes the ordering operators usable, since
/// `compare_ord` only compares values of the same JSON kind.
#[test]
fn test_derive_int_type_supports_ordering_filter() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().join("data");
    let dest_dir = tmp.path().join("dist");
    let config_file = tmp.path().join("fauxrest.json");

    fs::create_dir(&data_dir).unwrap();
    fs::write(
        data_dir.join("papers.json"),
        r#"[
    {"id": 1, "year": 2023, "title": "old"},
    {"id": 2, "year": 2024, "title": "mid"},
    {"id": 3, "year": 2025, "title": "new"}
]"#,
    )
    .unwrap();

    let config_json = format!(
        r#"{{
    "$config": [{{"serializer": "json", "layout": "index", "dest": "{}"}}],
    "papers": {{
        "since": {{
            "${{year}}": {{
                "$derive": {{ "field": "year", "type": "int" }},
                "$filter": [{{"field": "year", "op": "gte", "value": "{{year}}"}}],
                "$emit": ["list"]
            }}
        }}
    }}
}}"#,
        dest_dir.display()
    );
    fs::write(&config_file, &config_json).unwrap();

    let config: Config = Config::load_from_file(Path::new(&config_file)).unwrap();
    assert!(fauxrest::run(config, data_dir).is_ok());

    let since2024 = fs::read_to_string(dest_dir.join("papers/since/2024/index.json")).unwrap();
    assert!(since2024.contains("mid"), "gte 2024 should keep 2024");
    assert!(since2024.contains("new"), "gte 2024 should keep 2025");
    assert!(!since2024.contains("old"), "gte 2024 should drop 2023");
}

/// `$derive.type: "string"` goes the other way, stringifying a numeric field
/// so it can be matched against string-typed data.
#[test]
fn test_derive_string_type_stringifies_numeric_field() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().join("data");
    let dest_dir = tmp.path().join("dist");
    let config_file = tmp.path().join("fauxrest.json");

    fs::create_dir(&data_dir).unwrap();
    fs::write(
        data_dir.join("papers.json"),
        r#"[
    {"id": 1, "year": 2024, "label": "2024"},
    {"id": 2, "year": 2025, "label": "2025"}
]"#,
    )
    .unwrap();

    let config_json = format!(
        r#"{{
    "$config": [{{"serializer": "json", "layout": "index", "dest": "{}"}}],
    "papers": {{
        "${{year}}": {{
            "$derive": {{ "field": "year", "type": "string" }},
            "$filter": [{{"field": "label", "op": "eq", "value": "{{year}}"}}],
            "$emit": ["list"]
        }}
    }}
}}"#,
        dest_dir.display()
    );
    fs::write(&config_file, &config_json).unwrap();

    let config: Config = Config::load_from_file(Path::new(&config_file)).unwrap();
    assert!(fauxrest::run(config, data_dir).is_ok());

    let y2024 = fs::read_to_string(dest_dir.join("papers/2024/index.json")).unwrap();
    assert!(
        y2024.contains("\"id\": 1"),
        "2024 should keep the id 1 item"
    );
    assert!(
        !y2024.contains("\"id\": 2"),
        "2024 should drop the id 2 item"
    );
}

/// `$derive.type` accepts only `string` and `int`, and rejects the wider set
/// that 0.0.3 shipped. Rejecting at load time is the point: a config written
/// against `auto`/`float`/`bool` fails instead of quietly changing behaviour.
///
/// See `test_config_errors_name_what_is_wrong` for the message itself, which
/// now names the rejected variant.
#[test]
fn test_derive_type_accepts_only_string_and_int() {
    let config = |value: &str| {
        format!(
            r#"{{
    "$config": [{{"serializer": "json", "layout": "index", "dest": "dist"}}],
    "rooms": {{
        "${{code}}": {{
            "$derive": {{ "field": "code", "type": "{}" }}
        }}
    }}
}}"#,
            value
        )
    };

    for unsupported in ["auto", "float", "bool"] {
        assert!(
            Config::load_from_str(config(unsupported)).is_err(),
            "$derive.type {:?} should be rejected",
            unsupported
        );
    }

    for supported in ["string", "int"] {
        assert!(
            Config::load_from_str(config(supported)).is_ok(),
            "$derive.type {:?} should still load",
            supported
        );
    }
}

/// Values that cannot be converted to the requested `$derive.type` are
/// skipped rather than aborting the build, matching how non-derivable values
/// are already handled.
#[test]
fn test_derive_type_conversion_failure_skips_value() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().join("data");
    let dest_dir = tmp.path().join("dist");
    let config_file = tmp.path().join("fauxrest.json");

    fs::create_dir(&data_dir).unwrap();
    // Every `year` is a number, so the fixture isolates conversion failure
    // from the mixed-kind rejection: 2024.5 has no exact `int` form and is
    // skipped, while a string here would fail the run instead.
    fs::write(
        data_dir.join("papers.json"),
        r#"[
    {"id": 1, "year": 2024},
    {"id": 2, "year": 2024.5}
]"#,
    )
    .unwrap();

    let config_json = format!(
        r#"{{
    "$config": [{{"serializer": "json", "layout": "index", "dest": "{}"}}],
    "papers": {{
        "${{year}}": {{
            "$derive": {{ "field": "year", "type": "int" }},
            "$filter": [{{"field": "year", "op": "eq", "value": "{{year}}"}}],
            "$emit": ["list"]
        }}
    }}
}}"#,
        dest_dir.display()
    );
    fs::write(&config_file, &config_json).unwrap();

    let config: Config = Config::load_from_file(Path::new(&config_file)).unwrap();
    assert!(fauxrest::run(config, data_dir).is_ok());

    assert_file(&dest_dir, "papers/2024/index.json");
    assert!(
        !dest_dir.join("papers/2024.5").exists(),
        "non-convertible value should not produce an endpoint"
    );
}

/// A `$derive` without `type` keeps the pre-0.0.4 behavior: a `pattern`
/// extracts a string, and the raw field value is otherwise passed through
/// unchanged.
#[test]
fn test_derive_without_type_keeps_legacy_behavior() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().join("data");
    let dest_dir = tmp.path().join("dist");
    let config_file = tmp.path().join("fauxrest.json");

    fs::create_dir(&data_dir).unwrap();
    fs::write(
        data_dir.join("papers.json"),
        r#"[
    {"id": 1, "year": 2024, "from": "2024-04-01"},
    {"id": 2, "year": 2025, "from": "2025-04-01"}
]"#,
    )
    .unwrap();

    let config_json = format!(
        r#"{{
    "$config": [{{"serializer": "json", "layout": "index", "dest": "{}"}}],
    "papers": {{
        "${{year}}": {{
            "$derive": {{ "field": "from", "pattern": "^(\\d{{4}}).*" }},
            "$filter": [{{"field": "from", "op": "contains", "value": "{{year}}"}}],
            "$emit": ["list"]
        }}
    }}
}}"#,
        dest_dir.display()
    );
    fs::write(&config_file, &config_json).unwrap();

    let config: Config = Config::load_from_file(Path::new(&config_file)).unwrap();
    assert!(fauxrest::run(config, data_dir).is_ok());

    assert_contains(&dest_dir, "papers/2024/index.json", "2024-04-01");
    assert_contains(&dest_dir, "papers/2025/index.json", "2025-04-01");
}

#[test]
fn test_template_with_values_and_derive_is_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let config_file = tmp.path().join("fauxrest.json");
    let config_json = r#"{
    "$config": [{"serializer": "json", "layout": "index", "dest": "dist"}],
    "activities": {
        "${year}": {
            "$values": ["2024"],
            "$derive": "from"
        }
    }
}"#;
    fs::write(&config_file, config_json).unwrap();

    let err = match Config::load_from_file(Path::new(&config_file)) {
        Ok(_) => panic!("config should be rejected"),
        Err(e) => e,
    };
    assert!(format!("{}", err).contains("$values and $derive cannot be used together"));
}

#[test]
fn test_pick_directive_keeps_only_specified_fields() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().join("data");
    let dest_dir = tmp.path().join("dist");
    let config_file = tmp.path().join("fauxrest.json");

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
    "$config": [{{"serializer": "json", "layout": "index", "dest": "{}"}}],
    "users": {{
        "$pick": ["id", "name"]
    }}
}}"#,
        dest_dir.display()
    );
    fs::write(&config_file, &config_json).unwrap();

    let config: Config = Config::load_from_file(Path::new(&config_file)).unwrap();
    assert!(fauxrest::run(config, data_dir).is_ok());

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
    let config_file = tmp.path().join("fauxrest.json");

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
    "$config": [{{"serializer": "json", "layout": "index", "dest": "{}"}}],
    "users": {{
        "$omit": ["password", "email"]
    }}
}}"#,
        dest_dir.display()
    );
    fs::write(&config_file, &config_json).unwrap();

    let config: Config = Config::load_from_file(Path::new(&config_file)).unwrap();
    assert!(fauxrest::run(config, data_dir).is_ok());

    let content = fs::read_to_string(dest_dir.join("users/index.json")).unwrap();
    assert!(content.contains("\"id\""));
    assert!(content.contains("\"name\""));
    assert!(!content.contains("\"email\""));
    assert!(!content.contains("\"password\""));
}

#[test]
fn test_overlay_only_aggregate_endpoint_is_generated() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().join("data");
    let dest_dir = tmp.path().join("dist");
    let config_file = tmp.path().join("fauxrest.json");

    fs::create_dir(&data_dir).unwrap();
    fs::write(
        data_dir.join("users.json"),
        r#"[
        {"id": 1, "name": "Alice"},
        {"id": 2, "name": "Bob"}
    ]"#,
    )
    .unwrap();
    fs::write(
        data_dir.join("skills.json"),
        r#"[
        {"id": "s1", "label": "Rust"}
    ]"#,
    )
    .unwrap();

    let config_json = format!(
        r#"{{
        "$config": [{{"serializer": "json", "layout": "index", "dest": "{}"}}],
        "profile": {{
        "$aggregate": ["users", "skills"]
        }}
    }}"#,
        dest_dir.display()
    );
    fs::write(&config_file, &config_json).unwrap();

    let config: Config = Config::load_from_file(Path::new(&config_file)).unwrap();
    assert!(fauxrest::run(config, data_dir).is_ok());

    let content = fs::read_to_string(dest_dir.join("profile/index.json")).unwrap();
    assert!(content.contains("\"Alice\""));
    assert!(content.contains("\"Rust\""));

    let discovery = fs::read_to_string(dest_dir.join("index.json")).unwrap();
    assert!(discovery.contains("\"/profile\""));
}

#[test]
fn test_overlay_only_keyed_aggregate_endpoint_is_generated() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().join("data");
    let dest_dir = tmp.path().join("dist");
    let config_file = tmp.path().join("fauxrest.json");

    fs::create_dir(&data_dir).unwrap();
    fs::write(
        data_dir.join("users.json"),
        r#"[
    {"id": 1, "name": "Alice"}
]"#,
    )
    .unwrap();
    fs::write(
        data_dir.join("skills.json"),
        r#"[
    {"id": "s1", "label": "Rust"}
]"#,
    )
    .unwrap();

    let config_json = format!(
        r#"{{
    "$config": [{{"serializer": "json", "layout": "index", "dest": "{}"}}],
    "profile": {{
        "$aggregate": {{
            "mode": "keyed",
            "sources": [
                {{"from": "users", "as": "members"}},
                "skills"
            ]
        }}
    }}
}}"#,
        dest_dir.display()
    );
    fs::write(&config_file, &config_json).unwrap();

    let config: Config = Config::load_from_file(Path::new(&config_file)).unwrap();
    assert!(fauxrest::run(config, data_dir).is_ok());

    let content = fs::read_to_string(dest_dir.join("profile/index.json")).unwrap();
    assert!(content.contains("\"members\""));
    assert!(content.contains("\"skills\""));
    assert!(content.contains("\"Alice\""));
    assert!(content.contains("\"Rust\""));
}

#[test]
fn test_run_fails_when_dest_is_not_empty_without_overwrite() {
    let tmp = tempfile::tempdir().unwrap();
    let dist_dir = tmp.path().join("dist");
    fs::create_dir(&dist_dir).unwrap();
    // Pre-existing file makes the destination non-empty.
    fs::write(dist_dir.join("stale.json"), "{}").unwrap();

    let config = Config::new("json".into(), Layout::Index, &dist_dir);
    let err = fauxrest::run(config, "testdata/example1")
        .expect_err("run should fail when dest is not empty and overwrite is disabled");
    let msg = format!("{}", err);
    assert!(
        msg.contains("dest is not empty") && msg.contains("--overwrite"),
        "unexpected error message: {}",
        msg
    );
    // Nothing should have been generated.
    assert!(!dist_dir.join("index.json").exists());
}

#[test]
fn test_run_succeeds_when_dest_is_not_empty_with_overwrite() {
    let tmp = tempfile::tempdir().unwrap();
    let dist_dir = tmp.path().join("dist");
    fs::create_dir(&dist_dir).unwrap();
    fs::write(dist_dir.join("stale.json"), "{}").unwrap();

    let config = Config {
        serializers: vec![fauxrest::SerializerConfig {
            serializer: "json".into(),
            layout: Layout::Index,
            dest: dist_dir.clone(),
            minify: false,
            overwrite: true,
        }],
        api: std::collections::HashMap::new(),
        ..Default::default()
    };
    fauxrest::run(config, "testdata/example1")
        .expect("run should succeed when overwrite is enabled");
    assert_file(&dist_dir, "index.json");
    assert_file(&dist_dir, "profile/index.json");
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
        serializers: vec![fauxrest::SerializerConfig {
            serializer: "json".into(),
            layout: Layout::Index,
            dest: pretty_dir.clone(),
            minify: false,
            overwrite: false,
        }],
        api: std::collections::HashMap::new(),
        ..Default::default()
    };
    let minified = Config {
        serializers: vec![fauxrest::SerializerConfig {
            serializer: "json".into(),
            layout: Layout::Index,
            dest: minify_dir.clone(),
            minify: true,
            overwrite: false,
        }],
        api: std::collections::HashMap::new(),
        ..Default::default()
    };

    assert!(fauxrest::run(pretty, &data_dir).is_ok());
    assert!(fauxrest::run(minified, &data_dir).is_ok());

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
        serializers: vec![fauxrest::SerializerConfig {
            serializer: "typescript".into(),
            layout: Layout::Index,
            dest: pretty_dir.clone(),
            minify: false,
            overwrite: false,
        }],
        api: std::collections::HashMap::new(),
        ..Default::default()
    };
    let minified = Config {
        serializers: vec![fauxrest::SerializerConfig {
            serializer: "typescript".into(),
            layout: Layout::Index,
            dest: minify_dir.clone(),
            minify: true,
            overwrite: false,
        }],
        api: std::collections::HashMap::new(),
        ..Default::default()
    };

    assert!(fauxrest::run(pretty, &data_dir).is_ok());
    assert!(fauxrest::run(minified, &data_dir).is_ok());

    let pretty_text = fs::read_to_string(pretty_dir.join("users/index.ts")).unwrap();
    let minified_text = fs::read_to_string(minify_dir.join("users/index.ts")).unwrap();
    assert!(pretty_text.contains("\n  {"));
    assert!(!minified_text.contains("\n  {"));
    assert!(
        minified_text
            .contains("export const data = [{\"id\":1,\"name\":\"Bob\",\"team\":\"R&D\"}];")
    );
}

/// Sets up a data directory containing a JSON dataset alongside some static
/// files (in the root and a sub-directory) and returns the data/dest dirs.
fn setup_static_data(tmp: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let data_dir = tmp.join("data");
    let dest_dir = tmp.join("dist");
    fs::create_dir_all(data_dir.join("css")).unwrap();
    fs::create_dir_all(data_dir.join("secret")).unwrap();
    fs::write(data_dir.join("users.json"), r#"[{"id": 1, "name": "Bob"}]"#).unwrap();
    fs::write(data_dir.join("logo.png"), "PNGDATA").unwrap();
    fs::write(data_dir.join("notes.txt"), "hello").unwrap();
    fs::write(data_dir.join("css/site.css"), "body{}").unwrap();
    fs::write(data_dir.join("secret/key.pem"), "TOPSECRET").unwrap();
    (data_dir, dest_dir)
}

#[test]
fn test_static_files_are_not_copied_by_default() {
    let tmp = tempfile::tempdir().unwrap();
    let (data_dir, dest_dir) = setup_static_data(tmp.path());

    let config = Config::new("json".into(), Layout::Index, &dest_dir);
    fauxrest::run(config, &data_dir).expect("run should succeed");

    // The JSON dataset is generated ...
    assert!(dest_dir.join("users/index.json").exists());
    // ... but no static file is copied by default (deny by default).
    assert!(!dest_dir.join("logo.png").exists());
    assert!(!dest_dir.join("notes.txt").exists());
    assert!(!dest_dir.join("css/site.css").exists());
}

#[test]
fn test_static_files_copied_when_config_include_matches() {
    let tmp = tempfile::tempdir().unwrap();
    let (data_dir, dest_dir) = setup_static_data(tmp.path());

    let config_json = format!(
        r#"{{
    "$config": [{{"serializer": "json", "layout": "index", "dest": "{}"}}],
    "$static": ["*.png", "css/**"]
}}"#,
        dest_dir.display()
    );
    let config = Config::load_from_str(&config_json).unwrap();
    fauxrest::run(config, &data_dir).expect("run should succeed");

    // Included globs are copied, preserving sub-directory structure.
    assert!(dest_dir.join("logo.png").exists());
    assert!(dest_dir.join("css/site.css").exists());
    // Non-matching static files are not copied.
    assert!(!dest_dir.join("notes.txt").exists());
    // JSON files are never copied as static assets.
    assert!(!dest_dir.join("users.json").exists());
}

#[test]
fn test_static_exclude_wins_over_cli_allow_all() {
    let tmp = tempfile::tempdir().unwrap();
    let (data_dir, dest_dir) = setup_static_data(tmp.path());

    // Full form: no include, but an exclude for the secret directory. The CLI
    // allow-all flag (copy_static_all) is simulated by setting it on Config.
    let config_json = format!(
        r#"{{
    "$config": [{{"serializer": "json", "layout": "index", "dest": "{}"}}],
    "$static": {{"include": [], "exclude": ["secret/**"]}}
}}"#,
        dest_dir.display()
    );
    let mut config = Config::load_from_str(&config_json).unwrap();
    config.copy_static_all = true; // as if --copy-static was passed

    fauxrest::run(config, &data_dir).expect("run should succeed");

    // allow-all copies ordinary static files ...
    assert!(dest_dir.join("logo.png").exists());
    assert!(dest_dir.join("notes.txt").exists());
    assert!(dest_dir.join("css/site.css").exists());
    // ... but the exclude glob always wins, even under allow-all.
    assert!(!dest_dir.join("secret/key.pem").exists());
}

#[test]
fn test_static_invalid_glob_is_rejected() {
    let config_json = r#"{
    "$config": [{"serializer": "json", "layout": "index", "dest": "dist"}],
    "$static": ["a[b"]
}"#;
    let err = match Config::load_from_str(config_json) {
        Ok(_) => panic!("config with invalid $static glob should be rejected"),
        Err(e) => e,
    };
    assert!(format!("{}", err).contains("invalid $static glob"));
}

/// An unusable `$filter` pattern must be rejected at load time. Left to
/// runtime it failed open and emitted every record, including the ones the
/// condition existed to withhold.
#[test]
fn test_invalid_filter_regex_is_rejected() {
    let config_json = r#"{
    "$config": [{"serializer": "json", "layout": "index", "dest": "dist"}],
    "items": {
        "broken": {
            "$emit": ["list"],
            "$filter": [{"field": "name", "op": "regeq", "value": "([unclosed"}]
        }
    }
}"#;
    let err = match Config::load_from_str(config_json) {
        Ok(_) => panic!("config with an invalid $filter regex should be rejected"),
        Err(e) => e,
    };
    let msg = format!("{}", err);
    assert!(
        msg.contains("invalid $filter regeq pattern"),
        "unexpected error: {}",
        msg
    );
    assert!(
        msg.contains("items/broken"),
        "error should name the offending node: {}",
        msg
    );
}

/// A non-string `value` for a regex operator cannot be evaluated either, and
/// is caught by the same validation.
#[test]
fn test_non_string_regex_filter_value_is_rejected() {
    let config_json = r#"{
    "$config": [{"serializer": "json", "layout": "index", "dest": "dist"}],
    "items": {
        "broken": {
            "$emit": ["list"],
            "$filter": [{"field": "name", "op": "regneq", "value": 42}]
        }
    }
}"#;
    let err = match Config::load_from_str(config_json) {
        Ok(_) => panic!("regex filter with a numeric value should be rejected"),
        Err(e) => e,
    };
    let msg = format!("{}", err);
    assert!(
        msg.contains("requires a string value"),
        "unexpected error: {}",
        msg
    );
}

/// Rejecting unusable patterns must not reject working ones: a valid `regeq`
/// still narrows the collection to the matching records.
#[test]
fn test_valid_regex_filter_still_selects_matching_items() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().join("data");
    let dest_dir = tmp.path().join("dist");
    let config_file = tmp.path().join("fauxrest.json");

    fs::create_dir(&data_dir).unwrap();
    fs::write(
        data_dir.join("items.json"),
        r#"[{"id": 1, "name": "alpha"}, {"id": 2, "name": "beta"}]"#,
    )
    .unwrap();

    let config_json = format!(
        r#"{{
    "$config": [{{"serializer": "json", "layout": "index", "dest": "{}"}}],
    "items": {{
        "selected": {{
            "$emit": ["list"],
            "$filter": [{{"field": "name", "op": "regeq", "value": "^al"}}]
        }}
    }}
}}"#,
        dest_dir.display()
    );
    fs::write(&config_file, &config_json).unwrap();

    let config: Config = Config::load_from_file(Path::new(&config_file)).unwrap();
    fauxrest::run(config, &data_dir).expect("run should succeed");

    let content = fs::read_to_string(dest_dir.join("items/selected/index.json"))
        .expect("filtered endpoint should exist");
    assert!(
        content.contains("alpha"),
        "alpha should be kept: {}",
        content
    );
    assert!(
        !content.contains("beta"),
        "beta should have been filtered out: {}",
        content
    );
}

/// The motivating case for string ordering: a date-range endpoint over a field
/// that stores ISO-8601 dates as strings. `gte` used to accept every record,
/// since both operands collapsed to `0.0` and `0.0 >= 0.0` holds.
#[test]
fn test_date_range_endpoint_filters_on_string_dates() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().join("data");
    let dest_dir = tmp.path().join("dist");
    let config_file = tmp.path().join("fauxrest.json");

    fs::create_dir(&data_dir).unwrap();
    fs::write(
        data_dir.join("activities.json"),
        r#"[
    {"id": 1, "from": "2018-04-01", "label": "old"},
    {"id": 2, "from": "2021-09-15", "label": "recent"},
    {"id": 3, "from": "2026-10-16", "label": "newest"}
]"#,
    )
    .unwrap();

    let config_json = format!(
        r#"{{
    "$config": [{{"serializer": "json", "layout": "index", "dest": "{}"}}],
    "activities": {{
        "$emit": [],
        "since2020": {{
            "$emit": ["list"],
            "$filter": [{{"field": "from", "op": "gte", "value": "2020-01-01"}}]
        }},
        "before2020": {{
            "$emit": ["list"],
            "$filter": [{{"field": "from", "op": "lt", "value": "2020-01-01"}}]
        }}
    }}
}}"#,
        dest_dir.display()
    );
    fs::write(&config_file, &config_json).unwrap();

    let config: Config = Config::load_from_file(Path::new(&config_file)).unwrap();
    fauxrest::run(config, &data_dir).expect("run should succeed");

    let since = fs::read_to_string(dest_dir.join("activities/since2020/index.json")).unwrap();
    assert!(
        since.contains("recent"),
        "2021 is after the pivot: {}",
        since
    );
    assert!(
        since.contains("newest"),
        "2026 is after the pivot: {}",
        since
    );
    assert!(
        !since.contains("\"old\""),
        "2018 must be excluded, not swept in by a degenerate comparison: {}",
        since
    );

    let before = fs::read_to_string(dest_dir.join("activities/before2020/index.json")).unwrap();
    assert!(
        before.contains("\"old\""),
        "2018 is before the pivot: {}",
        before
    );
    assert!(
        !before.contains("recent"),
        "2021 must be excluded: {}",
        before
    );
}

/// Config errors must name what is actually wrong.
///
/// `$derive`, `$aggregate` and `$static` each accept two JSON shapes. While those
/// enums derived `#[serde(untagged)]`, serde tried both variants and, when both
/// failed, reported only "data did not match any variant of untagged enum ..."
/// — every mistake below produced that same sentence, naming none of them.
#[test]
fn test_config_errors_name_what_is_wrong() {
    let derive = |body: &str| {
        format!(
            r#"{{
    "$config": [{{"serializer": "json", "layout": "index", "dest": "dist"}}],
    "papers": {{"${{year}}": {{"$derive": {}}}}}
}}"#,
            body
        )
    };
    let static_spec = |body: &str| {
        format!(
            r#"{{
    "$config": [{{"serializer": "json", "layout": "index", "dest": "dist"}}],
    "$static": {}
}}"#,
            body
        )
    };
    let aggregate = |body: &str| {
        format!(
            r#"{{
    "$config": [{{"serializer": "json", "layout": "index", "dest": "dist"}}],
    "bundle": {{"$aggregate": {}}}
}}"#,
            body
        )
    };

    let cases: [(String, &str); 10] = [
        // The value rejected by the 0.0.4 narrowing has to be named, since a
        // config carried over from 0.0.3 fails on exactly this.
        (derive(r#"{"field": "year", "type": "auto"}"#), "auto"),
        (derive(r#"{"fields": "year"}"#), "missing field `field`"),
        (
            derive(r#"{"field": "year", "pattern": 42}"#),
            "invalid type",
        ),
        (derive("42"), "$derive must be a field name or an object"),
        (
            aggregate(r#"{"mode": "nested", "sources": ["papers"]}"#),
            "nested",
        ),
        (
            aggregate(r#"{"sources": [42]}"#),
            "$aggregate source entry must be",
        ),
        (
            aggregate(r#"{"sources": [{"form": "papers"}]}"#),
            "missing field `from`",
        ),
        (
            aggregate(r#""papers""#),
            "$aggregate must be a list of source paths or an object",
        ),
        (static_spec(r#"{"include": [42]}"#), "invalid type"),
        (
            static_spec("42"),
            "$static must be a list of globs or an object",
        ),
    ];

    for (config_json, expected) in cases {
        let err = match Config::load_from_str(&config_json) {
            Ok(_) => panic!("should be rejected: {}", config_json),
            Err(e) => format!("{}", e),
        };
        assert!(
            err.contains(expected),
            "error should mention {:?}, got: {}",
            expected,
            err
        );
        assert!(
            !err.contains("did not match any variant"),
            "error should not fall back to the untagged message, got: {}",
            err
        );
    }
}

/// Writes a `members.json` whose categories and generations both contain a
/// value that should not reach the output, and runs `api` over it.
fn members_fixture(tmp: &Path, api: &str) -> (PathBuf, PathBuf) {
    let data_dir = tmp.join("data");
    let dest_dir = tmp.join("dist");
    let config_file = tmp.join("fauxrest.json");

    fs::create_dir(&data_dir).unwrap();
    fs::write(
        data_dir.join("members.json"),
        r#"[
    {"id": 1, "name": "a", "category": "active", "generation": 0},
    {"id": 2, "name": "b", "category": "active", "generation": 10},
    {"id": 3, "name": "c", "category": "non_member", "generation": 3}
]"#,
    )
    .unwrap();

    let config_json = format!(
        r#"{{
    "$config": [{{"serializer": "json", "layout": "index", "dest": "{}"}}],
    "members": {}
}}"#,
        dest_dir.display(),
        api
    );
    fs::write(&config_file, &config_json).unwrap();

    (data_dir, dest_dir)
}

/// A record removed by the node's `$filter` must not bring a template
/// endpoint into existence. Deriving from pre-filter data used to publish
/// `/members/non_member` as an empty collection, showing the excluded
/// category name in the output.
#[test]
fn test_filtered_out_records_do_not_create_template_endpoints() {
    let tmp = tempfile::tempdir().unwrap();
    let (data_dir, dest_dir) = members_fixture(
        tmp.path(),
        r#"{
        "$filter": [{"field": "category", "op": "neq", "value": "non_member"}],
        "${category}": {
            "$derive": "category",
            "$filter": [{"field": "category", "op": "eq", "value": "{category}"}],
            "$emit": ["list"]
        }
    }"#,
    );

    let config: Config = Config::load_from_file(tmp.path().join("fauxrest.json")).unwrap();
    assert!(fauxrest::run(config, data_dir).is_ok());

    assert_file(&dest_dir, "members/active/index.json");
    assert!(
        !dest_dir.join("members/non_member").exists(),
        "a filtered-out category must not produce an endpoint"
    );
}

/// `$derive.exclude` drops the values it names, and only those. Excluding
/// generation 0 must leave 10 alone — the regex workarounds this replaces
/// truncated it to 1 or dropped it outright.
#[test]
fn test_derive_exclude_drops_only_named_values() {
    let tmp = tempfile::tempdir().unwrap();
    let (data_dir, dest_dir) = members_fixture(
        tmp.path(),
        r#"{
        "${generation}": {
            "$derive": {"field": "generation", "exclude": [0]},
            "$filter": [{"field": "generation", "op": "eq", "value": "{generation}"}],
            "$emit": ["list"]
        }
    }"#,
    );

    let config: Config = Config::load_from_file(tmp.path().join("fauxrest.json")).unwrap();
    assert!(fauxrest::run(config, data_dir).is_ok());

    assert!(
        !dest_dir.join("members/0").exists(),
        "excluded generation must not produce an endpoint"
    );
    assert_eq!(
        read_json(&dest_dir, "members/10/index.json"),
        serde_json::json!([{"id": 2, "name": "b", "category": "active", "generation": 10}]),
        "10 must survive an exclusion of 0"
    );
    assert_file(&dest_dir, "members/3/index.json");
}

/// `exclude` compares by JSON kind, matching how `$filter` compares. An
/// entry of the wrong kind excludes nothing rather than quietly matching.
#[test]
fn test_derive_exclude_compares_by_kind() {
    let tmp = tempfile::tempdir().unwrap();
    let (data_dir, dest_dir) = members_fixture(
        tmp.path(),
        r#"{
        "${generation}": {
            "$derive": {"field": "generation", "exclude": ["0"]},
            "$filter": [{"field": "generation", "op": "eq", "value": "{generation}"}],
            "$emit": ["list"]
        }
    }"#,
    );

    let config: Config = Config::load_from_file(tmp.path().join("fauxrest.json")).unwrap();
    assert!(fauxrest::run(config, data_dir).is_ok());

    assert_file(&dest_dir, "members/0/index.json");
}

/// `$omit` must not reach the data `$derive` reads. Hiding a field from the
/// output is about the payload, and letting it also erase the endpoints
/// derived from that field would delete them without saying so.
#[test]
fn test_omit_does_not_hide_the_derived_field() {
    let tmp = tempfile::tempdir().unwrap();
    let (data_dir, dest_dir) = members_fixture(
        tmp.path(),
        r#"{
        "$omit": ["generation"],
        "${generation}": {
            "$derive": "generation",
            "$filter": [{"field": "generation", "op": "eq", "value": "{generation}"}],
            "$emit": ["list"]
        }
    }"#,
    );

    let config: Config = Config::load_from_file(tmp.path().join("fauxrest.json")).unwrap();
    assert!(fauxrest::run(config, data_dir).is_ok());

    assert_file(&dest_dir, "members/0/index.json");
    assert_file(&dest_dir, "members/10/index.json");

    let root = read_json(&dest_dir, "members/index.json");
    assert!(
        !root.to_string().contains("generation"),
        "the omitted field must still be absent from the payload: {}",
        root
    );
}
