//! Sandbox backend abstraction.
//!
//! Each backend implements the `SandboxBackend` trait, which provides a
//! single `execute` method responsible for sandboxing and exec-ing the
//! target command.

pub mod bubblewrap;
pub mod landlock;
pub mod none;
pub mod seatbelt;
pub mod selector;

pub use selector::select_backend;

use crate::config::Config;
use crate::env::Environment;
use crate::error::Result;

/// Abstraction over OS-level sandbox mechanisms.
///
/// Implementations must `exec`-replace the current process when `dry_run` is
/// `false`. When `dry_run` is `true`, the implementation must print the command
/// that *would* be executed and return `Ok(())`.
pub trait SandboxBackend {
    /// Execute `command` with `args` inside the sandbox defined by `config`.
    ///
    /// - `dry_run = true`: print the sandbox command to stdout and return.
    /// - `dry_run = false`: exec-replace the process; this function never returns on success.
    /// - `verbose = true`: emit additional diagnostic output.
    fn execute(
        &self,
        command: &str,
        args: &[String],
        config: &Config,
        env: &Environment,
        dry_run: bool,
        verbose: bool,
    ) -> Result<()>;
}
