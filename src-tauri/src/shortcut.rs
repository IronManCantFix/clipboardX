use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplicationActivationOptions, NSApplicationActivationPolicy};
use crate::commands::lock_settings;
use crate::models::{now_ms, PositionMode};
use std::sync::atomic::{AtomicI32, AtomicI64, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, LogicalPosition, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState};

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: *const std::ffi::c_void) -> bool;
    static kAXTrustedCheckOptionPrompt: *const std::ffi::c_void;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    static kCFBooleanTrue: *const std::ffi::c_void;
    fn CFDictionaryCreate(
        allocator: *const std::ffi::c_void,
        keys: *const *const std::ffi::c_void,
        values: *const *const std::ffi::c_void,
        num_values: isize,
        key_callbacks: *const std::ffi::c_void,
        value_callbacks: *const std::ffi::c_void,
    ) -> *mut std::ffi::c_void;
    fn CFRelease(cf: *const std::ffi::c_void);
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGMainDisplayID() -> u32;
    fn CGDisplayPixelsHigh(display: u32) -> usize;
}

/// 当前进程是否已获得 macOS 辅助功能权限。
pub fn ax_is_trusted() -> bool {
    unsafe { AXIsProcessTrusted() }
}

/// 是否已在本会话中弹过辅助功能授权对话框（避免每次粘贴都弹）。
static AX_PROMPTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// 检查辅助功能权限。未授权时仅首次弹出系统授权对话框，之后静默。
/// release 构建的二进制与 dev 不同，需要单独授权。
pub fn ax_check_and_prompt() -> bool {
    if ax_is_trusted() {
        return true;
    }
    // 本会话已弹过，不再重复弹窗
    if AX_PROMPTED.swap(true, Ordering::SeqCst) {
        return false;
    }
    unsafe {
        let key = kAXTrustedCheckOptionPrompt;
        let value = kCFBooleanTrue;
        let keys = [key];
        let values = [value];
        let dict = CFDictionaryCreate(
            std::ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            1,
            std::ptr::null(),
            std::ptr::null(),
        );
        let trusted = AXIsProcessTrustedWithOptions(dict);
        if !dict.is_null() {
            CFRelease(dict);
        }
        trusted
    }
}

/// 唤起弹窗时记录的前台应用 pid，粘贴时用于把焦点还给它。
static LAST_FRONT_APP_PID: AtomicI32 = AtomicI32::new(0);

/// 唤起时记录的鼠标位置，用于弹窗避让定位。
static LAST_CURSOR: Mutex<Option<(f64, f64)>> = Mutex::new(None);

/// 最近一次主动唤起弹窗的时间（毫秒），用于聚焦保护期判断。
static LAST_SHOWN_AT: AtomicI64 = AtomicI64::new(0);

/// 记录主动唤起时间；此后短暂时间内即使窗口未获得焦点也不应隐藏。
pub fn mark_shown() {
    LAST_SHOWN_AT.store(now_ms(), Ordering::SeqCst);
}

