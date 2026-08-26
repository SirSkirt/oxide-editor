use serde::Serialize;
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    env,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{mpsc, Arc, Mutex},
    thread,
    time::Duration,
};
use url::Url;

#[derive(Default)]
struct AnalyzerShared {
    session: Mutex<Option<AnalyzerSession>>,
}

#[derive(Clone, Default)]
pub struct RustAnalyzerRuntime {
    inner: Arc<AnalyzerShared>,
}

struct AnalyzerSession {
    project_path: PathBuf,
    child: Child,
    stdin: Arc<Mutex<ChildStdin>>,
    pending: Arc<Mutex<HashMap<u64, mpsc::Sender<Value>>>>,
    next_id: u64,
    open_documents: HashSet<PathBuf>,
    document_versions: HashMap<PathBuf, i32>,
    document_contents: HashMap<PathBuf, String>,
    semantic_token_types: Vec<String>,
    semantic_token_modifiers: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzerStatus {
    pub available: bool,
    pub version: String,
    pub message: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LspPositionView {
    pub line: u32,
    pub character: u32,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LspRangeView {
    pub start: LspPositionView,
    pub end: LspPositionView,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionTextEditView {
    pub range: LspRangeView,
    pub new_text: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionItemView {
    pub label: String,
    pub kind: String,
    pub detail: String,
    pub documentation: String,
    pub insert_text: String,
    pub filter_text: String,
    pub sort_text: String,
    pub text_edit: Option<CompletionTextEditView>,
    pub additional_text_edits: Vec<CompletionTextEditView>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignatureHelpView {
    pub label: String,
    pub documentation: String,
    pub active_parameter: usize,
    pub parameters: Vec<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticTokenView {
    pub line: u32,
    pub start_character: u32,
    pub length: u32,
    pub token_type: String,
    pub modifiers: Vec<String>,
}

fn executable_name(program: &str) -> String {
    if cfg!(windows) && !program.to_ascii_lowercase().ends_with(".exe") {
        format!("{program}.exe")
    } else {
        program.to_string()
    }
}

fn resolve_program(program: &str) -> PathBuf {
    let executable_name = executable_name(program);

    if let Some(path_value) = env::var_os("PATH") {
        for directory in env::split_paths(&path_value) {
            let candidate = directory.join(&executable_name);
            if candidate.is_file() {
                return candidate;
            }
        }
    }

    if let Some(home) = env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }) {
        let candidate = PathBuf::from(home).join(".cargo").join("bin").join(&executable_name);
        if candidate.is_file() {
            return candidate;
        }
    }

    PathBuf::from(program)
}

fn analyzer_command() -> Command {
    Command::new(resolve_program("rust-analyzer"))
}

pub fn status() -> AnalyzerStatus {
    match analyzer_command().arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            AnalyzerStatus {
                available: true,
                message: "Rust Code Analyzer/Completer ready.".into(),
                version,
            }
        }
        _ => AnalyzerStatus {
            available: false,
            version: "rust-analyzer: not found".into(),
            message: "Install rust-analyzer with `rustup component add rust-analyzer` to enable Rust Code Analyzer/Completer.".into(),
        },
    }
}

fn write_message(stdin: &Arc<Mutex<ChildStdin>>, value: &Value) -> Result<(), String> {
    let body = serde_json::to_vec(value).map_err(|error| format!("Could not encode rust-analyzer request: {error}"))?;
    let mut input = stdin.lock().map_err(|_| "rust-analyzer input lock was poisoned.".to_string())?;
    write!(input, "Content-Length: {}\r\n\r\n", body.len())
        .and_then(|_| input.write_all(&body))
        .and_then(|_| input.flush())
        .map_err(|error| format!("Could not write to rust-analyzer: {error}"))
}

fn reader_loop<R: Read + Send + 'static>(
    reader: R,
    pending: Arc<Mutex<HashMap<u64, mpsc::Sender<Value>>>>,
    stdin: Arc<Mutex<ChildStdin>>,
) {
    thread::spawn(move || {
        let mut reader = BufReader::new(reader);
        loop {
            let mut content_length = None;
            loop {
                let mut header = String::new();
                match reader.read_line(&mut header) {
                    Ok(0) | Err(_) => return,
                    Ok(_) => {}
                }
                let trimmed = header.trim_end_matches(&['\r', '\n'][..]);
                if trimmed.is_empty() {
                    break;
                }
                if let Some(value) = trimmed.strip_prefix("Content-Length:") {
                    content_length = value.trim().parse::<usize>().ok();
                }
            }

            let Some(length) = content_length else { continue; };
            let mut body = vec![0u8; length];
            if reader.read_exact(&mut body).is_err() {
                return;
            }
            let Ok(message) = serde_json::from_slice::<Value>(&body) else { continue; };
            let Some(id) = message.get("id").and_then(Value::as_u64) else { continue; };

            // Requests from rust-analyzer also carry an id. Handle those before
            // looking in Oxide's pending-response table so equal numeric ids in
            // opposite JSON-RPC directions can never be confused.
            if let Some(method) = message.get("method").and_then(Value::as_str) {
                let result = match method {
                    "workspace/configuration" => {
                        let count = message
                            .get("params")
                            .and_then(|params| params.get("items"))
                            .and_then(Value::as_array)
                            .map(Vec::len)
                            .unwrap_or(0);
                        Value::Array((0..count).map(|_| Value::Null).collect())
                    }
                    "workspace/workspaceFolders" => Value::Null,
                    "workspace/applyEdit" => json!({ "applied": false }),
                    _ => Value::Null,
                };
                let _ = write_message(&stdin, &json!({ "jsonrpc": "2.0", "id": id, "result": result }));
                continue;
            }

            if let Ok(mut pending) = pending.lock() {
                if let Some(sender) = pending.remove(&id) {
                    let _ = sender.send(message);
                }
            }
        }
    });
}

fn document_end_position(text: &str) -> Value {
    let mut line = 0u32;
    let mut character = 0u32;
    for ch in text.chars() {
        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += ch.len_utf16() as u32;
        }
    }
    json!({ "line": line, "character": character })
}

impl AnalyzerSession {
    fn spawn(project_path: &Path) -> Result<Self, String> {
        let mut child = analyzer_command()
            .current_dir(project_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("Could not start rust-analyzer. Install it with `rustup component add rust-analyzer`. {error}"))?;

        let stdin = Arc::new(Mutex::new(child.stdin.take().ok_or_else(|| "Could not open rust-analyzer stdin.".to_string())?));
        let stdout = child.stdout.take().ok_or_else(|| "Could not open rust-analyzer stdout.".to_string())?;
        let pending = Arc::new(Mutex::new(HashMap::new()));
        reader_loop(stdout, pending.clone(), stdin.clone());

        let mut session = Self {
            project_path: project_path.to_path_buf(),
            child,
            stdin,
            pending,
            next_id: 1,
            open_documents: HashSet::new(),
            document_versions: HashMap::new(),
            document_contents: HashMap::new(),
            semantic_token_types: Vec::new(),
            semantic_token_modifiers: Vec::new(),
        };

        let root_uri = Url::from_directory_path(project_path)
            .map_err(|_| format!("Could not convert project path to an LSP URI: {}", project_path.display()))?
            .to_string();

        let initialize = json!({
            "processId": std::process::id(),
            "rootUri": root_uri.clone(),
            "capabilities": {
                "general": { "positionEncodings": ["utf-16"] },
                "textDocument": {
                    "completion": {
                        "completionItem": {
                            "snippetSupport": false,
                            "documentationFormat": ["markdown", "plaintext"]
                        }
                    },
                    "signatureHelp": {
                        "signatureInformation": {
                            "documentationFormat": ["markdown", "plaintext"],
                            "parameterInformation": { "labelOffsetSupport": true }
                        }
                    },
                    "semanticTokens": {
                        "requests": { "full": true },
                        "tokenTypes": [
                            "namespace", "type", "class", "enum", "interface", "struct", "typeParameter",
                            "parameter", "variable", "property", "enumMember", "event", "function", "method",
                            "macro", "keyword", "modifier", "comment", "string", "number", "regexp", "operator",
                            "decorator", "attribute", "derive", "trait", "typeAlias", "union", "boolean",
                            "character", "escapeSequence", "formatSpecifier", "lifetime", "selfKeyword",
                            "selfTypeKeyword", "punctuation", "unresolvedReference", "builtinAttribute", "builtinType",
                            "constParameter", "deriveHelper", "generic", "label", "toolModule", "constant",
                            "arithmetic", "bitwise", "comparison", "logical", "attributeBracket", "angle", "brace",
                            "bracket", "parenthesis", "colon", "comma", "dot", "semi", "macroBang"
                        ],
                        "tokenModifiers": [
                            "declaration", "definition", "readonly", "static", "deprecated", "abstract", "async",
                            "modification", "documentation", "defaultLibrary", "mutable", "consuming", "controlFlow",
                            "crateRoot", "library", "public", "reference", "trait", "unsafe", "callable", "injected",
                            "intraDocLink", "macro", "attribute"
                        ],
                        "formats": ["relative"],
                        "overlappingTokenSupport": false,
                        "multilineTokenSupport": false
                    }
                },
                "workspace": { "workspaceFolders": true }
            },
            "workspaceFolders": [{ "uri": root_uri, "name": project_path.file_name().and_then(|v| v.to_str()).unwrap_or("Oxide Project") }],
            "clientInfo": { "name": "Oxide Editor", "version": env!("CARGO_PKG_VERSION") }
        });

        let initialize_result = session.request("initialize", initialize, Duration::from_secs(12))?;
        if let Some(legend) = initialize_result
            .get("capabilities")
            .and_then(|capabilities| capabilities.get("semanticTokensProvider"))
            .and_then(|provider| provider.get("legend"))
        {
            session.semantic_token_types = legend
                .get("tokenTypes")
                .and_then(Value::as_array)
                .map(|items| items.iter().filter_map(Value::as_str).map(str::to_string).collect())
                .unwrap_or_default();
            session.semantic_token_modifiers = legend
                .get("tokenModifiers")
                .and_then(Value::as_array)
                .map(|items| items.iter().filter_map(Value::as_str).map(str::to_string).collect())
                .unwrap_or_default();
        }
        session.notify("initialized", json!({}))?;
        Ok(session)
    }

