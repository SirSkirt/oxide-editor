use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager};

use crate::ToolchainInfo;

pub fn workspace_root(app: &AppHandle) -> Result<PathBuf, String> {
    let root = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not resolve Rivet's Android workspace: {error}"))?
        .join("projects");
    fs::create_dir_all(&root)
        .map_err(|error| format!("Could not create Rivet's Android workspace: {error}"))?;
    Ok(root)
}

pub fn preview_toolchain_info() -> ToolchainInfo {
    ToolchainInfo {
        cargo_found: false,
        rustc_found: false,
        cargo: "Cargo: Android backend pending".into(),
        rustc: "rustc: Android backend pending".into(),
        backend_ready: false,
        note: Some("B1.3.6 Build 9 is an editor-first Android preview. Cargo, rustc, rust-analyzer, debugger and program execution will be added through the dedicated Android backend in later builds.".into()),
    }
}
