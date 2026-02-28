//! Path variable expansion utilities.
//!
//! Expands shell-like variables (`$HOME`, `$CWD`, `$XDG_*`) in path strings
//! using a provided variable map. Unknown variables produce an error rather
//! than being left unexpanded, ensuring configuration mistakes surface early.

use std::collections::HashMap;

use crate::error::{OxsbError, Result};

/// Expands all `$VAR` and `${VAR}` occurrences in `path` using `vars`.
///
/// Variables are looked up in `vars` by their bare name (without the leading
/// `$`). Returns `OxsbError::UnknownVariable` if a variable is referenced but
/// not present in the map.
pub fn expand_path(path: &str, vars: &HashMap<String, String>) -> Result<String> {
    let mut result = String::with_capacity(path.len());
    let chars: Vec<char> = path.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '$' {
            i += 1; // skip '$'

            if i < chars.len() && chars[i] == '{' {
                // ${VAR} form
                i += 1; // skip '{'
                let start = i;
                while i < chars.len() && chars[i] != '}' {
                    i += 1;
                }
                let var_name: String = chars[start..i].iter().collect();
                if i < chars.len() {
                    i += 1; // skip '}'
                }
                match vars.get(&var_name) {
                    Some(val) => result.push_str(val),
                    None => return Err(OxsbError::UnknownVariable { var: var_name }),
                }
            } else {
                // $VAR form — variable name ends at the next non-alphanumeric/non-underscore
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let var_name: String = chars[start..i].iter().collect();
                if var_name.is_empty() {
                    // Lone '$' with no variable name — pass through literally
                    result.push('$');
                } else {
                    match vars.get(&var_name) {
                        Some(val) => result.push_str(val),
                        None => return Err(OxsbError::UnknownVariable { var: var_name }),
                    }
                }
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    Ok(result)
}

/// Builds the default variable map from the current environment.
///
/// Includes `HOME`, `CWD` (current working directory), and all XDG Base
/// Directory variables (`XDG_CONFIG_HOME`, `XDG_CACHE_HOME`,
/// `XDG_DATA_HOME`, `XDG_STATE_HOME`, `XDG_RUNTIME_DIR`).
pub fn default_vars() -> HashMap<String, String> {
    let mut vars = HashMap::new();

    if let Some(home) = dirs::home_dir() {
        vars.insert("HOME".to_string(), home.to_string_lossy().into_owned());
    }

    if let Ok(cwd) = std::env::current_dir() {
        vars.insert("CWD".to_string(), cwd.to_string_lossy().into_owned());
    }

    // XDG Base Directory Specification
    let xdg_defaults = [
        ("XDG_CONFIG_HOME", ".config"),
        ("XDG_CACHE_HOME", ".cache"),
        ("XDG_DATA_HOME", ".local/share"),
        ("XDG_STATE_HOME", ".local/state"),
    ];

    if let Some(home) = dirs::home_dir() {
        for (key, subdir) in &xdg_defaults {
            let val = std::env::var(key)
                .unwrap_or_else(|_| home.join(subdir).to_string_lossy().into_owned());
            vars.insert(key.to_string(), val);
        }
    }

    if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
        vars.insert("XDG_RUNTIME_DIR".to_string(), runtime);
    }

    vars
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_vars() -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("HOME".to_string(), "/home/user".to_string());
        m.insert("CWD".to_string(), "/work/project".to_string());
        m.insert("XDG_CONFIG_HOME".to_string(), "/home/user/.config".to_string());
        m
    }

    #[test]
    fn expands_dollar_var() {
        let vars = simple_vars();
        assert_eq!(
            expand_path("$HOME/.claude", &vars).unwrap(),
            "/home/user/.claude"
        );
    }

    #[test]
    fn expands_braced_var() {
        let vars = simple_vars();
        assert_eq!(
            expand_path("${HOME}/.claude", &vars).unwrap(),
            "/home/user/.claude"
        );
    }

    #[test]
    fn expands_multiple_vars() {
        let vars = simple_vars();
        assert_eq!(
            expand_path("$HOME/$CWD", &vars).unwrap(),
            "/home/user//work/project"
        );
    }

    #[test]
    fn expands_cwd() {
        let vars = simple_vars();
        assert_eq!(expand_path("$CWD", &vars).unwrap(), "/work/project");
    }

    #[test]
    fn expands_xdg_config_home() {
        let vars = simple_vars();
        assert_eq!(
            expand_path("$XDG_CONFIG_HOME/app", &vars).unwrap(),
            "/home/user/.config/app"
        );
    }

    #[test]
    fn no_variables_passes_through() {
        let vars = simple_vars();
        assert_eq!(
            expand_path("/absolute/path/no/vars", &vars).unwrap(),
            "/absolute/path/no/vars"
        );
    }

    #[test]
    fn unknown_variable_returns_error() {
        let vars = simple_vars();
        let err = expand_path("$UNKNOWN_VAR/path", &vars).unwrap_err();
        match err {
            OxsbError::UnknownVariable { var } => assert_eq!(var, "UNKNOWN_VAR"),
            other => panic!("Expected UnknownVariable, got: {other}"),
        }
    }

    #[test]
    fn unknown_braced_variable_returns_error() {
        let vars = simple_vars();
        let err = expand_path("${MYSTERY}/path", &vars).unwrap_err();
        match err {
            OxsbError::UnknownVariable { var } => assert_eq!(var, "MYSTERY"),
            other => panic!("Expected UnknownVariable, got: {other}"),
        }
    }

    #[test]
    fn lone_dollar_passes_through() {
        let vars = simple_vars();
        assert_eq!(expand_path("$", &vars).unwrap(), "$");
    }

    #[test]
    fn var_at_end_of_path() {
        let vars = simple_vars();
        assert_eq!(expand_path("/prefix/$HOME", &vars).unwrap(), "/prefix//home/user");
    }

    #[test]
    fn default_vars_contains_home_and_cwd() {
        let vars = default_vars();
        assert!(vars.contains_key("HOME"), "should contain HOME");
        assert!(vars.contains_key("CWD"), "should contain CWD");
    }

    #[test]
    fn default_vars_contains_xdg_keys() {
        let vars = default_vars();
        assert!(vars.contains_key("XDG_CONFIG_HOME"));
        assert!(vars.contains_key("XDG_CACHE_HOME"));
        assert!(vars.contains_key("XDG_DATA_HOME"));
        assert!(vars.contains_key("XDG_STATE_HOME"));
    }
}
