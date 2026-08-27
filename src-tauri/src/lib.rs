mod analyzer;
mod debugger;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::{
    env,
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{Arc, Mutex, mpsc},
    collections::HashMap,
    thread,
    time::{Duration, SystemTime},
};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_updater::UpdaterExt;
use toml_edit::{Array, DocumentMut, InlineTable, Item, Value};

#[derive(Serialize)]
struct ToolchainInfo {
    cargo_found: bool,
    rustc_found: bool,
    cargo: String,
    rustc: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlatformInfo {
    os: String,
    arch: String,
    path_case_sensitive: bool,
    automatic_updates: bool,
    update_mode: String,
}


#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OxideUpdateInfo {
    version: String,
    display_version: String,
    build_number: u64,
    current_version: String,
    current_build_number: u64,
    body: Option<String>,
    date: Option<String>,
    install_supported: bool,
    install_hint: Option<String>,
    update_mode: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct OxideUpdateDownloadEvent {
    event: String,
    downloaded: usize,
    content_length: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OxideUpdateStageResult {
    version: String,
    build_number: u64,
    helper_started: bool,
}

#[derive(Serialize)]
struct ProjectEntry {
    name: String,
    path: String,
    kind: String,
    depth: usize,
}

#[derive(Serialize)]
struct DependencyView {
    name: String,
    display: String,
}

#[derive(Serialize)]
struct ManifestView {
    package_name: String,
    version: String,
    edition: String,
    dependencies: Vec<DependencyView>,
}

#[derive(Serialize)]
struct BrowserRoot {
    label: String,
    path: String,
}

#[derive(Serialize)]
struct BrowserEntry {
    name: String,
    path: String,
    kind: String,
}

#[derive(Serialize)]
struct DirectoryListing {
    current_path: String,
    parent_path: Option<String>,
    is_cargo_project: bool,
    entries: Vec<BrowserEntry>,
}

#[derive(Clone, Serialize)]
struct CargoLine {
    stream: String,
    line: String,
}

#[derive(Clone, Serialize)]
struct CargoStateEvent {
    state: String,
    detail: String,
}

#[derive(Serialize)]
struct CargoResult {
    success: bool,
    exit_code: Option<i32>,
}

#[derive(Clone, Serialize)]
struct RustDiagnostic {
    level: String,
    message: String,
    code: Option<String>,
    file_path: String,
    file_name: String,
    line: usize,
    column: usize,
    end_line: usize,
    end_column: usize,
    label: Option<String>,
    suggestions: Vec<String>,
    rendered: Option<String>,
}

#[derive(Serialize)]
struct DiagnosticsResult {
    success: bool,
    diagnostics: Vec<RustDiagnostic>,
}

#[derive(Clone, Serialize)]
struct TerminalChunk {
    stream: String,
    data: String,
}

#[derive(Clone, Serialize)]
struct TerminalEvent {
    state: String,
    detail: String,
    exit_code: Option<i32>,
}



#[derive(Clone, Serialize)]
struct TutorialExamplePart {
    token: String,
    meaning: String,
}

#[derive(Clone, Serialize)]
struct TutorialStep {
    id: String,
    title: String,
    explanation: String,
    objective: String,
    learn_more_text: String,
    run_required: bool,
    example_code: Option<String>,
    example_parts: Vec<TutorialExamplePart>,
}

#[derive(Clone, Serialize)]
struct TutorialLesson {
    id: String,
    course: String,
    title: String,
    summary: String,
    skill: String,
    steps: Vec<TutorialStep>,
}

#[derive(Serialize)]
struct TutorialCatalog {
    beginner: Vec<TutorialLesson>,
    advanced_topics: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize, Default)]
struct TutorialProgressEntry {
    completed_steps: usize,
    completed: bool,
    checkpoint_source: String,
}

#[derive(Serialize, Deserialize, Default)]
struct TutorialProgressFile {
    lessons: HashMap<String, TutorialProgressEntry>,
}

#[derive(Deserialize)]
struct TutorialEvaluationRequest {
    lesson_id: String,
    step_index: usize,
    source: String,
    run_output: String,
    run_success: Option<bool>,
    diagnostic_codes: Vec<String>,
    diagnostic_messages: Vec<String>,
    diagnostic_levels: Vec<String>,
}

#[derive(Serialize)]
struct TutorialEvaluationResult {
    complete: bool,
    feedback: String,
}

struct TerminalRuntime {
    child: Arc<Mutex<Option<Child>>>,
    stdin: Arc<Mutex<Option<ChildStdin>>>,
    preparing: Arc<Mutex<bool>>,
}

impl Default for TerminalRuntime {
    fn default() -> Self {
        Self {
            child: Arc::new(Mutex::new(None)),
            stdin: Arc::new(Mutex::new(None)),
            preparing: Arc::new(Mutex::new(false)),
        }
    }
}

fn resolve_program(program: &str) -> PathBuf {
    let executable_name = if cfg!(windows) && !program.to_ascii_lowercase().ends_with(".exe") {
        format!("{program}.exe")
    } else {
        program.to_string()
    };

    if let Some(path_value) = env::var_os("PATH") {
        for directory in env::split_paths(&path_value) {
            let candidate = directory.join(&executable_name);
            if candidate.is_file() {
                return candidate;
            }
        }
    }

    #[cfg(target_os = "linux")]
    for directory in ["/usr/local/bin", "/usr/bin", "/bin"] {
        let candidate = Path::new(directory).join(&executable_name);
        if candidate.is_file() {
            return candidate;
        }
    }

    // Linux desktop launchers do not necessarily inherit ~/.cargo/bin from shell dotfiles.
    if let Some(home) = env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }) {
        let candidate = PathBuf::from(home).join(".cargo").join("bin").join(&executable_name);
        if candidate.is_file() {
            return candidate;
        }
    }

    PathBuf::from(program)
}

fn program_command(program: &str) -> Command {
    Command::new(resolve_program(program))
}

fn command_version(program: &str) -> Option<String> {
    program_command(program)
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(target_os = "linux")]
fn linux_is_appimage() -> bool {
    env::var_os("APPIMAGE")
        .map(PathBuf::from)
        .map(|path| path.is_file())
        .unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn linux_is_deb_install() -> bool {
    if cfg!(debug_assertions) {
        return false;
    }
    let Ok(current_exe) = env::current_exe() else {
        return false;
    };
    program_command("dpkg-query")
        .arg("-S")
        .arg(&current_exe)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn linux_deb_update_tools_available() -> bool {
    resolve_program("pkexec").is_file() && resolve_program("dpkg").is_file()
}

#[cfg(target_os = "linux")]
fn linux_update_capability() -> (bool, String, Option<String>) {
    if linux_is_appimage() {
        return (true, "appimage-package".to_string(), None);
    }
    if linux_is_deb_install() {
        if linux_deb_update_tools_available() {
            return (true, "deb-package".to_string(), None);
        }
        return (
            false,
            "deb-package".to_string(),
            Some("This .deb installation can update automatically when polkit/pkexec and dpkg are available. Install the missing system tools or update Oxide through your package manager.".to_string()),
        );
    }
    (
        false,
        "linux-development".to_string(),
        Some("Automatic installation is disabled for unpackaged Linux development builds. Use the AppImage or .deb release build to test Oxide updates.".to_string()),
    )
}

#[tauri::command]
fn platform_info() -> PlatformInfo {
    #[cfg(target_os = "windows")]
    let (automatic_updates, update_mode) = (true, "native-package".to_string());

    #[cfg(target_os = "linux")]
    let (automatic_updates, update_mode, _) = linux_update_capability();

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    let (automatic_updates, update_mode) = (false, "unsupported".to_string());

    PlatformInfo {
        os: env::consts::OS.to_string(),
        arch: env::consts::ARCH.to_string(),
        path_case_sensitive: !cfg!(windows),
        automatic_updates,
        update_mode,
    }
}

#[tauri::command]
fn toolchain_info() -> ToolchainInfo {
    let cargo = command_version("cargo");
    let rustc = command_version("rustc");

    ToolchainInfo {
        cargo_found: cargo.is_some(),
        rustc_found: rustc.is_some(),
        cargo: cargo.unwrap_or_else(|| "Cargo: not found".into()),
        rustc: rustc.unwrap_or_else(|| "rustc: not found".into()),
    }
}

fn should_skip(name: &str) -> bool {
    matches!(name, "target" | ".git" | "node_modules" | ".idea" | ".vscode")
}

fn should_skip_project_copy(name: &str) -> bool {
    matches!(name, "target" | ".git" | "node_modules" | "dist")
}

fn collect_entries(
    root: &Path,
    current: &Path,
    depth: usize,
    output: &mut Vec<ProjectEntry>,
) -> Result<(), String> {
    if depth > 8 {
        return Ok(());
    }

    let mut entries: Vec<_> = fs::read_dir(current)
        .map_err(|error| format!("Could not read {}: {error}", current.display()))?
        .filter_map(Result::ok)
        .collect();

    entries.sort_by_key(|entry| {
        let is_file = entry.file_type().map(|kind| kind.is_file()).unwrap_or(true);
        (is_file, entry.file_name().to_string_lossy().to_lowercase())
    });

    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if should_skip(&name) {
            continue;
        }

        let path = entry.path();
        let kind = entry.file_type().map_err(|error| error.to_string())?;
        let relative_depth = path
            .strip_prefix(root)
            .map(|p| p.components().count().saturating_sub(1))
            .unwrap_or(depth);

        if kind.is_dir() {
            output.push(ProjectEntry {
                name: name.clone(),
                path: path.to_string_lossy().to_string(),
                kind: "folder".into(),
                depth: relative_depth,
            });
            collect_entries(root, &path, depth + 1, output)?;
        } else if kind.is_file() {
            output.push(ProjectEntry {
                name,
                path: path.to_string_lossy().to_string(),
                kind: "file".into(),
                depth: relative_depth,
            });
        }
    }

    Ok(())
}

#[tauri::command]
fn list_project_files(project_path: String) -> Result<Vec<ProjectEntry>, String> {
    let root = PathBuf::from(project_path);
    if !root.is_dir() {
        return Err("That project folder does not exist or is not a directory.".into());
    }

    let mut output = Vec::new();
    collect_entries(&root, &root, 0, &mut output)?;
    Ok(output)
}

#[tauri::command]
fn read_text_file(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|error| format!("Could not read {path}: {error}"))
}

#[tauri::command]
fn write_text_file(path: String, content: String) -> Result<(), String> {
    fs::write(&path, content).map_err(|error| format!("Could not save {path}: {error}"))
}

fn default_browse_path_value() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(profile) = env::var("USERPROFILE") {
            let path = PathBuf::from(profile);
            if path.is_dir() {
                return path;
            }
        }
    }

    if let Ok(home) = env::var("HOME") {
        let path = PathBuf::from(home);
        if path.is_dir() {
            return path;
        }
    }

    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

#[tauri::command]
fn default_browse_path() -> String {
    default_browse_path_value().to_string_lossy().to_string()
}

#[tauri::command]
fn filesystem_roots() -> Vec<BrowserRoot> {
    let mut roots = Vec::new();
    let home = default_browse_path_value();
    roots.push(BrowserRoot {
        label: "HOME".into(),
        path: home.to_string_lossy().to_string(),
    });

    #[cfg(target_os = "windows")]
    {
        for letter in b'A'..=b'Z' {
            let drive = format!("{}:\\", letter as char);
            if Path::new(&drive).is_dir() {
                roots.push(BrowserRoot {
                    label: format!("{}:", letter as char),
                    path: drive,
                });
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        roots.push(BrowserRoot {
            label: "ROOT".into(),
            path: "/".into(),
        });
    }

    roots.dedup_by(|a, b| {
        if cfg!(windows) {
            a.path.eq_ignore_ascii_case(&b.path)
        } else {
            a.path == b.path
        }
    });
    roots
}

#[tauri::command]
fn browse_directory(path: String) -> Result<DirectoryListing, String> {
    let requested = if path.trim().is_empty() {
        default_browse_path_value()
    } else {
        PathBuf::from(path.trim())
    };

    if !requested.is_dir() {
        return Err(format!("{} is not a directory.", requested.display()));
    }

    let current = requested;
    let parent_path = current.parent().map(|parent| parent.to_string_lossy().to_string());
    let is_cargo_project = current.join("Cargo.toml").is_file();

    let mut entries: Vec<_> = fs::read_dir(&current)
        .map_err(|error| format!("Could not read {}: {error}", current.display()))?
        .filter_map(Result::ok)
        .collect();

    entries.sort_by_key(|entry| {
        let is_file = entry.file_type().map(|kind| kind.is_file()).unwrap_or(true);
        (is_file, entry.file_name().to_string_lossy().to_lowercase())
    });

    let entries = entries
        .into_iter()
        .filter_map(|entry| {
            let kind = entry.file_type().ok()?;
            if kind.is_symlink() {
                return None;
            }
            Some(BrowserEntry {
                name: entry.file_name().to_string_lossy().to_string(),
                path: entry.path().to_string_lossy().to_string(),
                kind: if kind.is_dir() { "folder".into() } else { "file".into() },
            })
        })
        .collect();

    Ok(DirectoryListing {
        current_path: current.to_string_lossy().to_string(),
        parent_path,
        is_cargo_project,
        entries,
    })
}

fn validate_folder_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Folder name cannot be empty.".into());
    }
    if matches!(trimmed, "." | "..") || trimmed.contains('/') || trimmed.contains('\\') {
        return Err("Folder name cannot contain path separators.".into());
    }
    if trimmed
        .chars()
        .any(|character| matches!(character, '<' | '>' | ':' | '"' | '|' | '?' | '*'))
    {
        return Err("Folder name contains characters Windows cannot use.".into());
    }
    Ok(trimmed.to_string())
}

#[tauri::command]
fn create_directory(parent_path: String, folder_name: String) -> Result<String, String> {
    let parent = PathBuf::from(parent_path);
    if !parent.is_dir() {
        return Err("The parent folder does not exist.".into());
    }
    let folder_name = validate_folder_name(&folder_name)?;
    let destination = parent.join(folder_name);
    if destination.exists() {
        return Err(format!("{} already exists.", destination.display()));
    }
    fs::create_dir_all(&destination)
        .map_err(|error| format!("Could not create {}: {error}", destination.display()))?;
    Ok(destination.to_string_lossy().to_string())
}

fn validate_project_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Project name cannot be empty.".into());
    }
    if !trimmed
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err("Rust package names in Oxide may contain only letters, numbers, '-' and '_'.".into());
    }
    Ok(trimmed.to_string())
}

fn validate_version(version: &str) -> Result<String, String> {
    let trimmed = version.trim();
    let parts: Vec<_> = trimmed.split('.').collect();
    if parts.len() != 3 || parts.iter().any(|part| part.is_empty() || !part.chars().all(|c| c.is_ascii_digit())) {
        return Err("Version must use numeric SemVer form such as 0.0.1 or 1.2.0.".into());
    }
    Ok(trimmed.to_string())
}

#[tauri::command]
fn create_project(destination_parent: String, project_name: String, version: String) -> Result<String, String> {
    let parent = PathBuf::from(destination_parent);
    if !parent.is_dir() {
        return Err("Choose an existing destination folder first.".into());
    }

    let package_name = validate_project_name(&project_name)?;
    let version = validate_version(&version)?;
    let destination = parent.join(&package_name);
    if destination.exists() {
        return Err(format!("{} already exists.", destination.display()));
    }

    fs::create_dir_all(destination.join("src"))
        .map_err(|error| format!("Could not create project folders: {error}"))?;

    let manifest = format!(
        "[package]\nname = \"{}\"\nversion = \"{}\"\nedition = \"2024\"\n\n[dependencies]\n",
        package_name, version
    );
    fs::write(destination.join("Cargo.toml"), manifest)
        .map_err(|error| format!("Could not create Cargo.toml: {error}"))?;

    let hello_world = "fn main() {\n    println!(\"Hello, world!\");\n}\n";
    fs::write(destination.join("src").join("main.rs"), hello_world)
        .map_err(|error| format!("Could not create src/main.rs: {error}"))?;

    Ok(destination.to_string_lossy().to_string())
}

