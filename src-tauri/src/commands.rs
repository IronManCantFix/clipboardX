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
    std::thread::sleep(std::time::Duration::from_millis(200));
    // 文件条目不模拟 Cmd+V：文件写入剪贴板后由用户在目标位置手动粘贴
    if e.kind == crate::models::EntryType::File {
        return Ok(());
    }
    // 模拟 Cmd+V 自动粘贴；权限不足时静默失败（内容已写入剪贴板，用户可手动 Cmd+V）
    crate::paste::send_cmd_v();
    Ok(())
}

/// 查询当前辅助功能权限状态（前端用于展示引导）。
#[tauri::command]
pub fn get_accessibility_status() -> bool {
    crate::shortcut::ax_is_trusted()
}

#[tauri::command]
pub fn show_settings(app: tauri::AppHandle) -> Result<(), String> {
    ensure_settings_window(&app)
}

/// 懒创建设置窗口：平时不占内存，关闭隐藏而非销毁，下次打开瞬间生效。
pub fn ensure_settings_window(app: &tauri::AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("settings") {
        let w = win;
        // 推迟到下一帧：macOS accessory app 在菜单回调里无法同期 set_focus
        let _ = app.run_on_main_thread(Box::new(move || {
            let _ = w.show();
            let _ = w.set_focus();
        }));
        return Ok(());
    }
    let settings_url = if cfg!(debug_assertions) {
        tauri::WebviewUrl::App("/settings".into())
    } else {
        tauri::WebviewUrl::App("settings.html".into())
    };
    let win = tauri::WebviewWindowBuilder::new(app, "settings", settings_url)
        .title("ClipboardX 设置")
        .inner_size(560.0, 640.0)
        .resizable(false)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?;
    // 关闭时隐藏而非销毁，避免下次冷启动 WebView
    let hide_win = win.clone();
    win.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = hide_win.hide();
        }
    });
    // WebView 冷启动需要一个 runloop cycle，推到下一帧再显示
    let w = win;
    let _ = app.run_on_main_thread(Box::new(move || {
        let _ = w.show();
        let _ = w.set_focus();
    }));
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

#[derive(Serialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub has_update: bool,
    pub message: String,
    pub url: Option<String>,
}

fn parse_version(v: &str) -> Option<(u64, u64, u64)> {
    let parts: Vec<&str> = v.split('.').take(3).collect();
    if parts.len() < 3 {
        return None;
    }
    let num = |s: &str| {
        s.chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse::<u64>()
            .ok()
    };
    Some((num(parts[0])?, num(parts[1])?, num(parts[2])?))
}

fn version_gt(a: &str, b: &str) -> bool {
    match (parse_version(a), parse_version(b)) {
        (Some(x), Some(y)) => x > y,
        _ => a != b,
    }
}

/// 检查 GitHub 最新 release 版本（无需签名密钥；点击设置里的「检查更新」时触发）。
#[tauri::command]
pub fn check_for_updates() -> Result<UpdateInfo, String> {
    let current = env!("CARGO_PKG_VERSION").to_string();
    let out = std::process::Command::new("curl")
        .args([
            "-fsSL",
            "--max-time",
            "10",
            "-H",
            "User-Agent: ClipboardX",
            "https://api.github.com/repos/IronManCantFix/clipboardX/releases/latest",
        ])
        .output()
        .map_err(|e| format!("无法连接 GitHub：{e}"))?;
    if !out.status.success() {
        // 尚未发布或网络异常时给出友好提示，不当作错误弹出
        return Ok(UpdateInfo {
            current_version: current,
            latest_version: None,
            has_update: false,
            message: "尚未发布版本或网络不可用".into(),
            url: None,
        });
    }
    let body = String::from_utf8_lossy(&out.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("解析 GitHub 响应失败：{e}"))?;
    let latest = json
        .get("tag_name")
        .and_then(|v| v.as_str())
        .map(|s| s.trim_start_matches('v').to_string());
    let Some(latest) = latest else {
        return Ok(UpdateInfo {
            current_version: current,
            latest_version: None,
            has_update: false,
            message: "暂未获取到版本信息".into(),
            url: None,
        });
    };
    let has_update = version_gt(&latest, &current);
    Ok(UpdateInfo {
        current_version: current,
        latest_version: Some(latest.clone()),
        has_update,
        message: if has_update {
            format!("发现新版本 {latest}")
        } else {
            "已是最新版本".into()
        },
        url: if has_update {
            Some(format!("https://github.com/IronManCantFix/clipboardX/releases/tag/{}", latest))
        } else {
            None
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_comparison() {
        assert!(version_gt("0.1.1", "0.1.0"));
        assert!(version_gt("0.10.0", "0.9.9"));
        assert!(version_gt("1.0.0", "0.99.99"));
        assert!(!version_gt("0.1.0", "0.1.0"));
        assert!(!version_gt("0.1.0", "0.1.1"));
        assert!(!version_gt("0.9.9", "0.10.0"));
    }

    #[test]
    fn version_parsing() {
        assert_eq!(parse_version("1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_version("1.2"), None);
        assert_eq!(parse_version("1.2.3-beta"), Some((1, 2, 3)));
        assert_eq!(parse_version(""), None);
    }
}
