// build.rs — macOS SDK path detection.
//
// On macOS, `xcrun` is normally used by rustc to find the SDK.
// When the Xcode license has not been agreed to, `xcrun` fails and the
// linker cannot locate system libraries (libiconv, libSystem, etc.).
//
// This script probes known SDK locations and emits a `rustc-link-search`
// directive so that `cargo build` works without relying on `xcrun`.

fn main() {
    #[cfg(target_os = "macos")]
    find_macos_sdk();
}

#[cfg(target_os = "macos")]
fn find_macos_sdk() {
    // Probe candidate SDK locations in preference order.
    let candidates = [
        // Xcode.app full SDK
        "/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk",
        // Command-line tools SDK
        "/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk",
    ];

    for sdk in &candidates {
        let lib_path = format!("{sdk}/usr/lib");
        if std::path::Path::new(&lib_path).exists() {
            // Tell the linker where to find libiconv, libSystem, etc.
            println!("cargo:rustc-link-search={lib_path}");
            return;
        }
    }

    // If neither candidate is found, emit a warning but do not fail —
    // the user may have a working xcrun despite the warning.
    println!("cargo:warning=Could not find macOS SDK at known locations; linker may fail if xcrun is also unavailable.");
}
