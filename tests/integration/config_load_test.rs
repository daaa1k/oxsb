// Integration tests for config loading.

use std::collections::HashMap;
use std::path::Path;

use oxsb::config::{load_config, load_config_dry};
use oxsb::error::OxsbError;

fn home_and_tmp_vars() -> HashMap<String, String> {
    let mut vars = HashMap::new();
    if let Some(home) = dirs::home_dir() {
        vars.insert("HOME".to_string(), home.to_string_lossy().into_owned());
    }
    vars.insert("CWD".to_string(), "/tmp".to_string());
    vars
}

#[test]
fn loads_minimal_config() {
    let path = Path::new("tests/fixtures/config_minimal.yaml");
    let vars = HashMap::new();
    let config = load_config(path, &vars).expect("should load minimal config");
    assert_eq!(config.write_allow.len(), 1);
    assert_eq!(config.write_allow[0].path, "/tmp");
}

#[test]
fn optional_missing_path_is_ignored() {
    let path = Path::new("tests/fixtures/config_optional_paths.yaml");
    let vars = home_and_tmp_vars();
    // Should succeed even though the second path does not exist.
    let config = load_config(path, &vars).expect("should load config with optional paths");
    assert_eq!(config.write_allow.len(), 3);
}

#[test]
fn required_missing_path_returns_error() {
    // Use a temp config with a non-existent required path.
    let dir = tempfile::tempdir().unwrap();
    let cfg_path = dir.path().join("bad.yaml");
    std::fs::write(
        &cfg_path,
        "write_allow:\n  - path: \"/absolutely/nonexistent/path/oxsb-test\"\n",
    )
    .unwrap();
    let vars = HashMap::new();
    let err = load_config(&cfg_path, &vars).unwrap_err();
    assert!(
        matches!(err, OxsbError::RequiredPathMissing { .. }),
        "expected RequiredPathMissing, got: {err}"
    );
}

#[test]
fn missing_config_file_returns_config_not_found() {
    let path = Path::new("/nonexistent/path/config.yaml");
    let vars = HashMap::new();
    let err = load_config(path, &vars).unwrap_err();
    assert!(
        matches!(err, OxsbError::ConfigNotFound { .. }),
        "expected ConfigNotFound, got: {err}"
    );
}

#[test]
fn invalid_yaml_returns_config_parse_error() {
    let dir = tempfile::tempdir().unwrap();
    let cfg_path = dir.path().join("invalid.yaml");
    std::fs::write(&cfg_path, "write_allow: [unclosed bracket").unwrap();
    let vars = HashMap::new();
    let err = load_config(&cfg_path, &vars).unwrap_err();
    assert!(
        matches!(err, OxsbError::ConfigParse(_)),
        "expected ConfigParse, got: {err}"
    );
}

#[test]
fn loads_full_config_env_vars() {
    let path = Path::new("tests/fixtures/config_full.yaml");
    let vars = home_and_tmp_vars();
    let config = load_config(path, &vars).expect("should load full config");
    assert_eq!(config.env.set.get("IN_SANDBOX"), Some(&"1".to_string()));
    assert_eq!(config.env.set.get("MY_VAR"), Some(&"hello".to_string()));
}

#[test]
fn loads_full_config_bubblewrap_extra_args() {
    let path = Path::new("tests/fixtures/config_full.yaml");
    let vars = home_and_tmp_vars();
    let config = load_config(path, &vars).expect("should load full config");
    assert_eq!(config.bubblewrap.extra_args, vec!["--share-net"]);
}

#[test]
fn dry_load_does_not_require_paths_to_exist() {
    let dir = tempfile::tempdir().unwrap();
    let cfg_path = dir.path().join("dry.yaml");
    std::fs::write(
        &cfg_path,
        "write_allow:\n  - path: \"/absolutely/nonexistent/path/oxsb-test\"\n",
    )
    .unwrap();
    let vars = HashMap::new();
    // load_config_dry should succeed without checking existence.
    let config = load_config_dry(&cfg_path, &vars).expect("dry load should not check paths");
    assert_eq!(config.write_allow[0].path, "/absolutely/nonexistent/path/oxsb-test");
}

#[test]
fn create_flag_creates_directory() {
    let dir = tempfile::tempdir().unwrap();
    let new_dir = dir.path().join("oxsb-create-test");
    assert!(!new_dir.exists());

    let cfg_path = dir.path().join("create.yaml");
    std::fs::write(
        &cfg_path,
        format!(
            "write_allow:\n  - path: \"{}\"\n    create: true\n",
            new_dir.to_string_lossy()
        ),
    )
    .unwrap();

    let vars = HashMap::new();
    load_config(&cfg_path, &vars).expect("should create directory");
    assert!(new_dir.exists(), "directory should have been created");
}

#[test]
fn touch_flag_creates_file() {
    let dir = tempfile::tempdir().unwrap();
    let new_file = dir.path().join("oxsb-touch-test.txt");
    assert!(!new_file.exists());

    let cfg_path = dir.path().join("touch.yaml");
    std::fs::write(
        &cfg_path,
        format!(
            "write_allow:\n  - path: \"{}\"\n    file: true\n    touch: true\n",
            new_file.to_string_lossy()
        ),
    )
    .unwrap();

    let vars = HashMap::new();
    load_config(&cfg_path, &vars).expect("should touch file");
    assert!(new_file.exists(), "file should have been created");
}
