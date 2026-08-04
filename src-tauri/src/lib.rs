mod clipboard;
mod commands;
mod listener;
mod models;
mod paste;
mod settings;
mod shortcut;
mod store;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            shortcut::set_accessory_policy();
            // 主窗口原生失焦隐藏（macOS 上最可靠的隐藏触发）
            if let Some(main_win) = app.get_webview_window("main") {
                let hide_win = main_win.clone();
                main_win.on_window_event(move |event| {
                    if let tauri::WindowEvent::Focused(false) = event {
                        // 唤起保护期内忽略失焦，避免 show 后聚焦未稳定时被误隐藏
                        if !crate::shortcut::just_shown() {
                            let _ = hide_win.hide();
                        }
                    }
                });
            }
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let store = store::Store::open(&data_dir.join("entries.db"))
                .map_err(|e| format!("db open failed: {e}"))?;
            let settings = settings::SettingsStore::load(&data_dir.join("settings.json"))
                .map_err(|e| format!("settings load failed: {e}"))?;
            let state = commands::AppState {
                store: std::sync::Mutex::new(store),
                settings: std::sync::Mutex::new(settings),
                data_dir: data_dir.clone(),
            };
            app.manage(state);

            // 设置窗口不在此创建：首次打开时懒加载，关闭即销毁，减少常驻内存

            // 托盘：左键/菜单弹出设置窗口
            {
                use tauri::menu::{Menu, MenuItem};
                use tauri::tray::TrayIconBuilder;
                let show_settings =
                    MenuItem::with_id(app, "settings", "设置…", true, None::<&str>)?;
                let quit =
                    MenuItem::with_id(app, "quit", "退出 ClipboardX", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&show_settings, &quit])?;
                // 菜单栏图标：显式使用与应用图标一致的 32x32 彩色图标
                let tray_icon = tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png"))
                    .map_err(|e| format!("tray icon load failed: {e}"))?;
                TrayIconBuilder::with_id("main-tray")
                    .icon(tray_icon)
                    .menu(&menu)
                    .show_menu_on_left_click(true)
                    .on_menu_event(|app, event| match event.id().as_ref() {
                        "settings" => {
                            let _ = commands::ensure_settings_window(app);
                        }
                        "quit" => app.exit(0),
                        _ => {}
                    })
                    .build(app)?;
            }

            // 辅助功能权限检测（仅记录，不自动弹窗打扰用户；粘贴失败时前端会提示）
            if !shortcut::ax_is_trusted() {
                println!("[clipboardx] 辅助功能权限未授予：模拟粘贴不可用，请在系统设置中授权后重启应用");
            }

            // 全局快捷键：从设置读取（默认 Command+Shift+V），重启后保持用户自定义值
            let shortcut_str = app
                .state::<crate::commands::AppState>()
                .settings
                .lock()
                .unwrap()
                .settings
                .shortcut
                .clone();
            shortcut::register_shortcut(app.handle(), &shortcut_str)
                .map_err(|e| format!("register shortcut failed: {e}"))?;

            // 清理历史遗留的孤儿图片文件（未被数据库引用的）
            {
                let app_state = app.state::<crate::commands::AppState>();
                let store = app_state.store.lock().unwrap();
                listener::cleanup_orphan_images(&data_dir, &store);
            }

            // 启动剪贴板监听线程
            listener::spawn_listener(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_entries,
            commands::set_pinned,
            commands::delete_entry,
            commands::paste_entry,
            commands::get_accessibility_status,
            commands::get_settings,
            commands::update_settings,
            commands::show_settings,
            commands::get_storage_stats,
            commands::clear_images,
            commands::clear_history,
            commands::get_autostart,
            commands::set_autostart,
            commands::check_for_updates,
            shortcut::position_popover,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");
    app.run(|_, _| {});
}
