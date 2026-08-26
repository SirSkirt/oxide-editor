use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    env,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{mpsc, Arc, Condvar, Mutex},
    thread,
    time::Duration,
};
use tauri::{AppHandle, Emitter};

#[derive(Default)]
struct DebuggerShared {
    session: Mutex<Option<DebuggerSession>>,
}

#[derive(Clone, Default)]
pub struct DebuggerRuntime {
    inner: Arc<DebuggerShared>,
}

struct DebuggerSession {
    child: Child,
    stdin: Arc<Mutex<ChildStdin>>,
    pending: Arc<Mutex<HashMap<i64, mpsc::Sender<Value>>>>,
    next_seq: i64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebuggerStatus {
    pub available: bool,
    pub adapter: String,
    pub path: String,
    pub version: String,
    pub message: String,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugBreakpointSet {
    pub path: String,
    pub lines: Vec<u32>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugStartResult {
    pub executable: String,
    pub adapter: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugStackFrame {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugScope {
    pub name: String,
    pub variables_reference: i64,
    pub expensive: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugVariable {
    pub name: String,
    pub value: String,
    pub type_name: String,
    pub variables_reference: i64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugEvaluateResult {
    pub result: String,
    pub type_name: String,
    pub variables_reference: i64,
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

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DebuggerStateEvent {
    state: String,
    detail: String,
}

fn executable_name(program: &str) -> String {
    if cfg!(windows) && !program.to_ascii_lowercase().ends_with(".exe") {
        format!("{program}.exe")
    } else {
        program.to_string()
    }
}

fn find_on_path(program: &str) -> Option<PathBuf> {
    let name = executable_name(program);
    if let Some(path_value) = env::var_os("PATH") {
        for directory in env::split_paths(&path_value) {
            let candidate = directory.join(&name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn adapter_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for name in ["lldb-dap", "lldb-vscode"] {
        if let Some(path) = find_on_path(name) {
            candidates.push(path);
        }
    }
    for version in (14..=22).rev() {
        if let Some(path) = find_on_path(&format!("lldb-dap-{version}")) {
            candidates.push(path);
        }
    }

    #[cfg(target_os = "windows")]
    {
        for variable in ["ProgramFiles", "ProgramW6432"] {
            if let Some(base) = env::var_os(variable) {
                for name in ["lldb-dap.exe", "lldb-vscode.exe"] {
                    let candidate = PathBuf::from(&base).join("LLVM").join("bin").join(name);
                    if candidate.is_file() {
                        candidates.push(candidate);
                    }
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        for raw in ["/usr/bin/lldb-dap", "/usr/local/bin/lldb-dap", "/usr/bin/lldb-vscode", "/usr/local/bin/lldb-vscode"] {
            let candidate = PathBuf::from(raw);
            if candidate.is_file() {
                candidates.push(candidate);
            }
        }
    }

    candidates.dedup();
    candidates
}

fn resolve_adapter() -> Option<PathBuf> {
    adapter_candidates().into_iter().next()
}

pub fn status() -> DebuggerStatus {
    let Some(adapter) = resolve_adapter() else {
        return DebuggerStatus {
            available: false,
            adapter: "LLDB DAP".into(),
            path: String::new(),
            version: "not found".into(),
            message: if cfg!(windows) {
                "LLDB's Debug Adapter (lldb-dap) was not found. Install LLVM for Windows and make sure its bin folder is on PATH.".into()
            } else {
                "LLDB's Debug Adapter (lldb-dap) was not found. Install LLDB from your Linux package manager (for Pop!_OS/Ubuntu: sudo apt install lldb).".into()
            },
        };
    };

    let version = Command::new(&adapter)
        .arg("--version")
        .output()
        .ok()
        .map(|output| {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if stdout.is_empty() { stderr } else { stdout }
        })
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "LLDB Debug Adapter".into());

    DebuggerStatus {
        available: true,
        adapter: adapter.file_name().and_then(|name| name.to_str()).unwrap_or("lldb-dap").into(),
        path: adapter.to_string_lossy().to_string(),
        version,
        message: "Oxide Debugger ready.".into(),
    }
}

fn write_dap(stdin: &Arc<Mutex<ChildStdin>>, value: &Value) -> Result<(), String> {
    let bytes = serde_json::to_vec(value).map_err(|error| format!("Could not encode debugger request: {error}"))?;
    let mut input = stdin.lock().map_err(|_| "Debugger input lock was poisoned.".to_string())?;
    write!(input, "Content-Length: {}\r\n\r\n", bytes.len())
        .and_then(|_| input.write_all(&bytes))
        .and_then(|_| input.flush())
        .map_err(|error| format!("Could not write to debugger adapter: {error}"))
}

fn debugger_reader<R: Read + Send + 'static>(
    app: AppHandle,
    reader: R,
    pending: Arc<Mutex<HashMap<i64, mpsc::Sender<Value>>>>,
    initialized: Arc<(Mutex<bool>, Condvar)>,
) {
    thread::spawn(move || {
        let mut reader = BufReader::new(reader);
        loop {
            let mut content_length = None;
            loop {
                let mut header = String::new();
                match reader.read_line(&mut header) {
                    Ok(0) => {
                        let _ = app.emit("debugger-state", DebuggerStateEvent { state: "adapter-exited".into(), detail: "Debugger adapter exited.".into() });
                        return;
                    }
                    Ok(_) => {}
                    Err(_) => return,
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
            let mut body = vec![0_u8; length];
            if reader.read_exact(&mut body).is_err() {
                return;
            }
            let Ok(message) = serde_json::from_slice::<Value>(&body) else { continue; };
            match message.get("type").and_then(Value::as_str) {
                Some("response") => {
                    if let Some(request_seq) = message.get("request_seq").and_then(Value::as_i64) {
                        if let Ok(mut guard) = pending.lock() {
                            if let Some(sender) = guard.remove(&request_seq) {
                                let _ = sender.send(message);
                            }
                        }
                    }
                }
                Some("event") => {
                    if message.get("event").and_then(Value::as_str) == Some("initialized") {
                        let (flag, cv) = &*initialized;
                        if let Ok(mut ready) = flag.lock() {
                            *ready = true;
                            cv.notify_all();
                        }
                    }
                    let _ = app.emit("debugger-event", message);
                }
                _ => {}
            }
        }
    });
}

fn request(runtime: &DebuggerRuntime, command: &str, arguments: Value, timeout: Duration) -> Result<Value, String> {
    let (seq, stdin, pending) = {
        let mut guard = runtime.inner.session.lock().map_err(|_| "Debugger state is unavailable.".to_string())?;
        let session = guard.as_mut().ok_or_else(|| "No debugger session is active.".to_string())?;
        session.next_seq += 1;
        (session.next_seq, Arc::clone(&session.stdin), Arc::clone(&session.pending))
    };

    let (sender, receiver) = mpsc::channel();
    pending.lock().map_err(|_| "Debugger response state is unavailable.".to_string())?.insert(seq, sender);
    write_dap(&stdin, &json!({
        "seq": seq,
        "type": "request",
        "command": command,
        "arguments": arguments,
    }))?;

    let response = receiver.recv_timeout(timeout).map_err(|_| {
        if let Ok(mut guard) = pending.lock() { guard.remove(&seq); }
        format!("Debugger request '{command}' timed out.")
    })?;

    if !response.get("success").and_then(Value::as_bool).unwrap_or(false) {
        return Err(response.get("message").and_then(Value::as_str).unwrap_or("Debugger request failed.").to_string());
    }
    Ok(response.get("body").cloned().unwrap_or_else(|| json!({})))
}

fn send_request(runtime: &DebuggerRuntime, command: &str, arguments: Value) -> Result<(), String> {
    let (seq, stdin) = {
        let mut guard = runtime.inner.session.lock().map_err(|_| "Debugger state is unavailable.".to_string())?;
        let session = guard.as_mut().ok_or_else(|| "No debugger session is active.".to_string())?;
        session.next_seq += 1;
        (session.next_seq, Arc::clone(&session.stdin))
    };
    write_dap(&stdin, &json!({
        "seq": seq,
        "type": "request",
        "command": command,
        "arguments": arguments,
    }))
}

fn cargo_program() -> PathBuf {
    if let Some(path) = find_on_path("cargo") {
        return path;
    }
    if let Some(home) = env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }) {
        let candidate = PathBuf::from(home).join(".cargo").join("bin").join(executable_name("cargo"));
        if candidate.is_file() {
            return candidate;
        }
    }
    PathBuf::from(executable_name("cargo"))
}

fn build_debug_binary(app: &AppHandle, project_path: &Path) -> Result<PathBuf, String> {
    let _ = app.emit("debugger-state", DebuggerStateEvent { state: "building".into(), detail: "Building debug target before debugger launch.".into() });
    let _ = app.emit("cargo-state", CargoStateEvent { state: "started".into(), detail: "cargo build --message-format=json".into() });

    let mut command = Command::new(cargo_program());
    command
        .arg("build")
        .arg("--message-format=json")
        .current_dir(project_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }

    let mut child = command.spawn().map_err(|error| format!("Could not start Cargo debug build: {error}"))?;
    let stdout = child.stdout.take().ok_or_else(|| "Could not capture Cargo debug build output.".to_string())?;
    let stderr = child.stderr.take().ok_or_else(|| "Could not capture Cargo debug build errors.".to_string())?;

    let stderr_app = app.clone();
    let stderr_thread = thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            let _ = stderr_app.emit("cargo-output", CargoLine { stream: "stderr".into(), line });
        }
    });

    let mut executables = Vec::<PathBuf>::new();
    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
        match serde_json::from_str::<Value>(&line) {
            Ok(message) => match message.get("reason").and_then(Value::as_str) {
                Some("compiler-message") => {
                    if let Some(rendered) = message.get("message").and_then(|m| m.get("rendered")).and_then(Value::as_str) {
                        for rendered_line in rendered.lines() {
                            let _ = app.emit("cargo-output", CargoLine { stream: "stderr".into(), line: rendered_line.to_string() });
                        }
                    }
                }
                Some("compiler-artifact") => {
                    let is_bin = message.get("target").and_then(|t| t.get("kind")).and_then(Value::as_array)
                        .map(|kinds| kinds.iter().any(|kind| kind.as_str() == Some("bin"))).unwrap_or(false);
                    if is_bin {
                        if let Some(executable) = message.get("executable").and_then(Value::as_str) {
                            let path = PathBuf::from(executable);
                            if !executables.iter().any(|existing| existing == &path) {
                                executables.push(path);
                            }
                        }
                    }
                }
                _ => {}
            },
            Err(_) => {
                let _ = app.emit("cargo-output", CargoLine { stream: "stdout".into(), line });
            }
        }
    }

    let status = child.wait().map_err(|error| format!("Could not wait for Cargo debug build: {error}"))?;
    let _ = stderr_thread.join();
    let _ = app.emit("cargo-state", CargoStateEvent {
        state: "finished".into(),
        detail: if status.success() { "Cargo debug build finished successfully.".into() } else { "Cargo debug build finished with errors.".into() },
    });

    if !status.success() {
        return Err("The project did not compile, so Oxide could not start debugging.".into());
    }
    match executables.len() {
        0 => Err("Cargo built successfully, but no runnable binary target was produced.".into()),
        1 => Ok(executables.remove(0)),
        _ => Err("This project has multiple binary targets. Debug target selection is not implemented in Build 1 yet.".into()),
    }
}

fn set_breakpoints_internal(runtime: &DebuggerRuntime, set: &DebugBreakpointSet) -> Result<(), String> {
    let breakpoints = set.lines.iter().copied().map(|line| json!({ "line": line })).collect::<Vec<_>>();
    request(runtime, "setBreakpoints", json!({
        "source": { "path": set.path.clone() },
        "breakpoints": breakpoints,
        "sourceModified": false,
    }), Duration::from_secs(5)).map(|_| ())
}

pub fn start(
    app: AppHandle,
    runtime: &DebuggerRuntime,
    project_path: String,
    breakpoints: Vec<DebugBreakpointSet>,
) -> Result<DebugStartResult, String> {
    {
        let guard = runtime.inner.session.lock().map_err(|_| "Debugger state is unavailable.".to_string())?;
        if guard.is_some() {
            return Err("A debugger session is already active.".into());
        }
    }

    let adapter = resolve_adapter().ok_or_else(|| status().message)?;
    let executable = build_debug_binary(&app, Path::new(&project_path))?;
    let executable_string = executable.to_string_lossy().to_string();
    let _ = app.emit("debugger-state", DebuggerStateEvent { state: "starting".into(), detail: format!("Starting {}.", adapter.display()) });

    let mut command = Command::new(&adapter);
    command.current_dir(&project_path).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command.spawn().map_err(|error| format!("Could not start LLDB Debug Adapter: {error}"))?;
    let stdin = Arc::new(Mutex::new(child.stdin.take().ok_or_else(|| "Could not open debugger input.".to_string())?));
    let stdout = child.stdout.take().ok_or_else(|| "Could not capture debugger output.".to_string())?;
    let stderr = child.stderr.take().ok_or_else(|| "Could not capture debugger errors.".to_string())?;
    let pending = Arc::new(Mutex::new(HashMap::new()));
    let initialized = Arc::new((Mutex::new(false), Condvar::new()));

    debugger_reader(app.clone(), stdout, Arc::clone(&pending), Arc::clone(&initialized));
    let error_app = app.clone();
    thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if !line.trim().is_empty() {
                let _ = error_app.emit("debugger-output", json!({ "category": "adapter", "output": format!("{line}\n") }));
            }
        }
    });

    *runtime.inner.session.lock().map_err(|_| "Debugger state is unavailable.".to_string())? = Some(DebuggerSession {
        child,
        stdin,
        pending,
        next_seq: 0,
    });

    let initialization = request(runtime, "initialize", json!({
        "clientID": "oxide-editor",
        "clientName": "Oxide Editor",
        "adapterID": "lldb",
        "locale": "en-US",
        "linesStartAt1": true,
        "columnsStartAt1": true,
        "pathFormat": "path",
        "supportsVariableType": true,
        "supportsVariablePaging": false,
        "supportsRunInTerminalRequest": false,
    }), Duration::from_secs(5));
    if let Err(error) = initialization {
        let _ = stop(runtime);
        return Err(error);
    }

    let configure = (|| -> Result<(), String> {
        // DAP launch is intentionally sent without waiting. LLDB may defer the launch
        // response until the client has completed breakpoint/configuration requests.
        send_request(runtime, "launch", json!({
            "program": executable_string.clone(),
            "cwd": project_path,
            "args": [],
            "env": {},
            "stopOnEntry": false,
        }))?;

        {
            let (flag, cv) = &*initialized;
            let ready = flag.lock().map_err(|_| "Debugger initialization state is unavailable.".to_string())?;
            let (ready, _) = cv.wait_timeout_while(ready, Duration::from_secs(4), |value| !*value)
                .map_err(|_| "Debugger initialization wait failed.".to_string())?;
            if !*ready {
                return Err("LLDB did not finish debugger initialization in time.".into());
            }
        }

        for set in &breakpoints {
            set_breakpoints_internal(runtime, set)?;
        }
        let _ = request(runtime, "setExceptionBreakpoints", json!({ "filters": [] }), Duration::from_secs(3));
        request(runtime, "configurationDone", json!({}), Duration::from_secs(5))?;
        Ok(())
    })();
    if let Err(error) = configure {
        let _ = stop(runtime);
        return Err(error);
    }

    let _ = app.emit("debugger-state", DebuggerStateEvent { state: "running".into(), detail: "Debugger session started.".into() });
    Ok(DebugStartResult {
        executable: executable_string,
        adapter: adapter.to_string_lossy().to_string(),
    })
}

pub fn set_breakpoints(runtime: &DebuggerRuntime, set: DebugBreakpointSet) -> Result<(), String> {
    set_breakpoints_internal(runtime, &set)
}

fn choose_thread(runtime: &DebuggerRuntime, supplied: Option<i64>) -> Result<i64, String> {
    if let Some(id) = supplied.filter(|id| *id > 0) { return Ok(id); }
    let body = request(runtime, "threads", json!({}), Duration::from_secs(3))?;
    body.get("threads").and_then(Value::as_array).and_then(|items| items.first())
        .and_then(|thread| thread.get("id")).and_then(Value::as_i64)
        .ok_or_else(|| "The debugger did not report an active thread.".to_string())
}

pub fn continue_execution(runtime: &DebuggerRuntime, thread_id: Option<i64>) -> Result<(), String> {
    let id = choose_thread(runtime, thread_id)?;
    request(runtime, "continue", json!({ "threadId": id }), Duration::from_secs(3)).map(|_| ())
}

pub fn pause(runtime: &DebuggerRuntime, thread_id: Option<i64>) -> Result<(), String> {
    let id = choose_thread(runtime, thread_id)?;
    request(runtime, "pause", json!({ "threadId": id }), Duration::from_secs(3)).map(|_| ())
}

pub fn next(runtime: &DebuggerRuntime, thread_id: Option<i64>) -> Result<(), String> {
    let id = choose_thread(runtime, thread_id)?;
    request(runtime, "next", json!({ "threadId": id, "singleThread": false }), Duration::from_secs(3)).map(|_| ())
}

pub fn step_in(runtime: &DebuggerRuntime, thread_id: Option<i64>) -> Result<(), String> {
    let id = choose_thread(runtime, thread_id)?;
    request(runtime, "stepIn", json!({ "threadId": id, "singleThread": false }), Duration::from_secs(3)).map(|_| ())
}

pub fn step_out(runtime: &DebuggerRuntime, thread_id: Option<i64>) -> Result<(), String> {
    let id = choose_thread(runtime, thread_id)?;
    request(runtime, "stepOut", json!({ "threadId": id, "singleThread": false }), Duration::from_secs(3)).map(|_| ())
}

pub fn stack_trace(runtime: &DebuggerRuntime, thread_id: i64) -> Result<Vec<DebugStackFrame>, String> {
    let body = request(runtime, "stackTrace", json!({ "threadId": thread_id, "startFrame": 0, "levels": 64 }), Duration::from_secs(4))?;
    Ok(body.get("stackFrames").and_then(Value::as_array).map(|frames| frames.iter().filter_map(|frame| {
        Some(DebugStackFrame {
            id: frame.get("id")?.as_i64()?,
            name: frame.get("name").and_then(Value::as_str).unwrap_or("frame").to_string(),
            path: frame.get("source").and_then(|source| source.get("path")).and_then(Value::as_str).unwrap_or("").to_string(),
            line: frame.get("line").and_then(Value::as_u64).unwrap_or(1) as u32,
            column: frame.get("column").and_then(Value::as_u64).unwrap_or(1) as u32,
        })
    }).collect()).unwrap_or_default())
}

pub fn scopes(runtime: &DebuggerRuntime, frame_id: i64) -> Result<Vec<DebugScope>, String> {
    let body = request(runtime, "scopes", json!({ "frameId": frame_id }), Duration::from_secs(4))?;
    Ok(body.get("scopes").and_then(Value::as_array).map(|scopes| scopes.iter().filter_map(|scope| {
        Some(DebugScope {
            name: scope.get("name")?.as_str()?.to_string(),
            variables_reference: scope.get("variablesReference").and_then(Value::as_i64).unwrap_or(0),
            expensive: scope.get("expensive").and_then(Value::as_bool).unwrap_or(false),
        })
    }).collect()).unwrap_or_default())
}

pub fn variables(runtime: &DebuggerRuntime, variables_reference: i64) -> Result<Vec<DebugVariable>, String> {
    let body = request(runtime, "variables", json!({ "variablesReference": variables_reference }), Duration::from_secs(4))?;
    Ok(body.get("variables").and_then(Value::as_array).map(|variables| variables.iter().filter_map(|variable| {
        Some(DebugVariable {
            name: variable.get("name")?.as_str()?.to_string(),
            value: variable.get("value").and_then(Value::as_str).unwrap_or("").to_string(),
            type_name: variable.get("type").and_then(Value::as_str).unwrap_or("").to_string(),
            variables_reference: variable.get("variablesReference").and_then(Value::as_i64).unwrap_or(0),
        })
    }).collect()).unwrap_or_default())
}

pub fn evaluate(runtime: &DebuggerRuntime, expression: String, frame_id: Option<i64>) -> Result<DebugEvaluateResult, String> {
    let mut arguments = json!({ "expression": expression, "context": "watch" });
    if let Some(frame_id) = frame_id {
        arguments["frameId"] = json!(frame_id);
    }
    let body = request(runtime, "evaluate", arguments, Duration::from_secs(4))?;
    Ok(DebugEvaluateResult {
        result: body.get("result").and_then(Value::as_str).unwrap_or("").to_string(),
        type_name: body.get("type").and_then(Value::as_str).unwrap_or("").to_string(),
        variables_reference: body.get("variablesReference").and_then(Value::as_i64).unwrap_or(0),
    })
}

pub fn stop(runtime: &DebuggerRuntime) -> Result<(), String> {
    {
        let guard = runtime.inner.session.lock().map_err(|_| "Debugger state is unavailable.".to_string())?;
        if guard.is_none() { return Ok(()); }
    }
    let _ = request(runtime, "disconnect", json!({ "restart": false, "terminateDebuggee": true }), Duration::from_secs(2));
    let mut guard = runtime.inner.session.lock().map_err(|_| "Debugger state is unavailable.".to_string())?;
    if let Some(mut session) = guard.take() {
        let _ = session.child.kill();
        let _ = session.child.wait();
    }
    Ok(())
}