fn copy_project_tree(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination)
        .map_err(|error| format!("Could not create {}: {error}", destination.display()))?;

    for entry in fs::read_dir(source)
        .map_err(|error| format!("Could not read {}: {error}", source.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        if should_skip_project_copy(&name) {
            continue;
        }

        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let kind = entry.file_type().map_err(|error| error.to_string())?;

        if kind.is_dir() {
            copy_project_tree(&source_path, &destination_path)?;
        } else if kind.is_file() {
            fs::copy(&source_path, &destination_path).map_err(|error| {
                format!(
                    "Could not copy {} to {}: {error}",
                    source_path.display(),
                    destination_path.display()
                )
            })?;
        }
    }

    Ok(())
}

#[tauri::command]
fn save_project_as(
    project_path: String,
    destination_parent: String,
    project_name: String,
) -> Result<String, String> {
    let source = PathBuf::from(project_path);
    if !source.is_dir() {
        return Err("The current project folder no longer exists.".into());
    }

    let destination_parent = PathBuf::from(destination_parent);
    if !destination_parent.is_dir() {
        return Err("The destination folder does not exist.".into());
    }

    let project_name = validate_folder_name(&project_name)?;
    let destination = destination_parent.join(project_name);
    if destination.exists() {
        return Err(format!("{} already exists.", destination.display()));
    }

    let source_canonical = source.canonicalize().unwrap_or(source.clone());
    let parent_canonical = destination_parent
        .canonicalize()
        .unwrap_or(destination_parent.clone());
    if parent_canonical.starts_with(&source_canonical) {
        return Err("Choose a destination outside the current project folder.".into());
    }

    copy_project_tree(&source, &destination)?;
    Ok(destination.to_string_lossy().to_string())
}

fn manifest_path(project_path: &str) -> PathBuf {
    Path::new(project_path).join("Cargo.toml")
}

fn load_manifest(project_path: &str) -> Result<(PathBuf, DocumentMut), String> {
    let path = manifest_path(project_path);
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("Could not read {}: {error}", path.display()))?;
    let document = text
        .parse::<DocumentMut>()
        .map_err(|error| format!("Cargo.toml is not valid TOML: {error}"))?;
    Ok((path, document))
}

#[tauri::command]
fn manifest_view(project_path: String) -> Result<ManifestView, String> {
    let (_, document) = load_manifest(&project_path)?;

    let package = document.get("package").and_then(Item::as_table);
    let package_name = package
        .and_then(|table| table.get("name"))
        .and_then(Item::as_str)
        .unwrap_or("Unnamed package")
        .to_string();
    let version = package
        .and_then(|table| table.get("version"))
        .and_then(Item::as_str)
        .unwrap_or("—")
        .to_string();
    let edition = package
        .and_then(|table| table.get("edition"))
        .and_then(Item::as_str)
        .unwrap_or("2021")
        .to_string();

    let mut dependencies = Vec::new();
    if let Some(table) = document.get("dependencies").and_then(Item::as_table_like) {
        for (name, item) in table.iter() {
            dependencies.push(DependencyView {
                name: name.to_string(),
                display: item.to_string().trim().to_string(),
            });
        }
    }
    dependencies.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Ok(ManifestView {
        package_name,
        version,
        edition,
        dependencies,
    })
}

fn validate_dependency_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || !name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err("Dependency names may contain only letters, numbers, '-' and '_'.".into());
    }
    Ok(())
}

#[tauri::command]
fn add_dependency(
    project_path: String,
    name: String,
    version: String,
    features: Vec<String>,
) -> Result<(), String> {
    validate_dependency_name(&name)?;
    if version.trim().is_empty() {
        return Err("Version cannot be empty.".into());
    }

    let (path, mut document) = load_manifest(&project_path)?;
    if !document.contains_key("dependencies") {
        document["dependencies"] = Item::Table(toml_edit::Table::new());
    }

    if features.is_empty() {
        document["dependencies"][&name] = toml_edit::value(version);
    } else {
        let mut table = InlineTable::new();
        table.insert("version", Value::from(version));
        let mut feature_array = Array::new();
        for feature in features {
            if !feature.trim().is_empty() {
                feature_array.push(feature);
            }
        }
        table.insert("features", Value::Array(feature_array));
        document["dependencies"][&name] = Item::Value(Value::InlineTable(table));
    }

    fs::write(&path, document.to_string())
        .map_err(|error| format!("Could not save {}: {error}", path.display()))
}

#[tauri::command]
fn remove_dependency(project_path: String, name: String) -> Result<(), String> {
    validate_dependency_name(&name)?;
    let (path, mut document) = load_manifest(&project_path)?;

    let dependencies = document
        .get_mut("dependencies")
        .and_then(Item::as_table_like_mut)
        .ok_or_else(|| "Cargo.toml has no [dependencies] table.".to_string())?;

    dependencies.remove(&name);
    fs::write(&path, document.to_string())
        .map_err(|error| format!("Could not save {}: {error}", path.display()))
}

fn cargo_args(action: &str, release: bool) -> Result<Vec<String>, String> {
    let mut args = match action {
        "check" => vec!["check".to_string()],
        "build" => vec!["build".to_string()],
        "run" => vec!["run".to_string()],
        "test" => vec!["test".to_string()],
        "clean" => vec!["clean".to_string()],
        _ => return Err(format!("Unsupported Cargo action: {action}")),
    };

    if release && matches!(action, "check" | "build" | "run" | "test") {
        args.push("--release".to_string());
    }

    Ok(args)
}

fn run_cargo(
    app: AppHandle,
    action: String,
    project_path: String,
    release: bool,
) -> Result<CargoResult, String> {
    let args = cargo_args(&action, release)?;
    let command_display = format!("cargo {}", args.join(" "));
    let _ = app.emit(
        "cargo-state",
        CargoStateEvent {
            state: "started".into(),
            detail: command_display.clone(),
        },
    );

    let mut command = program_command("cargo");
    command
        .args(&args)
        .current_dir(&project_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }

    let mut child = command
        .spawn()
        .map_err(|error| format!("Could not start Cargo. Is Rust installed and on PATH? {error}"))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Could not capture Cargo stdout.".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Could not capture Cargo stderr.".to_string())?;
    let (sender, receiver) = mpsc::channel::<CargoLine>();

    let stdout_sender = sender.clone();
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let _ = stdout_sender.send(CargoLine {
                stream: "stdout".into(),
                line,
            });
        }
    });

    let stderr_sender = sender.clone();
    thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            let _ = stderr_sender.send(CargoLine {
                stream: "stderr".into(),
                line,
            });
        }
    });

    drop(sender);
    for line in receiver {
        let _ = app.emit("cargo-output", line);
    }

    let status = child
        .wait()
        .map_err(|error| format!("Could not wait for Cargo: {error}"))?;
    let success = status.success();
    let _ = app.emit(
        "cargo-state",
        CargoStateEvent {
            state: "finished".into(),
            detail: if success {
                "Cargo finished successfully.".into()
            } else {
                "Cargo finished with errors.".into()
            },
        },
    );

    Ok(CargoResult {
        success,
        exit_code: status.code(),
    })
}

#[tauri::command]
async fn cargo_action(
    app: AppHandle,
    action: String,
    project_path: String,
    release: bool,
) -> Result<CargoResult, String> {
    tauri::async_runtime::spawn_blocking(move || run_cargo(app, action, project_path, release))
        .await
        .map_err(|error| format!("Cargo task could not be joined: {error}"))?
}

fn json_string(value: &JsonValue, key: &str) -> Option<String> {
    value.get(key)?.as_str().map(ToString::to_string)
}

fn diagnostic_from_message(project_path: &Path, message: &JsonValue) -> Option<RustDiagnostic> {
    let level = json_string(message, "level")?;
    if level != "error" && level != "warning" {
        return None;
    }

    let spans = message.get("spans")?.as_array()?;
    let primary = spans
        .iter()
        .find(|span| span.get("is_primary").and_then(JsonValue::as_bool).unwrap_or(false))
        .or_else(|| spans.first())?;

    let file_name = json_string(primary, "file_name").unwrap_or_default();
    let file_path = {
        let supplied = PathBuf::from(&file_name);
        if supplied.is_absolute() {
            supplied
        } else {
            project_path.join(&supplied)
        }
        .to_string_lossy()
        .to_string()
    };

    let code = message
        .get("code")
        .and_then(|value| value.get("code"))
        .and_then(JsonValue::as_str)
        .map(ToString::to_string);

    let mut suggestions = Vec::new();
    if let Some(replacement) = primary.get("suggested_replacement").and_then(JsonValue::as_str) {
        if !replacement.is_empty() {
            suggestions.push(format!("Suggested replacement: {replacement}"));
        }
    }
    if let Some(children) = message.get("children").and_then(JsonValue::as_array) {
        for child in children {
            let child_level = child.get("level").and_then(JsonValue::as_str).unwrap_or("");
            if matches!(child_level, "help" | "note") {
                if let Some(text) = child.get("message").and_then(JsonValue::as_str) {
                    if !text.is_empty() && !suggestions.iter().any(|existing| existing == text) {
                        suggestions.push(text.to_string());
                    }
                }
            }
        }
    }

    Some(RustDiagnostic {
        level,
        message: json_string(message, "message").unwrap_or_else(|| "Rust compiler diagnostic".into()),
        code,
        file_path,
        file_name,
        line: primary.get("line_start").and_then(JsonValue::as_u64).unwrap_or(1) as usize,
        column: primary.get("column_start").and_then(JsonValue::as_u64).unwrap_or(1) as usize,
        end_line: primary.get("line_end").and_then(JsonValue::as_u64).unwrap_or(1) as usize,
        end_column: primary.get("column_end").and_then(JsonValue::as_u64).unwrap_or(1) as usize,
        label: json_string(primary, "label"),
        suggestions,
        rendered: json_string(message, "rendered"),
    })
}

fn collect_cargo_diagnostics(project_path: String, release: bool) -> Result<DiagnosticsResult, String> {
    let mut command = program_command("cargo");
    command.arg("check").arg("--message-format=json");
    if release {
        command.arg("--release");
    }
    command.current_dir(&project_path);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }

    let output = command
        .output()
        .map_err(|error| format!("Could not run cargo check: {error}"))?;

    let root = PathBuf::from(&project_path);
    let mut diagnostics = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let Ok(value) = serde_json::from_str::<JsonValue>(line) else {
            continue;
        };
        if value.get("reason").and_then(JsonValue::as_str) != Some("compiler-message") {
            continue;
        }
        if let Some(message) = value.get("message") {
            if let Some(diagnostic) = diagnostic_from_message(&root, message) {
                diagnostics.push(diagnostic);
            }
        }
    }

    if !output.status.success() && diagnostics.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let message = stderr
            .lines()
            .filter(|line| !line.trim().is_empty())
            .take(8)
            .collect::<Vec<_>>()
            .join(" ");
        if !message.is_empty() {
            diagnostics.push(RustDiagnostic {
                level: "error".into(),
                message,
                code: None,
                file_path: root.join("Cargo.toml").to_string_lossy().to_string(),
                file_name: "Cargo.toml".into(),
                line: 1,
                column: 1,
                end_line: 1,
                end_column: 1,
                label: Some("Cargo could not complete the project check.".into()),
                suggestions: vec!["Open Cargo.toml or Raw Cargo output for the full Cargo error.".into()],
                rendered: None,
            });
        }
    }

    diagnostics.sort_by(|a, b| {
        a.file_path
            .cmp(&b.file_path)
            .then(a.line.cmp(&b.line))
            .then(a.column.cmp(&b.column))
    });

    Ok(DiagnosticsResult {
        success: output.status.success(),
        diagnostics,
    })
}

#[tauri::command]
async fn cargo_diagnostics(project_path: String, release: bool) -> Result<DiagnosticsResult, String> {
    tauri::async_runtime::spawn_blocking(move || collect_cargo_diagnostics(project_path, release))
        .await
        .map_err(|error| format!("Rust analysis task could not be joined: {error}"))?
}

fn emit_terminal_reader<R: Read + Send + 'static>(app: AppHandle, stream: &'static str, mut reader: R) {
    thread::spawn(move || {
        let mut buffer = [0_u8; 1024];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => {
                    let text = String::from_utf8_lossy(&buffer[..count]).to_string();
                    let _ = app.emit(
                        "terminal-output",
                        TerminalChunk {
                            stream: stream.into(),
                            data: text,
                        },
                    );
                }
                Err(_) => break,
            }
        }
    });
}

