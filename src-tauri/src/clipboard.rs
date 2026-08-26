use objc2::rc::{autoreleasepool, Retained};
#[allow(deprecated)]
use objc2_app_kit::{
    NSFilenamesPboardType, NSPasteboard, NSPasteboardTypePNG, NSPasteboardTypeRTF,
    NSPasteboardTypeString, NSPasteboardTypeTIFF,
};
use objc2_foundation::{NSArray, NSData, NSString};
use std::sync::Mutex;

/// 全局剪贴板访问锁：本进程内对 NSPasteboard 的所有读写必须串行化。
///
/// 为什么需要：AppKit 的 `[NSPasteboard generalPasteboard]` 共享实例内部有一个
/// 惰性初始化的类型缓存（`_updateTypeCacheIfNeeded`），它**不是线程安全**的。
/// 两个线程同时访问（一个在枚举该缓存、另一个在更新它）会触发
/// `__NSFastEnumerationMutationHandler` / `objc_msgSend` 处的 use-after-free，
/// 表现为 SIGSEGV（EXC_BAD_ACCESS, KERN_INVALID_ADDRESS，可能报 pointer
/// authentication failure）。这是操作系统层的硬崩溃，Rust 的 `catch_unwind` /
/// panic 钩子完全拦不住——监听线程轮询与`paste_entry`写回一旦并发即可复现
/// （历史崩溃报告均为两个线程同时卡在 NSPasteboard 内部）。
///
/// 因此所有读（change_count / read_clipboard）与写（write_*）统一经过此锁，
/// 确保任意时刻只有一个线程在触碰剪贴板。
static PASTEBOARD_LOCK: Mutex<()> = Mutex::new(());

/// 持锁执行剪贴板操作。锁中毒容错与 store/settings 一致：取回内部值继续用，
/// 剪贴板 API 本身不会 panic，毒化只可能是程序 bug，不值得因此中断功能。
fn with_pasteboard<T>(f: impl FnOnce() -> T) -> T {
    let _guard = PASTEBOARD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    f()
}

/// 一次剪贴板快照。M1 覆盖：文本 / RTF / 文件 / 图片标记。
pub struct ClipboardSnapshot {
    pub change_count: isize,
    pub text: Option<String>,
    pub rtf: Option<Vec<u8>>,
    pub files: Vec<String>,
    pub has_image: bool,
    pub tiff: Option<Vec<u8>>,
}

/// 只读取剪贴板 changeCount，不读取任何数据内容。
/// 轮询时先调它做廉价判断，避免每 500ms 全量拷贝文本/RTF/图片数据。
pub fn change_count() -> isize {
    with_pasteboard(|| autoreleasepool(|_| NSPasteboard::generalPasteboard().changeCount()))
}

#[allow(deprecated)]
fn type_constants() -> (
    &'static objc2_app_kit::NSPasteboardType,
    &'static objc2_app_kit::NSPasteboardType,
    &'static objc2_app_kit::NSPasteboardType,
    &'static objc2_app_kit::NSPasteboardType,
) {
    unsafe {
        (
            NSPasteboardTypeString,
            NSPasteboardTypeRTF,
            NSPasteboardTypeTIFF,
            NSFilenamesPboardType,
        )
    }
}

pub fn read_clipboard() -> ClipboardSnapshot {
    // 整个快照在多轮 objc 调用（stringForType/dataForType/propertyListForType）期间
    // 必须独占剪贴板，避免与粘贴写回或其它线程的读取交错
    with_pasteboard(|| {
        autoreleasepool(|_| {
            let (type_string, type_rtf, type_tiff, type_files) = type_constants();
            let pb = NSPasteboard::generalPasteboard();
            let change_count = pb.changeCount();
            let text = pb.stringForType(type_string).map(|s| s.to_string());
            let rtf = pb.dataForType(type_rtf).map(|d| d.to_vec());
            let files: Vec<String> = pb
                .propertyListForType(type_files)
                .map(|pl| {
                    // 剪贴板 filenames 属性必然是 NSString 数组
                    let arr: Retained<NSArray<NSString>> = unsafe { Retained::cast_unchecked(pl) };
                    arr.iter().map(|s| s.to_string()).collect()
                })
                .unwrap_or_default();
            let tiff = pb.dataForType(type_tiff).map(|d| d.to_vec());
            let has_image = tiff.is_some();
            ClipboardSnapshot {
                change_count,
                text,
                rtf,
                files,
                has_image,
                tiff,
            }
        })
    })
}

pub fn write_text(s: &str) {
    with_pasteboard(|| {
        autoreleasepool(|_| {
            let type_string = type_constants().0;
            let pb = NSPasteboard::generalPasteboard();
            pb.clearContents();
            let ns = NSString::from_str(s);
            let _ = pb.setString_forType(&ns, type_string);
        })
    });
}

pub fn write_image_png(path: &str) -> Result<(), String> {
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    with_pasteboard(|| {
        autoreleasepool(|_| {
            let type_png = unsafe { NSPasteboardTypePNG };
            let pb = NSPasteboard::generalPasteboard();
            pb.clearContents();
            let ns = NSData::from_vec(data);
            let _ = pb.setData_forType(Some(&ns), type_png);
        })
    });
    Ok(())
}

pub fn write_rtf(data: &[u8]) {
    with_pasteboard(|| {
        autoreleasepool(|_| {
            let type_rtf = type_constants().1;
            let pb = NSPasteboard::generalPasteboard();
            pb.clearContents();
            let ns = NSData::from_vec(data.to_vec());
            let _ = pb.setData_forType(Some(&ns), type_rtf);
        })
    });
}

pub fn write_files(paths: &[String]) {
    with_pasteboard(|| {
        autoreleasepool(|_| {
            let type_files = type_constants().3;
            let pb = NSPasteboard::generalPasteboard();
            pb.clearContents();
            let arr = NSArray::from_retained_slice(
                &paths
                    .iter()
                    .map(|p| NSString::from_str(p))
                    .collect::<Vec<_>>(),
            );
            let types = NSArray::from_slice(&[type_files]);
            unsafe {
                let _ = pb.declareTypes_owner(&types, None);
                let _ = pb.setPropertyList_forType(&arr, type_files);
            }
        })
    });
}
