//! YAML configuration loading, validation, and filesystem side-effects.
//!
//! This module is responsible for:
//! 1. Reading and deserializing the YAML config file.
//! 2. Expanding path variables in each `PathEntry`.
//! 3. Creating directories / touching files as specified by `create`/`touch`.
//! 4. Validating that required paths exist (when `optional: false`).

use std::collections::HashMap;
use std::path::Path;

use crate::config::schema::Config;
use crate::error::{OxsbError, Result};
use crate::expand::expand_path;

/// Load, expand, validate, and apply filesystem side-effects for a config file.
///
/// Steps performed:
/// 1. Read and parse the YAML file at `path`.
/// 2. Expand path variables in every `PathEntry` using `vars`.
/// 3. For entries with `create: true`, create the directory tree.
/// 4. For entries with `touch: true`, create the file if absent.
/// 5. Verify that non-optional paths exist.
///
/// The returned `Config` contains fully-expanded path strings.
pub fn load_config(path: &Path, vars: &HashMap<String, String>) -> Result<Config> {
    let content = std::fs::read_to_string(path).map_err(|_| OxsbError::ConfigNotFound {
        path: path.to_string_lossy().into_owned(),
    })?;

    let mut config: Config = serde_yml::from_str(&content)?;

    for entry in &mut config.write_allow {
        entry.path = expand_path(&entry.path, vars)?;

        let p = Path::new(&entry.path);

        if entry.create && !entry.file {
            std::fs::create_dir_all(p)?;
        }

        if entry.touch && entry.file {
            if !p.exists() {
                // Create parent directory if needed
                if let Some(parent) = p.parent() {
                    if !parent.exists() {
                        std::fs::create_dir_all(parent)?;
                    }
                }
                std::fs::File::create(p)?;
            }
        }

        if !entry.optional && !p.exists() {
            return Err(OxsbError::RequiredPathMissing {
                path: entry.path.clone(),
            });
        }
    }

    Ok(config)
}

/// Load config with validation but without creating directories or touching files.
///
/// Useful for `--dry-run` where we want to validate the config without
/// modifying the filesystem.
pub fn load_config_dry(path: &Path, vars: &HashMap<String, String>) -> Result<Config> {
    let content = std::fs::read_to_string(path).map_err(|_| OxsbError::ConfigNotFound {
        path: path.to_string_lossy().into_owned(),
    })?;

    let mut config: Config = serde_yml::from_str(&content)?;

    for entry in &mut config.write_allow {
        entry.path = expand_path(&entry.path, vars)?;
    }

    Ok(config)
}
