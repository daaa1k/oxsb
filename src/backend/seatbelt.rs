//! macOS seatbelt (`sandbox-exec`) backend.
//!
//! Dynamically generates a TrustOS `.sb` sandbox profile from the
//! `write_allow` configuration entries and invokes `sandbox-exec -f <profile>`
//! to replace the current process.
//!
//! The generated profile:
//! - Denies all writes by default.
//! - Allows writes to each entry in `write_allow`.
//! - Uses `(literal "…")` for `file: true` entries.
//! - Uses `(subpath "…")` for directory entries.
//! - Resolves macOS symlinks (e.g., `/tmp` → `/private/tmp`).

use std::os::unix::process::CommandExt;

use crate::backend::SandboxBackend;
use crate::config::Config;
use crate::env::Environment;
use crate::error::{OxsbError, Result};

/// macOS seatbelt backend.
pub struct SeatbeltBackend;

impl SeatbeltBackend {
    /// Generate a TrustOS `.sb` profile from `config.write_allow`.
    pub fn generate_profile(&self, config: &Config) -> String {
        let mut sb = String::new();
        sb.push_str("(version 1)\n");
        sb.push_str("(deny default)\n");
        sb.push_str("(allow file-read*)\n");
        sb.push_str("(allow process*)\n");
        sb.push_str("(allow signal)\n");
        sb.push_str("(allow network*)\n");
        sb.push_str("(allow ipc*)\n");
        sb.push_str("(allow mach*)\n");
        sb.push_str("(allow sysctl*)\n");

        for entry in &config.write_allow {
            let resolved = resolve_macos_symlink(&entry.path);
            if entry.file {
                sb.push_str(&format!("(allow file-write* (literal \"{resolved}\"))\n"));
            } else {
                sb.push_str(&format!("(allow file-write* (subpath \"{resolved}\"))\n"));
            }
        }

        sb
    }

    /// Build `-D KEY=VALUE` env args for `sandbox-exec`.
    pub fn build_env_args(&self, config: &Config) -> Vec<String> {
        let mut sargs: Vec<String> = Vec::new();
        for (key, value) in &config.env.set {
            sargs.extend(["-D".to_string(), format!("{key}={value}")]);
        }
        sargs
    }
}

/// Resolve macOS-specific symlinks in well-known paths.
///
/// On macOS, `/tmp` is a symlink to `/private/tmp` and `/var` to `/private/var`.
/// The seatbelt profile must reference the canonical path for rules to take effect.
fn resolve_macos_symlink(path: &str) -> String {
    if let Ok(canonical) = std::fs::canonicalize(path) {
        return canonical.to_string_lossy().into_owned();
    }
    if path.starts_with("/tmp/") || path == "/tmp" {
        return path.replacen("/tmp", "/private/tmp", 1);
    }
    if path.starts_with("/var/") || path == "/var" {
        return path.replacen("/var", "/private/var", 1);
    }
    path.to_string()
}

impl SandboxBackend for SeatbeltBackend {
    fn execute(
        &self,
        command: &str,
        args: &[String],
        config: &Config,
        _env: &Environment,
        dry_run: bool,
        verbose: bool,
    ) -> Result<()> {
        let profile_content = if config.seatbelt.generate_profile {
            self.generate_profile(config)
        } else {
            return Err(OxsbError::BackendUnavailable {
                backend: "seatbelt (generate_profile: false not yet supported)".to_string(),
            });
        };

        if verbose {
            eprintln!("[oxsb] seatbelt profile:\n{profile_content}");
        }

        let profile_path = format!("/tmp/oxsb-{}.sb", std::process::id());

        if dry_run {
            let env_args = self.build_env_args(config);
            let mut parts = vec!["sandbox-exec".to_string()];
            parts.extend(env_args);
            parts.extend(["-f".to_string(), profile_path, "--".to_string()]);
            parts.push(command.to_string());
            parts.extend(args.iter().cloned());
            println!("{}", parts.join(" "));
            if verbose {
                println!("--- seatbelt profile ---\n{profile_content}");
            }
            return Ok(());
        }

        std::fs::write(&profile_path, &profile_content)?;
        // Use scopeguard::guard to clean up the temp profile file on scope exit.
        let _guard = scopeguard::guard(profile_path.clone(), |p| {
            let _ = std::fs::remove_file(&p);
        });

        let mut sandbox_args = self.build_env_args(config);
        sandbox_args.extend(["-f".to_string(), profile_path]);
        sandbox_args.push("--".to_string());
        sandbox_args.push(command.to_string());
        sandbox_args.extend(args.iter().cloned());

        // Replace the current process via execve(2). No shell interpolation occurs.
        let err = std::process::Command::new("sandbox-exec")
            .args(&sandbox_args)
            .exec();

        Err(OxsbError::ExecFailed(err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::env::{Environment, OsKind};

    fn env_macos() -> Environment {
        Environment::with(OsKind::MacOs, None, Some("/Users/test".to_string()))
    }

    fn config_from(yaml: &str) -> Config {
        serde_yml::from_str(yaml).unwrap()
    }

    #[test]
    fn generate_profile_has_deny_default() {
        let backend = SeatbeltBackend;
        let config = config_from("{}");
        let profile = backend.generate_profile(&config);
        assert!(
            profile.contains("(deny default)"),
            "profile should deny by default"
        );
    }

    #[test]
    fn generate_profile_subpath_for_directory() {
        let backend = SeatbeltBackend;
        let config = config_from("write_allow:\n  - path: \"/Users/test/.config\"\n");
        let profile = backend.generate_profile(&config);
        assert!(
            profile.contains("(subpath \"/Users/test/.config\")"),
            "directory should use subpath: {profile}"
        );
    }

    #[test]
    fn generate_profile_literal_for_file() {
        let backend = SeatbeltBackend;
        let config =
            config_from("write_allow:\n  - path: \"/Users/test/.claude.json\"\n    file: true\n");
        let profile = backend.generate_profile(&config);
        assert!(
            profile.contains("(literal \"/Users/test/.claude.json\")"),
            "file entry should use literal: {profile}"
        );
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn generate_profile_resolves_tmp_symlink() {
        let backend = SeatbeltBackend;
        let config = config_from("write_allow:\n  - path: \"/tmp\"\n");
        let profile = backend.generate_profile(&config);
        assert!(
            profile.contains("/private/tmp"),
            "should resolve /tmp to /private/tmp: {profile}"
        );
    }

    #[test]
    fn build_env_args_formats_d_flag() {
        let backend = SeatbeltBackend;
        let config = config_from("env:\n  set:\n    IN_SANDBOX: \"1\"\n");
        let args = backend.build_env_args(&config);
        assert!(args.contains(&"-D".to_string()));
        assert!(args.contains(&"IN_SANDBOX=1".to_string()));
    }

    #[test]
    fn dry_run_returns_ok() {
        let backend = SeatbeltBackend;
        let config = config_from("write_allow:\n  - path: \"/tmp\"\n");
        let env = env_macos();
        let result = backend.execute("echo", &["hello".to_string()], &config, &env, true, false);
        assert!(result.is_ok());
    }

    #[test]
    fn dry_run_verbose_shows_profile() {
        let backend = SeatbeltBackend;
        let config = config_from("write_allow:\n  - path: \"/tmp\"\n");
        let env = env_macos();
        let result = backend.execute("echo", &[], &config, &env, true, true);
        assert!(result.is_ok());
    }
}
