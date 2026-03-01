//! Linux Landlock sandbox backend.
//!
//! Uses the `landlock` crate to restrict filesystem write access to the
//! paths listed in `write_allow`, then replaces the current process with
//! the target command.
//!
//! - ABI V5 (kernel 6.1+) is preferred; `CompatLevel::BestEffort` ensures
//!   graceful degradation on older kernels.
//! - All filesystem paths are granted read access; write access is limited
//!   to the `write_allow` list.

#[cfg(target_os = "linux")]
use landlock::{
    Access, AccessFs, CompatLevel, Compatible, PathBeneath, PathFd, Ruleset, RulesetAttr,
    RulesetCreatedAttr, ABI,
};

#[cfg(target_os = "linux")]
use std::os::unix::process::CommandExt;

use crate::backend::SandboxBackend;
use crate::config::Config;
use crate::env::Environment;
use crate::error::{OxsbError, Result};

/// Linux Landlock sandbox backend.
pub struct LandlockBackend;

impl SandboxBackend for LandlockBackend {
    fn execute(
        &self,
        command: &str,
        args: &[String],
        config: &Config,
        _env: &Environment,
        dry_run: bool,
        verbose: bool,
    ) -> Result<()> {
        if dry_run {
            // Display what would be sandboxed.
            println!(
                "landlock: restrict_self() then execve {:?} {:?}",
                command, args
            );
            if verbose {
                for entry in &config.write_allow {
                    println!("  write_allow: {}", entry.path);
                }
            }
            return Ok(());
        }

        #[cfg(target_os = "linux")]
        {
            apply_landlock(config, verbose)?;
            // Replace the current process via execve(2). No shell interpolation occurs.
            let err = std::process::Command::new(command).args(args).exec();
            Err(OxsbError::ExecFailed(err.to_string()))
        }

        #[cfg(not(target_os = "linux"))]
        Err(OxsbError::BackendUnavailable {
            backend: "landlock".to_string(),
        })
    }
}

/// Apply Landlock restrictions to the current thread/process.
#[cfg(target_os = "linux")]
fn apply_landlock(config: &Config, verbose: bool) -> Result<()> {
    use std::path::Path;

    let abi = ABI::V5;

    let ll_err = |e: &dyn std::fmt::Display| OxsbError::SandboxSetupFailed(e.to_string());

    let mut ruleset = Ruleset::default()
        .set_compatibility(CompatLevel::BestEffort)
        .handle_access(AccessFs::from_all(abi))
        .map_err(|e| ll_err(&e))?
        .create()
        .map_err(|e| ll_err(&e))?;

    // Grant read access to the entire filesystem.
    let root = PathFd::new("/").map_err(|e| ll_err(&e))?;
    ruleset = ruleset
        .add_rule(PathBeneath::new(root, AccessFs::from_read(abi)))
        .map_err(|e| ll_err(&e))?;

    // Grant write access to each allowed path.
    for entry in &config.write_allow {
        let p = Path::new(&entry.path);
        if !p.exists() && entry.optional {
            if verbose {
                eprintln!("[oxsb] skipping optional missing path: {}", entry.path);
            }
            continue;
        }
        let fd = PathFd::new(&entry.path).map_err(|e| ll_err(&e))?;
        ruleset = ruleset
            .add_rule(PathBeneath::new(fd, AccessFs::from_all(abi)))
            .map_err(|e| ll_err(&e))?;
    }

    ruleset.restrict_self().map_err(|e| ll_err(&e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::env::{Environment, OsKind};

    fn env_linux() -> Environment {
        Environment::with(OsKind::Linux, None, Some("/home/test".to_string()))
    }

    fn config_from(yaml: &str) -> Config {
        serde_yml::from_str(yaml).unwrap()
    }

    #[test]
    fn dry_run_returns_ok() {
        let backend = LandlockBackend;
        let config = config_from("write_allow:\n  - path: \"/tmp\"\n");
        let env = env_linux();
        let result = backend.execute("echo", &["hello".to_string()], &config, &env, true, false);
        assert!(result.is_ok());
    }

    #[test]
    fn dry_run_verbose_returns_ok() {
        let backend = LandlockBackend;
        let config = config_from("write_allow:\n  - path: \"/tmp\"\n");
        let env = env_linux();
        let result = backend.execute("echo", &[], &config, &env, true, true);
        assert!(result.is_ok());
    }
}
