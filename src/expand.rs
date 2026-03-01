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
/// `$`). A lone `$` with no following identifier is passed through literally.
///
/// # Errors
///
/// Returns `OxsbError::UnknownVariable` if a variable is referenced but not present in `vars`.
pub fn expand_path(path: &str, vars: &HashMap<String, String>) -> Result<String> {
    let mut result = String::with_capacity(path.len());
    let mut chars = path.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '$' {
            result.push(ch);
            continue;
        }

        if chars.peek() == Some(&'{') {
            // ${VAR} form
            chars.next(); // consume '{'
            let mut var_name = String::new();
            while let Some(&c) = chars.peek() {
                if c == '}' {
                    chars.next(); // consume '}'
                    break;
                }
                var_name.push(c);
                chars.next();
            }
            match vars.get(&var_name) {
                Some(val) => result.push_str(val),
                None => return Err(OxsbError::UnknownVariable { var: var_name }),
            }
        } else {
            // $VAR form — variable name ends at the next non-alphanumeric/non-underscore
            let mut var_name = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_alphanumeric() || c == '_' {
                    var_name.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
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

    // Resolve home once; used for both HOME and XDG fallbacks below.
    let home = dirs::home_dir();

    if let Some(ref h) = home {
        vars.insert("HOME".to_string(), h.to_string_lossy().into_owned());
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

    if let Some(ref h) = home {
        for (key, subdir) in &xdg_defaults {
            let val = std::env::var(key)
                .unwrap_or_else(|_| h.join(subdir).to_string_lossy().into_owned());
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
        m.insert(
            "XDG_CONFIG_HOME".to_string(),
            "/home/user/.config".to_string(),
        );
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
        assert_eq!(
            expand_path("/prefix/$HOME", &vars).unwrap(),
            "/prefix//home/user"
        );
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
