use crate::models::{EntryType, Settings};
use serde::Serialize;
use crate::settings::SettingsStore;
use crate::store::Store;
use std::sync::Mutex;
use tauri::Manager;

pub struct AppState {
    pub store: Mutex<Store>,
    pub settings: Mutex<SettingsStore>,
    pub data_dir: std::path::PathBuf,
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
pub fn paste_entry(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    id: i64,
) -> Result<(), String> {
    let list = state
        .store
        .lock()
        .map_err(|e| e.to_string())?
        .list()
        .map_err(|e| e.to_string())?;
    let e = list.into_iter().find(|x| x.id == id).ok_or("entry not found")?;
    crate::paste::write_back(&e).map_err(|e| e)?;
    let now = chrono::Utc::now().timestamp();
    state
        .store
        .lock()
        .map_err(|e| e.to_string())?
        .touch(id, now)
        .map_err(|e| e.to_string())?;
    // 隐藏弹窗，把焦点还给唤起前的前台应用，再模拟 Cmd+V
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
    crate::shortcut::activate_front_app();
    std::thread::sleep(std::time::Duration::from_millis(150));
    crate::paste::send_cmd_v()
}

#[tauri::command]
pub fn show_settings(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("settings") {
        win.show().map_err(|e| e.to_string())?;
        win.set_focus().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[derive(Serialize)]
pub struct StorageStats {
    pub db_size: u64,
    pub images_dir: String,
    pub images_size: u64,
    pub image_count: i64,
    pub entry_count: i64,
}

fn dir_size(dir: &std::path::Path) -> u64 {
    let mut total = 0;
    if let Ok(rd) = std::fs::read_dir(dir) {
        for entry in rd.flatten() {
            if let Ok(md) = entry.metadata() {
                if md.is_file() {
                    total += md.len();
                }
            }
        }
    }
    total
}

#[tauri::command]
pub fn get_storage_stats(state: tauri::State<'_, AppState>) -> Result<StorageStats, String> {
    let data_dir = &state.data_dir;
    let db_size = std::fs::metadata(data_dir.join("entries.db"))
        .map(|m| m.len())
        .unwrap_or(0);
    let images_dir = data_dir.join("images");
    let images_size = dir_size(&images_dir);
    let image_count = std::fs::read_dir(&images_dir).map(|rd| rd.count() as i64).unwrap_or(0);
    let entry_count = state
        .store
        .lock()
        .map_err(|e| e.to_string())?
        .count()
        .map_err(|e| e.to_string())?;
    Ok(StorageStats {
        db_size,
        images_dir: images_dir.to_string_lossy().into_owned(),
        images_size,
        image_count,
        entry_count,
    })
}

#[tauri::command]
pub fn clear_images(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let images_dir = state.data_dir.join("images");
    if images_dir.exists() {
        for entry in std::fs::read_dir(&images_dir).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            let _ = std::fs::remove_file(path);
        }
    }
    state
        .store
        .lock()
        .map_err(|e| e.to_string())?
        .delete_by_type(EntryType::Image)
        .map_err(|e| e.to_string())?;
    Ok(())
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
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    settings: Settings,
) -> Result<(), String> {
    let old_shortcut = state.settings.lock().map_err(|e| e.to_string())?.settings.shortcut.clone();
    {
        let mut guard = state.settings.lock().map_err(|e| e.to_string())?;
        guard.settings = settings;
        guard.save().map_err(|e| e.to_string())?;
    }
    let new_shortcut = state.settings.lock().map_err(|e| e.to_string())?.settings.shortcut.clone();
    if new_shortcut != old_shortcut {
        crate::shortcut::register_shortcut(&app, &new_shortcut)?;
    }
    Ok(())
}
