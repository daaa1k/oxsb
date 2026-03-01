# oxsb

**Cross-platform sandbox wrapper** that runs commands inside OS-level sandboxes using a unified YAML configuration. The backend is selected automatically based on the detected OS.

| Platform | Backend | Mechanism |
|----------|---------|-----------|
| macOS | seatbelt | `sandbox-exec` with a generated `.sb` profile |
| Linux | landlock | Landlock LSM via the `landlock` crate |
| WSL2 | bubblewrap | `bwrap` with bind mounts |
| Any | none | Pass-through (no sandboxing) |

## Installation

### Cargo

```sh
cargo install --path .
```

### Nix (flake)

```sh
nix profile install github:daaa1k/oxsb
```

#### Home Manager module

```nix
# flake.nix
inputs.oxsb.url = "github:daaa1k/oxsb";

# home.nix
{ inputs, ... }: {
  imports = [ inputs.oxsb.homeManagerModules.default ];
  programs.oxsb = {
    enable = true;
    settings = {
      backend.auto = true;
      write_allow = [
        { path = "$HOME/.config"; }
        { path = "/tmp"; }
      ];
      env.set.IN_SANDBOX = "1";
    };
  };
}
```

## Usage

```sh
# Run a command inside the sandbox using the default config
oxsb -- claude --help

# Specify a custom config file
oxsb --config /path/to/config.yaml -- my-command

# Override the backend
oxsb --backend landlock -- my-command

# Preview the sandbox command without executing
oxsb --dry-run -- my-command

# Verbose output (logs skipped optional paths, etc.)
oxsb --verbose -- my-command
```

## Configuration

Default location: `~/.config/oxsb/config.yaml`

```yaml
backend:
  auto: true          # auto-detect based on OS (default)
  # linux: landlock   # override per-platform
  # wsl2: bubblewrap
  # macos: seatbelt

write_allow:
  - path: "$HOME/.config"
  - path: "$HOME/.cache"
  - path: "$HOME/.local/share"
  - path: "$HOME/.local/state"
    create: true              # create directory if missing
  - path: "$HOME/.claude.json"
    file: true                # treat as a file, not a directory
    touch: true               # create empty file if missing
  - path: "/tmp"
  - path: "/nix"
    optional: true            # silently skip if path does not exist

bubblewrap:
  extra_args: ["--share-net"] # extra flags passed to bwrap

seatbelt:
  generate_profile: true      # generate .sb profile from write_allow

env:
  set:
    IN_SANDBOX: "1"           # inject environment variables
```

### Path variables

The following variables are expanded in `path` values:

| Variable | Expands to |
|----------|-----------|
| `$HOME` | Home directory |
| `$CWD` | Current working directory |
| `$XDG_CONFIG_HOME` | `$HOME/.config` |
| `$XDG_CACHE_HOME` | `$HOME/.cache` |
| `$XDG_DATA_HOME` | `$HOME/.local/share` |
| `$XDG_STATE_HOME` | `$HOME/.local/state` |
| `$XDG_RUNTIME_DIR` | Runtime directory (Linux) |

Both `$VAR` and `${VAR}` syntax are supported.

### PathEntry options

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `path` | string | — | Path to allow writes to (variables expanded) |
| `optional` | bool | `false` | Skip silently if path does not exist |
| `create` | bool | `false` | Create as directory if missing |
| `file` | bool | `false` | Treat as a file (affects seatbelt profile rule) |
| `touch` | bool | `false` | Create as empty file if missing (requires `file: true`) |

## Development

```sh
cargo build
cargo test
cargo clippy
cargo fmt
```

## License

MIT
