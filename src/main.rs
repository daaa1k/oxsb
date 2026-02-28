//! oxsb — cross-platform sandbox wrapper entry point.

use anyhow::{Context, Result};
use clap::Parser;

// Re-use library modules rather than re-declaring them.
use oxsb::backend;
use oxsb::config;
use oxsb::env;
use oxsb::expand;
mod cli;

use backend::select_backend;
use cli::Args;
use config::{load_config, load_config_dry};
use env::Environment;
use expand::default_vars;

fn main() -> Result<()> {
    let args = Args::parse();

    let config_path = resolve_config_path(args.config.as_deref());

    let vars = default_vars();
    let environment = Environment::detect();

    let config = if args.dry_run {
        load_config_dry(&config_path, &vars)
            .with_context(|| format!("Failed to load config: {}", config_path.display()))?
    } else {
        load_config(&config_path, &vars)
            .with_context(|| format!("Failed to load config: {}", config_path.display()))?
    };

    if args.verbose {
        eprintln!("[oxsb] os_kind: {:?}", environment.os_kind);
        eprintln!("[oxsb] config: {}", config_path.display());
        eprintln!("[oxsb] backend override: {:?}", args.backend);
    }

    let backend = select_backend(args.backend.as_deref(), &config, &environment)
        .with_context(|| "Failed to select backend")?;

    backend
        .execute(
            args.cmd_name(),
            args.cmd_args(),
            &config,
            &environment,
            args.dry_run,
            args.verbose,
        )
        .with_context(|| format!("Backend execution failed for command: {}", args.cmd_name()))?;

    Ok(())
}

/// Resolve the config file path.
///
/// Priority:
/// 1. `--config` CLI flag.
/// 2. `~/.config/oxsb/config.yaml` (XDG default).
fn resolve_config_path(cli_path: Option<&str>) -> std::path::PathBuf {
    if let Some(p) = cli_path {
        return std::path::PathBuf::from(p);
    }
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("oxsb")
        .join("config.yaml")
}
