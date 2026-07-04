use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum ToolError {
    OutsideProject,
    Io(String),
    BadArgs(String),
    NotFound(String),
    #[allow(dead_code)]
    Timeout,
}

impl ToolError {
    pub fn message(&self) -> String {
        match self {
            ToolError::OutsideProject => "Refused: path is outside the open project folder.".into(),
            ToolError::Io(e) => format!("IO error: {e}"),
            ToolError::BadArgs(e) => format!("Bad arguments: {e}"),
            ToolError::NotFound(p) => format!("Not found: {p}"),
            ToolError::Timeout => "Command timed out after 30s.".into(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DirEntryInfo {
    pub name: String,
    pub is_dir: bool,
}

// ── Desktop-only (file system + process execution) ───────────────────────────

#[cfg(not(target_os = "android"))]
fn resolve_safe(root: &Path, rel: &str) -> Result<PathBuf, ToolError> {
    let candidate = root.join(rel);
    let check_path = if candidate.exists() {
        candidate.canonicalize().map_err(|e| ToolError::Io(e.to_string()))?
    } else {
        let parent = candidate.parent().unwrap_or(root);
        let parent_canon = parent
            .canonicalize()
            .map_err(|_| ToolError::NotFound(parent.display().to_string()))?;
        parent_canon.join(candidate.file_name().unwrap_or_default())
    };
    let root_canon = root.canonicalize().map_err(|e| ToolError::Io(e.to_string()))?;
    if !check_path.starts_with(&root_canon) {
        return Err(ToolError::OutsideProject);
    }
    Ok(check_path)
}

#[cfg(not(target_os = "android"))]
pub fn read_file(root: &Path, rel_path: &str) -> Result<String, ToolError> {
    let p = resolve_safe(root, rel_path)?;
    std::fs::read_to_string(&p).map_err(|e| ToolError::Io(e.to_string()))
}

#[cfg(not(target_os = "android"))]
pub fn write_file(root: &Path, rel_path: &str, content: &str) -> Result<String, ToolError> {
    let p = resolve_safe(root, rel_path)?;
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).map_err(|e| ToolError::Io(e.to_string()))?;
    }
    std::fs::write(&p, content).map_err(|e| ToolError::Io(e.to_string()))?;
    Ok(format!("Wrote {} bytes to {}", content.len(), rel_path))
}

#[cfg(not(target_os = "android"))]
pub fn edit_file(root: &Path, rel_path: &str, old_str: &str, new_str: &str) -> Result<String, ToolError> {
    let p = resolve_safe(root, rel_path)?;
    let content = std::fs::read_to_string(&p).map_err(|e| ToolError::Io(e.to_string()))?;
    let occurrences = content.matches(old_str).count();
    if occurrences == 0 {
        return Err(ToolError::BadArgs(format!(
            "old_str not found in {rel_path}. Re-read the file and match exactly."
        )));
    }
    if occurrences > 1 {
        return Err(ToolError::BadArgs(format!(
            "old_str appears {occurrences} times in {rel_path}; add more context to make it unique."
        )));
    }
    let updated = content.replacen(old_str, new_str, 1);
    std::fs::write(&p, &updated).map_err(|e| ToolError::Io(e.to_string()))?;
    Ok(format!("Edited {rel_path}"))
}

#[cfg(not(target_os = "android"))]
pub fn list_dir(root: &Path, rel_path: &str) -> Result<Vec<DirEntryInfo>, ToolError> {
    let p = if rel_path.trim().is_empty() || rel_path == "." {
        root.canonicalize().map_err(|e| ToolError::Io(e.to_string()))?
    } else {
        resolve_safe(root, rel_path)?
    };
    let mut out = vec![];
    for entry in std::fs::read_dir(&p).map_err(|e| ToolError::Io(e.to_string()))?.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "node_modules" || name == ".git" || name == "target" { continue; }
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        out.push(DirEntryInfo { name, is_dir });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

#[cfg(not(target_os = "android"))]
pub fn run_command(root: &Path, command: &str) -> Result<String, ToolError> {
    use std::time::Duration;
    let root_canon = root.canonicalize().map_err(|e| ToolError::Io(e.to_string()))?;

    #[cfg(target_os = "windows")]
    let mut cmd = { let mut c = std::process::Command::new("cmd"); c.arg("/C").arg(command); c };
    #[cfg(not(target_os = "windows"))]
    let mut cmd = { let mut c = std::process::Command::new("sh"); c.arg("-c").arg(command); c };

    cmd.current_dir(&root_canon)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let child = cmd.spawn().map_err(|e| ToolError::Io(e.to_string()))?;
    let output = wait_with_timeout(child, Duration::from_secs(30))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    Ok(format!("exit_code: {}\nstdout:\n{}\nstderr:\n{}",
        output.status.code().unwrap_or(-1), truncate(&stdout, 8000), truncate(&stderr, 4000)))
}

#[cfg(not(target_os = "android"))]
fn truncate(s: &str, max: usize) -> String {
    if s.len() > max { format!("{}\n... [truncated]", &s[..max]) } else { s.to_string() }
}

#[cfg(not(target_os = "android"))]
fn wait_with_timeout(mut child: std::process::Child, timeout: Duration) -> Result<std::process::Output, ToolError> {
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return child.wait_with_output().map_err(|e| ToolError::Io(e.to_string())),
            Ok(None) => {
                if start.elapsed() > timeout { let _ = child.kill(); return Err(ToolError::Timeout); }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(e) => return Err(ToolError::Io(e.to_string())),
        }
    }
}

// ── Android stubs (no-ops; these paths are never called on Android) ──────────

#[cfg(target_os = "android")]
pub fn read_file(_root: &Path, _rel: &str) -> Result<String, ToolError> {
    Err(ToolError::Io("File tools not available on Android.".into()))
}
#[cfg(target_os = "android")]
pub fn write_file(_root: &Path, _rel: &str, _content: &str) -> Result<String, ToolError> {
    Err(ToolError::Io("File tools not available on Android.".into()))
}
#[cfg(target_os = "android")]
pub fn edit_file(_root: &Path, _rel: &str, _old: &str, _new: &str) -> Result<String, ToolError> {
    Err(ToolError::Io("File tools not available on Android.".into()))
}
#[cfg(target_os = "android")]
pub fn list_dir(_root: &Path, _rel: &str) -> Result<Vec<DirEntryInfo>, ToolError> {
    Err(ToolError::Io("File tools not available on Android.".into()))
}
#[cfg(target_os = "android")]
pub fn run_command(_root: &Path, _cmd: &str) -> Result<String, ToolError> {
    Err(ToolError::Io("Command execution not available on Android.".into()))
}

// ── Tool schema (Android gets empty array — no tools offered to the model) ───

pub fn tool_schema() -> Value {
    #[cfg(target_os = "android")]
    return json!([]);

    #[cfg(not(target_os = "android"))]
    json!([
        {
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Read the full contents of a file (path relative to project root).",
                "parameters": {
                    "type": "object",
                    "properties": { "path": { "type": "string" } },
                    "required": ["path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "write_file",
                "description": "Create or overwrite a file with the given content.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "content": { "type": "string" }
                    },
                    "required": ["path", "content"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "edit_file",
                "description": "Replace one exact occurrence of old_str with new_str in a file.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "old_str": { "type": "string" },
                        "new_str": { "type": "string" }
                    },
                    "required": ["path", "old_str", "new_str"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_dir",
                "description": "List files and folders at a path relative to the project root. Use '.' for root.",
                "parameters": {
                    "type": "object",
                    "properties": { "path": { "type": "string" } },
                    "required": ["path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "run_command",
                "description": "Run a shell command in the project root.",
                "parameters": {
                    "type": "object",
                    "properties": { "command": { "type": "string" } },
                    "required": ["command"]
                }
            }
        }
    ])
}