    fn notify(&mut self, method: &str, params: Value) -> Result<(), String> {
        write_message(&self.stdin, &json!({ "jsonrpc": "2.0", "method": method, "params": params }))
    }

    fn request(&mut self, method: &str, params: Value, timeout: Duration) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        let (sender, receiver) = mpsc::channel();
        self.pending
            .lock()
            .map_err(|_| "rust-analyzer request table was poisoned.".to_string())?
            .insert(id, sender);

        if let Err(error) = write_message(&self.stdin, &json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })) {
            if let Ok(mut pending) = self.pending.lock() {
                pending.remove(&id);
            }
            return Err(error);
        }

        let message = receiver
            .recv_timeout(timeout)
            .map_err(|_| format!("rust-analyzer did not answer {method} in time."))?;
        if let Some(error) = message.get("error") {
            return Err(format!("rust-analyzer {method} failed: {error}"));
        }
        Ok(message.get("result").cloned().unwrap_or(Value::Null))
    }

    fn sync_document(&mut self, path: &Path, content: &str) -> Result<String, String> {
        let uri = Url::from_file_path(path)
            .map_err(|_| format!("Could not convert file path to an LSP URI: {}", path.display()))?
            .to_string();
        let version = {
            let entry = self.document_versions.entry(path.to_path_buf()).or_insert(0);
            *entry += 1;
            *entry
        };

        if self.open_documents.insert(path.to_path_buf()) {
            self.notify(
                "textDocument/didOpen",
                json!({
                    "textDocument": {
                        "uri": uri.clone(),
                        "languageId": "rust",
                        "version": version,
                        "text": content
                    }
                }),
            )?;
            self.document_contents.insert(path.to_path_buf(), content.to_string());
        } else {
            let previous = self.document_contents.get(path).cloned().unwrap_or_default();
            self.notify(
                "textDocument/didChange",
                json!({
                    "textDocument": { "uri": uri.clone(), "version": version },
                    "contentChanges": [{
                        "range": {
                            "start": { "line": 0, "character": 0 },
                            "end": document_end_position(&previous)
                        },
                        "text": content
                    }]
                }),
            )?;
            self.document_contents.insert(path.to_path_buf(), content.to_string());
        }
        Ok(uri)
    }

    fn shutdown(&mut self) {
        let _ = self.request("shutdown", Value::Null, Duration::from_secs(1));
        let _ = self.notify("exit", Value::Null);
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for AnalyzerShared {
    fn drop(&mut self) {
        if let Ok(slot) = self.session.get_mut() {
            if let Some(mut session) = slot.take() {
                session.shutdown();
            }
        }
    }
}

impl RustAnalyzerRuntime {
    fn with_session<T>(&self, project_path: &Path, operation: impl FnOnce(&mut AnalyzerSession) -> Result<T, String>) -> Result<T, String> {
        let mut slot = self.inner.session.lock().map_err(|_| "Rust Code Analyzer/Completer state was poisoned.".to_string())?;
        let replace = match slot.as_mut() {
            Some(session) => session.project_path.as_path() != project_path || session.child.try_wait().ok().flatten().is_some(),
            None => true,
        };
        if replace {
            if let Some(mut previous) = slot.take() {
                previous.shutdown();
            }
            *slot = Some(AnalyzerSession::spawn(project_path)?);
        }
        operation(slot.as_mut().expect("analyzer session exists"))
    }

    pub fn stop(&self) {
        if let Ok(mut slot) = self.inner.session.lock() {
            if let Some(mut session) = slot.take() {
                session.shutdown();
            }
        }
    }
}

fn markup_to_string(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Object(map)) => map.get("value").and_then(Value::as_str).unwrap_or_default().to_string(),
        _ => String::new(),
    }
}

