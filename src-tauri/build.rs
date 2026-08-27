use std::{env, fs, path::PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let package_path = manifest_dir
        .parent()
        .expect("src-tauri should have a project root")
        .join("package.json");

    let package_text = fs::read_to_string(&package_path)
        .expect("Rivet package.json should be readable during the Rust build");
    let package: serde_json::Value =
        serde_json::from_str(&package_text).expect("Rivet package.json should contain valid JSON");

    let build_number = package
        .get("buildNumber")
        .and_then(|value| value.as_u64())
        .unwrap_or(1);
    let display_version = package
        .get("displayVersion")
        .and_then(|value| value.as_str())
        .unwrap_or(env!("CARGO_PKG_VERSION"));

    println!("cargo:rerun-if-changed={}", package_path.display());
    println!("cargo:rustc-env=OXIDE_BUILD_NUMBER={build_number}");
    println!("cargo:rustc-env=OXIDE_DISPLAY_VERSION={display_version}");

    tauri_build::build()
}
