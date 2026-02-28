//! Pass-through backend — no sandboxing applied.
//!
//! Useful for testing, platforms where no sandbox is available, or when the
//! user explicitly requests `--backend none`.

use std::os::unix::process::CommandExt;

use crate::backend::SandboxBackend;
use crate::config::Config;
use crate::env::Environment;
use crate::error::{OxsbError, Result};

/// Executes the command directly without any sandboxing.
pub struct NoneBackend;

impl SandboxBackend for NoneBackend {
    fn execute(
        &self,
        command: &str,
        args: &[String],
        _config: &Config,
        _env: &Environment,
        dry_run: bool,
        _verbose: bool,
    ) -> Result<()> {
        if dry_run {
            let cmd_str = std::iter::once(command)
                .chain(args.iter().map(String::as_str))
                .collect::<Vec<_>>()
                .join(" ");
            println!("{cmd_str}");
            return Ok(());
        }

        // Safety: CommandExt::exec() is Unix execve(2) — replaces the current
        // process image. This is NOT a shell invocation; the command and args
        // are passed directly to the OS without shell interpretation.
        let err = std::process::Command::new(command).args(args).exec();

        Err(OxsbError::ExecFailed(err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::env::{Environment, OsKind};

    fn dummy_config() -> Config {
        serde_yml::from_str("{}").unwrap()
    }

    fn dummy_env() -> Environment {
        Environment::with(OsKind::MacOs, None, Some("/home/test".to_string()))
    }

    #[test]
    fn dry_run_prints_command() {
        let backend = NoneBackend;
        let result = backend.execute(
            "echo",
            &["hello".to_string(), "world".to_string()],
            &dummy_config(),
            &dummy_env(),
            true,
            false,
        );
        assert!(result.is_ok());
    }
}
