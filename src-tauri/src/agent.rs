use crate::tools;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use tauri::{Emitter, Window};

const MAX_ITERATIONS: usize = 25;

const SYSTEM_PROMPT: &str = "You are Frox Morph Code, an agentic coding assistant embedded in a \
desktop app. You can read, write, and edit files, list directories, and run shell commands in the \
user's open project folder via the tools provided. Work step by step: inspect relevant files before \
editing them, make focused changes, and verify your work (e.g. by running tests or reading the file \
back) when reasonable. Prefer the edit_file tool for small changes to existing files, and write_file \
only for new files or full rewrites. Explain what you're doing briefly as you go, then summarize what \
changed when you're done.";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChatMessage {
    pub role: String, // "system" | "user" | "assistant" | "tool"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Clone, Serialize)]
struct AgentEvent<'a> {
    kind: &'a str, // "assistant_text" | "tool_call" | "tool_result" | "error" | "done"
    payload: Value,
}

fn emit(window: &Window, kind: &str, payload: Value) {
    let _ = window.emit("frox-agent-event", AgentEvent { kind, payload });
}

pub struct ModelConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

pub async fn run_agent_loop(
    window: Window,
    project_root: Option<PathBuf>,
    config: ModelConfig,
    mut history: Vec<ChatMessage>,
    user_message: String,
) -> Result<Vec<ChatMessage>, String> {
    if history.is_empty() {
        let prompt = if project_root.is_some() {
            SYSTEM_PROMPT.to_string()
        } else {
            // Mobile / no project open: plain chat, no tool access.
            "You are Frox, a helpful multimodal AI assistant. No project folder is open, so you \
             cannot read, write, or run anything right now — just answer normally."
                .to_string()
        };
        history.push(ChatMessage {
            role: "system".into(),
            content: Some(prompt),
            tool_calls: None,
            tool_call_id: None,
        });
    }
    history.push(ChatMessage {
        role: "user".into(),
        content: Some(user_message),
        tool_calls: None,
        tool_call_id: None,
    });

    let client = reqwest::Client::new();

    for _ in 0..MAX_ITERATIONS {
        let mut body = json!({
            "model": config.model,
            "messages": history,
        });
        if project_root.is_some() {
            body["tools"] = tools::tool_schema();
        }

        let resp = client
            .post(format!("{}/chat/completions", config.base_url.trim_end_matches('/')))
            .bearer_auth(&config.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Request to model API failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            let msg = format!("Model API returned {status}: {text}");
            emit(&window, "error", json!({ "message": msg.clone() }));
            return Err(msg);
        }

        let data: Value = resp
            .json()
            .await
            .map_err(|e| format!("Couldn't parse model API response: {e}"))?;

        let choice = data["choices"].get(0).cloned().unwrap_or(json!({}));
        let message = choice["message"].clone();
        let content = message.get("content").and_then(|c| c.as_str()).map(String::from);
        let tool_calls = message.get("tool_calls").cloned().filter(|v| !v.is_null());

        history.push(ChatMessage {
            role: "assistant".into(),
            content: content.clone(),
            tool_calls: tool_calls.clone(),
            tool_call_id: None,
        });

        if let Some(text) = &content {
            if !text.trim().is_empty() {
                emit(&window, "assistant_text", json!({ "text": text }));
            }
        }

        let calls = match tool_calls {
            Some(Value::Array(arr)) if !arr.is_empty() && project_root.is_some() => arr,
            _ => {
                // No tool calls: the model is done for this turn.
                emit(&window, "done", json!({}));
                return Ok(history);
            }
        };

        let root = project_root.as_ref().expect("checked above");

        for call in calls {
            let call_id = call["id"].as_str().unwrap_or("").to_string();
            let name = call["function"]["name"].as_str().unwrap_or("").to_string();
            let args_str = call["function"]["arguments"].as_str().unwrap_or("{}");
            let args: Value = serde_json::from_str(args_str).unwrap_or(json!({}));

            emit(
                &window,
                "tool_call",
                json!({ "name": name, "args": args }),
            );

            let result = execute_tool(root, &name, &args);

            emit(
                &window,
                "tool_result",
                json!({ "name": name, "result": result }),
            );

            history.push(ChatMessage {
                role: "tool".into(),
                content: Some(result),
                tool_calls: None,
                tool_call_id: Some(call_id),
            });
        }
    }

    emit(
        &window,
        "error",
        json!({ "message": "Stopped after 25 tool-use steps without finishing. The task may be too large, or the model may be stuck in a loop." }),
    );
    Ok(history)
}

fn execute_tool(root: &PathBuf, name: &str, args: &Value) -> String {
    match name {
        "read_file" => {
            let path = args["path"].as_str().unwrap_or("");
            tools::read_file(root, path).unwrap_or_else(|e| e.message())
        }
        "write_file" => {
            let path = args["path"].as_str().unwrap_or("");
            let content = args["content"].as_str().unwrap_or("");
            tools::write_file(root, path, content).unwrap_or_else(|e| e.message())
        }
        "edit_file" => {
            let path = args["path"].as_str().unwrap_or("");
            let old_str = args["old_str"].as_str().unwrap_or("");
            let new_str = args["new_str"].as_str().unwrap_or("");
            tools::edit_file(root, path, old_str, new_str).unwrap_or_else(|e| e.message())
        }
        "list_dir" => {
            let path = args["path"].as_str().unwrap_or(".");
            match tools::list_dir(root, path) {
                Ok(entries) => serde_json::to_string_pretty(&entries).unwrap_or_default(),
                Err(e) => e.message(),
            }
        }
        "run_command" => {
            let command = args["command"].as_str().unwrap_or("");
            tools::run_command(root, command).unwrap_or_else(|e| e.message())
        }
        other => format!("Unknown tool: {other}"),
    }
}
