use tauri::{AppHandle, LogicalPosition, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState};

/// 当前鼠标位置（逻辑坐标，左上原点），用于弹窗定位。
pub fn cursor_position() -> Option<(f64, f64)> {
    objc2::rc::autoreleasepool(|_| {
        let p = objc2_app_kit::NSEvent::mouseLocation();
        let marker = objc2::MainThreadMarker::new()?;
        let screen = objc2_app_kit::NSScreen::mainScreen(marker)?;
        let frame = screen.frame();
        let x = p.x - frame.origin.x;
        let y = frame.origin.y + frame.size.height - p.y;
        Some((x, y))
    })
}

pub fn show_popover(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        if let Some((x, y)) = cursor_position() {
            let _ = win.set_position(LogicalPosition::new(x, y));
        }
        let _ = win.show();
        let _ = win.set_focus();
    }
}

pub fn shortcut_handler() -> impl Fn(&AppHandle, &Shortcut, ShortcutEvent) + Send + Sync + 'static {
    |app, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            show_popover(app);
        }
    }
}

/// 注销全部快捷键并按新值注册。
pub fn register_shortcut(app: &AppHandle, shortcut_str: &str) -> Result<(), String> {
    let shortcut: Shortcut = shortcut_str
        .parse()
        .map_err(|e| format!("invalid shortcut {shortcut_str}: {e}"))?;
    let _ = app.global_shortcut().unregister_all();
    app.global_shortcut()
        .on_shortcut(shortcut, shortcut_handler())
        .map_err(|e| format!("register shortcut failed: {e}"))
}