#[tauri::command]
fn terminal_start(
    app: AppHandle,
    terminal: State<'_, TerminalRuntime>,
    project_path: String,
    release: bool,
) -> Result<(), String> {
    {
        let mut preparing = terminal.preparing.lock().map_err(|_| "Terminal preparation state is unavailable.".to_string())?;
        if *preparing {
            return Err("Oxide is already preparing a program to run.".into());
        }
        let mut child_guard = terminal.child.lock().map_err(|_| "Terminal state is unavailable.".to_string())?;
        if let Some(child) = child_guard.as_mut() {
            if child.try_wait().map_err(|error| error.to_string())?.is_none() {
                return Err("A program is already running in the Oxide Terminal.".into());
            }
        }
        *child_guard = None;
        *preparing = true;
    }

    let child_state = Arc::clone(&terminal.child);
    let stdin_state = Arc::clone(&terminal.stdin);
    let preparing_state = Arc::clone(&terminal.preparing);

    thread::spawn(move || {
        let set_preparing = |value: bool| {
            if let Ok(mut guard) = preparing_state.lock() {
                *guard = value;
            }
        };

        let _ = app.emit(
            "terminal-state",
            TerminalEvent {
                state: "building".into(),
                detail: "Building project before launch.".into(),
                exit_code: None,
            },
        );
        let _ = app.emit(
            "cargo-state",
            CargoStateEvent {
                state: "started".into(),
                detail: format!("cargo build{}", if release { " --release" } else { "" }),
            },
        );

        let mut build = program_command("cargo");
        build.arg("build").arg("--message-format=json");
        if release {
            build.arg("--release");
        }
        build
            .current_dir(&project_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            build.creation_flags(0x08000000);
        }

        let mut build_child = match build.spawn() {
            Ok(child) => child,
            Err(error) => {
                set_preparing(false);
                let message = format!("Could not start Cargo build: {error}");
                let _ = app.emit(
                    "cargo-state",
                    CargoStateEvent { state: "finished".into(), detail: message.clone() },
                );
                let _ = app.emit(
                    "terminal-state",
                    TerminalEvent { state: "build-failed".into(), detail: message, exit_code: None },
                );
                return;
            }
        };

        let build_stdout = match build_child.stdout.take() {
            Some(stdout) => stdout,
            None => {
                let _ = build_child.kill();
                set_preparing(false);
                let message = "Could not capture Cargo build output.".to_string();
                let _ = app.emit("terminal-state", TerminalEvent { state: "build-failed".into(), detail: message, exit_code: None });
                return;
            }
        };
        let build_stderr = match build_child.stderr.take() {
            Some(stderr) => stderr,
            None => {
                let _ = build_child.kill();
                set_preparing(false);
                let message = "Could not capture Cargo build errors.".to_string();
                let _ = app.emit("terminal-state", TerminalEvent { state: "build-failed".into(), detail: message, exit_code: None });
                return;
            }
        };

        let stderr_app = app.clone();
        thread::spawn(move || {
            for line in BufReader::new(build_stderr).lines().map_while(Result::ok) {
                let _ = stderr_app.emit(
                    "cargo-output",
                    CargoLine { stream: "stderr".into(), line },
                );
            }
        });

        let mut executable_by_name: HashMap<String, String> = HashMap::new();
        for line in BufReader::new(build_stdout).lines().map_while(Result::ok) {
            match serde_json::from_str::<JsonValue>(&line) {
                Ok(message) => match message.get("reason").and_then(JsonValue::as_str) {
                    Some("compiler-message") => {
                        if let Some(rendered) = message
                            .get("message")
                            .and_then(|value| value.get("rendered"))
                            .and_then(JsonValue::as_str)
                        {
                            for rendered_line in rendered.lines() {
                                let _ = app.emit(
                                    "cargo-output",
                                    CargoLine { stream: "stderr".into(), line: rendered_line.to_string() },
                                );
                            }
                        }
                    }
                    Some("compiler-artifact") => {
                        let is_binary = message
                            .get("target")
                            .and_then(|value| value.get("kind"))
                            .and_then(JsonValue::as_array)
                            .map(|kinds| kinds.iter().any(|kind| kind.as_str() == Some("bin")))
                            .unwrap_or(false);
                        if is_binary {
                            if let (Some(name), Some(executable)) = (
                                message.get("target").and_then(|value| value.get("name")).and_then(JsonValue::as_str),
                                message.get("executable").and_then(JsonValue::as_str),
                            ) {
                                executable_by_name.insert(name.to_string(), executable.to_string());
                            }
                        }
                    }
                    _ => {}
                },
                Err(_) => {
                    let _ = app.emit(
                        "cargo-output",
                        CargoLine { stream: "stdout".into(), line },
                    );
                }
            }
        }

        let build_status = match build_child.wait() {
            Ok(status) => status,
            Err(error) => {
                set_preparing(false);
                let message = format!("Could not wait for Cargo build: {error}");
                let _ = app.emit("terminal-state", TerminalEvent { state: "build-failed".into(), detail: message, exit_code: None });
                return;
            }
        };

        let build_success = build_status.success();
        let _ = app.emit(
            "cargo-state",
            CargoStateEvent {
                state: "finished".into(),
                detail: if build_success { "Cargo build finished successfully.".into() } else { "Cargo build finished with errors.".into() },
            },
        );

        if !build_success {
            set_preparing(false);
            let _ = app.emit(
                "terminal-state",
                TerminalEvent {
                    state: "build-failed".into(),
                    detail: "The project did not compile.".into(),
                    exit_code: build_status.code(),
                },
            );
            return;
        }

        let preferred_name = manifest_view(project_path.clone()).ok().map(|manifest| manifest.package_name);
        let executable_path = preferred_name
            .as_ref()
            .and_then(|name| executable_by_name.get(name).cloned())
            .or_else(|| {
                if executable_by_name.len() == 1 {
                    executable_by_name.values().next().cloned()
                } else {
                    None
                }
            });

        let Some(executable_path) = executable_path else {
            set_preparing(false);
            let detail = if executable_by_name.is_empty() {
                "Cargo built the project, but Oxide could not find a runnable binary target.".to_string()
            } else {
                "This project has multiple runnable binary targets. Oxide needs target selection before it can launch one.".to_string()
            };
            let _ = app.emit("terminal-state", TerminalEvent { state: "build-failed".into(), detail, exit_code: None });
            return;
        };

        let mut command = Command::new(&executable_path);
        command
            .current_dir(&project_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }

        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) => {
                set_preparing(false);
                let detail = format!("The project built, but Oxide could not launch the executable: {error}");
                let _ = app.emit("terminal-state", TerminalEvent { state: "build-failed".into(), detail, exit_code: None });
                return;
            }
        };

        let stdin = match child.stdin.take() {
            Some(stdin) => stdin,
            None => {
                let _ = child.kill();
                set_preparing(false);
                let _ = app.emit("terminal-state", TerminalEvent { state: "build-failed".into(), detail: "Could not open program stdin.".into(), exit_code: None });
                return;
            }
        };
        let stdout = match child.stdout.take() {
            Some(stdout) => stdout,
            None => {
                let _ = child.kill();
                set_preparing(false);
                let _ = app.emit("terminal-state", TerminalEvent { state: "build-failed".into(), detail: "Could not capture program stdout.".into(), exit_code: None });
                return;
            }
        };
        let stderr = match child.stderr.take() {
            Some(stderr) => stderr,
            None => {
                let _ = child.kill();
                set_preparing(false);
                let _ = app.emit("terminal-state", TerminalEvent { state: "build-failed".into(), detail: "Could not capture program stderr.".into(), exit_code: None });
                return;
            }
        };

        if let Ok(mut guard) = stdin_state.lock() {
            *guard = Some(stdin);
        }
        if let Ok(mut guard) = child_state.lock() {
            *guard = Some(child);
        }
        set_preparing(false);

        let _ = app.emit(
            "terminal-state",
            TerminalEvent {
                state: "started".into(),
                detail: "Program started.".into(),
                exit_code: None,
            },
        );

        emit_terminal_reader(app.clone(), "stdout", stdout);
        emit_terminal_reader(app.clone(), "stderr", stderr);

        loop {
            let exit = {
                let mut guard = match child_state.lock() {
                    Ok(guard) => guard,
                    Err(_) => return,
                };
                match guard.as_mut() {
                    Some(child) => match child.try_wait() {
                        Ok(Some(status)) => Some((status.success(), status.code())),
                        Ok(None) => None,
                        Err(_) => Some((false, None)),
                    },
                    None => return,
                }
            };

            if let Some((success, code)) = exit {
                if let Ok(mut guard) = child_state.lock() {
                    *guard = None;
                }
                if let Ok(mut guard) = stdin_state.lock() {
                    *guard = None;
                }
                let _ = app.emit(
                    "terminal-state",
                    TerminalEvent {
                        state: "finished".into(),
                        detail: if success { "Program exited successfully.".into() } else { "Program exited with an error.".into() },
                        exit_code: code,
                    },
                );
                return;
            }
            thread::sleep(Duration::from_millis(80));
        }
    });

    Ok(())
}

#[tauri::command]
fn terminal_write(terminal: State<'_, TerminalRuntime>, data: String) -> Result<(), String> {
    let mut guard = terminal.stdin.lock().map_err(|_| "Terminal input state is unavailable.".to_string())?;
    let stdin = guard
        .as_mut()
        .ok_or_else(|| "There is no running terminal program.".to_string())?;
    stdin
        .write_all(data.as_bytes())
        .map_err(|error| format!("Could not write to program input: {error}"))?;
    stdin.flush().map_err(|error| format!("Could not flush program input: {error}"))
}

#[tauri::command]
fn terminal_stop(terminal: State<'_, TerminalRuntime>) -> Result<(), String> {
    let mut guard = terminal.child.lock().map_err(|_| "Terminal process state is unavailable.".to_string())?;
    let child = guard
        .as_mut()
        .ok_or_else(|| "There is no running terminal program.".to_string())?;
    child.kill().map_err(|error| format!("Could not stop the program: {error}"))
}



fn tutorial_step(id: &str, title: &str, explanation: &str, objective: &str, learn_more_text: &str, run_required: bool) -> TutorialStep {
    TutorialStep {
        id: id.into(),
        title: title.into(),
        explanation: explanation.into(),
        objective: objective.into(),
        learn_more_text: learn_more_text.into(),
        run_required,
        example_code: None,
        example_parts: Vec::new(),
    }
}

fn tutorial_step_with_example(
    id: &str,
    title: &str,
    explanation: &str,
    example_code: &str,
    example_parts: Vec<(&str, &str)>,
    objective: &str,
    learn_more_text: &str,
    run_required: bool,
) -> TutorialStep {
    TutorialStep {
        id: id.into(),
        title: title.into(),
        explanation: explanation.into(),
        objective: objective.into(),
        learn_more_text: learn_more_text.into(),
        run_required,
        example_code: Some(example_code.into()),
        example_parts: example_parts
            .into_iter()
            .map(|(token, meaning)| TutorialExamplePart { token: token.into(), meaning: meaning.into() })
            .collect(),
    }
}

