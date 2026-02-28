//! Bubblewrap (`bwrap`) sandbox backend.
//!
//! Constructs a `bwrap` invocation from the oxsb configuration and replaces
//! the current process with the sandboxed command via Unix execve(2).
//!
//! # Argument layout
//!
//! ```text
//! bwrap
//!   --ro-bind / /          (read-only root)
//!   --dev-bind /dev /dev   (device access)
//!   --proc /proc           (procfs)
//!   --bind <path> <path>   (write_allow entries)
//!   --bind <xdg_runtime>   (if present)
//!   --setenv KEY VALUE     (env.set entries)
//!   [extra_args]
//!   -- <command> [args]
//! ```

use std::os::unix::process::CommandExt;
use std::path::Path;

use crate::backend::SandboxBackend;
use crate::config::Config;
use crate::env::Environment;
use crate::error::{OxsbError, Result};

/// Bubblewrap sandbox backend.
pub struct BubblewrapBackend;

impl BubblewrapBackend {
    /// Build the full argument list to pass to `bwrap`.
    pub fn build_args(
        &self,
        command: &str,
        args: &[String],
        config: &Config,
        env: &Environment,
        verbose: bool,
    ) -> Vec<String> {
        let mut bwrap_args: Vec<String> = Vec::new();

        // Fixed base mounts
        bwrap_args.extend(["--ro-bind", "/", "/"].map(String::from));
        bwrap_args.extend(["--dev-bind", "/dev", "/dev"].map(String::from));
        bwrap_args.extend(["--proc", "/proc"].map(String::from));

        // Write-allow paths
        for entry in &config.write_allow {
            let p = Path::new(&entry.path);
            if !p.exists() {
                if entry.optional {
                    if verbose {
                        eprintln!("[oxsb] skipping optional missing path: {}", entry.path);
                    }
                    continue;
                }
            }
            bwrap_args.extend(["--bind".to_string(), entry.path.clone(), entry.path.clone()]);
        }

        // XDG_RUNTIME_DIR (Linux-only, may not exist)
        if let Some(ref runtime) = env.xdg_runtime_dir {
            if Path::new(runtime).exists() {
                bwrap_args.extend(["--bind".to_string(), runtime.clone(), runtime.clone()]);
            }
        }

        // Environment variable injection
        for (key, value) in &config.env.set {
            bwrap_args.extend(["--setenv".to_string(), key.clone(), value.clone()]);
        }

        // Extra args from config
        bwrap_args.extend(config.bubblewrap.extra_args.iter().cloned());

        // Separator + command + its args
        bwrap_args.push("--".to_string());
        bwrap_args.push(command.to_string());
        bwrap_args.extend(args.iter().cloned());

        bwrap_args
    }
}

impl SandboxBackend for BubblewrapBackend {
    fn execute(
        &self,
        command: &str,
        args: &[String],
        config: &Config,
        env: &Environment,
        dry_run: bool,
        verbose: bool,
    ) -> Result<()> {
        let bwrap_args = self.build_args(command, args, config, env, verbose);

        if dry_run {
            let parts: Vec<&str> = std::iter::once("bwrap")
                .chain(bwrap_args.iter().map(String::as_str))
                .collect();
            println!("{}", parts.join(" "));
            return Ok(());
        }

        // Replace the current process with bwrap via execve(2).
        // Arguments are passed as a list — no shell interpolation occurs.
        let err = std::process::Command::new("bwrap")
            .args(&bwrap_args)
            .exec();

        Err(OxsbError::ExecFailed(err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::env::{Environment, OsKind};

    fn env_linux() -> Environment {
        Environment::with(OsKind::Linux, None, Some("/home/test".to_string()))
    }

    fn config_with_write_allow(yaml: &str) -> Config {
        serde_yaml::from_str(yaml).unwrap()
    }

    #[test]
    fn build_args_contains_base_mounts() {
        let backend = BubblewrapBackend;
        let config = config_with_write_allow("{}");
        let env = env_linux();
        let args = backend.build_args("echo", &[], &config, &env, false);

        assert!(args.windows(2).any(|w| w == ["--ro-bind", "/"]), "should have --ro-bind /");
        assert!(args.windows(2).any(|w| w == ["--dev-bind", "/dev"]), "should have --dev-bind /dev");
        assert!(args.contains(&"--proc".to_string()), "should have --proc");
    }

    #[test]
    fn build_args_appends_write_allow_paths() {
        let backend = BubblewrapBackend;
        let config = config_with_write_allow("write_allow:\n  - path: \"/tmp\"\n");
        let env = env_linux();
        let args = backend.build_args("echo", &[], &config, &env, false);

        let has_bind_tmp = args.windows(3).any(|w| {
            w[0] == "--bind" && w[1] == "/tmp" && w[2] == "/tmp"
        });
        assert!(has_bind_tmp, "should bind /tmp: {args:?}");
    }

    #[test]
    fn build_args_skips_optional_missing_path() {
        let backend = BubblewrapBackend;
        let config = config_with_write_allow(
            "write_allow:\n  - path: \"/nonexistent/oxsb-test\"\n    optional: true\n",
        );
        let env = env_linux();
        let args = backend.build_args("echo", &[], &config, &env, false);

        assert!(
            !args.contains(&"/nonexistent/oxsb-test".to_string()),
            "optional missing path should be skipped"
        );
    }

    #[test]
    fn build_args_injects_env_vars() {
        let backend = BubblewrapBackend;
        let config = config_with_write_allow("env:\n  set:\n    IN_SANDBOX: \"1\"\n");
        let env = env_linux();
        let args = backend.build_args("echo", &[], &config, &env, false);

        let has_setenv = args.windows(3).any(|w| {
            w[0] == "--setenv" && w[1] == "IN_SANDBOX" && w[2] == "1"
        });
        assert!(has_setenv, "should inject IN_SANDBOX=1: {args:?}");
    }

    #[test]
    fn build_args_ends_with_command() {
        let backend = BubblewrapBackend;
        let config = config_with_write_allow("{}");
        let env = env_linux();
        let args = backend.build_args("mycommand", &["arg1".to_string()], &config, &env, false);

        let sep_idx = args.iter().position(|a| a == "--").expect("should have --");
        assert_eq!(args[sep_idx + 1], "mycommand");
        assert_eq!(args[sep_idx + 2], "arg1");
    }

    #[test]
    fn build_args_includes_extra_args() {
        let backend = BubblewrapBackend;
        let config = config_with_write_allow("bubblewrap:\n  extra_args: [\"--share-net\"]\n");
        let env = env_linux();
        let args = backend.build_args("echo", &[], &config, &env, false);

        assert!(args.contains(&"--share-net".to_string()), "should include extra args");
    }

    #[test]
    fn dry_run_returns_ok() {
        let backend = BubblewrapBackend;
        let config = config_with_write_allow("write_allow:\n  - path: \"/tmp\"\n");
        let env = env_linux();
        let result = backend.execute("echo", &["hello".to_string()], &config, &env, true, false);
        assert!(result.is_ok());
    }
}
