// build.rs — macOS SDK library path detection.
//
// On macOS 26+ (Xcode 26 SDK), `libiconv` and other system libraries are not
// automatically found by the linker unless the SDK `usr/lib` directory is in
// the library search path. This script probes known SDK locations and emits a
// `rustc-link-search` directive so that `cargo build` works without requiring
// LIBRARY_PATH or SDKROOT to be set manually in the shell environment.
//
// This is a macOS SDK structure issue, unrelated to Xcode license agreements.

fn main() {
    #[cfg(target_os = "macos")]
    find_macos_sdk();
}

#[cfg(target_os = "macos")]
fn find_macos_sdk() {
    // 1. Prefer the path reported by xcrun (respects active Xcode and xcode-select).
    if let Ok(output) = std::process::Command::new("xcrun")
        .args(["--sdk", "macosx", "--show-sdk-path"])
        .output()
    {
        if output.status.success() {
            let sdk = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let lib = format!("{sdk}/usr/lib");
            if std::path::Path::new(&lib).exists() {
                println!("cargo:rustc-link-search={lib}");
                return;
            }
        }
    }

    // 2. Fallback: probe well-known Xcode and Command Line Tools locations.
    let candidates = [
        "/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk",
        "/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk",
    ];
    for sdk in &candidates {
        let lib = format!("{sdk}/usr/lib");
        if std::path::Path::new(&lib).exists() {
            println!("cargo:rustc-link-search={lib}");
            return;
        }
    }

    println!("cargo:warning=Could not locate macOS SDK usr/lib; linker may fail to find libiconv.");
}
