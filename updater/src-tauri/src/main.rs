#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::{
    env,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::Duration,
};
use tauri::{AppHandle, Emitter};
use zip::ZipArchive;

#[derive(Clone, Serialize)]
struct UpdateProgress {
    stage: String,
    detail: String,
    percent: u8,
    failed: bool,
}

#[derive(Debug, Deserialize)]
struct PackageManifest {
    format: u32,
    version: String,
    display_version: String,
}

#[derive(Debug, Clone)]
struct UpdateArgs {
    package: PathBuf,
    install_dir: PathBuf,
    app_exe: String,
    requested_version: String,
}

fn emit(app: &AppHandle, stage: &str, detail: impl Into<String>, percent: u8) {
    let _ = app.emit(
        "oxide-updater-progress",
        UpdateProgress {
            stage: stage.into(),
            detail: detail.into(),
            percent,
            failed: false,
        },
    );
}

fn emit_failure(app: &AppHandle, detail: impl Into<String>) {
    let _ = app.emit(
        "oxide-updater-progress",
        UpdateProgress {
            stage: "UPDATE FAILED".into(),
            detail: detail.into(),
            percent: 100,
            failed: true,
        },
    );
}

fn parse_args() -> Result<UpdateArgs, String> {
    let args: Vec<String> = env::args().collect();
    let value = |flag: &str| -> Result<String, String> {
        let index = args
            .iter()
            .position(|arg| arg == flag)
            .ok_or_else(|| format!("Missing required argument {flag}"))?;
        args.get(index + 1)
            .cloned()
            .ok_or_else(|| format!("Missing value for {flag}"))
    };

    Ok(UpdateArgs {
        package: PathBuf::from(value("--package")?),
        install_dir: PathBuf::from(value("--install-dir")?),
        app_exe: value("--app-exe")?,
        requested_version: value("--version")?,
    })
}

fn copy_file(source: &Path, destination: &Path) -> Result<(), String> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Could not create {}: {e}", parent.display()))?;
    }
    fs::copy(source, destination)
        .map(|_| ())
        .map_err(|e| format!("Could not copy {} to {}: {e}", source.display(), destination.display()))
}

fn collect_files(root: &Path, current: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(current).map_err(|e| format!("Could not read {}: {e}", current.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, files)?;
        } else if path.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|e| e.to_string())?
                .to_path_buf();
            if relative != Path::new("update-package.json") {
                files.push(relative);
            }
        }
    }
    Ok(())
}

fn extract_package(package: &Path, staging: &Path) -> Result<PackageManifest, String> {
    if staging.exists() {
        fs::remove_dir_all(staging).map_err(|e| format!("Could not clear staging directory: {e}"))?;
    }
    fs::create_dir_all(staging).map_err(|e| format!("Could not create staging directory: {e}"))?;

    let file = File::open(package).map_err(|e| format!("Could not open update package: {e}"))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("Update package is not a valid ZIP: {e}"))?;

    for index in 0..archive.len() {
        let mut item = archive.by_index(index).map_err(|e| format!("Could not read package entry: {e}"))?;
        let relative = item
            .enclosed_name()
            .ok_or_else(|| "Update package contained an unsafe path.".to_string())?
            .to_path_buf();
        let output = staging.join(relative);

        if item.is_dir() {
            fs::create_dir_all(&output).map_err(|e| format!("Could not create {}: {e}", output.display()))?;
            continue;
        }

        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Could not create {}: {e}", parent.display()))?;
        }
        let mut out = File::create(&output).map_err(|e| format!("Could not create {}: {e}", output.display()))?;
        io::copy(&mut item, &mut out).map_err(|e| format!("Could not extract {}: {e}", output.display()))?;
        out.flush().ok();
    }

    let manifest_path = staging.join("update-package.json");
    let manifest_text = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("The update package is missing update-package.json: {e}"))?;
    let manifest: PackageManifest = serde_json::from_str(&manifest_text)
        .map_err(|e| format!("The update package manifest is invalid: {e}"))?;

    if manifest.format != 1 {
        return Err(format!("Unsupported Oxide update package format {}.", manifest.format));
    }
    if !staging.join("oxide-editor.exe").is_file() {
        return Err("The update package does not contain oxide-editor.exe.".into());
    }
    if !staging.join("oxide-updater.exe").is_file() {
        return Err("The update package does not contain oxide-updater.exe.".into());
    }

    Ok(manifest)
}

fn backup_files(staging: &Path, install_dir: &Path, backup: &Path, files: &[PathBuf]) -> Result<(), String> {
    let _ = staging;
    if backup.exists() {
        fs::remove_dir_all(backup).map_err(|e| format!("Could not clear update backup: {e}"))?;
    }
    fs::create_dir_all(backup).map_err(|e| format!("Could not create update backup: {e}"))?;

    for relative in files {
        let installed = install_dir.join(relative);
        if installed.is_file() {
            copy_file(&installed, &backup.join(relative))?;
        }
    }
    Ok(())
}

