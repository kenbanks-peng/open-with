mod db;
mod scanner;

use db::Database;
use serde::Serialize;
use std::sync::Mutex;
use std::time::Duration;
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
    scanner::log_line(&format!(
        "reassign_extensions start: exts={exts:?}, target_app_id={target_app_id}"
    ));
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Get the target app's path so we can set the OS-level default
    let apps = db.get_apps(None).map_err(|e| e.to_string())?;
    let target_app = apps
        .iter()
        .find(|a| a.id == target_app_id)
        .ok_or_else(|| format!("app with id {target_app_id} not found"))?;
    // Set OS-level defaults first. macOS may prompt asynchronously, so avoid
    // querying Launch Services again in this command path; it can hang after
    // "Keep Unchanged".
    for ext in &exts {
        scanner::log_line(&format!("setting default handler for .{ext}"));
        scanner::set_default_handler(ext, &target_app.path).map_err(|e| {
            let msg = format!(
                "Failed to set macOS default for .{ext} to {}: {e}",
                target_app.name
            );
            scanner::log_line(&msg);
            msg
        })?;
        scanner::log_line(&format!("default handler request sent for .{ext}"));
    }

    std::thread::spawn(move || {
        scanner::log_line(&format!(
            "delayed verification scheduled: exts={exts:?}, target_app_id={target_app_id}"
        ));
        std::thread::sleep(Duration::from_secs(10));

        let db = match Database::open_or_create() {
            Ok(db) => db,
            Err(e) => {
                scanner::log_line(&format!("delayed refresh: DB open failed: {e}"));
                return;
            }
        };

        for ext in exts {
            let current = scanner::ls_default_app_for_extension(&ext);
            scanner::log_line(&format!(
                "delayed refresh: ext={ext}, current_default={current:?}"
            ));
            let Some((_, app_path)) = current else {
                continue;
            };

            match db.refresh_default_app_by_path(&ext, &app_path) {
                Ok(true) => scanner::log_line(&format!(
                    "delayed refresh: DB updated for .{ext} to {app_path}"
                )),
                Ok(false) => scanner::log_line(&format!(
                    "delayed refresh: no known app for .{ext} path {app_path}"
                )),
                Err(e) => scanner::log_line(&format!(
                    "delayed refresh: DB update failed for .{ext}: {e}"
                )),
            }
        }
    });

    scanner::log_line("reassign_extensions done: verification pending");
    Ok(())
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
    scanner::log_line("app starting");
    std::panic::set_hook(Box::new(|info| {
        scanner::log_line(&format!("panic: {info}"));
    }));

    let db = Database::open_or_create().expect("Failed to open database");

    if let Err(e) = scanner::scan_and_populate(&db) {
        scanner::log_line(&format!("Startup scan failed: {e}"));
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
        .on_window_event(|_, event| {
            scanner::log_line(&format!("window event: {event:?}"));
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                scanner::log_line("close requested; preventing close");
                api.prevent_close();
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_, event| match event {
            tauri::RunEvent::MainEventsCleared => {}
            tauri::RunEvent::ExitRequested { api, code, .. } => {
                scanner::log_line(&format!(
                    "exit requested with code {code:?}; preventing exit"
                ));
                api.prevent_exit();
            }
            event => scanner::log_line(&format!("run event: {event:?}")),
        });
}