fn position_view(value: &Value) -> Option<LspPositionView> {
    Some(LspPositionView {
        line: value.get("line")?.as_u64()? as u32,
        character: value.get("character")?.as_u64()? as u32,
    })
}

fn range_view(value: &Value) -> Option<LspRangeView> {
    Some(LspRangeView {
        start: position_view(value.get("start")?)?,
        end: position_view(value.get("end")?)?,
    })
}

fn text_edit_view(value: &Value) -> Option<CompletionTextEditView> {
    let new_text = value.get("newText")?.as_str()?.to_string();
    let range_value = value.get("range").or_else(|| value.get("replace"))?;
    Some(CompletionTextEditView {
        range: range_view(range_value)?,
        new_text,
    })
}

fn completion_kind(kind: Option<u64>) -> String {
    match kind.unwrap_or(1) {
        2 => "Method",
        3 => "Function",
        4 => "Constructor",
        5 => "Field",
        6 => "Variable",
        7 => "Class",
        8 => "Trait",
        9 => "Module",
        10 => "Property",
        11 => "Unit",
        12 => "Value",
        13 => "Enum",
        14 => "Keyword",
        15 => "Snippet",
        17 => "File",
        18 => "Reference",
        20 => "Enum Member",
        21 => "Constant",
        22 => "Struct",
        23 => "Event",
        25 => "Type Parameter",
        _ => "Symbol",
    }
    .to_string()
}

