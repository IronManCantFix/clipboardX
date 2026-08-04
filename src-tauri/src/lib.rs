mod clipboard;
mod commands;
mod listener;
mod models;
mod paste;
mod settings;
mod store;

use tauri::Manager;

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
            let state = commands::AppState {
                store: std::sync::Mutex::new(store),
                settings: std::sync::Mutex::new(settings),
            };
            app.manage(state);
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
