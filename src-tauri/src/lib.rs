mod clipboard;
mod commands;
mod listener;
mod logging;
mod models;
mod paste;
mod settings;
mod shortcut;
mod store;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 尽早安装 panic 钩子：任何 panic（含后台线程）都会先写入日志再退出/恢复
    logging::install_panic_hook();
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
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
            // 日志文件：<app_data_dir>/clipboardx.log（macOS 上即
            // ~/Library/Application Support/com.clipboardx.app/clipboardx.log）
            logging::init(data_dir.join("clipboardx.log"));
            logging::info(&format!(
                "=== ClipboardX {} starting (pid={}, data_dir={}) ===",
                env!("CARGO_PKG_VERSION"),
                std::process::id(),
                data_dir.display()
            ));
            let store = store::Store::open(&data_dir.join("entries.db"))
                .map_err(|e| format!("db open failed: {e}"))?;
            // 大库（如长期积累的富文本/长文本历史）在后台线程 VACUUM 一次，回收被 prune 释放的空间
            store.vacuum_if_large(32 * 1024 * 1024);
            let settings = settings::SettingsStore::load(&data_dir.join("settings.json"))
                .map_err(|e| format!("settings load failed: {e}"))?;
            let state = commands::AppState {
                store: std::sync::Mutex::new(store),
                settings: std::sync::Mutex::new(settings),
                data_dir: data_dir.clone(),
            };
            app.manage(state);

            // 设置窗口懒加载：首次打开时创建，关闭即销毁（不常驻 WebView 进程）

            // 托盘：左键/菜单弹出设置窗口
            {
                use tauri::menu::{Menu, MenuItem};
                use tauri::tray::TrayIconBuilder;
                let show_settings =
                    MenuItem::with_id(app, "settings", "设置…", true, None::<&str>)?;
                let quit = MenuItem::with_id(app, "quit", "退出 ClipboardX", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&show_settings, &quit])?;
                // 菜单栏图标：显式使用与应用图标一致的 32x32 彩色图标
                let tray_icon =
                    tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png"))
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
                let msg = "辅助功能权限未授予：模拟粘贴不可用，请在系统设置中授权后重启应用";
                println!("[clipboardx] {msg}");
                logging::info(msg);
            }

            // 全局快捷键：从设置读取（默认 Command+Shift+V），重启后保持用户自定义值。
            // 注册失败（如与其它应用冲突）不阻断启动：记录日志，用户可在设置中更换。
            let shortcut_str = commands::lock_settings(&app.state::<crate::commands::AppState>())
                .settings
                .shortcut
                .clone();
            if let Err(e) = shortcut::register_shortcut(app.handle(), &shortcut_str) {
                eprintln!("[clipboardx] 快捷键注册失败（可在设置中更换）: {e}");
                logging::error(&format!("shortcut register failed: {e}"));
            }

            // 清理历史遗留的孤儿图片文件（未被数据库引用的）
            {
                let app_state = app.state::<crate::commands::AppState>();
                let store = commands::lock_store(&app_state);
                listener::cleanup_orphan_images(&data_dir, &store);
            }

            // 启动剪贴板监听线程
            listener::spawn_listener(app.handle().clone());

            // 心跳日志：每 10 分钟一条，用于确定「最后一次正常运行」的时间点，
            // 配合退出/panic 日志即可定位崩溃发生的时段。
            std::thread::spawn(|| loop {
                std::thread::sleep(std::time::Duration::from_secs(600));
                logging::info("heartbeat: app alive");
            });
            logging::info("setup completed; listener spawned");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_entries,
            commands::set_pinned,
            commands::delete_entry,
            commands::paste_entry,
            commands::get_settings,
            commands::update_settings,
            commands::get_storage_stats,
            commands::clear_images,
            commands::clear_history,
            commands::get_autostart,
            commands::set_autostart,
            commands::check_update,
            commands::install_update,
            shortcut::position_popover,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");
    // 生命周期日志：记录每一次退出请求与窗口销毁，区分「用户主动退出」与「意外退出」。
    // ExitRequested code=None 表示用户交互触发（如最后窗口关闭/菜单退出），
    // code=Some(n) 表示程序内调用（托盘退出 app.exit(0)、更新后 app.restart()）。
    app.run(|_app, event| match event {
        tauri::RunEvent::ExitRequested { code, .. } => {
            logging::info(&format!("RunEvent::ExitRequested (code={code:?})"));
        }
        tauri::RunEvent::Exit => {
            logging::info("RunEvent::Exit — app exiting");
        }
        tauri::RunEvent::WindowEvent { label, event, .. } => match event {
            tauri::WindowEvent::CloseRequested { .. } => {
                logging::info(&format!("window '{label}' close requested"));
            }
            tauri::WindowEvent::Destroyed => {
                logging::info(&format!("window '{label}' destroyed"));
            }
            _ => {}
        },
        tauri::RunEvent::Reopen { .. } => {
            logging::info("RunEvent::Reopen (macOS reopen)");
        }
        _ => {}
    });
}