fn completion_items(result: Value) -> Vec<CompletionItemView> {
    let items = if let Some(array) = result.as_array() {
        array.clone()
    } else {
        result.get("items").and_then(Value::as_array).cloned().unwrap_or_default()
    };

    items
        .into_iter()
        .filter_map(|item| {
            let label = item.get("label")?.as_str()?.to_string();
            let text_edit = item.get("textEdit").and_then(text_edit_view);
            let insert_text = item
                .get("insertText")
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| text_edit.as_ref().map(|edit| edit.new_text.clone()))
                .unwrap_or_else(|| label.clone());
            let additional_text_edits = item
                .get("additionalTextEdits")
                .and_then(Value::as_array)
                .map(|edits| edits.iter().filter_map(text_edit_view).collect())
                .unwrap_or_default();

            Some(CompletionItemView {
                label: label.clone(),
                kind: completion_kind(item.get("kind").and_then(Value::as_u64)),
                detail: item.get("detail").and_then(Value::as_str).unwrap_or_default().to_string(),
                documentation: markup_to_string(item.get("documentation")),
                insert_text,
                filter_text: item.get("filterText").and_then(Value::as_str).unwrap_or(&label).to_string(),
                sort_text: item.get("sortText").and_then(Value::as_str).unwrap_or(&label).to_string(),
                text_edit,
                additional_text_edits,
            })
        })
        .take(200)
        .collect()
}

fn parameter_label(parameter: &Value, signature_label: &str) -> String {
    match parameter.get("label") {
        Some(Value::String(label)) => label.clone(),
        Some(Value::Array(range)) if range.len() == 2 => {
            let start = range[0].as_u64().unwrap_or(0) as usize;
            let end = range[1].as_u64().unwrap_or(start as u64) as usize;
            signature_label.get(start..end).unwrap_or_default().to_string()
        }
        _ => String::new(),
    }
}

