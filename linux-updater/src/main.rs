use serde::Deserialize;
use std::{
    env,
    fs::{self, File},
    io::{self, Write},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::Duration,
};
use zip::ZipArchive;

#[derive(Debug, Deserialize)]
struct PackageManifest {
    format: u32,
    version: String,
    release_version: Option<String>,
    display_version: String,
    build: u64,
    platform: String,
}

#[derive(Debug)]
enum InstallMode {
    AppImage { path: PathBuf },
    Deb { app_exe: PathBuf },
}

#[derive(Debug)]
struct Args {
    package: PathBuf,
    version: String,
    build: u64,
    pid: u32,
    mode: InstallMode,
}

fn arg_value(args: &[String], flag: &str) -> Result<String, String> {
    let index = args
        .iter()
        .position(|arg| arg == flag)
        .ok_or_else(|| format!("Missing required argument {flag}"))?;
    args.get(index + 1)
        .cloned()
        .ok_or_else(|| format!("Missing value for {flag}"))
}

fn optional_arg_value(args: &[String], flag: &str) -> Option<String> {
    let index = args.iter().position(|arg| arg == flag)?;
    args.get(index + 1).cloned()
}

fn parse_args() -> Result<Args, String> {
    let args: Vec<String> = env::args().collect();
    let pid = arg_value(&args, "--pid")?
        .parse::<u32>()
        .map_err(|_| "--pid must be a process id.".to_string())?;

    let mode_name = optional_arg_value(&args, "--mode").unwrap_or_else(|| {
        if optional_arg_value(&args, "--appimage").is_some() {
            "appimage".to_string()
        } else {
            "deb".to_string()
        }
    });

    let mode = match mode_name.as_str() {
        "appimage" => InstallMode::AppImage {
            path: PathBuf::from(arg_value(&args, "--appimage")?),
        },
        "deb" => InstallMode::Deb {
            app_exe: PathBuf::from(arg_value(&args, "--app-exe")?),
        },
        other => return Err(format!("Unsupported Linux updater mode: {other}")),
    };

    Ok(Args {
        package: PathBuf::from(arg_value(&args, "--package")?),
        version: arg_value(&args, "--version")?,
        build: arg_value(&args, "--build")?
            .parse::<u64>()
            .map_err(|_| "--build must be a positive integer.".to_string())?,
        pid,
        mode,
    })
}

fn log(message: impl AsRef<str>) {
    let message = message.as_ref();
    eprintln!("[Oxide Update Service] {message}");
    if let Ok(path) = env::var("OXIDE_UPDATE_LOG") {
        if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(file, "{message}");
        }
    }
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
    }

    let manifest_text = fs::read_to_string(staging.join("update-package.json"))
        .map_err(|e| format!("The update package is missing update-package.json: {e}"))?;
    let manifest: PackageManifest = serde_json::from_str(&manifest_text)
        .map_err(|e| format!("The update package manifest is invalid: {e}"))?;
    if manifest.format != 1 {
        return Err(format!("Unsupported Oxide update package format {}.", manifest.format));
    }
    if !manifest.platform.starts_with("linux-") {
        return Err(format!("This update package targets {}, not Linux.", manifest.platform));
    }
    Ok(manifest)
}

fn wait_for_process(pid: u32) -> Result<(), String> {
    let proc_path = PathBuf::from(format!("/proc/{pid}"));
    for _ in 0..160 {
        if !proc_path.exists() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(250));
    }
    Err("Oxide Editor did not close in time, so the update was cancelled.".into())
}

fn sibling_with_suffix(path: &Path, suffix: &str) -> Result<PathBuf, String> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "Could not determine the AppImage filename.".to_string())?;
    Ok(path.with_file_name(format!("{name}{suffix}")))
}

fn set_executable(path: &Path) -> Result<(), String> {
    let mut permissions = fs::metadata(path)
        .map_err(|e| format!("Could not inspect {}: {e}", path.display()))?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)
        .map_err(|e| format!("Could not make {} executable: {e}", path.display()))
}