fn tutorial_lessons() -> Vec<TutorialLesson> {
    vec![
        TutorialLesson {
            id: "hello-world".into(),
            course: "Beginner".into(),
            title: "Hello, Rust".into(),
            summary: "Run a real Rust program, change it, and run your version.".into(),
            skill: "Running Rust programs".into(),
            steps: vec![
                tutorial_step_with_example(
                    "run-first-program",
                    "Print something",
                    "println! prints a line of text to the terminal.",
                    "println!(\"Hello, world!\");",
                    vec![
                        ("println!", "prints a line of text"),
                        ("\"Hello, world!\"", "is the text to print"),
                        (";", "ends the statement"),
                    ],
                    "Now you try: press Run, choose Run in Oxide Terminal, and make the program print Hello, world!",
                    "println! is a Rust macro from the standard library. The exclamation mark shows that it is a macro invocation rather than a normal function call. The text inside the parentheses is a string literal, and the semicolon completes the statement.",
                    true,
                ),
                tutorial_step_with_example(
                    "change-message",
                    "Change the text",
                    "Text inside double quotes is a string literal.",
                    "println!(\"example\");",
                    vec![("\"example\"", "is the string value being printed")],
                    "Now you try: change the message to Hello, Oxide! and run the program again.",
                    "A string literal is text written directly into source code between double quotes. Here it becomes the value passed to println!, so changing the literal changes what the program prints.",
                    true,
                ),
            ],
        },
        TutorialLesson {
            id: "variables".into(),
            course: "Beginner".into(),
            title: "Variables by Doing".into(),
            summary: "Store values, print them, then build a tiny score display.".into(),
            skill: "Variables".into(),
            steps: vec![
                tutorial_step_with_example(
                    "make-name",
                    "Meet variables",
                    "let creates a variable that stores a value under a name.",
                    "let name = \"example\";",
                    vec![
                        ("let", "creates a variable"),
                        ("name", "is the variable's name"),
                        ("\"example\"", "is the value stored in it"),
                        (";", "ends the statement"),
                    ],
                    "Now you try: inside main, create a variable named name and store \"Quinn\" in it.",
                    "Rust uses let to create a local variable binding. In this example Rust infers the variable's type from the value, so you do not have to write a type annotation. The variable name lets later code refer to the stored value.",
                    false,
                ),
                tutorial_step_with_example(
                    "print-name",
                    "Use the value",
                    "A name inside braces lets println! insert that variable's value into text.",
                    "println!(\"{name}\");",
                    vec![
                        ("println!", "prints a line of text"),
                        ("{name}", "inserts the value stored in name"),
                        (";", "ends the statement"),
                    ],
                    "Now you try: add a println! statement that prints the value stored in name.",
                    "println! supports formatted strings. A named capture such as {name} asks Rust to format the current value of that variable and insert it at that position in the output.",
                    false,
                ),
                tutorial_step(
                    "run-name",
                    "Run what you wrote",
                    "Running the program proves that the code behaves the way you expect.",
                    "Run the program in the Oxide Terminal and make sure Quinn appears in the output.",
                    "Oxide builds the same real project used by the editor, then launches the resulting executable. The Run Terminal receives only your program's stdout and stderr, while Cargo information stays in Build Bay.",
                    true,
                ),
                tutorial_step(
                    "score-challenge",
                    "Challenge: add a score",
                    "Use the variable and printing syntax you just practiced with a new value.",
                    "Create a variable named score with the value 10, print it, and run the program so 10 appears in the output.",
                    "The value 10 is an integer literal, so it is written without quotation marks. Rust can infer an integer type from the literal and how the value is used.",
                    true,
                ),
            ],
        },
        TutorialLesson {
            id: "warnings-errors".into(),
            course: "Beginner".into(),
            title: "Warnings vs Errors".into(),
            summary: "See the difference between suspicious code and code Rust refuses to build.".into(),
            skill: "Compiler feedback".into(),
            steps: vec![
                tutorial_step_with_example(
                    "see-warning",
                    "A warning is not a stop sign",
                    "Warnings point out suspicious code, but Rust can still build the program.",
                    "let unused = 5;",
                    vec![("unused", "is never used, so rustc warns about it")],
                    "Now you try: let Rust Check show the unused-variable warning, then Run the program and confirm it still prints Program still runs.",
                    "Warnings are compiler feedback about code that is legal but potentially accidental, wasteful, or unclear. Cargo can still produce an executable when only warnings are present.",
                    true,
                ),
                tutorial_step_with_example(
                    "make-error",
                    "Turn it into an error",
                    "Errors stop Rust from building the program.",
                    "let number = 5",
                    vec![("missing ;", "leaves the let statement unfinished")],
                    "Now you try: remove the semicolon after let message = \"Hello\" and wait for Rust Check to report an error.",
                    "A let statement must be completed before the next statement begins. Without its semicolon, the source no longer matches Rust's grammar, so rustc rejects the program instead of producing an executable.",
                    false,
                ),
                tutorial_step_with_example(
                    "fix-warning",
                    "Fix the error, then use the value",
                    "Using a variable removes an unused-variable warning because its value now matters.",
                    "println!(\"{message}\");",
                    vec![("{message}", "uses the value stored in message")],
                    "Now you try: restore the semicolon, print message instead of Program still runs, and Run the program so it prints Hello without that warning.",
                    "Compiler feedback is easiest to treat by severity: errors must be fixed before the program can build, while warnings are invitations to inspect code that still compiles. Good Rust code usually aims to resolve both intentionally.",
                    true,
                ),
            ],
        },
        TutorialLesson {
            id: "mutability".into(),
            course: "Beginner".into(),
            title: "Break It: Mutability".into(),
            summary: "Break working code on purpose, read rustc's complaint, then fix it.".into(),
            skill: "Mutable vs immutable values".into(),
            steps: vec![
                tutorial_step_with_example(
                    "run-mutable",
                    "Let a value change",
                    "mut tells Rust that a variable is allowed to change after it is created.",
                    "let mut score = 10;",
                    vec![
                        ("let", "creates the variable"),
                        ("mut", "allows its value to change"),
                        ("score", "is the variable's name"),
                        ("10", "is its starting value"),
                    ],
                    "Now you try: run the working program and make sure it prints Starting score: 10 and Score: 11.",
                    "Rust variables are immutable by default. Adding mut is an explicit promise that this binding may be assigned a new value later, which makes mutation visible to both the compiler and anyone reading the code.",
                    true,
                ),
                tutorial_step(
                    "break-mut",
                    "Break it on purpose",
                    "Without mut, Rust will reject code that tries to assign a new value to score.",
                    "Remove mut from let mut score = 10; and wait for Oxide's Rust check to report the immutability error.",
                    "Because Rust bindings are immutable by default, the second assignment to score violates the binding's rules. rustc reports E0384 so you can see the rule enforced by the real compiler.",
                    false,
                ),
                tutorial_step(
                    "fix-mut",
                    "Fix what you broke",
                    "Putting mut back gives the program permission to change score again.",
                    "Restore mut, then run the program successfully and get Score: 11 again.",
                    "Explicit mutability makes state changes easier to identify during code review and reasoning. Rust requires you to opt into mutation instead of allowing every variable to change silently.",
                    true,
                ),
            ],
        },
        TutorialLesson {
            id: "basic-types".into(),
            course: "Beginner".into(),
            title: "Values Have Types".into(),
            summary: "Work with numbers, text, and true/false values without drowning in type theory.".into(),
            skill: "Basic data types".into(),
            steps: vec![
                tutorial_step_with_example(
                    "make-number",
                    "Store a number",
                    "A whole number written without quotes is an integer value.",
                    "let lives = 3;",
                    vec![("3", "is an integer value")],
                    "Now you try: create a variable named lives with the value 3.",
                    "Rust has several integer types such as i32, i64, u32, and u64. When the exact type is not constrained, an integer literal like 3 commonly defaults to i32.",
                    false,
                ),
                tutorial_step_with_example(
                    "make-bool",
                    "Store true or false",
                    "A bool stores either true or false.",
                    "let ready = true;",
                    vec![("true", "is a boolean value")],
                    "Now you try: create a variable named ready with the value true.",
                    "The bool type has only two possible values: true and false. Booleans are commonly used for conditions, switches, and state that has two possibilities.",
                    false,
                ),
                tutorial_step_with_example(
                    "print-types",
                    "Put the values together",
                    "println! can insert several variables into the same line.",
                    "println!(\"Lives: {lives}, Ready: {ready}\");",
                    vec![("{lives}", "inserts the number"), ("{ready}", "inserts true or false")],
                    "Now you try: print both variables and Run the program so the output contains Lives: 3 and Ready: true.",
                    "Formatting does not change the variables' types. println! asks each value for a display representation and combines those representations into the output string.",
                    true,
                ),
            ],
        },
        TutorialLesson {
            id: "functions".into(),
            course: "Beginner".into(),
            title: "Make Your Own Functions".into(),
            summary: "Group reusable code under a name, call it, then make another function yourself.".into(),
            skill: "Functions".into(),
            steps: vec![
                tutorial_step_with_example(
                    "create-function",
                    "Give code a name",
                    "fn creates a function whose code can be run later.",
                    "fn greet() {\n    println!(\"Hello!\");\n}",
                    vec![("fn", "creates a function"), ("greet", "is the function's name"), ("()", "means it takes no inputs yet"), ("{ ... }", "contains the function's code")],
                    "Now you try: above main, create a function named greet that prints Hello!.",
                    "Functions let you group a task behind a meaningful name. Rust checks each function independently and lets other code call it whenever that task is needed.",
                    false,
                ),
                tutorial_step_with_example(
                    "call-function",
                    "Run your function",
                    "Writing a function's name followed by () calls it.",
                    "greet();",
                    vec![("greet", "chooses the function"), ("()", "calls it with no arguments"), (";", "ends the call statement")],
                    "Now you try: call greet inside main, then Run the program and make sure Hello! appears.",
                    "A function definition does not run by itself. A call transfers execution into that function, then returns to the line after the call when the function finishes.",
                    true,
                ),
                tutorial_step(
                    "function-challenge",
                    "Challenge: make another one",
                    "Reuse the function pattern without another worked example.",
                    "Create another function that prints You can do this!, call it from main, and Run the program. You choose the function name.",
                    "This challenge asks you to transfer the same structure to a new function instead of copying a new worked answer. That gradual reduction in guidance is how later Oxide lessons will build independence.",
                    true,
                ),
            ],
        },
        TutorialLesson {
            id: "parameters".into(),
            course: "Beginner".into(),
            title: "Give Functions Input".into(),
            summary: "Pass different values into the same function instead of rewriting it.".into(),
            skill: "Function parameters".into(),
            steps: vec![
                tutorial_step_with_example(
                    "make-parameter",
                    "Give the function an input",
                    "A parameter gives a function a named value to work with.",
                    "fn show_score(score: i32) {\n    println!(\"Score: {score}\");\n}",
                    vec![
                        ("score", "is the parameter's name"),
                        (": i32", "says the input is an integer"),
                        ("{score}", "uses that input inside the function"),
                    ],
                    "Now you try: above main, create show_score(score: i32) and make it print Score: followed by the value.",
                    "A parameter is a local name that receives a value whenever the function is called. The : i32 annotation tells Rust that this parameter accepts a 32-bit signed integer, using the integer type you already worked with earlier.",
                    false,
                ),
                tutorial_step_with_example(
                    "call-parameter",
                    "Send a value in",
                    "A value passed into a function call is called an argument.",
                    "show_score(10);",
                    vec![("10", "is the argument sent into score")],
                    "Now you try: call show_score with 10 inside main, then Run the program and make it print Score: 10.",
                    "The function's parameter and the call's argument are two sides of the same handoff. When show_score(10) runs, score receives 10 for the duration of that call.",
                    true,
                ),
                tutorial_step(
                    "parameter-challenge",
                    "Challenge: reuse it",
                    "The same function can work with another argument without changing its definition.",
                    "Reuse the parameterized function with 25 and Run again so Score: 25 appears too. You can rename your function or parameter if you want; the result is what matters here.",
                    "Reusable functions are one of the main reasons parameters exist. The behavior stays in one place while each call supplies the data that changes.",
                    true,
                ),
            ],
        },
        TutorialLesson {
            id: "return-values".into(),
            course: "Beginner".into(),
            title: "Get Values Back".into(),
            summary: "Make functions calculate a value and return it to the caller.".into(),
            skill: "Return values".into(),
            steps: vec![
                tutorial_step_with_example(
                    "make-return",
                    "Return a value",
                    "-> i32 says this function gives an integer back to its caller.",
                    "fn add_one(number: i32) -> i32 {\n    number + 1\n}",
                    vec![
                        ("-> i32", "declares the return type"),
                        ("number + 1", "is the value returned from the function"),
                        ("no ;", "lets the final expression become the return value"),
                    ],
                    "Now you try: above main, create add_one(number: i32) -> i32 and return number + 1.",
                    "Rust functions can return the value of their final expression automatically. Leaving the semicolon off that final expression is significant: adding one would turn it into a statement whose value is (), so it would no longer satisfy an i32 return type.",
                    false,
                ),
                tutorial_step_with_example(
                    "use-return",
                    "Use what came back",
                    "A returned value can be stored in a variable like any other value.",
                    "let result = add_one(4);",
                    vec![("result", "stores the value returned by add_one"), ("4", "is the argument sent into number")],
                    "Now you try: store add_one(4) in result, print Result: {result}, and Run until the output says Result: 5.",
                    "A function call is an expression, so it can appear anywhere Rust expects a value: in a let binding, another function call, a condition, or a larger calculation.",
                    true,
                ),
                tutorial_step(
                    "return-challenge",
                    "Challenge: double it",
                    "Use the same parameter-and-return pattern for a different calculation.",
                    "Create another function that takes an i32 and returns twice that value. Call it with 6 and make the program print 12. You choose the function and variable names.",
                    "This is the same structure as add_one with a different expression. Learning to recognize reusable shapes is more useful than memorizing individual examples.",
                    true,
                ),
            ],
        },
        TutorialLesson {
            id: "conditions".into(),
            course: "Beginner".into(),
            title: "Make Decisions".into(),
            summary: "Use if, else, and comparisons to choose what the program does.".into(),
            skill: "Conditions".into(),
            steps: vec![
                tutorial_step_with_example(
                    "make-if",
                    "Run code only when something is true",
                    "if runs its block only when the condition is true.",
                    "if score >= 10 {\n    println!(\"High score!\");\n}",
                    vec![("if", "starts a condition"), (">= 10", "means greater than or equal to 10"), ("{ ... }", "runs only when the condition is true")],
                    "Now you try: use the existing score variable and print High score! when score is at least 10, then Run it.",
                    "An if condition must produce a bool. Rust does not automatically treat numbers or other values as true or false, so the condition has to be explicit.",
                    true,
                ),
                tutorial_step_with_example(
                    "make-else",
                    "Handle the other case",
                    "else runs when the if condition is false.",
                    "else {\n    println!(\"Low score\");\n}",
                    vec![("else", "handles the false case")],
                    "Now you try: add an else branch, change score to 5, and Run until the program prints Low score.",
                    "if and else form one expression with mutually exclusive branches. Exactly one branch runs when the program reaches it.",
                    true,
                ),
                tutorial_step_with_example(
                    "equality-challenge",
                    "Check for an exact value",
                    "== compares two values for equality.",
                    "if lives == 0 {\n    println!(\"Game over\");\n}",
                    vec![("==", "asks whether both values are equal")],
                    "Now you try: create lives = 0, check whether it equals 0, and Run until Game over appears.",
                    "A single = assigns a value; == compares values. Keeping those jobs visually different helps prevent accidental assignment where a comparison was intended.",
                    true,
                ),
            ],
        },
        TutorialLesson {
            id: "loops".into(),
            course: "Beginner".into(),
            title: "Repeat Work".into(),
            summary: "Repeat code with for and while instead of copying the same line.".into(),
            skill: "Loops".into(),
            steps: vec![
                tutorial_step_with_example(
                    "for-range",
                    "Loop over a range",
                    "for repeats a block once for each value in a sequence.",
                    "for number in 1..=3 {\n    println!(\"Number: {number}\");\n}",
                    vec![("for number in", "gives each value the name number"), ("1..=3", "means 1 through 3, including 3")],
                    "Now you try: add this kind of for loop and Run until Number: 1, Number: 2, and Number: 3 appear.",
                    "Ranges generate a sequence of values. The ..= form includes both ends; 1..=3 produces 1, 2, and 3.",
                    true,
                ),
                tutorial_step_with_example(
                    "while-loop",
                    "Loop while a condition stays true",
                    "while repeats as long as its condition remains true.",
                    "let mut count = 1;\nwhile count <= 3 {\n    println!(\"Count: {count}\");\n    count += 1;\n}",
                    vec![("while", "checks a condition before each repetition"), ("count += 1", "adds 1 to count after each pass")],
                    "Now you try: make count start at 1 and use a while loop that prints Count: 1 through Count: 3.",
                    "A while loop needs something to eventually change its condition. Here count increases each time, so count <= 3 eventually becomes false and the loop stops.",
                    true,
                ),
                tutorial_step(
                    "loop-challenge",
                    "Challenge: choose the simpler loop",
                    "Use a for loop when you already know the range of values you want.",
                    "Add a for loop that prints Number: 4, Number: 5, and Number: 6, then Run it.",
                    "Both for and while can repeat work, but for is usually clearer when iterating over a known sequence. while is useful when repetition depends on changing state or an open-ended condition.",
                    true,
                ),
            ],
        },
        TutorialLesson {
            id: "strings".into(),
            course: "Beginner".into(),
            title: "Build Text".into(),
            summary: "Create owned Strings, change them, and print the result.".into(),
            skill: "Strings".into(),
            steps: vec![
                tutorial_step_with_example(
                    "make-string",
                    "Create owned text",
                    "String::from creates a String value that your program owns.",
                    "let mut message = String::from(\"Hello\");",
                    vec![("String::from", "creates an owned String from text"), ("mut", "lets the String be changed later")],
                    "Now you try: create a mutable String named message containing Hello.",
                    "Rust distinguishes borrowed string slices such as &str from owned String values. String owns its text and can grow or change, which makes it useful for text built while a program runs.",
                    false,
                ),
                tutorial_step_with_example(
                    "append-string",
                    "Add more text",
                    "push_str appends text to the end of a String.",
                    "message.push_str(\", Oxide!\");",
                    vec![(".", "calls a method on message"), ("push_str", "adds text to the end")],
                    "Now you try: append , Oxide! to message.",
                    "Methods are functions associated with a value or type and are commonly called with dot syntax. push_str mutates the existing String rather than creating a completely separate one.",
                    false,
                ),
                tutorial_step_with_example(
                    "print-string",
                    "See the finished String",
                    "A String can be formatted by println! just like the values you used earlier.",
                    "println!(\"{message}\");",
                    vec![("{message}", "inserts the current String contents")],
                    "Now you try: print message and Run until the terminal says Hello, Oxide!",
                    "String implements Rust's Display formatting trait, so println! knows how to represent its contents as normal text.",
                    true,
                ),
            ],
        },
        TutorialLesson {
            id: "vectors".into(),
            course: "Beginner".into(),
            title: "Store a List of Values".into(),
            summary: "Create a Vec, read an item, then grow the list.".into(),
            skill: "Vectors".into(),
            steps: vec![
                tutorial_step_with_example(
                    "make-vector",
                    "Create a vector",
                    "vec! creates a growable list whose items share one type.",
                    "let scores = vec![10, 20, 30];",
                    vec![("vec!", "creates a Vec"), ("[10, 20, 30]", "contains the starting items")],
                    "Now you try: create scores containing 10, 20, and 30.",
                    "Vec<T> stores values next to one another and can grow at runtime. Rust infers the element type from the values here, so this becomes a vector of integers.",
                    false,
                ),
                tutorial_step_with_example(
                    "read-vector",
                    "Read one item",
                    "An index chooses an item by position, starting from 0.",
                    "println!(\"First: {}\", scores[0]);",
                    vec![("scores[0]", "reads the first item because indexes start at 0")],
                    "Now you try: print the first score and Run until First: 10 appears.",
                    "Indexing is concise, but an out-of-bounds index will panic at runtime. Later Rust code often uses safer access patterns when the index is not known to be valid.",
                    true,
                ),
                tutorial_step_with_example(
                    "grow-vector",
                    "Add another item",
                    "push adds one value to the end of a mutable vector.",
                    "scores.push(40);",
                    vec![("push(40)", "adds 40 after the existing items")],
                    "Now you try: make scores mutable, push 40, print scores[3] as Last: 40, and Run it.",
                    "Growing a Vec can require it to move its storage to a larger allocation. Rust manages that memory automatically while still enforcing ownership rules around who may use or modify the vector.",
                    true,
                ),
            ],
        },
        TutorialLesson {
            id: "structs".into(),
            course: "Beginner".into(),
            title: "Group Related Data".into(),
            summary: "Define a struct, create a value, and read its fields.".into(),
            skill: "Structs".into(),
            steps: vec![
                tutorial_step_with_example(
                    "define-struct",
                    "Describe a Player",
                    "struct defines a new type with named fields.",
                    "struct Player {\n    name: String,\n    score: i32,\n}",
                    vec![("Player", "is the new type's name"), ("name: String", "defines a text field"), ("score: i32", "defines an integer field")],
                    "Now you try: above main, define Player with name: String and score: i32.",
                    "A struct lets several related values travel together as one type. Each field has a name and type, so Rust knows exactly what a valid Player must contain.",
                    false,
                ),
                tutorial_step_with_example(
                    "make-struct",
                    "Create one Player",
                    "A struct value supplies a value for each field.",
                    "let player = Player {\n    name: String::from(\"Quinn\"),\n    score: 10,\n};",
                    vec![("Player { ... }", "creates a Player value"), ("name:", "sets the name field"), ("score:", "sets the score field")],
                    "Now you try: inside main, create player with name Quinn and score 10.",
                    "Struct construction names each field explicitly. This makes the meaning of each value visible and lets the compiler reject missing, duplicated, or incorrectly typed fields.",
                    false,
                ),
                tutorial_step_with_example(
                    "use-fields",
                    "Read the fields",
                    "Dot syntax reads a named field from a struct value.",
                    "println!(\"{}: {}\", player.name, player.score);",
                    vec![("player.name", "reads the name field"), ("player.score", "reads the score field")],
                    "Now you try: print the player's name and score, then Run until the terminal contains Quinn: 10.",
                    "Field access is checked at compile time. Rust knows Player has name and score, so a typo such as player.scroe becomes a compiler error instead of silently reading the wrong thing.",
                    true,
                ),
            ],
        },
        TutorialLesson {
            id: "enums-match".into(),
            course: "Beginner".into(),
            title: "Choose Between Possibilities".into(),
            summary: "Represent a fixed set of states with an enum and handle them with match.".into(),
            skill: "Enums and match".into(),
            steps: vec![
                tutorial_step_with_example(
                    "define-enum",
                    "Define the possible directions",
                    "enum creates a type whose value must be one of its listed variants.",
                    "enum Direction {\n    Left,\n    Right,\n}",
                    vec![("enum", "defines a set of named possibilities"), ("Left / Right", "are Direction variants")],
                    "Now you try: above main, define Direction with Left and Right variants.",
                    "Enums are useful when a value can be one of a known set of meaningful states. Unlike loose strings or numbers, the compiler knows every valid variant of the type.",
                    false,
                ),
                tutorial_step_with_example(
                    "make-enum",
                    "Choose one variant",
                    ":: selects a variant that belongs to an enum type.",
                    "let direction = Direction::Left;",
                    vec![("Direction::Left", "creates the Left variant of Direction")],
                    "Now you try: create direction containing Direction::Left.",
                    "The :: path syntax identifies an item inside a type, module, or namespace. Here it makes clear that Left is specifically a Direction variant.",
                    false,
                ),
                tutorial_step_with_example(
                    "match-enum",
                    "Handle every direction",
                    "match compares a value against patterns and runs the matching arm.",
                    "match direction {\n    Direction::Left => println!(\"Going left\"),\n    Direction::Right => println!(\"Going right\"),\n}",
                    vec![("match direction", "examines direction"), ("=>", "connects a pattern to what should run")],
                    "Now you try: match direction and Run until Going left appears.",
                    "Rust checks whether a match is exhaustive. For an enum, that means every possible variant must be handled unless a broader pattern intentionally catches the remaining cases.",
                    true,
                ),
                tutorial_step(
                    "match-challenge",
                    "Challenge: take the other path",
                    "Changing the enum value should make the other match arm run without changing the match itself.",
                    "Change direction to Direction::Right and Run until Going right appears.",
                    "The data chooses the behavior. This is a common Rust pattern: represent state explicitly with an enum, then use match to handle each valid state deliberately.",
                    true,
                ),
            ],
        },
        TutorialLesson {
            id: "ownership".into(),
            course: "Beginner".into(),
            title: "Break It: Ownership".into(),
            summary: "Move a String, trigger the real moved-value error, then deliberately copy it.".into(),
            skill: "Ownership basics".into(),
            steps: vec![
                tutorial_step_with_example(
                    "see-move",
                    "Move ownership",
                    "Assigning a String to another variable moves that owned value by default.",
                    "let moved_name = name;",
                    vec![("name", "gives its owned String to moved_name"), ("moved_name", "becomes the new owner")],
                    "Now you try: Run the starter program and confirm it prints Oxide through moved_name.",
                    "String owns heap-allocated text, so Rust does not silently duplicate it during ordinary assignment. Moving the value transfers responsibility for that allocation to the new binding.",
                    true,
                ),
                tutorial_step_with_example(
                    "break-move",
                    "Use the old owner",
                    "After a move, using the old variable makes rustc report a moved-value error.",
                    "println!(\"Original: {name}\");",
                    vec![("name", "tries to use the value after ownership moved away")],
                    "Now you try: add that println! after the move and wait for Oxide to show rustc error E0382.",
                    "E0382 prevents use-after-move bugs. Rust knows name no longer owns the String, so it rejects later access instead of risking two owners freeing or mutating the same resource incorrectly.",
                    false,
                ),
                tutorial_step_with_example(
                    "clone-value",
                    "Make a real copy",
                    "clone explicitly duplicates a value when you truly need two owned copies.",
                    "let moved_name = name.clone();",
                    vec![("clone()", "creates a separate owned copy instead of moving the original")],
                    "Now you try: change the move to name.clone(), keep both print statements, and Run successfully.",
                    "Cloning can be useful, but it may allocate and copy data. Rust makes that cost explicit. The next lesson shows how borrowing often lets code use a value without moving or cloning it.",
                    true,
                ),
            ],
        },
        TutorialLesson {
            id: "borrowing".into(),
            course: "Beginner".into(),
            title: "Borrow Instead of Move".into(),
            summary: "Let a function read a String while the original owner keeps it.".into(),
            skill: "Borrowing and references".into(),
            steps: vec![
                tutorial_step_with_example(
                    "borrow-value",
                    "Lend the String to a function",
                    "& creates a reference so code can use a value without taking ownership.",
                    "show_name(&name);",
                    vec![("&name", "borrows name instead of moving its String")],
                    "Now you try: call the provided show_name function with &name, then Run until Borrowed: Oxide appears.",
                    "A shared reference gives temporary read access to a value. The owner remains responsible for the String and can continue using it after the borrowed access ends.",
                    true,
                ),
                tutorial_step_with_example(
                    "owner-still-works",
                    "Use the owner again",
                    "Borrowing leaves ownership where it was.",
                    "println!(\"Still mine: {name}\");",
                    vec![("name", "is still valid because show_name only borrowed it")],
                    "Now you try: print Still mine: {name} after show_name and Run until both lines appear.",
                    "The borrow used by show_name ends when that call no longer needs the reference. Because ownership never moved, name remains valid afterward.",
                    true,
                ),
                tutorial_step_with_example(
                    "borrow-challenge",
                    "Challenge: borrow it again",
                    "A value can be borrowed again whenever the borrowing rules allow it.",
                    "fn show_twice(name: &String) {\n    println!(\"{name} {name}\");\n}",
                    vec![("&String", "accepts a shared reference to a String")],
                    "Now you try: create show_twice(name: &String), call it with &name, and Run until Oxide Oxide appears.",
                    "Shared references are cheap handles to existing data. Rust tracks their lifetimes so the referenced value cannot disappear while a valid reference still needs it.",
                    true,
                ),
            ],
        },
        TutorialLesson {
            id: "string-slices".into(),
            course: "Beginner".into(),
            title: "Borrow Just the Text".into(),
            summary: "Use &str when code only needs a borrowed view of string text.".into(),
            skill: "String slices".into(),
            steps: vec![
                tutorial_step_with_example(
                    "slice-function",
                    "Accept borrowed text",
                    "&str is a borrowed view of string text.",
                    "fn show_label(label: &str) {\n    println!(\"Label: {label}\");\n}",
                    vec![("&str", "borrows string text without taking ownership"), ("label", "is the borrowed text inside the function")],
                    "Now you try: create show_label(label: &str), call it with &label, and Run until Label: Oxide appears.",
                    "A string slice does not own the text it points at. It describes a valid region of UTF-8 string data for as long as that data remains available. Functions that only need to read text often accept &str because both String values and string literals can be borrowed as slices.",
                    true,
                ),
                tutorial_step_with_example(
                    "literal-slice",
                    "Pass text directly",
                    "A string literal such as \"Rust\" already behaves like borrowed &str text.",
                    "show_label(\"Rust\");",
                    vec![("\"Rust\"", "is a string literal that can be passed as &str")],
                    "Now you try: call show_label with the literal \"Rust\" and Run until Label: Rust appears.",
                    "String literals live in the program binary and have a static lifetime. Their type is &str, so a function taking &str can use them directly without creating an owned String first.",
                    true,
                ),
                tutorial_step_with_example(
                    "slice-challenge",
                    "Challenge: reuse the pattern",
                    "The same &str parameter can be used by any function that only needs to read text.",
                    "fn shout(text: &str) {\n    println!(\"{text}!\");\n}",
                    vec![("text: &str", "accepts borrowed text")],
                    "Now you try: create shout(text: &str), call shout(\"Go\"), and Run until Go! appears.",
                    "Using &str in read-only text APIs makes them flexible: callers can pass a slice, a literal, or a borrowed String without transferring ownership.",
                    true,
                ),
            ],
        },
        TutorialLesson {
            id: "mutable-references".into(),
            course: "Beginner".into(),
            title: "Borrow It and Change It".into(),
            summary: "Use &mut to temporarily let a function modify a value it does not own.".into(),
            skill: "Mutable references".into(),
            steps: vec![
                tutorial_step_with_example(
                    "mut-ref-function",
                    "Give temporary write access",
                    "&mut creates a mutable reference that can change the borrowed value.",
                    "fn add_point(score: &mut i32) {\n    *score += 1;\n}",
                    vec![("&mut i32", "borrows an integer with permission to change it"), ("*score", "accesses the value through the reference")],
                    "Now you try: create add_point(score: &mut i32), make score mutable, call add_point(&mut score), print score afterward, and Run until Score: 11 appears.",
                    "A mutable reference temporarily grants exclusive write access to a value. The * operator dereferences the reference so the code can modify the integer stored behind it.",
                    true,
                ),
                tutorial_step(
                    "break-mut-ref",
                    "Break the permission",
                    "A value must itself be mutable before Rust will let you borrow it as &mut.",
                    "Remove mut from let mut score = 10; and wait for rustc to reject add_point(&mut score).",
                    "Rust does not let mutable borrowing bypass the original binding's rules. If score is immutable, creating &mut score would grant write access that the binding never allowed, so rustc reports an error such as E0596.",
                    false,
                ),
                tutorial_step_with_example(
                    "fix-mut-ref",
                    "Restore it and borrow twice",
                    "Separate mutable borrows can happen one after another.",
                    "add_point(&mut score);\nadd_point(&mut score);",
                    vec![("&mut score", "temporarily lends score with write access")],
                    "Now you try: restore mut, make the program call add_point exactly twice in total, and Run until Score: 12 appears.",
                    "Rust permits repeated mutable borrows when the previous borrow is finished. What it prevents is conflicting access to the same value at the same time.",
                    true,
                ),
            ],
        },
        TutorialLesson {
            id: "methods".into(),
            course: "Beginner".into(),
            title: "Give a Struct Behavior".into(),
            summary: "Use impl, self, and methods to put behavior next to a struct.".into(),
            skill: "Methods and impl".into(),
            steps: vec![
                tutorial_step_with_example(
                    "show-method",
                    "Add a method",
                    "impl is where methods for a type are defined.",
                    "impl Counter {\n    fn show(&self) {\n        println!(\"Count: {}\", self.value);\n    }\n}",
                    vec![("impl Counter", "adds behavior to Counter"), ("&self", "borrows the Counter the method was called on"), ("self.value", "reads that Counter's value field")],
                    "Now you try: add show(&self), replace the direct println! with counter.show(), and Run until Count: 3 appears.",
                    "Methods are ordinary functions associated with a type, but self gives them convenient access to the value they were called on. &self means the method only needs shared read access.",
                    true,
                ),
                tutorial_step_with_example(
                    "changing-method",
                    "Make a changing method",
                    "&mut self lets a method change the value it was called on.",
                    "fn add_one(&mut self) {\n    self.value += 1;\n}",
                    vec![("&mut self", "borrows this Counter with permission to change it"), ("self.value += 1", "increments its value field")],
                    "Now you try: add add_one(&mut self), make counter mutable, call counter.add_one(), then counter.show().",
                    "&mut self follows the same mutable-reference rules you just practiced. The method receives temporary exclusive access to the Counter while it changes the field.",
                    false,
                ),
                tutorial_step(
                    "method-challenge",
                    "Challenge: call it twice",
                    "Methods can be called repeatedly as long as Rust's borrowing rules are satisfied.",
                    "Call counter.add_one() twice before show(), then Run until Count: 5 appears.",
                    "Each method call finishes its mutable borrow before the next call begins, so the same Counter can be modified repeatedly in sequence.",
                    true,
                ),
            ],
        },
        TutorialLesson {
            id: "option".into(),
            course: "Beginner".into(),
            title: "A Value Might Be Missing".into(),
            summary: "Use Option, Some, and None instead of inventing magic missing values.".into(),
            skill: "Option".into(),
            steps: vec![
                tutorial_step_with_example(
                    "run-some",
                    "See Some",
                    "Some(value) means an Option currently contains a value.",
                    "let score = Some(10);",
                    vec![("Some(10)", "stores the value 10 inside an Option")],
                    "Now you try: Run the starter and confirm it prints Score: 10.",
                    "Option<T> represents either Some(T) or None. The type forces code to acknowledge that a value might be absent instead of relying on a special number or null pointer.",
                    true,
                ),
                tutorial_step_with_example(
                    "use-none",
                    "Represent no value",
                    "None means an Option contains no value.",
                    "let score: Option<i32> = None;",
                    vec![("Option<i32>", "means an optional i32"), ("None", "means there is currently no i32 value")],
                    "Now you try: change score to Option<i32> = None and Run until No score appears.",
                    "None does not carry a value, so Rust sometimes needs an explicit type annotation to know what kind of Option you mean. Here Option<i32> says a future value would be an i32.",
                    true,
                ),
                tutorial_step_with_example(
                    "option-challenge",
                    "Challenge: make another Option",
                    "match lets you safely handle both Some and None.",
                    "match lives {\n    Some(value) => println!(\"Lives: {value}\"),\n    None => println!(\"No lives\"),\n}",
                    vec![("Some(value)", "extracts the contained value"), ("None", "handles the missing case")],
                    "Now you try: create lives = Some(3), match it, and Run until Lives: 3 appears.",
                    "Matching an Option is exhaustive: Rust requires both the present and missing cases to be covered unless another pattern deliberately handles the remainder.",
                    true,
                ),
            ],
        },
        TutorialLesson {
            id: "result".into(),
            course: "Beginner".into(),
            title: "Success or Failure".into(),
            summary: "Use Result to handle operations that can succeed or fail.".into(),
            skill: "Result and basic error handling".into(),
            steps: vec![
                tutorial_step_with_example(
                    "parse-result",
                    "Get a Result",
                    "parse::<i32>() tries to convert text into an integer and returns a Result.",
                    "let parsed = \"42\".parse::<i32>();",
                    vec![("parse::<i32>()", "tries to make an i32 from the text"), ("parsed", "stores the Result of that attempt")],
                    "Now you try: Run the starter and confirm it prints Number: 42.",
                    "Result<T, E> is Rust's standard way to represent an operation that can either return a useful value or an error. Parsing text can fail, so parse does not pretend success is guaranteed.",
                    true,
                ),
                tutorial_step_with_example(
                    "result-error",
                    "Take the error path",
                    "Err means the operation failed instead of producing the requested value.",
                    "let parsed = \"not a number\".parse::<i32>();",
                    vec![("Err", "is the Result variant used for failure")],
                    "Now you try: change the input text to not a number and Run until Could not parse appears.",
                    "The error value inside Err can carry detailed information. This lesson ignores the details with Err(_) so you can focus on the success/failure shape first.",
                    true,
                ),
                tutorial_step_with_example(
                    "result-challenge",
                    "Challenge: parse another number",
                    "Ok(value) contains the successful value produced by a Result.",
                    "Ok(number) => println!(\"Number: {number}\"),",
                    vec![("Ok(number)", "extracts the successful integer")],
                    "Now you try: change the input to \"100\" and Run until Number: 100 appears.",
                    "Matching Result makes failure handling explicit. Later Rust code can also use helpers and the ? operator to propagate errors when handling them locally would only add noise.",
                    true,
                ),
            ],
        },
        TutorialLesson {
            id: "hashmaps".into(),
            course: "Beginner".into(),
            title: "Look Up Values by Name".into(),
            summary: "Store key/value pairs in a HashMap and retrieve them safely.".into(),
            skill: "Hash maps".into(),
            steps: vec![
                tutorial_step_with_example(
                    "make-map",
                    "Create a hash map",
                    "HashMap::new creates an empty key/value collection.",
                    "let scores: HashMap<&str, i32> = HashMap::new();",
                    vec![("HashMap::new()", "creates an empty hash map"), ("HashMap<&str, i32>", "stores text keys with i32 values")],
                    "Now you try: Run the starter and confirm it reports 0 entries.",
                    "A HashMap stores values by keys instead of numeric positions. Rust's standard HashMap lives in std::collections, which is why the starter imports it with use.",
                    true,
                ),
                tutorial_step_with_example(
                    "insert-map",
                    "Insert a score",
                    "insert stores a value under a key.",
                    "scores.insert(\"Quinn\", 10);",
                    vec![("\"Quinn\"", "is the lookup key"), ("10", "is the value stored under that key")],
                    "Now you try: make scores mutable, insert Quinn with score 10, and print the map length so it reports 1 entry.",
                    "If a key already exists, insert replaces its value and returns the old one. The map owns or borrows its keys and values according to the types you put into it.",
                    true,
                ),
                tutorial_step_with_example(
                    "get-map",
                    "Look up the score",
                    "get returns an Option because the requested key might not exist.",
                    "match scores.get(\"Quinn\") {\n    Some(score) => println!(\"Quinn: {score}\"),\n    None => println!(\"Missing\"),\n}",
                    vec![("get(\"Quinn\")", "looks up the key"), ("Some(score)", "handles a value that was found")],
                    "Now you try: look up Quinn and Run until Quinn: 10 appears.",
                    "HashMap::get returns Option<&V>, which combines two ideas you already know: the value might be missing, and a successful lookup borrows the stored value instead of moving it out of the map.",
                    true,
                ),
            ],
        },
        TutorialLesson {
            id: "modules".into(),
            course: "Beginner".into(),
            title: "Organize Code into Modules".into(),
            summary: "Group related code with mod, expose functions with pub, and call them by path.".into(),
            skill: "Modules".into(),
            steps: vec![
                tutorial_step_with_example(
                    "run-module",
                    "Use a module",
                    "mod groups related Rust items under one name.",
                    "mod greetings {\n    pub fn hello() {\n        println!(\"Hello from module!\");\n    }\n}",
                    vec![("mod greetings", "creates the greetings module"), ("pub", "makes hello callable from outside that module")],
                    "Now you try: Run the starter and confirm it prints Hello from module!.",
                    "Modules create namespaces and privacy boundaries. Items are private by default, so pub intentionally exposes an item to code outside its module.",
                    true,
                ),
                tutorial_step_with_example(
                    "module-path",
                    "Call through the module path",
                    ":: selects an item inside a module.",
                    "greetings::hello();",
                    vec![("greetings::", "enters the greetings module"), ("hello()", "calls its public function")],
                    "Now you try: add a public goodbye function that prints Goodbye from module!.",
                    "The :: path syntax is used throughout Rust to locate items in modules, types, crates, and the standard library.",
                    false,
                ),
                tutorial_step(
                    "module-challenge",
                    "Challenge: call the new function",
                    "Once a module function is public, main can call it through the same module path.",
                    "Call greetings::goodbye() and Run until Goodbye from module! appears.",
                    "Larger Rust projects commonly move modules into separate files, but the visibility and path rules remain the same. This inline module lets you practice the core idea before file layout is added later.",
                    true,
                ),
            ],
        },
        TutorialLesson {
            id: "user-input".into(),
            course: "Beginner".into(),
            title: "Talk to the Program".into(),
            summary: "Read keyboard input through stdin and use what the user typed.".into(),
            skill: "Interactive terminal input".into(),
            steps: vec![
                tutorial_step_with_example(
                    "read-line",
                    "Read one line",
                    "read_line stores one line of terminal input into a mutable String.",
                    "let mut name = String::new();\nstd::io::stdin().read_line(&mut name).expect(\"Failed to read input\");",
                    vec![("String::new()", "creates an empty String for the input"), ("read_line(&mut name)", "fills name with what the user types"), ("expect(...) ", "stops with a message if reading fails")],
                    "Now you try: add those two lines inside main so the program can receive terminal input.",
                    "stdin returns the process's standard input handle. read_line appends input into the provided String and returns a Result, while expect is a simple way to stop the program if that operation unexpectedly fails.",
                    false,
                ),
                tutorial_step_with_example(
                    "trim-input",
                    "Remove the Enter key",
                    "trim returns the text without surrounding whitespace such as the newline from Enter.",
                    "let name = name.trim();",
                    vec![("trim()", "borrows the String without its leading or trailing whitespace")],
                    "Now you try: trim name and print Hello, {name}!.",
                    "read_line keeps the newline that ended the input. trim returns a &str slice into the same String with surrounding whitespace removed, so the greeting does not contain an unwanted line break.",
                    false,
                ),
                tutorial_step(
                    "input-run",
                    "Have a conversation",
                    "The Oxide Run Terminal sends your typed input to the real program's stdin.",
                    "Run the program, type Quinn into the Run Terminal, press Enter, and make it print Hello, Quinn!.",
                    "Oxide's terminal is connected to the child process's real stdin/stdout pipes. The tutorial is not simulating the interaction; your compiled executable receives the characters you type.",
                    true,
                ),
            ],
        },
        TutorialLesson {
            id: "mini-calculator".into(),
            course: "Beginner".into(),
            title: "Mini Project: Calculator".into(),
            summary: "Combine functions, input, parsing, variables, and arithmetic into a usable CLI program.".into(),
            skill: "Combining input and calculations".into(),
            steps: vec![
                tutorial_step_with_example(
                    "second-number",
                    "Read another number",
                    "A helper function lets you reuse the same input-and-parse workflow.",
                    "let second = read_number(\"Second number:\");\nprintln!(\"Second: {second}\");",
                    vec![("read_number(...) ", "calls the provided helper and receives its i32 result"), ("{second}", "uses the returned value immediately")],
                    "Now you try: after first, read a second number using read_number and print Second: {second}.",
                    "The helper hides repeated stdin and parsing details behind a function that returns i32. This is one reason functions matter: main can describe the program at a higher level.",
                    false,
                ),
                tutorial_step_with_example(
                    "calculator-total",
                    "Add the values",
                    "The + operator adds numeric values.",
                    "let total = first + second;\nprintln!(\"Total: {total}\");",
                    vec![("first + second", "adds the two i32 values"), ("total", "stores the result")],
                    "Now you try: calculate total and print it.",
                    "Both inputs are i32 values because read_number parsed them before returning. That means ordinary integer arithmetic works without another conversion step.",
                    false,
                ),
                tutorial_step(
                    "calculator-run",
                    "Run your calculator",
                    "A project becomes useful when you can feed it real values and verify the behavior.",
                    "Run the calculator, enter 4 and 6 when prompted, and make the terminal print Total: 10.",
                    "This small program already combines reusable functions, borrowed prompt text, terminal input, Result-based parsing through expect, variables, return values, and arithmetic.",
                    true,
                ),
                tutorial_step(
                    "calculator-challenge",
                    "Challenge: add subtraction",
                    "Use the same two values to calculate another result without another worked example.",
                    "Also print Difference: -2 when the inputs are 4 and 6, then Run it again.",
                    "The challenge deliberately gives you the desired behavior rather than the exact code. At this point you already know variables, arithmetic expressions, and println!, so Oxide can reduce the scaffolding.",
                    true,
                ),
            ],
        },
        TutorialLesson {
            id: "mini-scoreboard".into(),
            course: "Beginner".into(),
            title: "Mini Project: Scoreboard".into(),
            summary: "Combine structs, vectors, loops, and conditions into a small data-driven program.".into(),
            skill: "Combining data structures and control flow".into(),
            steps: vec![
                tutorial_step_with_example(
                    "add-player",
                    "Grow the roster",
                    "A Vec can store several values of the same struct type.",
                    "players.push(Player { name: String::from(\"Oxide\"), score: 20 });",
                    vec![("players.push(...) ", "adds another Player to the vector")],
                    "Now you try: make players mutable, then add an Oxide player with score 20 to the provided players vector.",
                    "Because every item is a Player, Rust guarantees that each vector entry has the same fields and field types. That makes later iteration predictable.",
                    false,
                ),
                tutorial_step_with_example(
                    "scoreboard-loop",
                    "Print every player",
                    "for can visit each value stored in a collection.",
                    "for player in &players {\n    println!(\"{}: {}\", player.name, player.score);\n}",
                    vec![("&players", "borrows the vector so the loop does not consume it"), ("player", "is each borrowed Player in turn")],
                    "Now you try: loop over &players and print every player's name and score.",
                    "Borrowing the vector in the loop keeps players available afterward. Each player is therefore a shared reference to an item rather than an owned Player moved out of the Vec.",
                    false,
                ),
                tutorial_step(
                    "scoreboard-run",
                    "Run the scoreboard",
                    "The program should now turn structured data into visible output.",
                    "Run until the terminal contains both Quinn: 10 and Oxide: 20.",
                    "This is a small but real data flow: values are grouped into structs, stored in a vector, borrowed by a loop, and formatted for the user.",
                    true,
                ),
                tutorial_step(
                    "scoreboard-challenge",
                    "Challenge: mark the winner",
                    "Use a condition inside the loop to react to a player's score.",
                    "Make the program also print Winner: Oxide for the player whose score is 20, then Run it.",
                    "You now have enough pieces to solve this without a worked answer: field access gives you player.score, an if expression can test it, and player.name can be printed when the condition is true.",
                    true,
                ),
            ],
        },
    ]
}

