//! OS and WSL2 detection logic.

/// The detected operating system / virtualization context.
#[derive(Debug, Clone, PartialEq)]
pub enum OsKind {
    /// Running on macOS (Darwin).
    MacOs,
    /// Running on Linux inside WSL2.
    Wsl2,
    /// Running on plain Linux (not WSL2).
    Linux,
    /// Unknown / unsupported platform.
    Other,
}

/// Detect the current OS kind.
///
/// WSL2 detection is performed by checking:
/// 1. Whether `/proc/version` contains the string `"microsoft"` (case-insensitive).
/// 2. Whether the `WSL_DISTRO_NAME` environment variable is set.
///
/// These checks are only compiled in on Linux; on macOS the function
/// unconditionally returns `OsKind::MacOs`.
pub fn detect_os() -> OsKind {
    #[cfg(target_os = "macos")]
    {
        OsKind::MacOs
    }
    #[cfg(target_os = "linux")]
    {
        if is_wsl2() {
            OsKind::Wsl2
        } else {
            OsKind::Linux
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        OsKind::Other
    }
}

/// Returns `true` when running inside WSL2.
///
/// Checks `/proc/version` for the `"microsoft"` substring, falling back to
/// the `WSL_DISTRO_NAME` environment variable.
#[cfg(target_os = "linux")]
pub fn is_wsl2() -> bool {
    if std::env::var("WSL_DISTRO_NAME").is_ok() {
        return true;
    }
    if let Ok(content) = std::fs::read_to_string("/proc/version") {
        if content.to_lowercase().contains("microsoft") {
            return true;
        }
    }
    false
}

#[cfg(not(target_os = "linux"))]
pub fn is_wsl2() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_os_returns_some_kind() {
        // This test just verifies that detect_os() returns a value without
        // panicking. The exact variant depends on the host platform.
        let kind = detect_os();
        assert!(matches!(
            kind,
            OsKind::MacOs | OsKind::Wsl2 | OsKind::Linux | OsKind::Other
        ));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn detect_os_is_macos_on_macos() {
        assert_eq!(detect_os(), OsKind::MacOs);
    }

    #[test]
    fn wsl2_detection_without_env_var() {
        // On non-Linux hosts, is_wsl2() must return false.
        // On Linux, the result depends on the actual environment, so we only
        // assert it doesn't panic.
        let _ = is_wsl2();
    }
}
