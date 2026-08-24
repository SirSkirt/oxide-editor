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
    display_version: String,
    platform: String,
}

struct Args {
    package: PathBuf,
    appimage: PathBuf,
    version: String,
    pid: u32,
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

fn parse_args() -> Result<Args, String> {
    let args: Vec<String> = env::args().collect();
    let pid = arg_value(&args, "--pid")?
        .parse::<u32>()
        .map_err(|_| "--pid must be a process id.".to_string())?;
    Ok(Args {
        package: PathBuf::from(arg_value(&args, "--package")?),
        appimage: PathBuf::from(arg_value(&args, "--appimage")?),
        version: arg_value(&args, "--version")?,
        pid,
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
    if !staging.join("oxide-editor.AppImage").is_file() {
        return Err("The Linux update package does not contain oxide-editor.AppImage.".into());
    }
    Ok(manifest)
}

fn wait_for_process(pid: u32) -> Result<(), String> {
    let proc_path = PathBuf::from(format!("/proc/{pid}"));
    for _ in 0..120 {
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

fn install(args: Args) -> Result<(), String> {
    if !args.appimage.is_file() {
        return Err(format!("The running AppImage was not found at {}.", args.appimage.display()));
    }

    let work = args
        .package
        .parent()
        .ok_or_else(|| "Update package has no working directory.".to_string())?;
    let staging = work.join("linux-staging");
    log("Verifying Linux package layout...");
    let manifest = extract_package(&args.package, &staging)?;
    if manifest.version != args.version {
        return Err(format!(
            "Package version {} does not match requested version {}.",
            manifest.version, args.version
        ));
    }

    log(format!("{} verified. Waiting for Oxide to close...", manifest.display_version));
    wait_for_process(args.pid)?;

    let staged_app = staging.join("oxide-editor.AppImage");
    set_executable(&staged_app)?;
    let replacement = sibling_with_suffix(&args.appimage, ".oxide-new")?;
    let backup = sibling_with_suffix(&args.appimage, ".oxide-backup")?;

    let _ = fs::remove_file(&replacement);
    let _ = fs::remove_file(&backup);
    fs::copy(&staged_app, &replacement)
        .map_err(|e| format!("Could not stage the new AppImage beside the current one: {e}"))?;
    set_executable(&replacement)?;

    log("Replacing AppImage...");
    fs::rename(&args.appimage, &backup)
        .map_err(|e| format!("Could not create rollback copy of the current AppImage: {e}"))?;
    if let Err(error) = fs::rename(&replacement, &args.appimage) {
        let _ = fs::rename(&backup, &args.appimage);
        return Err(format!("Could not install the new AppImage: {error}"));
    }
    set_executable(&args.appimage)?;

    log("Update installed. Restarting Oxide...");
    if let Err(error) = Command::new(&args.appimage).spawn() {
        let _ = fs::remove_file(&args.appimage);
        let _ = fs::rename(&backup, &args.appimage);
        return Err(format!("The update was installed but Oxide could not restart: {error}"));
    }

    let _ = fs::remove_file(&backup);
    Ok(())
}

fn main() {
    let result = parse_args().and_then(install);
    if let Err(error) = result {
        log(format!("UPDATE FAILED: {error}"));
    }
}