fn tutorial_initial_source(lesson_id: &str) -> Option<&'static str> {
    match lesson_id {
        "hello-world" => Some("fn main() {\n    println!(\"Hello, world!\");\n}\n"),
        "variables" => Some("fn main() {\n\n}\n"),
        "warnings-errors" => Some("fn main() {\n    let message = \"Hello\";\n    println!(\"Program still runs.\");\n}\n"),
        "mutability" => Some("fn main() {\n    let mut score = 10;\n    println!(\"Starting score: {score}\");\n    score = 11;\n    println!(\"Score: {score}\");\n}\n"),
        "basic-types" => Some("fn main() {\n\n}\n"),
        "functions" => Some("fn main() {\n\n}\n"),
        "parameters" => Some("fn main() {\n\n}\n"),
        "return-values" => Some("fn main() {\n\n}\n"),
        "conditions" => Some("fn main() {\n    let score = 12;\n\n}\n"),
        "loops" => Some("fn main() {\n\n}\n"),
        "strings" => Some("fn main() {\n\n}\n"),
        "vectors" => Some("fn main() {\n\n}\n"),
        "structs" => Some("fn main() {\n\n}\n"),
        "enums-match" => Some("fn main() {\n\n}\n"),
        "ownership" => Some("fn main() {\n    let name = String::from(\"Oxide\");\n    let moved_name = name;\n    println!(\"{moved_name}\");\n}\n"),
        "borrowing" => Some("fn show_name(name: &String) {\n    println!(\"Borrowed: {name}\");\n}\n\nfn main() {\n    let name = String::from(\"Oxide\");\n\n}\n"),
        "string-slices" => Some("fn main() {\n    let label = String::from(\"Oxide\");\n    println!(\"{label}\");\n}\n"),
        "mutable-references" => Some("fn main() {\n    let score = 10;\n    println!(\"Score: {score}\");\n}\n"),
        "methods" => Some("struct Counter {\n    value: i32,\n}\n\nfn main() {\n    let counter = Counter { value: 3 };\n    println!(\"Count: {}\", counter.value);\n}\n"),
        "option" => Some("fn main() {\n    let score = Some(10);\n\n    match score {\n        Some(value) => println!(\"Score: {value}\"),\n        None => println!(\"No score\"),\n    }\n}\n"),
        "result" => Some("fn main() {\n    let parsed = \"42\".parse::<i32>();\n\n    match parsed {\n        Ok(number) => println!(\"Number: {number}\"),\n        Err(_) => println!(\"Could not parse\"),\n    }\n}\n"),
        "hashmaps" => Some("use std::collections::HashMap;\n\nfn main() {\n    let scores: HashMap<&str, i32> = HashMap::new();\n    println!(\"Entries: {}\", scores.len());\n}\n"),
        "modules" => Some("mod greetings {\n    pub fn hello() {\n        println!(\"Hello from module!\");\n    }\n}\n\nfn main() {\n    greetings::hello();\n}\n"),
        "user-input" => Some("fn main() {\n\n}\n"),
        "mini-calculator" => Some("fn read_number(prompt: &str) -> i32 {\n    println!(\"{prompt}\");\n    let mut input = String::new();\n    std::io::stdin()\n        .read_line(&mut input)\n        .expect(\"Failed to read input\");\n    input.trim().parse::<i32>().expect(\"Please enter a number\")\n}\n\nfn main() {\n    let first = read_number(\"First number:\");\n    println!(\"First: {first}\");\n}\n"),
        "mini-scoreboard" => Some("struct Player {\n    name: String,\n    score: i32,\n}\n\nfn main() {\n    let players = vec![\n        Player { name: String::from(\"Quinn\"), score: 10 },\n    ];\n\n    println!(\"{}: {}\", players[0].name, players[0].score);\n}\n"),
        _ => None,
    }
}

