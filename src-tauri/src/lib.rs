mod clipboard;
mod commands;
mod listener;
mod models;
mod paste;
mod settings;
mod store;

use objc2::MainThreadMarker;
use tauri::{LogicalPosition, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

/// 当前鼠标位置（逻辑坐标，左上原点），用于弹窗定位。
fn cursor_position() -> Option<(f64, f64)> {
    objc2::rc::autoreleasepool(|_| {
        let p = objc2_app_kit::NSEvent::mouseLocation();
        let marker = MainThreadMarker::new()?;
        let screen = objc2_app_kit::NSScreen::mainScreen(marker)?;
        let frame = screen.frame();
        let x = p.x - frame.origin.x;
        let y = frame.origin.y + frame.size.height - p.y;
        Some((x, y))
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_positioner::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let store = store::Store::open(&data_dir.join("entries.db"))
                .map_err(|e| format!("db open failed: {e}"))?;
            let settings = settings::SettingsStore::load(&data_dir.join("settings.json"))
                .map_err(|e| format!("settings load failed: {e}"))?;
            let state = std::sync::Arc::new(commands::AppState {
                store: std::sync::Mutex::new(store),
                settings: std::sync::Mutex::new(settings),
            });
            app.manage(state.clone());

            // 全局快捷键：唤起弹窗（默认 Command+Shift+V）
            let shortcut = Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyV);
            app.global_shortcut()
                .on_shortcut(shortcut, move |app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        if let Some(win) = app.get_webview_window("main") {
                            if let Some((x, y)) = cursor_position() {
                                let _ = win.set_position(LogicalPosition::new(x, y));
                            }
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                })
                .map_err(|e| format!("register shortcut failed: {e}"))?;

            // 启动剪贴板监听线程
            listener::spawn_listener(app.handle().clone(), state);
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
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");
    app.run(|_, _| {});
}
