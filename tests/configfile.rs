use std::fs;

use envish::{ConfigFile, NEW_CONFIG_TEMPLATE};
use tempfile::tempdir;

#[test]
fn missing_config_reads_as_empty() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("missing.toml");
    let file = ConfigFile::new(path);

    assert_eq!(file.read().unwrap(), "");
}

#[test]
fn ensure_exists_creates_default_section() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("nested").join("config.toml");
    let file = ConfigFile::new(path.clone());

    assert!(!path.exists());
    file.ensure_exists().unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), NEW_CONFIG_TEMPLATE);

    // Second call leaves an existing file alone.
    fs::write(&path, "[dev]\nA = \"1\"\n").unwrap();
    file.ensure_exists().unwrap();
    assert_eq!(fs::read_to_string(path).unwrap(), "[dev]\nA = \"1\"\n");
}

#[test]
fn write_read_roundtrip_creates_parent_dirs() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("nested").join("config.toml");
    let file = ConfigFile::new(path.clone());

    let content = "[dev]\nFOO = \"bar\"\n";
    file.write(content).unwrap();

    assert_eq!(file.read().unwrap(), content);
    assert_eq!(fs::read_to_string(path).unwrap(), content);
}

#[test]
fn path_returns_configured_location() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let file = ConfigFile::new(path.clone());

    assert_eq!(file.path(), path.as_path());
}