fn oxide_data_dir() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("APPDATA") {
        return Ok(PathBuf::from(path).join("Oxide Editor"));
    }
    if let Some(path) = env::var_os("XDG_DATA_HOME") {
        return Ok(PathBuf::from(path).join("oxide-editor"));
    }
    if let Some(path) = env::var_os("HOME") {
        return Ok(PathBuf::from(path).join(".local").join("share").join("oxide-editor"));
    }
    Err("Oxide could not determine a writable application-data folder.".into())
}

fn tutorial_progress_path() -> Result<PathBuf, String> {
    Ok(oxide_data_dir()?.join("tutorial-progress.json"))
}

fn load_tutorial_progress_file() -> TutorialProgressFile {
    let Ok(path) = tutorial_progress_path() else { return TutorialProgressFile::default(); };
    let Ok(text) = fs::read_to_string(path) else { return TutorialProgressFile::default(); };
    serde_json::from_str(&text).unwrap_or_default()
}

fn save_tutorial_progress_file(progress: &TutorialProgressFile) -> Result<(), String> {
    let path = tutorial_progress_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("Could not create tutorial data folder: {error}"))?;
    }
    let json = serde_json::to_string_pretty(progress)
        .map_err(|error| format!("Could not serialize tutorial progress: {error}"))?;
    fs::write(&path, json).map_err(|error| format!("Could not save tutorial progress: {error}"))
}

#[tauri::command]
fn tutorial_catalog() -> TutorialCatalog {
    TutorialCatalog {
        beginner: tutorial_lessons(),
        advanced_topics: vec![
            "Deeper ownership and borrowing".into(),
            "Lifetimes".into(),
            "Traits and generics".into(),
            "Iterators and closures".into(),
            "Smart pointers".into(),
            "Concurrency, channels, and async Rust".into(),
            "Macros and testing".into(),
            "Performance, profiling, unsafe Rust, and FFI".into(),
        ],
    }
}

#[tauri::command]
fn tutorial_progress() -> TutorialProgressFile {
    load_tutorial_progress_file()
}

#[tauri::command]
fn tutorial_set_progress(
    lesson_id: String,
    completed_steps: usize,
    completed: bool,
    checkpoint_source: String,
) -> Result<(), String> {
    let mut progress = load_tutorial_progress_file();
    progress.lessons.insert(
        lesson_id,
        TutorialProgressEntry {
            completed_steps,
            completed,
            checkpoint_source,
        },
    );
    save_tutorial_progress_file(&progress)
}

#[tauri::command]
fn tutorial_prepare_lesson(lesson_id: String, reset: bool) -> Result<String, String> {
    let lesson = tutorial_lessons()
        .into_iter()
        .find(|lesson| lesson.id == lesson_id)
        .ok_or_else(|| "Unknown Oxide tutorial lesson.".to_string())?;
    let source = tutorial_initial_source(&lesson.id)
        .ok_or_else(|| "This lesson does not have a project template yet.".to_string())?;

    let destination = oxide_data_dir()?
        .join("tutorials")
        .join("beginner")
        .join(&lesson.id);
    let main_path = destination.join("src").join("main.rs");

    if reset && destination.exists() {
        fs::remove_dir_all(&destination)
            .map_err(|error| format!("Could not reset lesson project: {error}"))?;
    }

    if !destination.exists() {
        fs::create_dir_all(destination.join("src"))
            .map_err(|error| format!("Could not create lesson project: {error}"))?;
        let package_name = format!("oxide-tutorial-{}", lesson.id);
        let manifest = format!(
            "[package]\nname = \"{}\"\nversion = \"0.0.1\"\nedition = \"2024\"\n\n[dependencies]\n",
            package_name
        );
        fs::write(destination.join("Cargo.toml"), manifest)
            .map_err(|error| format!("Could not create lesson Cargo.toml: {error}"))?;
        fs::write(&main_path, source)
            .map_err(|error| format!("Could not create lesson main.rs: {error}"))?;
    } else if !main_path.exists() {
        fs::create_dir_all(destination.join("src"))
            .map_err(|error| format!("Could not repair lesson src folder: {error}"))?;
        fs::write(&main_path, source)
            .map_err(|error| format!("Could not repair lesson main.rs: {error}"))?;
    } else if lesson.id == "mutability" {
        let old_template = "fn main() {\n    let mut score = 10;\n    score = 11;\n    println!(\"Score: {score}\");\n}\n";
        if fs::read_to_string(&main_path).ok().as_deref() == Some(old_template) {
            fs::write(&main_path, source)
                .map_err(|error| format!("Could not update the mutability lesson template: {error}"))?;
        }
        let mut progress = load_tutorial_progress_file();
        if let Some(entry) = progress.lessons.get_mut("mutability") {
            if entry.checkpoint_source == old_template {
                entry.checkpoint_source = source.to_string();
                save_tutorial_progress_file(&progress)?;
            }
        }
    }

    Ok(destination.to_string_lossy().to_string())
}

fn compact_source(source: &str) -> String {
    source.chars().filter(|character| !character.is_whitespace()).collect()
}

