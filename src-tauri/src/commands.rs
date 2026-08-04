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

fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 删除图片条目时同步清理磁盘上的原图与缩略图。
pub fn remove_image_file(data_dir: &std::path::Path, e: &crate::models::Entry) {
    let _ = data_dir; // 保留参数签名，后续可直接从条目路径删除
    for p in [&e.image_path, &e.thumb_path].into_iter().flatten() {
        let _ = std::fs::remove_file(p);
    }
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
    let entry = {
        let store = state.store.lock().map_err(|e| e.to_string())?;
        store.get(id).map_err(|e| e.to_string())?
    };
    if let Some(e) = entry {
        state
            .store
            .lock()
            .map_err(|e| e.to_string())?
            .delete(id)
            .map_err(|e| e.to_string())?;
        remove_image_file(&state.data_dir, &e);
    }
    Ok(())
}

#[tauri::command]
pub fn paste_entry(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    id: i64,
) -> Result<(), String> {
    let e = state
        .store
        .lock()
        .map_err(|e| e.to_string())?
        .get(id)
        .map_err(|e| e.to_string())?
        .ok_or("entry not found")?;
    crate::paste::write_back(&e).map_err(|e| e)?;
    let now = now_ts();
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
    ensure_settings_window(&app)
}

/// 懒创建设置窗口：平时不占内存，关闭后销毁，下次打开再重建。
pub fn ensure_settings_window(app: &tauri::AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("settings") {
        win.show().map_err(|e| e.to_string())?;
        win.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }
    let settings_url = if cfg!(debug_assertions) {
        tauri::WebviewUrl::App("/settings".into())
    } else {
        tauri::WebviewUrl::App("settings.html".into())
    };
    tauri::WebviewWindowBuilder::new(app, "settings", settings_url)
        .title("ClipboardX 设置")
        .inner_size(560.0, 640.0)
        .resizable(false)
        .build()
        .map_err(|e| e.to_string())?;
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
pub fn clear_history(state: tauri::State<'_, AppState>) -> Result<usize, String> {
    state
        .store
        .lock()
        .map_err(|e| e.to_string())?
        .delete_non_pinned()
        .map(|entries| entries.len())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_autostart(app: tauri::AppHandle) -> Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_autostart(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let autostart = app.autolaunch();
    if enabled {
        autostart.enable().map_err(|e| e.to_string())
    } else {
        autostart.disable().map_err(|e| e.to_string())
    }
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
