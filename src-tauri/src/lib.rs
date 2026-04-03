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
    pub group_id: Option<i64>,
    pub description: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct Group {
    pub id: i64,
    pub name: String,
    pub assigned_app_id: Option<i64>,
    pub assigned_app_name: Option<String>,
    pub ext_count: i64,
}

#[derive(Debug, Serialize, Clone)]
pub struct GroupDetail {
    pub group: Group,
    pub extensions: Vec<Extension>,
    pub common_apps: Vec<App>,
}

#[tauri::command]
fn get_apps(state: State<AppState>, filter: Option<String>) -> Result<Vec<App>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_apps(filter.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_groups(state: State<AppState>, app_filter_id: Option<i64>, assigned_only: bool) -> Result<Vec<Group>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_groups(app_filter_id, assigned_only).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_group_detail(state: State<AppState>, group_id: Option<i64>) -> Result<GroupDetail, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_group_detail(group_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn validate_move(
    state: State<AppState>,
    exts: Vec<String>,
    target_group_id: i64,
) -> Result<bool, String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    db.validate_move(&exts, target_group_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn move_extensions(
    state: State<AppState>,
    exts: Vec<String>,
    target_group_id: Option<i64>,
) -> Result<(), String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    db.move_extensions(&exts, target_group_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn create_group(state: State<AppState>, name: String) -> Result<Group, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.create_group(&name).map_err(|e| e.to_string())
}

#[tauri::command]
fn rename_group(state: State<AppState>, group_id: i64, name: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.rename_group(group_id, &name).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_group(state: State<AppState>, group_id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_group(group_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn assign_app_to_group(
    state: State<AppState>,
    group_id: i64,
    app_id: Option<i64>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.assign_app_to_group(group_id, app_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_apps_for_extension(state: State<AppState>, ext: String) -> Result<Vec<App>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_apps_for_extension(&ext).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_common_apps_for_app(state: State<AppState>, app_id: i64) -> Result<Vec<App>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_common_apps_for_app(app_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn breakout_group(state: State<AppState>, group_id: i64) -> Result<usize, String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    db.breakout_group(group_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_summary(state: State<AppState>) -> Result<(i64, i64, i64), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_summary().map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut db = Database::open_or_create().expect("Failed to open database");

    // Reconcile on every startup: rescan app plists and cluster any ungrouped extensions
    if let Err(e) = scanner::scan_and_populate(&db) {
        eprintln!("Startup scan failed: {e}");
    }
    match db.auto_cluster_ungrouped() {
        Ok(0) => {}
        Ok(n) => eprintln!("Auto-created {n} groups from ungrouped extensions"),
        Err(e) => eprintln!("Auto-clustering failed: {e}"),
    }
    match db.reconcile_group_assignments() {
        Ok(0) => {}
        Ok(n) => eprintln!("Reconciled {n} group assignments"),
        Err(e) => eprintln!("Group assignment reconciliation failed: {e}"),
    }

    tauri::Builder::default()
        .manage(AppState { db: Mutex::new(db) })
        .invoke_handler(tauri::generate_handler![
            get_apps,
            get_groups,
            get_group_detail,
            validate_move,
            move_extensions,
            create_group,
            rename_group,
            delete_group,
            assign_app_to_group,
            get_apps_for_extension,
            get_common_apps_for_app,
            breakout_group,
            get_summary,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
