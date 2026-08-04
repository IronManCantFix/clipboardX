mod clipboard;
mod commands;
mod listener;
mod models;
mod paste;
mod settings;
mod shortcut;
mod store;

use tauri::Manager;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            shortcut::set_accessory_policy();
            // 主窗口原生失焦隐藏（macOS 上最可靠的隐藏触发）
            if let Some(main_win) = app.get_webview_window("main") {
                let hide_win = main_win.clone();
                main_win.on_window_event(move |event| {
                    if let tauri::WindowEvent::Focused(false) = event {
                        let _ = hide_win.hide();
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

            // 全局快捷键：唤起弹窗（默认 Command+Shift+V）
            let shortcut = Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyV);
            app.global_shortcut()
                .on_shortcut(shortcut, shortcut::shortcut_handler())
                .map_err(|e| format!("register shortcut failed: {e}"))?;

            // 启动剪贴板监听线程
            listener::spawn_listener(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_entries,
            commands::set_pinned,
            commands::delete_entry,
            commands::paste_entry,
            commands::get_settings,
            commands::update_settings,
            commands::show_settings,
            commands::get_storage_stats,
            commands::clear_images,
            commands::clear_history,
            commands::get_autostart,
            commands::set_autostart,
            shortcut::position_popover,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");
    app.run(|_, _| {});
}
