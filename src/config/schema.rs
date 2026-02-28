//! YAML configuration schema types.
//!
//! All types in this module implement `serde::Deserialize` so they can be
//! loaded directly from a YAML configuration file.

use std::collections::HashMap;

use serde::Deserialize;

/// A single path entry in the `write_allow` list.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct PathEntry {
    /// The path expression. May contain `$HOME`, `$CWD`, and XDG variables.
    pub path: String,

    /// If `true`, a missing path is silently ignored rather than causing an error.
    #[serde(default)]
    pub optional: bool,

    /// If `true`, the entry refers to a file (not a directory).
    /// Affects sandbox profile generation (e.g., seatbelt `literal` vs `subpath`).
    #[serde(default)]
    pub file: bool,

    /// If `true`, create the directory (and parents) when it does not exist.
    #[serde(default)]
    pub create: bool,

    /// If `true`, `touch` the file when it does not exist.
    #[serde(default)]
    pub touch: bool,
}

/// Backend auto-selection configuration.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct BackendAutoConfig {
    /// Whether to auto-detect the backend from the current OS/environment.
    #[serde(default = "default_true")]
    pub auto: bool,

    /// Override backend for plain Linux (non-WSL2).
    pub linux: Option<String>,

    /// Override backend for WSL2.
    pub wsl2: Option<String>,

    /// Override backend for macOS.
    pub macos: Option<String>,
}

impl Default for BackendAutoConfig {
    fn default() -> Self {
        Self {
            auto: true,
            linux: None,
            wsl2: None,
            macos: None,
        }
    }
}

/// Bubblewrap-specific configuration.
#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
pub struct BubblewrapConfig {
    /// Additional raw arguments passed verbatim to `bwrap`.
    #[serde(default)]
    pub extra_args: Vec<String>,
}

/// Seatbelt-specific configuration.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct SeatbeltConfig {
    /// If `true`, a `.sb` profile is dynamically generated from `write_allow`.
    #[serde(default = "default_true")]
    pub generate_profile: bool,
}

impl Default for SeatbeltConfig {
    fn default() -> Self {
        Self {
            generate_profile: true,
        }
    }
}

/// Environment variable injection configuration.
#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
pub struct EnvConfig {
    /// Environment variables to inject into the sandboxed process.
    #[serde(default)]
    pub set: HashMap<String, String>,
}

/// Top-level configuration loaded from `config.yaml`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Config {
    /// Backend selection rules.
    #[serde(default)]
    pub backend: BackendAutoConfig,

    /// Paths that the sandboxed process is allowed to write to.
    #[serde(default)]
    pub write_allow: Vec<PathEntry>,

    /// Bubblewrap-specific settings.
    #[serde(default)]
    pub bubblewrap: BubblewrapConfig,

    /// Seatbelt-specific settings.
    #[serde(default)]
    pub seatbelt: SeatbeltConfig,

    /// Environment variable injection settings.
    #[serde(default)]
    pub env: EnvConfig,
}

fn default_true() -> bool {
    true
}
