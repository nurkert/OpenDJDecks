//! Locates the Rubber Band library (keylock): pkg-config on macOS and Linux,
//! vcpkg (or an explicit `RUBBERBAND_LIB_DIR`) on Windows.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=PKG_CONFIG_PATH");
    println!("cargo:rerun-if-env-changed=RUBBERBAND_LIB_DIR");
    println!("cargo:rerun-if-env-changed=RUBBERBAND_LIB_NAME");
    if std::env::var_os("CARGO_FEATURE_RUBBERBAND").is_none() {
        return;
    }
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        windows();
        return;
    }
    match pkg_config::Config::new()
        .atleast_version("3.0")
        .probe("rubberband")
    {
        Ok(_) => {}
        Err(e) => panic!(
            "Rubber Band library not found ({e}).\n\
             Install it (macOS: `brew install rubberband`, Debian/Raspberry Pi OS: \
             `sudo apt install librubberband-dev pkg-config`) or build od-engine with \
             `--no-default-features` to disable keylock."
        ),
    }
}

/// Windows has no system pkg-config: an explicit directory wins, otherwise
/// the library comes from vcpkg (`vcpkg install rubberband:x64-windows`,
/// `VCPKG_ROOT` pointing at the vcpkg checkout).
fn windows() {
    if let Some(dir) = std::env::var_os("RUBBERBAND_LIB_DIR") {
        let name = std::env::var("RUBBERBAND_LIB_NAME").unwrap_or_else(|_| "rubberband".into());
        println!("cargo:rustc-link-search=native={}", dir.to_string_lossy());
        println!("cargo:rustc-link-lib={name}");
        return;
    }
    if let Err(e) = vcpkg::Config::new().find_package("rubberband") {
        panic!(
            "Rubber Band library not found ({e}).\n\
             Install it with vcpkg (`vcpkg install rubberband:x64-windows`, \
             set `VCPKG_ROOT`), point `RUBBERBAND_LIB_DIR` at a directory holding \
             `rubberband.lib`, or build od-engine with `--no-default-features` to \
             disable keylock."
        );
    }
}
