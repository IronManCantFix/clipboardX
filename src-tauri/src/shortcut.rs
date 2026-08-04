use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplicationActivationOptions, NSApplicationActivationPolicy};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, LogicalPosition, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState};

/// 唤起弹窗时记录的前台应用 pid，粘贴时用于把焦点还给它。
static LAST_FRONT_APP_PID: AtomicI32 = AtomicI32::new(0);

/// 唤起时记录的鼠标位置，用于弹窗避让定位。
static LAST_CURSOR: Mutex<Option<(f64, f64)>> = Mutex::new(None);

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
    let mode = app
        .state::<crate::commands::AppState>()
        .settings
        .lock()
        .unwrap()
        .settings
        .position_mode
        .clone();
    if let Some(cursor) = cursor_position() {
        *LAST_CURSOR.lock().unwrap() = Some(cursor);
    }
    if let Some(win) = app.get_webview_window("main") {
        if mode == "center" {
            let _ = win.center();
        } else {
            let _ = position_window(&win, None);
        }
        let _ = win.show();
        let _ = win.set_focus();
    }
}

/// 主屏幕逻辑坐标边界 (x, y, w, h)。
fn main_screen_bounds() -> Option<(f64, f64, f64, f64)> {
    objc2::rc::autoreleasepool(|_| {
        let marker = MainThreadMarker::new()?;
        let screen = objc2_app_kit::NSScreen::mainScreen(marker)?;
        let f = screen.frame();
        Some((f.origin.x, f.origin.y, f.size.width, f.size.height))
    })
}

/// 将窗口定位到鼠标附近，超出屏幕边缘时自动向内避让。
fn position_window(
    win: &tauri::WebviewWindow,
    size_override: Option<(f64, f64)>,
) -> Result<(), String> {
    let scale = win.scale_factor().map_err(|e| e.to_string())?;
    let (w, h) = match size_override {
        Some(sz) => sz,
        None => {
            let size = win.outer_size().map_err(|e| e.to_string())?;
            (size.width as f64 / scale, size.height as f64 / scale)
        }
    };
    let Some((sx, sy, sw, sh)) = main_screen_bounds() else {
        return Ok(());
    };
    let margin = 8.0;
    let (mut x, mut y) = LAST_CURSOR.lock().unwrap().unwrap_or((sx + sw / 2.0, sy + sh / 2.0));
    if x + w > sx + sw - margin {
        x = sx + sw - w - margin;
    }
    if y + h > sy + sh - margin {
        y = sy + sh - h - margin;
    }
    x = x.max(sx + margin);
    y = y.max(sy + margin);
    win.set_position(LogicalPosition::new(x, y))
        .map_err(|e| e.to_string())
}

/// 弹窗尺寸变化后重新定位（前端在 setSize 后调用）。居中模式不移动。
#[tauri::command]
pub fn position_popover(app: tauri::AppHandle) -> Result<(), String> {
    let mode = app
        .state::<crate::commands::AppState>()
        .settings
        .lock()
        .unwrap()
        .settings
        .position_mode
        .clone();
    if mode != "cursor" {
        return Ok(());
    }
    if let Some(win) = app.get_webview_window("main") {
        position_window(&win, None)?;
    }
    Ok(())
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
