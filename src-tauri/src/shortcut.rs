use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplicationActivationOptions, NSApplicationActivationPolicy};
use std::sync::atomic::{AtomicI32, Ordering};
use tauri::{AppHandle, LogicalPosition, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState};

/// 唤起弹窗时记录的前台应用 pid，粘贴时用于把焦点还给它。
static LAST_FRONT_APP_PID: AtomicI32 = AtomicI32::new(0);

/// 设为菜单栏应用（不显示 Dock 图标）。
pub fn set_accessory_policy() {
    objc2::rc::autoreleasepool(|_| {
        if let Some(marker) = MainThreadMarker::new() {
            let nsapp = objc2_app_kit::NSApplication::sharedApplication(marker);
            nsapp.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
        }
    });
}

/// 激活应用，使弹出窗口可以接收焦点与键盘输入。
pub fn activate_app() {
    objc2::rc::autoreleasepool(|_| {
        if let Some(marker) = MainThreadMarker::new() {
            let nsapp = objc2_app_kit::NSApplication::sharedApplication(marker);
            nsapp.activate();
        }
    });
}

/// 记录当前前台应用 pid（在快捷键唤起弹窗前调用）。
fn remember_front_app() {
    let pid = objc2::rc::autoreleasepool(|_| {
        let ws = objc2_app_kit::NSWorkspace::sharedWorkspace();
        ws.frontmostApplication()
            .map(|a| a.processIdentifier())
            .unwrap_or(0)
    });
    LAST_FRONT_APP_PID.store(pid, Ordering::SeqCst);
}

/// 把焦点还给唤起前的前台应用。
pub fn activate_front_app() {
    let pid = LAST_FRONT_APP_PID.load(Ordering::SeqCst);
    if pid <= 0 {
        return;
    }
    objc2::rc::autoreleasepool(|_| {
        if let Some(app) =
            objc2_app_kit::NSRunningApplication::runningApplicationWithProcessIdentifier(pid)
        {
            app.activateWithOptions(NSApplicationActivationOptions::ActivateAllWindows);
        }
    });
}

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
    remember_front_app();
    activate_app();
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