#[tauri::command]
fn tutorial_evaluate(request: TutorialEvaluationRequest) -> TutorialEvaluationResult {
    let compact = compact_source(&request.source);
    let output = request.run_output.replace('\r', "");
    // Tutorial output comparisons are intentionally case-insensitive unless a
    // future lesson explicitly teaches capitalization. Rust source itself stays
    // case-sensitive; only the observable program output is normalized here.
    let output_lower = output.to_lowercase();
    let output_compact_lower: String = output_lower.chars().filter(|character| !character.is_whitespace()).collect();
    let ran_ok = request.run_success == Some(true);
    let has_code = |code: &str| request.diagnostic_codes.iter().any(|item| item.eq_ignore_ascii_case(code));
    let has_message = |needle: &str| request.diagnostic_messages.iter().any(|item| item.to_lowercase().contains(&needle.to_lowercase()));
    let has_level = |level: &str| request.diagnostic_levels.iter().any(|item| item.eq_ignore_ascii_case(level));

    let complete = match (request.lesson_id.as_str(), request.step_index) {
        ("hello-world", 0) => ran_ok && output_lower.contains("hello, world!"),
        ("hello-world", 1) => compact.contains("println!(\"Hello,Oxide!\");") && ran_ok && output_lower.contains("hello, oxide!"),
        ("variables", 0) => compact.contains("letname") && compact.contains("=\"Quinn\";"),
        ("variables", 1) => {
            compact.contains("println!(\"{name}\");")
                || compact.contains("println!(\"{}\",name);")
                || (compact.contains("println!(") && compact.contains(",name);"))
        }
        ("variables", 2) => ran_ok && output_lower.contains("quinn"),
        ("variables", 3) => {
            compact.contains("letscore")
                && compact.contains("=10;")
                && (compact.contains("{score}") || compact.contains(",score)"))
                && ran_ok
                && output_lower.contains("10")
        }
        ("warnings-errors", 0) => {
            has_level("warning") && has_message("unused variable") && ran_ok && output_lower.contains("program still runs.")
        }
        ("warnings-errors", 1) => {
            compact.contains("letmessage=\"Hello\"")
                && !compact.contains("letmessage=\"Hello\";")
                && has_level("error")
        }
        ("warnings-errors", 2) => {
            compact.contains("letmessage=\"Hello\";")
                && (compact.contains("println!(\"{message}\");") || compact.contains("println!(\"{}\",message);"))
                && !has_level("error")
                && !has_message("unused variable")
                && ran_ok
                && output_lower.contains("hello")
        }
        ("mutability", 0) => ran_ok && output_lower.contains("starting score: 10") && output_lower.contains("score: 11"),
        ("mutability", 1) => {
            !(compact.contains("letmutscore") && compact.contains("=10;"))
                && (has_code("E0384") || has_message("cannot assign twice to immutable variable") || has_message("immutable"))
        }
        ("mutability", 2) => compact.contains("letmutscore") && compact.contains("=10;") && ran_ok && output_lower.contains("score: 11"),
        ("basic-types", 0) => compact.contains("letlives=3;"),
        ("basic-types", 1) => compact.contains("letready=true;"),
        ("basic-types", 2) => {
            ran_ok && output_lower.contains("lives: 3") && output.to_lowercase().contains("ready: true")
        }
        ("functions", 0) => {
            compact.contains("fngreet(){") && compact.contains("println!(\"Hello!\");")
        }
        ("functions", 1) => compact.contains("greet();") && ran_ok && output_lower.contains("hello!"),
        ("functions", 2) => {
            // Challenge steps are outcome-first: require the concept (a second function)
            // and the expected behavior, but do not require Oxide's suggested identifier.
            compact.matches("fn").count() >= 2
                && ran_ok
                && output_lower.contains("you can do this!")
        }
        ("parameters", 0) => {
            compact.contains("fnshow_score(score:i32){")
                && compact.contains("println!(\"Score:{score}\");")
        }
        ("parameters", 1) => compact.contains("show_score(10);") && ran_ok && output_lower.contains("score: 10"),
        ("parameters", 2) => {
            compact.contains(":i32")
                && compact.contains("(25)")
                && ran_ok
                && output_lower.contains("score: 25")
        },
        ("return-values", 0) => {
            compact.contains("fnadd_one(number:i32)->i32{") && compact.contains("number+1}")
        }
        ("return-values", 1) => {
            compact.contains("letresult=add_one(4);")
                && ran_ok
                && output_compact_lower.contains("result:5")
        }
        ("return-values", 2) => {
            // Accept double(), create_double(), twice(), etc. The learner must still
            // demonstrate the return-value concept instead of merely printing 12.
            compact.matches("->i32{").count() >= 2
                && compact.contains("*2")
                && compact.contains("(6)")
                && ran_ok
                && output_lower.contains("12")
        }
        ("conditions", 0) => compact.contains("ifscore>=10{") && ran_ok && output_lower.contains("high score!"),
        ("conditions", 1) => compact.contains("letscore=5;") && compact.contains("else{") && ran_ok && output_lower.contains("low score"),
        ("conditions", 2) => {
            compact.contains("if")
                && ran_ok
                && output_lower.contains("game over")
        }
        ("loops", 0) => {
            compact.contains("fornumberin1..=3{")
                && ran_ok
                && output_lower.contains("number: 1")
                && output_lower.contains("number: 2")
                && output_lower.contains("number: 3")
        }
        ("loops", 1) => {
            compact.contains("letmutcount=1;")
                && compact.contains("whilecount<=3{")
                && compact.contains("count+=1;")
                && ran_ok
                && output_lower.contains("count: 1")
                && output_lower.contains("count: 3")
        }
        ("loops", 2) => {
            (compact.contains("for") || compact.contains("while"))
                && ran_ok
                && output_lower.contains("number: 4")
                && output_lower.contains("number: 6")
        }
        ("strings", 0) => compact.contains("letmutmessage=String::from(\"Hello\");"),
        ("strings", 1) => compact.contains("message.push_str(\",Oxide!\");"),
        ("strings", 2) => ran_ok && output_lower.contains("hello, oxide!"),
        ("vectors", 0) => compact.contains("letscores=vec![10,20,30];") || compact.contains("letmutscores=vec![10,20,30];"),
        ("vectors", 1) => compact.contains("scores[0]") && ran_ok && output_compact_lower.contains("first:10"),
        ("vectors", 2) => {
            compact.contains(".push(40)")
                && ran_ok
                && output_compact_lower.contains("last:40")
        }
        ("structs", 0) => {
            compact.contains("structPlayer{")
                && compact.contains("name:String,")
                && compact.contains("score:i32,")
        }
        ("structs", 1) => {
            compact.contains("letplayer=Player{")
                && compact.contains("name:String::from(\"Quinn\")")
                && compact.contains("score:10")
        }
        ("structs", 2) => {
            compact.contains(".name")
                && compact.contains(".score")
                && ran_ok
                && output_compact_lower.contains("quinn:10")
        }
        ("enums-match", 0) => {
            compact.contains("enumDirection{") && compact.contains("Left,") && compact.contains("Right,")
        }
        ("enums-match", 1) => compact.contains("letdirection=Direction::Left;"),
        ("enums-match", 2) => {
            compact.contains("matchdirection{")
                && compact.contains("Direction::Left=>")
                && compact.contains("Direction::Right=>")
                && ran_ok
                && output_lower.contains("going left")
        }
        ("enums-match", 3) => compact.contains("match") && ran_ok && output_lower.contains("going right"),
        ("ownership", 0) => ran_ok && output_lower.contains("oxide"),
        ("ownership", 1) => {
            compact.contains("letmoved_name=name;")
                && compact.contains("println!(\"Original:{name}\");")
                && (has_code("E0382") || has_message("borrow of moved value") || has_message("moved value"))
        }
        ("ownership", 2) => {
            compact.contains("letmoved_name=name.clone();")
                && compact.contains("println!(\"Original:{name}\");")
                && !has_level("error")
                && ran_ok
                && output_lower.contains("oxide")
        }
        ("borrowing", 0) => compact.contains("show_name(&name);") && ran_ok && output_lower.contains("borrowed: oxide"),
        ("borrowing", 1) => {
            compact.contains("println!(\"Stillmine:{name}\");")
                && ran_ok
                && output_lower.contains("borrowed: oxide")
                && output_lower.contains("still mine: oxide")
        }
        ("borrowing", 2) => {
            compact.contains("&String")
                && compact.matches("fn").count() >= 2
                && ran_ok
                && output_lower.contains("oxide oxide")
        }
        ("string-slices", 0) => {
            compact.contains("fnshow_label(label:&str){")
                && compact.contains("show_label(&label);")
                && ran_ok
                && output_lower.contains("label: oxide")
        }
        ("string-slices", 1) => compact.contains("show_label(\"Rust\");") && ran_ok && output_lower.contains("label: rust"),
        ("string-slices", 2) => {
            compact.contains("&str")
                && compact.matches("fn").count() >= 2
                && ran_ok
                && output_lower.contains("go!")
        }
        ("mutable-references", 0) => {
            compact.contains("fnadd_point(score:&muti32){")
                && compact.contains("*score+=1;")
                && compact.contains("add_point(&mutscore);")
                && ran_ok
                && output_compact_lower.contains("score:11")
        }
        ("mutable-references", 1) => {
            !compact.contains("letmutscore=10;")
                && compact.contains("add_point(&mutscore);")
                && (has_code("E0596") || has_message("cannot borrow") || has_message("not declared as mutable"))
        }
        ("mutable-references", 2) => {
            compact.contains("&mut")
                && ran_ok
                && output_compact_lower.contains("score:12")
        }
        ("methods", 0) => {
            compact.contains("implCounter{")
                && compact.contains("fnshow(&self){")
                && compact.contains("counter.show();")
                && ran_ok
                && output_compact_lower.contains("count:3")
        }
        ("methods", 1) => {
            compact.contains("fnadd_one(&mutself){")
                && compact.contains("self.value+=1;")
                && compact.contains("letmutcounter=Counter{")
                && compact.contains("counter.add_one();")
        }
        ("methods", 2) => {
            compact.contains("implCounter{")
                && compact.contains("&mutself")
                && ran_ok
                && output_compact_lower.contains("count:5")
        }
        ("option", 0) => ran_ok && output_compact_lower.contains("score:10"),
        ("option", 1) => {
            compact.contains("letscore:Option<i32>=None;")
                && ran_ok
                && output_lower.contains("no score")
        }
        ("option", 2) => {
            compact.contains("Some(3)")
                && compact.contains("match")
                && ran_ok
                && output_compact_lower.contains("lives:3")
        }
        ("result", 0) => ran_ok && output_compact_lower.contains("number:42"),
        ("result", 1) => compact.contains("\"notanumber\".parse::<i32>()") && ran_ok && output_lower.contains("could not parse"),
        ("result", 2) => compact.contains("parse::<i32>()") && compact.contains("100") && ran_ok && output_compact_lower.contains("number:100"),
        ("hashmaps", 0) => {
            compact.contains("HashMap::new();")
                && ran_ok
                && output_compact_lower.contains("entries:0")
        }
        ("hashmaps", 1) => {
            compact.contains("letmutscores:HashMap<&str,i32>=HashMap::new();")
                && compact.contains("scores.insert(\"Quinn\",10);")
                && ran_ok
                && output_compact_lower.contains("entries:1")
        }
        ("hashmaps", 2) => {
            compact.contains("scores.get(\"Quinn\")")
                && compact.contains("Some(score)=>")
                && ran_ok
                && output_compact_lower.contains("quinn:10")
        }
        ("modules", 0) => ran_ok && output_lower.contains("hello from module!"),
        ("modules", 1) => {
            compact.contains("pubfngoodbye(){")
                && compact.contains("println!(\"Goodbyefrommodule!\");")
        }
        ("modules", 2) => compact.contains("::") && ran_ok && output_lower.contains("goodbye from module!"),
        ("user-input", 0) => {
            compact.contains("letmutname=String::new();")
                && compact.contains("std::io::stdin().read_line(&mutname).expect(")
        }
        ("user-input", 1) => {
            compact.contains("letname=name.trim();")
                && (compact.contains("println!(\"Hello,{name}!\");") || compact.contains("println!(\"Hello,{}!\",name);"))
        }
        ("user-input", 2) => ran_ok && output_lower.contains("hello, quinn!"),
        ("mini-calculator", 0) => {
            compact.contains("letsecond=read_number(\"Secondnumber:\");")
                && (compact.contains("println!(\"Second:{second}\");") || compact.contains("println!(\"Second:{}\",second);"))
        },
        ("mini-calculator", 1) => {
            compact.contains("lettotal=first+second;")
                && (compact.contains("println!(\"Total:{total}\");") || compact.contains("println!(\"Total:{}\",total);"))
        }
        ("mini-calculator", 2) => ran_ok && output_compact_lower.contains("total:10"),
        ("mini-calculator", 3) => ran_ok && output_compact_lower.contains("total:10") && output_compact_lower.contains("difference:-2"),
        ("mini-scoreboard", 0) => {
            compact.contains("letmutplayers=vec![")
                && compact.contains("players.push(Player{")
                && compact.contains("name:String::from(\"Oxide\")")
                && compact.contains("score:20")
        }
        ("mini-scoreboard", 1) => {
            compact.contains("forplayerin&players{")
                && compact.contains("player.name")
                && compact.contains("player.score")
        }
        ("mini-scoreboard", 2) => {
            ran_ok
                && output_compact_lower.contains("quinn:10")
                && output_compact_lower.contains("oxide:20")
        }
        ("mini-scoreboard", 3) => {
            ran_ok && output_lower.contains("winner: oxide")
        }
        _ => false
    };

    TutorialEvaluationResult {
        complete,
        feedback: if complete {
            "Objective complete.".into()
        } else if request.source.trim().is_empty() {
            "The editor is empty. Try the current activity when you're ready.".into()
        } else {
            "Keep experimenting. Oxide will recognize the objective when the code reaches it.".into()
        },
    }
}


fn current_oxide_build_number() -> u64 {
    option_env!("OXIDE_BUILD_NUMBER")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(1)
}

