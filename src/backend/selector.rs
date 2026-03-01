//! Backend auto-selection logic.
//!
//! Selects the appropriate `SandboxBackend` implementation based on:
//! 1. An explicit CLI `--backend` flag (highest priority).
//! 2. Per-platform overrides in the configuration file.
//! 3. Automatic OS detection (lowest priority).

use crate::backend::{
    bubblewrap::BubblewrapBackend, landlock::LandlockBackend, none::NoneBackend,
    seatbelt::SeatbeltBackend, SandboxBackend,
};
use crate::config::Config;
use crate::env::{Environment, OsKind};
use crate::error::{OxsbError, Result};

/// Select a backend by explicit name string.
///
/// Valid names: `"bubblewrap"`, `"landlock"`, `"seatbelt"`, `"none"`.
///
/// # Errors
///
/// Returns `OxsbError::BackendUnavailable` if `name` is not a recognised backend identifier.
pub fn backend_from_name(name: &str) -> Result<Box<dyn SandboxBackend>> {
    match name {
        "bubblewrap" => Ok(Box::new(BubblewrapBackend)),
        "landlock" => Ok(Box::new(LandlockBackend)),
        "seatbelt" => Ok(Box::new(SeatbeltBackend)),
        "none" => Ok(Box::new(NoneBackend)),
        other => Err(OxsbError::BackendUnavailable {
            backend: other.to_string(),
        }),
    }
}

/// Select the appropriate backend for the current environment.
///
/// Priority order:
/// 1. `cli_backend` — explicit `--backend` flag on the CLI.
/// 2. Per-platform config overrides (`config.backend.linux`, `.wsl2`, `.macos`).
/// 3. Auto-detection from `env.os_kind`.
///
/// # Errors
///
/// Returns `OxsbError::BackendUnavailable` if a named backend (from CLI or config) is not recognised.
pub fn select_backend(
    cli_backend: Option<&str>,
    config: &Config,
    env: &Environment,
) -> Result<Box<dyn SandboxBackend>> {
    // 1. CLI override
    if let Some(name) = cli_backend {
        return backend_from_name(name);
    }

    // 2. Config-file per-platform override
    if config.backend.auto {
        let config_name: Option<&str> = match &env.os_kind {
            OsKind::MacOs => config.backend.macos.as_deref(),
            OsKind::Wsl2 => config.backend.wsl2.as_deref(),
            OsKind::Linux => config.backend.linux.as_deref(),
            OsKind::Other => None,
        };
        if let Some(name) = config_name {
            return backend_from_name(name);
        }
    }

    // 3. Auto-detection
    match &env.os_kind {
        OsKind::MacOs => Ok(Box::new(SeatbeltBackend)),
        OsKind::Wsl2 => Ok(Box::new(BubblewrapBackend)),
        OsKind::Linux => Ok(Box::new(LandlockBackend)),
        OsKind::Other => Ok(Box::new(NoneBackend)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::env::{Environment, OsKind};

    fn default_config() -> Config {
        serde_yml::from_str("{}").unwrap()
    }

    fn env_for(os: OsKind) -> Environment {
        Environment::with(os, None, Some("/home/test".to_string()))
    }

    #[test]
    fn cli_backend_none_overrides_all() {
        let config = default_config();
        let env = env_for(OsKind::MacOs);
        let backend = select_backend(Some("none"), &config, &env).unwrap();
        // NoneBackend dry-run should succeed without side effects
        let result = backend.execute("echo", &[], &config, &env, true, false);
        assert!(result.is_ok());
    }

    #[test]
    fn auto_detects_seatbelt_on_macos() {
        let config = default_config();
        let env = env_for(OsKind::MacOs);
        // Just verify selection doesn't error; we can't test exec here
        let _backend = select_backend(None, &config, &env).unwrap();
    }

    #[test]
    fn auto_detects_bubblewrap_on_wsl2() {
        let config = default_config();
        let env = env_for(OsKind::Wsl2);
        let _backend = select_backend(None, &config, &env).unwrap();
    }

    #[test]
    fn auto_detects_landlock_on_linux() {
        let config = default_config();
        let env = env_for(OsKind::Linux);
        let _backend = select_backend(None, &config, &env).unwrap();
    }

    #[test]
    fn unknown_backend_name_returns_error() {
        let result = backend_from_name("invalid-backend");
        assert!(matches!(result, Err(OxsbError::BackendUnavailable { .. })));
    }

    #[test]
    fn config_override_applies_before_autodetect() {
        let config: Config =
            serde_yml::from_str("backend:\n  auto: true\n  macos: none\n").unwrap();
        let env = env_for(OsKind::MacOs);
        // Config says "none" for macOS — should select NoneBackend
        let backend = select_backend(None, &config, &env).unwrap();
        let result = backend.execute("echo", &[], &config, &env, true, false);
        assert!(result.is_ok());
    }
}
