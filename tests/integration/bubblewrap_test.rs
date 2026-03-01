// Integration tests for the bubblewrap backend.
//
// These tests require `bwrap` to be present on PATH. When it is not found
// the individual tests emit a diagnostic and return early, so the suite
// still passes in environments where bubblewrap is not installed (e.g.
// macOS CI or a plain Linux host without the package).

use oxsb::backend::bubblewrap::BubblewrapBackend;
use oxsb::config::Config;
use oxsb::env::{Environment, OsKind};

fn bwrap_available() -> bool {
    // Attempt a minimal real sandbox: if user namespaces are restricted (e.g. in
    // some container-based CI environments) this returns false and tests are skipped.
    std::process::Command::new("bwrap")
        .args(["--ro-bind", "/", "/", "--", "true"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn linux_env() -> Environment {
    Environment {
        os_kind: OsKind::Linux,
        xdg_runtime_dir: None,
        home_dir: Some("/home/test".to_string()),
    }
}

/// Verify that bwrap itself is functional by running a trivial sandboxed
/// command directly, without going through the oxsb backend.
#[test]
fn bwrap_runs_echo() {
    if !bwrap_available() {
        eprintln!("bwrap not found — skipping bwrap_runs_echo");
        return;
    }

    let output = std::process::Command::new("bwrap")
        .args([
            "--ro-bind", "/", "/",
            "--dev-bind", "/dev", "/dev",
            "--proc", "/proc",
            "--",
            "echo",
            "hello-from-sandbox",
        ])
        .output()
        .expect("failed to spawn bwrap");

    assert!(
        output.status.success(),
        "bwrap exited non-zero; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"hello-from-sandbox\n");
}

/// Verify that the argument list produced by `BubblewrapBackend::build_args`
/// is accepted by bwrap and yields the expected output.
#[test]
fn build_args_produces_working_bwrap_invocation() {
    if !bwrap_available() {
        eprintln!("bwrap not found — skipping build_args_produces_working_bwrap_invocation");
        return;
    }

    let backend = BubblewrapBackend;
    let config: Config = serde_yml::from_str("{}").unwrap();
    let env = linux_env();
    let args = backend.build_args("echo", &["sandbox-ok".to_string()], &config, &env, false);

    let output = std::process::Command::new("bwrap")
        .args(&args)
        .output()
        .expect("failed to spawn bwrap with generated args");

    assert!(
        output.status.success(),
        "bwrap exited non-zero; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        output.stdout,
        b"sandbox-ok\n",
        "unexpected stdout from sandboxed echo"
    );
}

/// Verify that write_allow paths are passed as --bind mounts and that the
/// resulting sandbox can write to that directory.
#[test]
fn write_allow_bind_mount_is_writable() {
    if !bwrap_available() {
        eprintln!("bwrap not found — skipping write_allow_bind_mount_is_writable");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let dir_path = dir.path().to_string_lossy().into_owned();

    let backend = BubblewrapBackend;
    let config: Config =
        serde_yml::from_str(&format!("write_allow:\n  - path: \"{dir_path}\"\n")).unwrap();
    let env = linux_env();
    let sentinel = format!("{dir_path}/bwrap-write-test");
    let args = backend.build_args("touch", std::slice::from_ref(&sentinel), &config, &env, false);

    let status = std::process::Command::new("bwrap")
        .args(&args)
        .status()
        .expect("failed to spawn bwrap");

    assert!(
        status.success(),
        "bwrap exited non-zero when writing to bound directory"
    );
    assert!(
        std::path::Path::new(&sentinel).exists(),
        "sentinel file was not created inside the sandbox"
    );
}

/// Verify that environment variables injected via `env.set` are visible
/// inside the sandbox.
#[test]
fn env_set_variables_visible_in_sandbox() {
    if !bwrap_available() {
        eprintln!("bwrap not found — skipping env_set_variables_visible_in_sandbox");
        return;
    }

    let backend = BubblewrapBackend;
    let config: Config =
        serde_yml::from_str("env:\n  set:\n    OXSB_TEST_VAR: \"hello-sandbox\"\n").unwrap();
    let env = linux_env();
    let args = backend.build_args(
        "sh",
        &["-c".to_string(), "echo $OXSB_TEST_VAR".to_string()],
        &config,
        &env,
        false,
    );

    let output = std::process::Command::new("bwrap")
        .args(&args)
        .output()
        .expect("failed to spawn bwrap");

    assert!(
        output.status.success(),
        "bwrap exited non-zero; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"hello-sandbox\n");
}