fn command_path(name: &str, known_paths: &[&str]) -> Result<PathBuf, String> {
    for candidate in known_paths {
        let path = PathBuf::from(candidate);
        if path.is_file() {
            return Ok(path);
        }
    }
    if let Some(path_value) = env::var_os("PATH") {
        for directory in env::split_paths(&path_value) {
            let candidate = directory.join(name);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    Err(format!("Required Linux command '{name}' was not found."))
}

fn install_appimage(staging: &Path, appimage: &Path, display_version: &str) -> Result<(), String> {
    if !appimage.is_file() {
        return Err(format!("The running AppImage was not found at {}.", appimage.display()));
    }

    let staged_app = staging.join("oxide-editor.AppImage");
    if !staged_app.is_file() {
        return Err("The Linux update package does not contain oxide-editor.AppImage.".into());
    }
    set_executable(&staged_app)?;

    let replacement = sibling_with_suffix(appimage, ".oxide-new")?;
    let backup = sibling_with_suffix(appimage, ".oxide-backup")?;
    let _ = fs::remove_file(&replacement);
    let _ = fs::remove_file(&backup);

    fs::copy(&staged_app, &replacement)
        .map_err(|e| format!("Could not stage the new AppImage beside the current one: {e}"))?;
    set_executable(&replacement)?;

    log(format!("Installing {display_version} AppImage..."));
    fs::rename(appimage, &backup)
        .map_err(|e| format!("Could not create rollback copy of the current AppImage: {e}"))?;
    if let Err(error) = fs::rename(&replacement, appimage) {
        let _ = fs::rename(&backup, appimage);
        return Err(format!("Could not install the new AppImage: {error}"));
    }
    set_executable(appimage)?;

    if let Err(error) = Command::new(appimage).spawn() {
        let _ = fs::remove_file(appimage);
        let _ = fs::rename(&backup, appimage);
        return Err(format!("The update was installed but Oxide could not restart: {error}"));
    }

    let _ = fs::remove_file(&backup);
    Ok(())
}

fn install_deb(staging: &Path, app_exe: &Path, display_version: &str) -> Result<(), String> {
    let staged_deb = staging.join("oxide-editor.deb");
    if !staged_deb.is_file() {
        return Err("The Linux update package does not contain oxide-editor.deb.".into());
    }

    let pkexec = command_path("pkexec", &["/usr/bin/pkexec", "/bin/pkexec"])?;
    let dpkg = command_path("dpkg", &["/usr/bin/dpkg", "/bin/dpkg"])?;

    log(format!("Requesting system authorization to install {display_version}..."));
    let status = Command::new(&pkexec)
        .arg(&dpkg)
        .arg("--install")
        .arg(&staged_deb)
        .status()
        .map_err(|e| format!("Could not start the Linux authorization service: {e}"))?;

    match status.code() {
        Some(0) => {}
        Some(126) => return Err("Linux update authorization was cancelled by the user.".into()),
        Some(127) => return Err("Linux could not authorize the Oxide package update.".into()),
        Some(code) => return Err(format!("dpkg could not install the Oxide update (exit code {code}).")),
        None => return Err("The privileged Oxide package installer ended unexpectedly.".into()),
    }

    if !app_exe.is_file() {
        return Err(format!("Oxide was updated, but the installed executable was not found at {}.", app_exe.display()));
    }

    log("Package manager update complete. Restarting Oxide...");
    Command::new(app_exe)
        .spawn()
        .map_err(|e| format!("Oxide updated successfully, but could not restart: {e}"))?;
    Ok(())
}

fn install(args: Args) -> Result<(), String> {
    let work = args
        .package
        .parent()
        .ok_or_else(|| "Update package has no working directory.".to_string())?;
    let staging = work.join("linux-staging");

    log("Verifying Linux package layout...");
    let manifest = extract_package(&args.package, &staging)?;
    let package_release_version = manifest
        .release_version
        .as_deref()
        .unwrap_or(&manifest.version);
    if package_release_version != args.version || manifest.build != args.build {
        return Err(format!(
            "Package {} Build {} does not match requested version {} Build {}.",
            package_release_version, manifest.build, args.version, args.build
        ));
    }

    log(format!("{} Build {} verified. Waiting for Oxide to close...", manifest.display_version, manifest.build));
    wait_for_process(args.pid)?;

    match &args.mode {
        InstallMode::AppImage { path } => install_appimage(&staging, path, &manifest.display_version),
        InstallMode::Deb { app_exe } => install_deb(&staging, app_exe, &manifest.display_version),
    }
}

fn restart_after_failure(args: &Args) {
    let candidate = match &args.mode {
        InstallMode::AppImage { path } => Some(path.as_path()),
        InstallMode::Deb { app_exe } => Some(app_exe.as_path()),
    };
    if let Some(path) = candidate.filter(|path| path.is_file()) {
        let _ = Command::new(path).spawn();
    }
}

fn main() {
    let args = match parse_args() {
        Ok(args) => args,
        Err(error) => {
            log(format!("UPDATE FAILED: {error}"));
            return;
        }
    };

    if let Err(error) = install(Args {
        package: args.package.clone(),
        version: args.version.clone(),
        build: args.build,
        pid: args.pid,
        mode: match &args.mode {
            InstallMode::AppImage { path } => InstallMode::AppImage { path: path.clone() },
            InstallMode::Deb { app_exe } => InstallMode::Deb { app_exe: app_exe.clone() },
        },
    }) {
        log(format!("UPDATE FAILED: {error}"));
        restart_after_failure(&args);
    }
}