fn updater_display_version(update: &tauri_plugin_updater::Update) -> String {
    update
        .raw_json
        .get("display_version")
        .or_else(|| update.raw_json.get("displayVersion"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| format!("B{}", updater_release_version(update)))
}

fn updater_release_version(update: &tauri_plugin_updater::Update) -> String {
    update
        .raw_json
        .get("release_version")
        .or_else(|| update.raw_json.get("releaseVersion"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| update.version.to_string())
}

fn updater_build_number(update: &tauri_plugin_updater::Update) -> u64 {
    update
        .raw_json
        .get("build")
        .or_else(|| update.raw_json.get("build_number"))
        .or_else(|| update.raw_json.get("buildNumber"))
        .and_then(|value| value.as_u64())
        .unwrap_or(1)
}

fn oxide_release_is_newer(
    current_version: &str,
    current_build: u64,
    remote_version: &str,
    remote_build: u64,
) -> Result<bool, String> {
    let current = semver::Version::parse(current_version.trim_start_matches('v'))
        .map_err(|error| format!("Oxide's installed version '{current_version}' is invalid: {error}"))?;
    let remote = semver::Version::parse(remote_version.trim_start_matches('v'))
        .map_err(|error| format!("The Oxide release feed has an invalid release_version '{remote_version}': {error}"))?;

    // Oxide releases are ordered by the pair (release version, build number).
    // A higher public release always wins. For the same release, a higher
    // build number is a real update even though Cargo/Tauri SemVer is equal.
    Ok(remote > current || (remote == current && remote_build > current_build))
}

fn remote_oxide_is_newer(update: &tauri_plugin_updater::Update) -> Result<bool, String> {
    let remote_version = updater_release_version(update);
    oxide_release_is_newer(
        env!("CARGO_PKG_VERSION"),
        current_oxide_build_number(),
        &remote_version,
        updater_build_number(update),
    )
}

async fn raw_oxide_update(app: &AppHandle) -> Result<Option<tauri_plugin_updater::Update>, String> {
    // Tauri's default comparator only understands SemVer and would normally
    // reject an equal 1.3.5 before Oxide could notice Build 1 -> Build 2.
    // Always retrieve the signed feed, then compare (release_version, build)
    // ourselves. Explicit no-cache headers also prevent a same-release feed
    // from being reused after a newer Oxide build is published.
    let updater = app
        .updater_builder()
        .version_comparator(|_, _| true)
        .header("Cache-Control", "no-cache")
        .map_err(|error| format!("Could not configure updater cache control: {error}"))?
        .header("Pragma", "no-cache")
        .map_err(|error| format!("Could not configure updater cache control: {error}"))?
        .build()
        .map_err(|error| format!("Could not initialize the Oxide update service: {error}"))?;

    updater
        .check()
        .await
        .map_err(|error| format!("Could not check the Oxide release feed: {error}"))
}

async fn available_oxide_update(app: &AppHandle) -> Result<Option<tauri_plugin_updater::Update>, String> {
    let Some(update) = raw_oxide_update(app).await? else {
        return Ok(None);
    };

    if remote_oxide_is_newer(&update)? {
        Ok(Some(update))
    } else {
        Ok(None)
    }
}

#[tauri::command]
async fn oxide_update_check(app: AppHandle) -> Result<Option<OxideUpdateInfo>, String> {
    let update = available_oxide_update(&app).await?;

    Ok(update.map(|update| {
        #[cfg(target_os = "windows")]
        let (install_supported, install_hint, update_mode) = (true, None, "native-package".to_string());

        #[cfg(target_os = "linux")]
        let (install_supported, update_mode, install_hint) = linux_update_capability();

        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        let (install_supported, install_hint, update_mode) = (false, Some("Automatic package updates are not implemented for this platform yet.".to_string()), "unsupported".to_string());

        OxideUpdateInfo {
            version: updater_release_version(&update),
            display_version: updater_display_version(&update),
            build_number: updater_build_number(&update),
            current_version: env!("CARGO_PKG_VERSION").to_string(),
            current_build_number: current_oxide_build_number(),
            body: update.body.clone(),
            date: update.date.map(|date| date.to_string()),
            install_supported,
            install_hint,
            update_mode,
        }
    }))
}

#[cfg(test)]
mod updater_version_tests {
    use super::oxide_release_is_newer;

    #[test]
    fn newer_build_of_same_release_is_an_update() {
        assert!(oxide_release_is_newer("1.3.5", 1, "1.3.5", 2).unwrap());
    }

    #[test]
    fn same_or_older_build_is_not_an_update() {
        assert!(!oxide_release_is_newer("1.3.5", 2, "1.3.5", 2).unwrap());
        assert!(!oxide_release_is_newer("1.3.5", 2, "1.3.5", 1).unwrap());
    }

    #[test]
    fn public_release_version_still_takes_priority() {
        assert!(oxide_release_is_newer("1.3.5", 99, "1.3.6", 1).unwrap());
        assert!(!oxide_release_is_newer("1.3.6", 1, "1.3.5", 999).unwrap());
    }
}

fn installed_updater_helper(install_dir: &Path) -> Result<PathBuf, String> {
    let direct = install_dir.join(if cfg!(windows) { "oxide-updater.exe" } else { "oxide-updater" });
    if direct.is_file() {
        return Ok(direct);
    }

    let mut candidates = fs::read_dir(install_dir)
        .map_err(|error| format!("Could not inspect the Oxide install directory: {error}"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with("oxide-updater") && path.is_file())
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    candidates.sort();
    candidates
        .into_iter()
        .next()
        .ok_or_else(|| "Oxide Update Service was not found beside the installed editor. Install B1.3.2 once with the normal installer before package updates can take over.".to_string())
}

#[tauri::command]
async fn oxide_update_prepare(app: AppHandle, version: String, build_number: u64) -> Result<OxideUpdateStageResult, String> {
    let update = available_oxide_update(&app)
        .await?
        .ok_or_else(|| "The selected update is no longer available.".to_string())?;

    let available_version = updater_release_version(&update);
    let available_build = updater_build_number(&update);
    if available_version != version || available_build != build_number {
        return Err(format!(
            "The available update changed from {version} Build {build_number} to {available_version} Build {available_build}. Check for updates again before installing."
        ));
    }

    let progress_app = app.clone();
    let mut downloaded = 0usize;
    let bytes = update
        .download(
            move |chunk_length, content_length| {
                downloaded = downloaded.saturating_add(chunk_length);
                let _ = progress_app.emit(
                    "oxide-update-download",
                    OxideUpdateDownloadEvent {
                        event: "progress".into(),
                        downloaded,
                        content_length,
                    },
                );
            },
            {
                let finished_app = app.clone();
                move || {
                    let _ = finished_app.emit(
                        "oxide-update-download",
                        OxideUpdateDownloadEvent {
                            event: "finished".into(),
                            downloaded: 0,
                            content_length: None,
                        },
                    );
                }
            },
        )
        .await
        .map_err(|error| format!("The update package could not be downloaded or its signature was rejected: {error}"))?;

    let current_exe = env::current_exe()
        .map_err(|error| format!("Could not locate the running Oxide executable: {error}"))?;
    let install_dir = current_exe
        .parent()
        .ok_or_else(|| "Could not determine the Oxide install directory.".to_string())?
        .to_path_buf();
    let helper = installed_updater_helper(&install_dir)?;

    let work_dir = env::temp_dir()
        .join("OxideEditor")
        .join(format!("update-{}-b{}-{}", version, build_number, std::process::id()));
    if work_dir.exists() {
        fs::remove_dir_all(&work_dir)
            .map_err(|error| format!("Could not clear the previous update staging directory: {error}"))?;
    }
    fs::create_dir_all(&work_dir)
        .map_err(|error| format!("Could not create the update staging directory: {error}"))?;

    let package_path = work_dir.join("oxide-update.zip");
    fs::write(&package_path, bytes)
        .map_err(|error| format!("Could not stage the verified update package: {error}"))?;

    let helper_name = if cfg!(windows) { "oxide-updater.exe" } else { "oxide-updater" };
    let temp_helper = work_dir.join(helper_name);
    fs::copy(&helper, &temp_helper)
        .map_err(|error| format!("Could not prepare the Oxide Update Service: {error}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&temp_helper)
            .map_err(|error| format!("Could not inspect the Oxide Update Service: {error}"))?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&temp_helper, permissions)
            .map_err(|error| format!("Could not make the Oxide Update Service executable: {error}"))?;
    }

    #[cfg(target_os = "windows")]
    {
        let app_exe_name = current_exe
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| "Could not determine the Oxide executable name.".to_string())?
            .to_string();

        Command::new(&temp_helper)
            .arg("--package")
            .arg(&package_path)
            .arg("--install-dir")
            .arg(&install_dir)
            .arg("--app-exe")
            .arg(&app_exe_name)
            .arg("--version")
            .arg(&version)
            .arg("--build")
            .arg(build_number.to_string())
            .spawn()
            .map_err(|error| format!("Could not launch the Oxide Update Service: {error}"))?;
    }

    #[cfg(target_os = "linux")]
    {
        if linux_is_appimage() {
            let appimage = env::var_os("APPIMAGE")
                .map(PathBuf::from)
                .filter(|path| path.is_file())
                .ok_or_else(|| "The running Oxide AppImage could not be located.".to_string())?;

            Command::new(&temp_helper)
                .arg("--mode")
                .arg("appimage")
                .arg("--package")
                .arg(&package_path)
                .arg("--appimage")
                .arg(&appimage)
                .arg("--version")
                .arg(&version)
                .arg("--build")
                .arg(build_number.to_string())
                .arg("--pid")
                .arg(std::process::id().to_string())
                .spawn()
                .map_err(|error| format!("Could not launch the Linux Oxide Update Service: {error}"))?;
        } else if linux_is_deb_install() {
            if !linux_deb_update_tools_available() {
                return Err("This .deb installation needs polkit/pkexec and dpkg for automatic updates.".into());
            }

            Command::new(&temp_helper)
                .arg("--mode")
                .arg("deb")
                .arg("--package")
                .arg(&package_path)
                .arg("--app-exe")
                .arg(&current_exe)
                .arg("--version")
                .arg(&version)
                .arg("--build")
                .arg(build_number.to_string())
                .arg("--pid")
                .arg(std::process::id().to_string())
                .spawn()
                .map_err(|error| format!("Could not launch the Linux Oxide Update Service: {error}"))?;
        } else {
            return Err("Automatic Linux installation is available for Oxide AppImage and .deb release builds. This appears to be an unpackaged/development build.".into());
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    return Err("Oxide package updates are not implemented for this operating system yet.".into());

    Ok(OxideUpdateStageResult {
        version,
        build_number,
        helper_started: true,
    })
}

#[tauri::command]
fn debugger_status() -> debugger::DebuggerStatus {
    debugger::status()
}

#[tauri::command]
async fn debugger_targets(project_path: String) -> Result<Vec<debugger::DebugTarget>, String> {
    tauri::async_runtime::spawn_blocking(move || debugger::debug_targets(project_path))
        .await
        .map_err(|error| format!("Debugger target discovery task could not be joined: {error}"))?
}

#[tauri::command]
async fn debugger_start(
    app: AppHandle,
    runtime: State<'_, debugger::DebuggerRuntime>,
    project_path: String,
    breakpoints: Vec<debugger::DebugBreakpointSet>,
    target: Option<debugger::DebugTarget>,
) -> Result<debugger::DebugStartResult, String> {
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || debugger::start(app, &runtime, project_path, breakpoints, target))
        .await
        .map_err(|error| format!("Debugger start task could not be joined: {error}"))?
}

#[tauri::command]
fn debugger_set_breakpoints(
    runtime: State<'_, debugger::DebuggerRuntime>,
    breakpoint_set: debugger::DebugBreakpointSet,
) -> Result<(), String> {
    debugger::set_breakpoints(runtime.inner(), breakpoint_set)
}

#[tauri::command]
fn debugger_continue(runtime: State<'_, debugger::DebuggerRuntime>, thread_id: Option<i64>) -> Result<(), String> {
    debugger::continue_execution(runtime.inner(), thread_id)
}

#[tauri::command]
fn debugger_pause(runtime: State<'_, debugger::DebuggerRuntime>, thread_id: Option<i64>) -> Result<(), String> {
    debugger::pause(runtime.inner(), thread_id)
}

#[tauri::command]
fn debugger_next(runtime: State<'_, debugger::DebuggerRuntime>, thread_id: Option<i64>) -> Result<(), String> {
    debugger::next(runtime.inner(), thread_id)
}

#[tauri::command]
fn debugger_step_in(runtime: State<'_, debugger::DebuggerRuntime>, thread_id: Option<i64>) -> Result<(), String> {
    debugger::step_in(runtime.inner(), thread_id)
}

#[tauri::command]
fn debugger_step_out(runtime: State<'_, debugger::DebuggerRuntime>, thread_id: Option<i64>) -> Result<(), String> {
    debugger::step_out(runtime.inner(), thread_id)
}

#[tauri::command]
fn debugger_threads(runtime: State<'_, debugger::DebuggerRuntime>) -> Result<Vec<debugger::DebugThread>, String> {
    debugger::threads(runtime.inner())
}

#[tauri::command]
fn debugger_restart(runtime: State<'_, debugger::DebuggerRuntime>) -> Result<(), String> {
    debugger::restart(runtime.inner())
}

#[tauri::command]
fn debugger_stack_trace(runtime: State<'_, debugger::DebuggerRuntime>, thread_id: i64) -> Result<Vec<debugger::DebugStackFrame>, String> {
    debugger::stack_trace(runtime.inner(), thread_id)
}

#[tauri::command]
fn debugger_scopes(runtime: State<'_, debugger::DebuggerRuntime>, frame_id: i64) -> Result<Vec<debugger::DebugScope>, String> {
    debugger::scopes(runtime.inner(), frame_id)
}

#[tauri::command]
fn debugger_variables(runtime: State<'_, debugger::DebuggerRuntime>, variables_reference: i64) -> Result<Vec<debugger::DebugVariable>, String> {
    debugger::variables(runtime.inner(), variables_reference)
}

#[tauri::command]
fn debugger_evaluate(runtime: State<'_, debugger::DebuggerRuntime>, expression: String, frame_id: Option<i64>) -> Result<debugger::DebugEvaluateResult, String> {
    debugger::evaluate(runtime.inner(), expression, frame_id)
}

#[tauri::command]
fn debugger_repl(runtime: State<'_, debugger::DebuggerRuntime>, expression: String, frame_id: Option<i64>) -> Result<debugger::DebugEvaluateResult, String> {
    debugger::repl(runtime.inner(), expression, frame_id)
}

#[tauri::command]
fn debugger_stop(runtime: State<'_, debugger::DebuggerRuntime>) -> Result<(), String> {
    debugger::stop(runtime.inner())
}

#[tauri::command]
fn rust_analyzer_status() -> analyzer::AnalyzerStatus {
    analyzer::status()
}

#[tauri::command]
async fn rust_analyzer_warmup(
    runtime: State<'_, analyzer::RustAnalyzerRuntime>,
    project_path: String,
) -> Result<(), String> {
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || analyzer::warmup(&runtime, project_path))
        .await
        .map_err(|error| format!("Rust Code Analyzer/Completer warmup could not be joined: {error}"))?
}

#[tauri::command]
async fn rust_completions(
    runtime: State<'_, analyzer::RustAnalyzerRuntime>,
    project_path: String,
    path: String,
    content: String,
    line: u32,
    character: u32,
) -> Result<Vec<analyzer::CompletionItemView>, String> {
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        analyzer::completions(&runtime, project_path, path, content, line, character)
    })
    .await
    .map_err(|error| format!("Rust Code Analyzer/Completer request could not be joined: {error}"))?
}

#[tauri::command]
async fn rust_semantic_tokens(
    runtime: State<'_, analyzer::RustAnalyzerRuntime>,
    project_path: String,
    path: String,
    content: String,
) -> Result<Vec<analyzer::SemanticTokenView>, String> {
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || analyzer::semantic_tokens(&runtime, project_path, path, content))
        .await
        .map_err(|error| format!("Semantic Readability Colors request could not be joined: {error}"))?
}

#[tauri::command]
async fn rust_signature_help(
    runtime: State<'_, analyzer::RustAnalyzerRuntime>,
    project_path: String,
    path: String,
    content: String,
    line: u32,
    character: u32,
) -> Result<Option<analyzer::SignatureHelpView>, String> {
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        analyzer::signature_help(&runtime, project_path, path, content, line, character)
    })
    .await
    .map_err(|error| format!("Rust signature-help request could not be joined: {error}"))?
}

#[tauri::command]
async fn rust_definition(
    runtime: State<'_, analyzer::RustAnalyzerRuntime>,
    project_path: String,
    path: String,
    content: String,
    line: u32,
    character: u32,
) -> Result<Vec<analyzer::LocationView>, String> {
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || analyzer::definition(&runtime, project_path, path, content, line, character))
        .await
        .map_err(|error| format!("Go to Definition request could not be joined: {error}"))?
}

#[tauri::command]
async fn rust_references(
    runtime: State<'_, analyzer::RustAnalyzerRuntime>,
    project_path: String,
    path: String,
    content: String,
    line: u32,
    character: u32,
) -> Result<Vec<analyzer::LocationView>, String> {
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || analyzer::references(&runtime, project_path, path, content, line, character))
        .await
        .map_err(|error| format!("Find References request could not be joined: {error}"))?
}

#[tauri::command]
async fn rust_prepare_rename(
    runtime: State<'_, analyzer::RustAnalyzerRuntime>,
    project_path: String,
    path: String,
    content: String,
    line: u32,
    character: u32,
) -> Result<Option<analyzer::PrepareRenameView>, String> {
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || analyzer::prepare_rename(&runtime, project_path, path, content, line, character))
        .await
        .map_err(|error| format!("Rename preparation request could not be joined: {error}"))?
}

#[tauri::command]
async fn rust_rename(
    runtime: State<'_, analyzer::RustAnalyzerRuntime>,
    project_path: String,
    path: String,
    content: String,
    line: u32,
    character: u32,
    new_name: String,
) -> Result<analyzer::WorkspaceEditView, String> {
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || analyzer::rename(&runtime, project_path, path, content, line, character, new_name))
        .await
        .map_err(|error| format!("Semantic Rename request could not be joined: {error}"))?
}

#[tauri::command]
async fn rust_code_actions(
    runtime: State<'_, analyzer::RustAnalyzerRuntime>,
    project_path: String,
    path: String,
    content: String,
    line: u32,
    character: u32,
) -> Result<Vec<analyzer::CodeActionView>, String> {
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || analyzer::code_actions(&runtime, project_path, path, content, line, character))
        .await
        .map_err(|error| format!("Code Actions request could not be joined: {error}"))?
}

#[tauri::command]
async fn rust_analyzer_stop(runtime: State<'_, analyzer::RustAnalyzerRuntime>) -> Result<(), String> {
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || runtime.stop())
        .await
        .map_err(|error| format!("Rust Code Analyzer/Completer shutdown could not be joined: {error}"))?;
    Ok(())
}

#[tauri::command]
fn quit_app(app: AppHandle, debugger_runtime: State<'_, debugger::DebuggerRuntime>) {
    let _ = debugger::stop(debugger_runtime.inner());
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|_| {
            thread::spawn(|| {
                thread::sleep(Duration::from_secs(5));
                let update_root = env::temp_dir().join("OxideEditor");
                if let Ok(entries) = fs::read_dir(&update_root) {
                    for entry in entries.filter_map(Result::ok) {
                        let path = entry.path();
                        if path.is_dir() && entry.file_name().to_string_lossy().starts_with("update-") {
                            let old_enough = fs::metadata(&path)
                                .and_then(|meta| meta.modified())
                                .ok()
                                .and_then(|modified| SystemTime::now().duration_since(modified).ok())
                                .map(|age| age > Duration::from_secs(600))
                                .unwrap_or(false);
                            if old_enough {
                                let _ = fs::remove_dir_all(path);
                            }
                        }
                    }
                }
            });
            Ok(())
        })
        .manage(TerminalRuntime::default())
        .manage(analyzer::RustAnalyzerRuntime::default())
        .manage(debugger::DebuggerRuntime::default())
        .invoke_handler(tauri::generate_handler![
            platform_info,
            toolchain_info,
            list_project_files,
            read_text_file,
            write_text_file,
            default_browse_path,
            filesystem_roots,
            browse_directory,
            create_directory,
            create_project,
            save_project_as,
            manifest_view,
            add_dependency,
            remove_dependency,
            cargo_action,
            cargo_diagnostics,
            terminal_start,
            terminal_write,
            terminal_stop,
            tutorial_catalog,
            tutorial_progress,
            tutorial_set_progress,
            tutorial_prepare_lesson,
            tutorial_evaluate,
            rust_analyzer_status,
            debugger_status,
            debugger_targets,
            debugger_start,
            debugger_set_breakpoints,
            debugger_continue,
            debugger_pause,
            debugger_next,
            debugger_step_in,
            debugger_step_out,
            debugger_threads,
            debugger_restart,
            debugger_stack_trace,
            debugger_scopes,
            debugger_variables,
            debugger_evaluate,
            debugger_repl,
            debugger_stop,
            rust_analyzer_warmup,
            rust_semantic_tokens,
            rust_completions,
            rust_signature_help,
            rust_definition,
            rust_references,
            rust_prepare_rename,
            rust_rename,
            rust_code_actions,
            rust_analyzer_stop,
            oxide_update_check,
            oxide_update_prepare,
            quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Oxide Editor");
}