/// 是否处于唤起保护期（刚主动显示过，聚焦还没稳定）。
pub fn just_shown() -> bool {
    now_ms().saturating_sub(LAST_SHOWN_AT.load(Ordering::SeqCst)) < 400
}

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
/// NSWorkspace 非线程安全，必须在主线程调用；快捷键回调运行在插件线程，
/// 通过 run_on_main_thread 派发并同步等待（500ms 超时，失败按 0 处理：
/// 粘贴后不归还焦点，不做任何 AppKit 调用）。
fn remember_front_app(app: &AppHandle) {
    let query = || -> i32 {
        objc2::rc::autoreleasepool(|_| {
            let ws = objc2_app_kit::NSWorkspace::sharedWorkspace();
            ws.frontmostApplication()
                .map(|a| a.processIdentifier())
                .unwrap_or(0)
        })
    };
    // 已在主线程（防御：理论上不会被这样调用）则直接执行，避免自我阻塞
    let pid = if MainThreadMarker::new().is_some() {
        query()
    } else {
        let (tx, rx) = std::sync::mpsc::sync_channel::<i32>(1);
        if app
            .run_on_main_thread(move || {
                let _ = tx.send(query());
            })
            .is_err()
        {
            0
        } else {
            rx.recv_timeout(std::time::Duration::from_millis(500))
                .unwrap_or(0)
        }
    };
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

/// 当前鼠标位置（AppKit 全局坐标，主屏左下为原点；副屏可为负坐标）。
pub fn cursor_global() -> Option<(f64, f64)> {
    objc2::rc::autoreleasepool(|_| {
        let p = objc2_app_kit::NSEvent::mouseLocation();
        Some((p.x, p.y))
    })
}

pub fn show_popover(app: &AppHandle) {
    remember_front_app(app);
    activate_app();
    // 设置锁可能被毒化（极低概率），弹窗唤起不容失败：统一走容错取锁
    let mode = lock_settings(&app.state::<crate::commands::AppState>())
        .settings
        .position_mode;
    if let Some(cursor) = cursor_global() {
        *LAST_CURSOR.lock().unwrap_or_else(|p| p.into_inner()) = Some(cursor);
    }
    if let Some(win) = app.get_webview_window("main") {
        if mode == PositionMode::Center {
            let _ = win.center();
        } else {
            let _ = position_window(&win);
        }
        // 标记唤起保护期，并推迟 show/set_focus 到下一帧：
        // accessory app 在同一帧内 set_focus 无效，且窗口未聚焦会触发
        // Focused(false) → hide 竞态（弹窗一闪而过）
        mark_shown();
        let opacity = lock_settings(&app.state::<crate::commands::AppState>()).settings.opacity;
        crate::commands::apply_native_window_opacity(app, opacity);
        let w = win;
        let _ = app.run_on_main_thread(Box::new(move || {
            let _ = w.show();
            let _ = w.set_focus();
        }));
        // 通知前端「弹窗已显示」：弹窗靠此事件重读设置与列表。
        // 不能只依赖窗口 Focused 事件——macOS accessory 窗口的焦点事件不可靠，
        // 设置里拖动滑杆后弹窗可能拿不到新值（表现为外观无变化）。
        let _ = app.emit("popover-shown", ());
    }
}

/// 主显示器像素高度。tao 在 macOS 上用 `CGDisplay::main().pixels_high() - y` 把
/// 窗口逻辑坐标（左上原点）翻转成 AppKit 坐标，这里用同一个常量保证多屏下坐标一致。
fn main_pixels_high() -> f64 {
    unsafe { CGDisplayPixelsHigh(CGMainDisplayID()) as f64 }
}

/// 鼠标所在屏幕的全局 frame（AppKit 坐标：主屏左下为原点，副屏可为负坐标）。
/// 遍历所有屏幕找到包含鼠标的那一个，多显示器下弹窗跟随鼠标所在屏幕。
fn cursor_screen_frame() -> Option<(f64, f64, f64, f64)> {
    objc2::rc::autoreleasepool(|_| {
        let p = objc2_app_kit::NSEvent::mouseLocation();
        let marker = MainThreadMarker::new()?;
        for screen in objc2_app_kit::NSScreen::screens(marker).iter() {
            let f = screen.frame();
            let (ox, oy, sw, sh) = (f.origin.x, f.origin.y, f.size.width, f.size.height);
            if p.x >= ox && p.x < ox + sw && p.y >= oy && p.y < oy + sh {
                return Some((ox, oy, sw, sh));
            }
        }
        // 兜底：主屏
        let f = objc2_app_kit::NSScreen::mainScreen(marker)?.frame();
        Some((f.origin.x, f.origin.y, f.size.width, f.size.height))
    })
}

/// 将窗口定位到鼠标附近，超出鼠标所在屏幕边缘时自动向内避让（多显示器跟随鼠标）。
fn position_window(win: &tauri::WebviewWindow) -> Result<(), String> {
    let scale = win.scale_factor().map_err(|e| e.to_string())?;
    let size = win.outer_size().map_err(|e| e.to_string())?;
    let (w, h) = (size.width as f64 / scale, size.height as f64 / scale);
    let Some((sx, sy, sw, sh)) = cursor_screen_frame() else {
        return Ok(());
    };
    let fallback = (sx + sw / 2.0, sy + sh / 2.0);
    let cursor = cursor_global()
        .or_else(|| *LAST_CURSOR.lock().unwrap_or_else(|p| p.into_inner()))
        .unwrap_or(fallback);
    // 换算成 tao 逻辑坐标（x 距主屏左缘，y 距主屏上缘；翻转常量与 tao 的 window_position 一致）
    let ph = main_pixels_high();
    let (mut x, mut y) = (cursor.0, ph - cursor.1);
    // 鼠标所在屏幕在 tao 逻辑坐标中的边界
    let left = sx;
    let top = ph - (sy + sh);
    let right = sx + sw;
    let bottom = ph - sy;
    let margin = 8.0;
    if x + w > right - margin {
        x = right - w - margin;
    }
    if y + h > bottom - margin {
        y = bottom - h - margin;
    }
    x = x.max(left + margin);
    y = y.max(top + margin);
    win.set_position(LogicalPosition::new(x, y))
        .map_err(|e| e.to_string())
}

/// 弹窗尺寸变化后重新定位（前端在 setSize 后调用）。居中模式不移动。
#[tauri::command]
pub fn position_popover(app: tauri::AppHandle) -> Result<(), String> {
    let mode = lock_settings(&app.state::<crate::commands::AppState>())
        .settings
        .position_mode;
    if mode != PositionMode::Cursor {
        return Ok(());
    }
    if let Some(win) = app.get_webview_window("main") {
        position_window(&win)?;
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

/// 注册全局快捷键（启动时使用；进程内尚无已注册的键）。
pub fn register_shortcut(app: &AppHandle, shortcut_str: &str) -> Result<(), String> {
    let shortcut: Shortcut = shortcut_str
        .parse()
        .map_err(|e| format!("invalid shortcut {shortcut_str}: {e}"))?;
    app.global_shortcut()
        .on_shortcut(shortcut, shortcut_handler())
        .map_err(|e| format!("register shortcut failed: {e}"))?;
    println!("[clipboardx] global shortcut registered: {shortcut_str}");
    Ok(())
}

/// 替换快捷键：先注册新键，成功后才注销旧键。
/// 注册失败（如与其它应用冲突）时旧键保持可用、设置不落盘，不会出现快捷键丢失。
pub fn replace_shortcut(
    app: &AppHandle,
    new_shortcut: &str,
    old_shortcut: &str,
) -> Result<(), String> {
    if new_shortcut == old_shortcut {
        return Ok(());
    }
    let new: Shortcut = new_shortcut
        .parse()
        .map_err(|e| format!("invalid shortcut {new_shortcut}: {e}"))?;
    // 幂等：若新键已被本应用注册（重复保存/失败残留），先注销再注册
    if app.global_shortcut().is_registered(new) {
        let _ = app.global_shortcut().unregister(new);
    }
    app.global_shortcut()
        .on_shortcut(new, shortcut_handler())
        .map_err(|e| format!("register shortcut {new_shortcut} failed: {e}"))?;
    // 新键注册成功后再注销旧键
    let _ = app.global_shortcut().unregister(old_shortcut);
    println!("[clipboardx] global shortcut updated: {old_shortcut} -> {new_shortcut}");
    Ok(())
}
