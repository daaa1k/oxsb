// Integration tests for path variable expansion.

use std::collections::HashMap;

use oxsb::expand::expand_path;

fn vars() -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert("HOME".to_string(), "/home/testuser".to_string());
    m.insert("CWD".to_string(), "/work/repo".to_string());
    m.insert(
        "XDG_CONFIG_HOME".to_string(),
        "/home/testuser/.config".to_string(),
    );
    m
}

#[test]
fn integration_expand_home() {
    assert_eq!(
        expand_path("$HOME/.claude", &vars()).unwrap(),
        "/home/testuser/.claude"
    );
}

#[test]
fn integration_expand_cwd() {
    assert_eq!(expand_path("$CWD", &vars()).unwrap(), "/work/repo");
}

#[test]
fn integration_expand_xdg() {
    assert_eq!(
        expand_path("$XDG_CONFIG_HOME/app", &vars()).unwrap(),
        "/home/testuser/.config/app"
    );
}

#[test]
fn integration_expand_braced() {
    assert_eq!(
        expand_path("${HOME}/.local/share", &vars()).unwrap(),
        "/home/testuser/.local/share"
    );
}

#[test]
fn integration_unknown_var_error() {
    let result = expand_path("$NOPE/path", &vars());
    assert!(result.is_err());
}
