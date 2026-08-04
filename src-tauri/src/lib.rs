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
        .plugin(tauri_plugin_positioner::init())
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

            // 设置窗口（隐藏，由托盘/齿轮打开）。dev 下 SvelteKit 路由为 /settings，
            // 生产构建产物为 settings.html
            let settings_url = if cfg!(debug_assertions) {
                tauri::WebviewUrl::App("/settings".into())
            } else {
                tauri::WebviewUrl::App("settings.html".into())
            };
            let settings_win = tauri::WebviewWindowBuilder::new(app, "settings", settings_url)
                .title("ClipboardX 设置")
                .inner_size(430.0, 560.0)
                .resizable(false)
                .visible(false)
                .build()?;
            let settings_win_ref = settings_win.clone();
            settings_win.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = settings_win_ref.hide();
                }
            });

            // 托盘：左键/菜单弹出设置窗口
            {
                use tauri::menu::{Menu, MenuItem};
                use tauri::tray::TrayIconBuilder;
                let show_settings =
                    MenuItem::with_id(app, "settings", "设置…", true, None::<&str>)?;
                let quit =
                    MenuItem::with_id(app, "quit", "退出 ClipboardX", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&show_settings, &quit])?;
                let icon = app.default_window_icon().cloned();
                let mut builder = TrayIconBuilder::with_id("main-tray")
                    .menu(&menu)
                    .show_menu_on_left_click(true)
                    .on_menu_event(|app, event| match event.id().as_ref() {
                        "settings" => {
                            if let Some(win) = app.get_webview_window("settings") {
                                let _ = win.show();
                                let _ = win.set_focus();
                            }
                        }
                        "quit" => app.exit(0),
                        _ => {}
                    });
                if let Some(icon) = icon {
                    builder = builder.icon(icon);
                }
                builder.build(app)?;
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
            commands::search_entries,
            commands::set_pinned,
            commands::delete_entry,
            commands::paste_entry,
            commands::get_settings,
            commands::update_settings,
            commands::show_settings,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");
    app.run(|_, _| {});
}
