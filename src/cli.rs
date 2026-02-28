//! CLI argument definitions using clap.

use clap::Parser;

/// Cross-platform sandbox wrapper.
///
/// Wraps a command inside an OS-level sandbox (seatbelt on macOS,
/// bubblewrap on WSL2, landlock on Linux) based on a YAML configuration.
#[derive(Parser, Debug)]
#[command(name = "oxsb", version, about)]
pub struct Args {
    /// Path to the configuration file.
    ///
    /// Defaults to `~/.config/oxsb/config.yaml`.
    #[arg(long, short = 'c')]
    pub config: Option<String>,

    /// Override the backend to use.
    ///
    /// Valid values: `bubblewrap`, `landlock`, `seatbelt`, `none`.
    #[arg(long, short = 'b')]
    pub backend: Option<String>,

    /// Print the sandbox command without executing it.
    #[arg(long)]
    pub dry_run: bool,

    /// Enable verbose output.
    #[arg(long, short = 'v')]
    pub verbose: bool,

    /// The command to run inside the sandbox, followed by its arguments.
    ///
    /// Use `--` to separate oxsb options from the command:
    ///
    ///   oxsb -- claude --help
    #[arg(trailing_var_arg = true, required = true)]
    pub command: Vec<String>,
}

impl Args {
    /// Returns the command name (first element of `command`).
    pub fn cmd_name(&self) -> &str {
        &self.command[0]
    }

    /// Returns the command arguments (everything after the command name).
    pub fn cmd_args(&self) -> &[String] {
        &self.command[1..]
    }
}
