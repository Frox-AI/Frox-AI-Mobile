use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

#[derive(Debug)]
pub enum ToolError {
    OutsideProject,
    Io(String),
    BadArgs(String),
    NotFound(String),
    Timeout,
}

impl ToolError {
    pub fn message(&self) -> String {
        match self {
            ToolError::OutsideProject => {
                "Refused: that path is outside the open project folder.".to_string()
            }
            ToolError::Io(e) => format!("IO error: {e}"),
            ToolError::BadArgs(e) => format!("Bad arguments: {e}"),
            ToolError::NotFound(p) => format!("Not found: {p}"),
            ToolError::Timeout => "Command timed out after 30s.".to_string(),
        }
    }
}

/// Resolves a user/model-supplied relative path against the project root,
/// and refuses to leave the project directory (blocks path traversal).
fn resolve_safe(root: &Path, rel: &str) -> Result<PathBuf, ToolError> {
    let candidate = root.join(rel);
    // Canonicalize what we can; for paths that don't exist yet (e.g. new file
    // to write), canonicalize the parent instead.
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

pub fn read_file(root: &Path, rel_path: &str) -> Result<String, ToolError> {
    let p = resolve_safe(root, rel_path)?;
    std::fs::read_to_string(&p).map_err(|e| ToolError::Io(e.to_string()))
}

pub fn write_file(root: &Path, rel_path: &str, content: &str) -> Result<String, ToolError> {
    let p = resolve_safe(root, rel_path)?;
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).map_err(|e| ToolError::Io(e.to_string()))?;
    }
    std::fs::write(&p, content).map_err(|e| ToolError::Io(e.to_string()))?;
    Ok(format!("Wrote {} bytes to {}", content.len(), rel_path))
}

/// Exact-match search & replace within a file, in the style of Claude Code's edit tool.
/// `old_str` must appear exactly once in the file, or this fails loudly rather than
/// guessing which occurrence was meant.
pub fn edit_file(
    root: &Path,
    rel_path: &str,
    old_str: &str,
    new_str: &str,
) -> Result<String, ToolError> {
    let p = resolve_safe(root, rel_path)?;
    let content = std::fs::read_to_string(&p).map_err(|e| ToolError::Io(e.to_string()))?;
    let occurrences = content.matches(old_str).count();
    if occurrences == 0 {
        return Err(ToolError::BadArgs(format!(
            "old_str not found in {rel_path}. Re-read the file and match text exactly."
        )));
    }
    if occurrences > 1 {
        return Err(ToolError::BadArgs(format!(
            "old_str appears {occurrences} times in {rel_path}; include more surrounding context so it's unique."
        )));
    }
    let updated = content.replacen(old_str, new_str, 1);
    std::fs::write(&p, &updated).map_err(|e| ToolError::Io(e.to_string()))?;
    Ok(format!("Edited {rel_path}"))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DirEntryInfo {
    pub name: String,
    pub is_dir: bool,
}

pub fn list_dir(root: &Path, rel_path: &str) -> Result<Vec<DirEntryInfo>, ToolError> {
    let p = if rel_path.trim().is_empty() || rel_path == "." {
        root.canonicalize().map_err(|e| ToolError::Io(e.to_string()))?
    } else {
        resolve_safe(root, rel_path)?
    };
    let mut out = vec![];
    let entries = std::fs::read_dir(&p).map_err(|e| ToolError::Io(e.to_string()))?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "node_modules" || name == ".git" || name == "target" {
            continue;
        }
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        out.push(DirEntryInfo { name, is_dir });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Runs a shell command with the project root as cwd.
///
/// SECURITY NOTE (v1 limitation): this executes commands directly on the host,
/// not inside a container/VM sandbox. That means a misbehaving or adversarial
/// model response can run arbitrary commands with your user's permissions.
/// Claude Code / Codex mitigate this with per-command approval prompts and/or
/// container sandboxing. Before shipping this to real users, add at minimum:
/// (1) a user-approval prompt before any run_command executes, and
/// (2) ideally, execution inside a disposable container instead of the host.
pub fn run_command(root: &Path, command: &str) -> Result<String, ToolError> {
    let root_canon = root.canonicalize().map_err(|e| ToolError::Io(e.to_string()))?;

    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = Command::new("cmd");
        c.arg("/C").arg(command);
        c
    };
    #[cfg(not(target_os = "windows"))]
    let mut cmd = {
        let mut c = Command::new("sh");
        c.arg("-c").arg(command);
        c
    };

    cmd.current_dir(&root_canon);

    let child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| ToolError::Io(e.to_string()))?;

    let output = wait_with_timeout(child, Duration::from_secs(30))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    Ok(format!(
        "exit_code: {}\nstdout:\n{}\nstderr:\n{}",
        output.status.code().unwrap_or(-1),
        truncate(&stdout, 8000),
        truncate(&stderr, 4000)
    ))
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}\n... [truncated]", &s[..max])
    } else {
        s.to_string()
    }
}

fn wait_with_timeout(
    mut child: std::process::Child,
    timeout: Duration,
) -> Result<std::process::Output, ToolError> {
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => {
                return child
                    .wait_with_output()
                    .map_err(|e| ToolError::Io(e.to_string()))
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    return Err(ToolError::Timeout);
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(ToolError::Io(e.to_string())),
        }
    }
}

/// OpenAI-style tool/function schema definitions sent to the model each turn.
pub fn tool_schema() -> Value {
    json!([
        {
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Read the full contents of a file, given a path relative to the project root.",
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
                "description": "Create a new file or overwrite an existing file with the given content.",
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
                "description": "Replace one exact occurrence of old_str with new_str in a file. old_str must match the file's current content exactly and appear only once — include enough surrounding context to make it unique. Prefer this over write_file for small changes to existing files.",
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
                "description": "List files and folders at a path relative to the project root (use '.' for the root).",
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
                "description": "Run a shell command in the project root (e.g. running tests, installing a package, git status). Use sparingly and prefer read-only commands when just inspecting state.",
                "parameters": {
                    "type": "object",
                    "properties": { "command": { "type": "string" } },
                    "required": ["command"]
                }
            }
        }
    ])
}
