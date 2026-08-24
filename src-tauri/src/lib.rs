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
    time::Duration,
};
use tauri::{AppHandle, Emitter, State};
use toml_edit::{Array, DocumentMut, InlineTable, Item, Value};

#[derive(Serialize)]
struct ToolchainInfo {
    cargo_found: bool,
    rustc_found: bool,
    cargo: String,
    rustc: String,
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

fn command_version(program: &str) -> Option<String> {
    Command::new(program)
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
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

    roots.dedup_by(|a, b| a.path.eq_ignore_ascii_case(&b.path));
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

    let mut command = Command::new("cargo");
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
    let mut command = Command::new("cargo");
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

        let mut build = Command::new("cargo");
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
                    "Create a function named cheer that prints You can do this!, call it from main, and Run the program.",
                    "This challenge asks you to transfer the same structure to a new function instead of copying a new worked answer. That gradual reduction in guidance is how later Oxide lessons will build independence.",
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
    let ran_ok = request.run_success == Some(true);
    let has_code = |code: &str| request.diagnostic_codes.iter().any(|item| item.eq_ignore_ascii_case(code));
    let has_message = |needle: &str| request.diagnostic_messages.iter().any(|item| item.to_lowercase().contains(&needle.to_lowercase()));
    let has_level = |level: &str| request.diagnostic_levels.iter().any(|item| item.eq_ignore_ascii_case(level));

    let complete = match (request.lesson_id.as_str(), request.step_index) {
        ("hello-world", 0) => ran_ok && output.contains("Hello, world!"),
        ("hello-world", 1) => compact.contains("println!(\"Hello,Oxide!\");") && ran_ok && output.contains("Hello, Oxide!"),
        ("variables", 0) => compact.contains("letname") && compact.contains("=\"Quinn\";"),
        ("variables", 1) => {
            compact.contains("println!(\"{name}\");")
                || compact.contains("println!(\"{}\",name);")
                || (compact.contains("println!(") && compact.contains(",name);"))
        }
        ("variables", 2) => ran_ok && output.contains("Quinn"),
        ("variables", 3) => {
            compact.contains("letscore")
                && compact.contains("=10;")
                && (compact.contains("{score}") || compact.contains(",score)"))
                && ran_ok
                && output.contains("10")
        }
        ("warnings-errors", 0) => {
            has_level("warning") && has_message("unused variable") && ran_ok && output.contains("Program still runs.")
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
                && output.contains("Hello")
        }
        ("mutability", 0) => ran_ok && output.contains("Starting score: 10") && output.contains("Score: 11"),
        ("mutability", 1) => {
            !(compact.contains("letmutscore") && compact.contains("=10;"))
                && (has_code("E0384") || has_message("cannot assign twice to immutable variable") || has_message("immutable"))
        }
        ("mutability", 2) => compact.contains("letmutscore") && compact.contains("=10;") && ran_ok && output.contains("Score: 11"),
        ("basic-types", 0) => compact.contains("letlives=3;"),
        ("basic-types", 1) => compact.contains("letready=true;"),
        ("basic-types", 2) => {
            ran_ok && output.contains("Lives: 3") && output.to_lowercase().contains("ready: true")
        }
        ("functions", 0) => {
            compact.contains("fngreet(){") && compact.contains("println!(\"Hello!\");")
        }
        ("functions", 1) => compact.contains("greet();") && ran_ok && output.contains("Hello!"),
        ("functions", 2) => {
            compact.contains("fncheer(){")
                && compact.contains("cheer();")
                && ran_ok
                && output.contains("You can do this!")
        }
        _ => false,
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


#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(TerminalRuntime::default())
        .invoke_handler(tauri::generate_handler![
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
            quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Oxide Editor");
}
