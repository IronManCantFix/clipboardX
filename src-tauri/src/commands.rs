use crate::models::Settings;
use crate::settings::SettingsStore;
use crate::store::Store;
use std::sync::Mutex;

pub struct AppState {
    pub store: Mutex<Store>,
    pub settings: Mutex<SettingsStore>,
}

#[tauri::command]
pub fn list_entries(
    state: tauri::State<'_, AppState>,
    tab: Option<String>,
) -> Result<Vec<crate::models::Entry>, String> {
    let store = state.store.lock().map_err(|e| e.to_string())?;
    match tab.as_deref() {
        Some("pinned") => store.list_pinned().map_err(|e| e.to_string()),
        _ => store.list().map_err(|e| e.to_string()),
    }
}

#[tauri::command]
pub fn search_entries(
    state: tauri::State<'_, AppState>,
    query: String,
) -> Result<Vec<crate::models::Entry>, String> {
    let store = state.store.lock().map_err(|e| e.to_string())?;
    store.search(&query).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_pinned(
    state: tauri::State<'_, AppState>,
    id: i64,
    pinned: bool,
) -> Result<(), String> {
    state
        .store
        .lock()
        .map_err(|e| e.to_string())?
        .set_pinned(id, pinned)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_entry(state: tauri::State<'_, AppState>, id: i64) -> Result<(), String> {
    state
        .store
        .lock()
        .map_err(|e| e.to_string())?
        .delete(id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn paste_entry(state: tauri::State<'_, AppState>, id: i64) -> Result<(), String> {
    let list = state
        .store
        .lock()
        .map_err(|e| e.to_string())?
        .list()
        .map_err(|e| e.to_string())?;
    let e = list.into_iter().find(|x| x.id == id).ok_or("entry not found")?;
    crate::paste::write_back(&e);
    let now = chrono::Utc::now().timestamp();
    state
        .store
        .lock()
        .map_err(|e| e.to_string())?
        .touch(id, now)
        .map_err(|e| e.to_string())?;
    crate::paste::send_cmd_v()
}

#[tauri::command]
pub fn get_settings(state: tauri::State<'_, AppState>) -> Result<Settings, String> {
    Ok(state
        .settings
        .lock()
        .map_err(|e| e.to_string())?
        .settings
        .clone())
}

#[tauri::command]
pub fn update_settings(
    state: tauri::State<'_, AppState>,
    settings: Settings,
) -> Result<(), String> {
    let mut guard = state.settings.lock().map_err(|e| e.to_string())?;
    guard.settings = settings;
    guard.save().map_err(|e| e.to_string())
}
