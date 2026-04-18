mod db;
mod scanner;

use db::Database;
use serde::Serialize;
use std::sync::Mutex;
use tauri::State;

struct AppState {
    db: Mutex<Database>,
}

#[derive(Debug, Serialize, Clone)]
pub struct App {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub ext_count: i64,
}

#[derive(Debug, Serialize, Clone)]
pub struct Extension {
    pub ext: String,
    pub description: String,
    pub default_app_id: Option<i64>,
    pub default_app_name: Option<String>,
}

#[tauri::command]
fn get_apps(state: State<AppState>, filter: Option<String>) -> Result<Vec<App>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_apps(filter.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_extensions_for_app(
    state: State<AppState>,
    app_id: Option<i64>,
) -> Result<Vec<Extension>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_extensions_for_app(app_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_candidate_targets(state: State<AppState>, source_app_id: i64) -> Result<Vec<App>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_candidate_targets(source_app_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_eligible_extensions(
    state: State<AppState>,
    source_app_id: i64,
    target_app_id: i64,
) -> Result<Vec<String>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_eligible_extensions(source_app_id, target_app_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn reassign_extensions(
    state: State<AppState>,
    exts: Vec<String>,
    target_app_id: i64,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Get the target app's path so we can set the OS-level default
    let apps = db.get_apps(None).map_err(|e| e.to_string())?;
    let target_app = apps
        .iter()
        .find(|a| a.id == target_app_id)
        .ok_or_else(|| format!("app with id {target_app_id} not found"))?;

    // Set OS-level default for each extension
    for ext in &exts {
        if let Err(e) = scanner::set_default_handler(ext, &target_app.path) {
            eprintln!("Failed to set OS default for .{ext}: {e}");
        }
    }

    db.reassign_extensions(&exts, target_app_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_apps_for_extension(state: State<AppState>, ext: String) -> Result<Vec<App>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_apps_for_extension(&ext).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_apps_for_extensions(
    state: State<AppState>,
    exts: Vec<String>,
    exclude_app_id: Option<i64>,
) -> Result<Vec<App>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_apps_for_extensions(&exts, exclude_app_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_extension_target_counts(
    state: State<AppState>,
    source_app_id: i64,
) -> Result<Vec<(String, i64)>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_extension_target_counts(source_app_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_summary(state: State<AppState>) -> Result<(i64, i64), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_summary().map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db = Database::open_or_create().expect("Failed to open database");

    if let Err(e) = scanner::scan_and_populate(&db) {
        eprintln!("Startup scan failed: {e}");
    }

    tauri::Builder::default()
        .manage(AppState { db: Mutex::new(db) })
        .invoke_handler(tauri::generate_handler![
            get_apps,
            get_extensions_for_app,
            get_candidate_targets,
            get_eligible_extensions,
            reassign_extensions,
            get_apps_for_extension,
            get_apps_for_extensions,
            get_extension_target_counts,
            get_summary,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
