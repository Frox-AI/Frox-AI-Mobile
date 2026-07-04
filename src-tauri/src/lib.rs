mod agent;
mod tools;

use agent::{ChatMessage, ModelConfig};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager, State, Window};
use tauri_plugin_dialog::DialogExt;

#[derive(Default)]
struct AppState {
    project_root: Mutex<Option<PathBuf>>,
    history: Mutex<Vec<ChatMessage>>,
}

#[derive(Serialize, Deserialize, Clone)]
struct Settings {
    base_url: String,
    api_key: String,
    model: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            base_url: "http://localhost:8000/v1".into(),
            api_key: "".into(),
            model: "frox-morph-code".into(),
        }
    }
}

fn settings_path(app: &tauri::AppHandle) -> PathBuf {
    let dir = app.path().app_config_dir().unwrap_or_else(|_| PathBuf::from("."));
    let _ = std::fs::create_dir_all(&dir);
    dir.join("settings.json")
}

#[tauri::command]
fn get_settings(app: tauri::AppHandle) -> Settings {
    let path = settings_path(&app);
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

#[tauri::command]
fn save_settings(app: tauri::AppHandle, settings: Settings) -> Result<(), String> {
    let path = settings_path(&app);
    let text = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| e.to_string())
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
async fn open_project(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let folder = app.dialog().file().blocking_pick_folder();
    match folder {
        Some(path) => {
            let path_buf = path.into_path().map_err(|e| e.to_string())?;
            let path_str = path_buf.display().to_string();
            *state.project_root.lock().unwrap() = Some(path_buf);
            state.history.lock().unwrap().clear();
            Ok(path_str)
        }
        None => Err("No folder selected".into()),
    }
}

#[cfg(target_os = "android")]
#[tauri::command]
async fn open_project(_app: tauri::AppHandle, _state: State<'_, AppState>) -> Result<String, String> {
    Err("Folder picker not available on Android.".into())
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
fn list_project_files(state: State<'_, AppState>, path: String) -> Result<Vec<tools::DirEntryInfo>, String> {
    let root = state.project_root.lock().unwrap();
    let root = root.as_ref().ok_or("No project open")?;
    tools::list_dir(root, &path).map_err(|e| e.message())
}

#[cfg(target_os = "android")]
#[tauri::command]
fn list_project_files(_state: State<'_, AppState>, _path: String) -> Result<Vec<tools::DirEntryInfo>, String> {
    Ok(vec![])
}

#[tauri::command]
async fn send_message(
    window: Window,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    message: String,
) -> Result<(), String> {
    let root = state.project_root.lock().unwrap().clone();
    let settings = get_settings(app);
    let history = state.history.lock().unwrap().clone();

    let config = ModelConfig {
        base_url: settings.base_url,
        api_key: settings.api_key,
        model: settings.model,
    };

    let updated_history = agent::run_agent_loop(window, root, config, history, message).await?;
    *state.history.lock().unwrap() = updated_history;
    Ok(())
}

#[tauri::command]
fn new_conversation(state: State<'_, AppState>) {
    state.history.lock().unwrap().clear();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_os::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            open_project,
            list_project_files,
            send_message,
            new_conversation
        ])
        .run(tauri::generate_context!())
        .expect("error while running Frox Code");
                    }
