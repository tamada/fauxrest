use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use prest::Generator;

/// Helper to create a temporary directory.
fn temp_dir() -> TempDir {
    tempfile::tempdir().unwrap()
}

/// Helper to write a file inside a directory.
fn write_file(dir: &Path, filename: &str, content: &str) -> PathBuf {
    let file_path = dir.join(filename);
    fs::write(&file_path, content).unwrap();
    file_path
}

/// Helper to assert that a file exists and has content containing a substring.
fn assert_contains(dest: &Path, rel_path: &str, expected: &str) {
    let file = dest.join(rel_path);
    assert!(file.exists(), "File {:?} should exist", file);
    let content = fs::read_to_string(file).unwrap();
    assert!(content.contains(expected));
}

/// Helper to assert that a file does not exist.
fn assert_missing(dest: &Path, rel_path: &str) {
    let file = dest.join(rel_path);
    assert!(!file.exists(), "File {:?} should not exist", file);
}

#[test]
fn test_profile_generation() {
    let dest = temp_dir();
    let generator = Generator::new(Path::new("testdata"), dest.path());
    assert!(generator.generate().is_ok());
    assert_contains(dest.path(), "profile/index.json", "Alice");
    assert_contains(dest.path(), "profile/index.json", "Developer");
}

#[test]
fn test_users_generation() {
    let dest = temp_dir();
    let generator = Generator::new(Path::new("testdata"), dest.path());
    assert!(generator.generate().is_ok());
    assert_contains(dest.path(), "users/index.json", "Bob");
    assert_contains(dest.path(), "users/1/index.json", "Designer");
    assert_contains(dest.path(), "users/2/index.json", "Manager");
}

#[test]
fn test_inputs_not_found() {
    let dest = temp_dir();
    let generator = Generator::new(Path::new("non_existent_folder_xyz"), dest.path());
    assert!(generator.generate().is_err());
}

#[test]
fn test_empty_inputs() {
    let in_dir = temp_dir();
    let dest = temp_dir();
    let generator = Generator::new(in_dir.path(), dest.path());
    assert!(generator.generate().is_ok());
    assert_eq!(fs::read_dir(dest.path()).unwrap().count(), 0);
}

#[test]
fn test_single_file_input() {
    let in_dir = temp_dir();
    let file = write_file(in_dir.path(), "data.json", r#"{"id": 1}"#);
    let dest = temp_dir();
    let generator = Generator::new(&file, dest.path());
    assert!(generator.generate().is_ok());
    assert_contains(dest.path(), "data/index.json", "id");
}

#[test]
fn test_ignore_non_json() {
    let in_dir = temp_dir();
    write_file(in_dir.path(), "test.txt", "hello");
    let dest = temp_dir();
    let generator = Generator::new(in_dir.path(), dest.path());
    assert!(generator.generate().is_ok());
    assert_missing(dest.path(), "test/index.json");
}

#[test]
fn test_array_no_id() {
    let in_dir = temp_dir();
    write_file(in_dir.path(), "items.json", r#"[{"name": "item_a"}]"#);
    let dest = temp_dir();
    let generator = Generator::new(in_dir.path(), dest.path());
    assert!(generator.generate().is_ok());
    assert_contains(dest.path(), "items/index.json", "item_a");
    assert_missing(dest.path(), "items/1/index.json");
}

#[test]
fn test_array_invalid_id() {
    let in_dir = temp_dir();
    write_file(in_dir.path(), "items.json", r#"[{"id": []}]"#);
    let dest = temp_dir();
    let generator = Generator::new(in_dir.path(), dest.path());
    assert!(generator.generate().is_ok());
    assert_contains(dest.path(), "items/index.json", "id");
    assert_missing(dest.path(), "items/[]/index.json");
}

#[test]
fn test_invalid_json() {
    let in_dir = temp_dir();
    write_file(in_dir.path(), "bad.json", "{invalid}");
    let dest = temp_dir();
    let generator = Generator::new(in_dir.path(), dest.path());
    assert!(generator.generate().is_err());
}
