use objc2::rc::{autoreleasepool, Retained};
#[allow(deprecated)]
use objc2::runtime::ProtocolObject;
use objc2_app_kit::{
    NSPasteboard, NSPasteboardTypeFileURL, NSPasteboardTypePNG, NSPasteboardTypeRTF,
    NSPasteboardTypeString, NSPasteboardTypeTIFF, NSPasteboardWriting,
};
use objc2_foundation::{NSArray, NSData, NSString, NSURL};
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
    /// 富文本原始数据；与 text 联合捕获，粘贴时可同时回退纯文本
    pub rtf: Option<Vec<u8>>,
    pub files: Vec<String>,
    pub has_image: bool,
    /// 图片字节数据。优先 TIFF（系统截图等），没有时回退 PNG
    /// （浏览器「复制图像」大多只提供 PNG）。格式由解码端自适应。
    pub tiff: Option<Vec<u8>>,
}

fn pb_type_string() -> &'static objc2_app_kit::NSPasteboardType {
    unsafe { NSPasteboardTypeString }
}

fn pb_type_rtf() -> &'static objc2_app_kit::NSPasteboardType {
    unsafe { NSPasteboardTypeRTF }
}

fn pb_type_tiff() -> &'static objc2_app_kit::NSPasteboardType {
    unsafe { NSPasteboardTypeTIFF }
}

fn pb_type_png() -> &'static objc2_app_kit::NSPasteboardType {
    unsafe { NSPasteboardTypePNG }
}

fn pb_type_file_url() -> &'static objc2_app_kit::NSPasteboardType {
    unsafe { NSPasteboardTypeFileURL }
}

/// 只读取剪贴板 changeCount，不读取任何数据内容。
/// 轮询时先调它做廉价判断，避免每 500ms 全量拷贝文本/RTF/图片数据。
pub fn change_count() -> isize {
    with_pasteboard(|| autoreleasepool(|_| NSPasteboard::generalPasteboard().changeCount()))
}

pub fn read_clipboard() -> ClipboardSnapshot {
    // 整个快照在多轮 objc 调用（stringForType/dataForType/propertyListForType）期间
    // 必须独占剪贴板，避免与粘贴写回或其它线程的读取交错
    with_pasteboard(|| {
        autoreleasepool(|_| {
            let pb = NSPasteboard::generalPasteboard();
            let change_count = pb.changeCount();
            let text = pb
                .stringForType(pb_type_string())
                .map(|s| s.to_string());
            let rtf = pb.dataForType(pb_type_rtf()).map(|d| d.to_vec());
            // 现代文件类型：NSPasteboardTypeFileURL（NSURL 数组），旧 NSFilenamesPboardType 已废弃。
            // path() 解出解码后的 POSIX 路径（浏览器拖拽/访达复制均适用）；非文件 URL 返回 None 直接跳过。
            let files: Vec<String> = pb
                .propertyListForType(pb_type_file_url())
                .map(|pl| {
                    let arr: Retained<NSArray<NSURL>> = unsafe { Retained::cast_unchecked(pl) };
                    arr.iter()
                        .filter_map(|u| u.path().map(|p| p.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            // 图片：优先 TIFF，缺失时回退 PNG-only 剪贴板内容（否则抓不到浏览器复制的图）
            let tiff = pb
                .dataForType(pb_type_tiff())
                .or_else(|| pb.dataForType(pb_type_png()))
                .map(|d| d.to_vec());
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
            let pb = NSPasteboard::generalPasteboard();
            pb.clearContents();
            let ns = NSString::from_str(s);
            let _ = pb.setString_forType(&ns, pb_type_string());
        })
    });
}

pub fn write_image_png(path: &str) -> Result<(), String> {
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    with_pasteboard(|| {
        autoreleasepool(|_| {
            let pb = NSPasteboard::generalPasteboard();
            pb.clearContents();
            let ns = NSData::from_vec(data);
            let _ = pb.setData_forType(Some(&ns), pb_type_png());
        })
    });
    Ok(())
}

/// 同时写入富文本与纯文本：目标应用按自身能力选择类型——
/// Pages/Word 等取 RTF 保留样式；终端、纯文本编辑器等只认 String 的应用也有内容可粘，
/// 否则它们会拿到空剪贴板（历史 bug：RTF 条目粘贴到终端无效）。
pub fn write_rtf_with_text(data: &[u8], text: &str) {
    with_pasteboard(|| {
        autoreleasepool(|_| {
            let pb = NSPasteboard::generalPasteboard();
            pb.clearContents();
            let d = NSData::from_vec(data.to_vec());
            let _ = pb.setData_forType(Some(&d), pb_type_rtf());
            if !text.is_empty() {
                let s = NSString::from_str(text);
                let _ = pb.setString_forType(&s, pb_type_string());
            }
        })
    });
}

/// 写入文件引用（现代 FileURL 方式）。代替已废弃的 declareTypes_owner +
/// NSFilenamesPboardType 组合；系统会自动补齐兼容所需的其它表示形式。
pub fn write_files(paths: &[String]) {
    with_pasteboard(|| {
        autoreleasepool(|_| {
            let pb = NSPasteboard::generalPasteboard();
            pb.clearContents();
            let objs: Vec<Retained<ProtocolObject<dyn NSPasteboardWriting>>> = paths
                .iter()
                .map(|p| {
                    let ns = NSString::from_str(p);
                    // fileURLWithPath 对任意路径都会构造 NSURL（空串除外，调用方不会传）
                    let url = NSURL::fileURLWithPath(&ns);
                    ProtocolObject::<dyn NSPasteboardWriting>::from_retained(url)
                })
                .collect();
            let arr = NSArray::from_retained_slice(&objs);
            let _ = pb.writeObjects(&arr);
        })
    });
}
