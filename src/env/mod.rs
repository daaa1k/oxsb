//! Runtime environment detection for oxsb.
//!
//! Provides the `Environment` struct that captures platform information and
//! resolved paths used throughout the backend implementations.

pub mod detect;

pub use detect::{detect_os, is_wsl2, OsKind};

/// Captured runtime environment information.
///
/// Created once at startup and passed to backend implementations.
#[derive(Debug, Clone)]
pub struct Environment {
    /// Detected operating system / virtualization context.
    pub os_kind: OsKind,

    /// Path to `XDG_RUNTIME_DIR`, if available (Linux-only).
    pub xdg_runtime_dir: Option<String>,

    /// Resolved home directory path.
    pub home_dir: Option<String>,
}

impl Environment {
    /// Detect and capture the current runtime environment.
    pub fn detect() -> Self {
        let os_kind = detect_os();
        let xdg_runtime_dir = std::env::var("XDG_RUNTIME_DIR").ok();
        let home_dir = dirs::home_dir().map(|p| p.to_string_lossy().into_owned());

        Self {
            os_kind,
            xdg_runtime_dir,
            home_dir,
        }
    }

    /// Create an `Environment` with explicitly provided values (useful for testing).
    #[cfg(test)]
    pub fn with(
        os_kind: OsKind,
        xdg_runtime_dir: Option<String>,
        home_dir: Option<String>,
    ) -> Self {
        Self {
            os_kind,
            xdg_runtime_dir,
            home_dir,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_does_not_panic() {
        let env = Environment::detect();
        // home_dir should be resolvable in CI and dev environments
        assert!(env.home_dir.is_some(), "home_dir should be set");
    }

    #[test]
    fn with_constructor_stores_values() {
        let env = Environment::with(
            OsKind::MacOs,
            Some("/run/user/1000".to_string()),
            Some("/home/test".to_string()),
        );
        assert_eq!(env.os_kind, OsKind::MacOs);
        assert_eq!(env.xdg_runtime_dir, Some("/run/user/1000".to_string()));
        assert_eq!(env.home_dir, Some("/home/test".to_string()));
    }
}