pub fn warmup(runtime: &RustAnalyzerRuntime, project_path: String) -> Result<(), String> {
    let project = PathBuf::from(project_path);
    runtime.with_session(&project, |_| Ok(()))
}

pub fn completions(
    runtime: &RustAnalyzerRuntime,
    project_path: String,
    path: String,
    content: String,
    line: u32,
    character: u32,
) -> Result<Vec<CompletionItemView>, String> {
    let project = PathBuf::from(project_path);
    let document = PathBuf::from(path);
    runtime.with_session(&project, |session| {
        let uri = session.sync_document(&document, &content)?;
        let result = session.request(
            "textDocument/completion",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
                "context": { "triggerKind": 1 }
            }),
            Duration::from_secs(5),
        )?;
        Ok(completion_items(result))
    })
}

pub fn semantic_tokens(
    runtime: &RustAnalyzerRuntime,
    project_path: String,
    path: String,
    content: String,
) -> Result<Vec<SemanticTokenView>, String> {
    let project = PathBuf::from(project_path);
    let document = PathBuf::from(path);
    runtime.with_session(&project, |session| {
        let uri = session.sync_document(&document, &content)?;
        if session.semantic_token_types.is_empty() {
            return Ok(Vec::new());
        }
        let result = session.request(
            "textDocument/semanticTokens/full",
            json!({ "textDocument": { "uri": uri } }),
            Duration::from_secs(5),
        )?;
        let data = result.get("data").and_then(Value::as_array).cloned().unwrap_or_default();
        let mut output = Vec::with_capacity(data.len() / 5);
        let mut line = 0u32;
        let mut start_character = 0u32;

        for chunk in data.chunks_exact(5) {
            let delta_line = chunk[0].as_u64().unwrap_or(0) as u32;
            let delta_start = chunk[1].as_u64().unwrap_or(0) as u32;
            let length = chunk[2].as_u64().unwrap_or(0) as u32;
            let token_type_index = chunk[3].as_u64().unwrap_or(u64::MAX) as usize;
            let modifier_bits = chunk[4].as_u64().unwrap_or(0);

            if delta_line == 0 {
                start_character = start_character.saturating_add(delta_start);
            } else {
                line = line.saturating_add(delta_line);
                start_character = delta_start;
            }
            let Some(token_type) = session.semantic_token_types.get(token_type_index).cloned() else { continue; };
            let modifiers = session
                .semantic_token_modifiers
                .iter()
                .enumerate()
                .filter_map(|(index, modifier)| {
                    if index < 64 && (modifier_bits & (1u64 << index)) != 0 { Some(modifier.clone()) } else { None }
                })
                .collect();
            output.push(SemanticTokenView { line, start_character, length, token_type, modifiers });
        }
        Ok(output)
    })
}

pub fn signature_help(
    runtime: &RustAnalyzerRuntime,
    project_path: String,
    path: String,
    content: String,
    line: u32,
    character: u32,
) -> Result<Option<SignatureHelpView>, String> {
    let project = PathBuf::from(project_path);
    let document = PathBuf::from(path);
    runtime.with_session(&project, |session| {
        let uri = session.sync_document(&document, &content)?;
        let result = session.request(
            "textDocument/signatureHelp",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }),
            Duration::from_secs(5),
        )?;
        if result.is_null() {
            return Ok(None);
        }
        let signatures = result.get("signatures").and_then(Value::as_array).cloned().unwrap_or_default();
        let active_signature = result.get("activeSignature").and_then(Value::as_u64).unwrap_or(0) as usize;
        let Some(signature) = signatures.get(active_signature).or_else(|| signatures.first()) else { return Ok(None); };
        let label = signature.get("label").and_then(Value::as_str).unwrap_or_default().to_string();
        let parameters = signature
            .get("parameters")
            .and_then(Value::as_array)
            .map(|items| items.iter().map(|item| parameter_label(item, &label)).collect())
            .unwrap_or_default();
        let active_parameter = result
            .get("activeParameter")
            .and_then(Value::as_u64)
            .or_else(|| signature.get("activeParameter").and_then(Value::as_u64))
            .unwrap_or(0) as usize;
        Ok(Some(SignatureHelpView {
            label,
            documentation: markup_to_string(signature.get("documentation")),
            active_parameter,
            parameters,
        }))
    })
}
