# oxsb — Cross-Platform Sandbox Wrapper

A CLI tool that wraps commands inside OS-level sandboxes using a unified
YAML configuration. Backend selection is automatic based on OS detection.

## Build and Test

```sh
cargo build
cargo test
cargo clippy
```

## Architecture

```
src/
  main.rs           # Entrypoint: parse CLI → load config → select backend → execute
  cli.rs            # clap Args definition
  error.rs          # OxsbError (thiserror) + Result alias
  expand.rs         # $HOME/$CWD/$XDG_* variable substitution
  config/
    schema.rs       # Deserialize types: Config, PathEntry, BackendAutoConfig, …
    loader.rs       # load_config() / load_config_dry()
    mod.rs          # Re-exports
  env/
    detect.rs       # OsKind enum + detect_os() + is_wsl2()
    mod.rs          # Environment struct + detect()
  backend/
    mod.rs          # SandboxBackend trait
    selector.rs     # select_backend() — CLI > config > auto-detect
    none.rs         # Pass-through (no sandbox)
    bubblewrap.rs   # bwrap backend (WSL2 / explicit)
    seatbelt.rs     # sandbox-exec backend (macOS)
    landlock.rs     # Landlock backend (Linux, cfg(linux))
```

## Key Invariants

- `SandboxBackend::execute()` must either exec-replace the process (dry_run=false)
  or print the command and return `Ok(())` (dry_run=true). It never returns `Ok`
  after a successful exec.
- All path variables are expanded before filesystem operations.
- Optional paths missing from disk are silently skipped (verbose mode logs them).
- Landlock backend is only compiled on `target_os = "linux"`.

## Config File Location

Default: `~/.config/oxsb/config.yaml`
Override: `oxsb --config /path/to/config.yaml`

## All comments and doc-comments must be written in English.
