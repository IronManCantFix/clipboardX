use crate::models::Settings;
use crate::settings::SettingsStore;
use crate::store::Store;
use serde::Serialize;
use std::sync::Mutex;
use tauri::{Emitter, Manager};

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
pub fn remove_image_file(e: &crate::models::Entry) {
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
pub fn set_pinned(state: tauri::State<'_, AppState>, id: i64, pinned: bool) -> Result<(), String> {
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
        remove_image_file(&e);
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
    crate::paste::write_back(&e)?;
    let now = now_ts();
    state
        .store
        .lock()
        .map_err(|e| e.to_string())?
        .touch(id, now)
        .map_err(|e| e.to_string())?;
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
    crate::shortcut::activate_front_app();
    std::thread::sleep(std::time::Duration::from_millis(120));
    // 辅助功能权限检测：未授权时弹窗引导（仅首次），但不阻止粘贴——
    // 未签名应用的 AXIsProcessTrusted() 可能误判，直接发 CGEvent 让有权限时正常工作
    let _ = crate::shortcut::ax_check_and_prompt();
    crate::paste::send_cmd_v();
    Ok(())
}

/// 懒创建设置窗口：平时不占内存（WebView 进程不常驻），关闭即销毁，下次打开时重建
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
    // 关闭即销毁：设置窗口低频使用，冷启动代价可接受，换来平时不常驻约 29MB 内存
    // （销毁后下次打开由 ensure_settings_window 重建）
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
    let image_count = std::fs::read_dir(&images_dir)
        .map(|rd| rd.count() as i64)
        .unwrap_or(0);
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
    // 仅删除非固定图片条目，固定图片保留（与「固定项永不被清理」的承诺一致）
    let victims = {
        let store = state.store.lock().map_err(|e| e.to_string())?;
        store
            .delete_images_non_pinned()
            .map_err(|e| e.to_string())?
    };
    for e in &victims {
        remove_image_file(e);
    }
    // 清理磁盘上未被任何条目引用的孤儿图片文件（固定条目引用的文件保留）
    let data_dir = state.data_dir.clone();
    let store = state.store.lock().map_err(|e| e.to_string())?;
    crate::listener::cleanup_orphan_images(&data_dir, &store);
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
    let old_shortcut = state
        .settings
        .lock()
        .map_err(|e| e.to_string())?
        .settings
        .shortcut
        .clone();
    let new_shortcut = settings.shortcut.clone();
    // 先替换快捷键：新键注册成功后才注销旧键；失败则整体失败（设置不落盘、旧键保留）
    if new_shortcut != old_shortcut {
        crate::shortcut::replace_shortcut(&app, &new_shortcut, &old_shortcut)?;
    }
    {
        let mut guard = state.settings.lock().map_err(|e| e.to_string())?;
        guard.settings = settings;
        if let Err(e) = guard.save() {
            // 设置写盘失败：把快捷键回滚到旧值，避免两侧状态不一致
            let _ =
                crate::shortcut::replace_shortcut(&app, &old_shortcut, &guard.settings.shortcut);
            return Err(e.to_string());
        }
    }
    Ok(())
}

#[derive(Serialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub has_update: bool,
}

/// 检查是否有可用更新（tauri-plugin-updater，从 tauri.conf.json 的 endpoints 拉取清单，
/// 用内置公钥校验签名）。不下载、不安装，由 install_update 完成实际更新。
#[tauri::command]
pub async fn check_update(app: tauri::AppHandle) -> Result<UpdateInfo, String> {
    use tauri_plugin_updater::UpdaterExt;
    let current = env!("CARGO_PKG_VERSION").to_string();
    let updater = app.updater_builder().build().map_err(|e| e.to_string())?;
    match updater.check().await.map_err(|e| e.to_string())? {
        Some(u) => Ok(UpdateInfo {
            current_version: current,
            latest_version: Some(u.version.clone()),
            has_update: true,
        }),
        None => Ok(UpdateInfo {
            current_version: current,
            latest_version: None,
            has_update: false,
        }),
    }
}

/// 下载并安装新版本（校验签名后替换应用），安装成功后自动重启。
/// 下载过程中通过 "update:progress" 事件向前端回报进度（{downloaded, total}）。
#[tauri::command]
pub async fn install_update(app: tauri::AppHandle) -> Result<String, String> {
    use tauri_plugin_updater::UpdaterExt;
    let updater = app.updater_builder().build().map_err(|e| e.to_string())?;
    let Some(update) = updater.check().await.map_err(|e| e.to_string())? else {
        return Ok("已是最新版本".into());
    };
    let emitter = app.clone();
    update
        .download_and_install(
            move |downloaded, total| {
                let _ = emitter.emit(
                    "update:progress",
                    serde_json::json!({ "downloaded": downloaded, "total": total }),
                );
            },
            || {},
        )
        .await
        .map_err(|e| e.to_string())?;
    // 安装完成，重启使新版本生效（restart 不返回，会直接退出当前进程）
    app.restart()
}