fn replace_one_with_retry(source: &Path, target: &Path) -> Result<(), String> {
    let mut last_error = None;
    for _ in 0..80 {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Could not create {}: {e}", parent.display()))?;
        }

        let temp_target = target.with_extension(format!(
            "{}.oxide-new",
            target.extension().and_then(|v| v.to_str()).unwrap_or("file")
        ));
        let _ = fs::remove_file(&temp_target);

        match fs::copy(source, &temp_target) {
            Ok(_) => {
                if target.exists() {
                    if let Err(error) = fs::remove_file(target) {
                        let _ = fs::remove_file(&temp_target);
                        last_error = Some(error.to_string());
                        thread::sleep(Duration::from_millis(250));
                        continue;
                    }
                }
                match fs::rename(&temp_target, target) {
                    Ok(_) => return Ok(()),
                    Err(error) => {
                        last_error = Some(error.to_string());
                        let _ = fs::remove_file(&temp_target);
                    }
                }
            }
            Err(error) => last_error = Some(error.to_string()),
        }
        thread::sleep(Duration::from_millis(250));
    }

    Err(format!(
        "Could not replace {} after waiting for Oxide to close: {}",
        target.display(),
        last_error.unwrap_or_else(|| "unknown file error".into())
    ))
}

fn restore_backup(backup: &Path, install_dir: &Path) {
    let mut files = Vec::new();
    if backup.exists() && collect_files(backup, backup, &mut files).is_ok() {
        for relative in files {
            let source = backup.join(&relative);
            let target = install_dir.join(&relative);
            let _ = copy_file(&source, &target);
        }
    }
}

fn perform_update(app: AppHandle, args: UpdateArgs) -> Result<(), String> {
    let work_dir = args
        .package
        .parent()
        .ok_or_else(|| "Update package has no parent directory.".to_string())?
        .to_path_buf();
    let staging = work_dir.join("staging");
    let backup = work_dir.join("backup");

    emit(&app, "VERIFYING PACKAGE", "Opening the signed Oxide package…", 8);
    let manifest = extract_package(&args.package, &staging)?;
    if manifest.version != args.requested_version {
        return Err(format!(
            "Package version {} does not match the requested update {}.",
            manifest.version, args.requested_version
        ));
    }

    emit(
        &app,
        "PACKAGE READY",
        format!("{} has been unpacked safely.", manifest.display_version),
        25,
    );

    let mut files = Vec::new();
    collect_files(&staging, &staging, &mut files)?;
    if files.is_empty() {
        return Err("The update package contains no runtime files.".into());
    }

    emit(&app, "BACKING UP", "Creating a rollback copy of the current Oxide files…", 38);
    backup_files(&staging, &args.install_dir, &backup, &files)?;

    emit(&app, "INSTALLING", "Waiting for Oxide Editor to release its program files…", 48);
    for (index, relative) in files.iter().enumerate() {
        let source = staging.join(relative);
        let target = args.install_dir.join(relative);
        if let Err(error) = replace_one_with_retry(&source, &target) {
            emit(&app, "ROLLING BACK", "The update failed. Restoring the previous Oxide build…", 88);
            restore_backup(&backup, &args.install_dir);
            return Err(error);
        }
        let fraction = (index + 1) as f32 / files.len() as f32;
        let percent = 50 + (fraction * 38.0) as u8;
        emit(&app, "INSTALLING", format!("Updated {}", relative.display()), percent.min(88));
    }

    let app_path = args.install_dir.join(&args.app_exe);
    if !app_path.is_file() {
        restore_backup(&backup, &args.install_dir);
        return Err(format!("Updated Oxide executable was not found at {}.", app_path.display()));
    }

    emit(&app, "UPDATE COMPLETE", format!("{} is installed. Restarting Oxide…", manifest.display_version), 100);
    thread::sleep(Duration::from_millis(650));
    Command::new(&app_path)
        .spawn()
        .map_err(|e| format!("Oxide updated successfully, but could not restart: {e}"))?;
    thread::sleep(Duration::from_millis(300));
    app.exit(0);
    Ok(())
}

#[tauri::command]
fn updater_close(app: AppHandle) {
    app.exit(0);
}

fn main() {
    let args = match parse_args() {
        Ok(args) => args,
        Err(error) => {
            eprintln!("Oxide updater could not start: {error}");
            return;
        }
    };

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![updater_close])
        .setup(move |app| {
            let handle = app.handle().clone();
            let update_args = args.clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(500));
                let restart_path = update_args.install_dir.join(&update_args.app_exe);
                if let Err(error) = perform_update(handle.clone(), update_args) {
                    // If installation fails, try to return the user to the previous/restored build.
                    if restart_path.is_file() {
                        let _ = Command::new(&restart_path).spawn();
                    }
                    emit_failure(&handle, error);
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Oxide Update Service");
}
