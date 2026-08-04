use objc2::rc::{autoreleasepool, Retained};
#[allow(deprecated)]
use objc2_app_kit::{
    NSFilenamesPboardType, NSPasteboard, NSPasteboardTypeRTF, NSPasteboardTypeString,
    NSPasteboardTypeTIFF,
};
use objc2_foundation::{NSArray, NSString};

/// 一次剪贴板快照。M1 覆盖：文本 / RTF / 文件 / 图片标记。
pub struct ClipboardSnapshot {
    pub change_count: isize,
    pub text: Option<String>,
    pub rtf: Option<Vec<u8>>,
    pub files: Vec<String>,
    pub has_image: bool,
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
    autoreleasepool(|_| {
        let (type_string, type_rtf, type_tiff, type_files) = type_constants();
        let pb = NSPasteboard::generalPasteboard();
        let change_count = pb.changeCount();
        let text = pb
            .stringForType(type_string)
            .map(|s| s.to_string());
        let rtf = pb.dataForType(type_rtf).map(|d| d.to_vec());
        let files: Vec<String> = pb
            .propertyListForType(type_files)
            .map(|pl| {
                // 剪贴板 filenames 属性必然是 NSString 数组
                let arr: Retained<NSArray<NSString>> =
                    unsafe { Retained::cast_unchecked(pl) };
                arr.iter().map(|s| s.to_string()).collect()
            })
            .unwrap_or_default();
        let has_image = pb.dataForType(type_tiff).is_some();
        ClipboardSnapshot {
            change_count,
            text,
            rtf,
            files,
            has_image,
        }
    })
}

pub fn write_text(s: &str) {
    autoreleasepool(|_| {
        let type_string = type_constants().0;
        let pb = NSPasteboard::generalPasteboard();
        pb.clearContents();
        let ns = NSString::from_str(s);
        let _ = pb.setString_forType(&ns, type_string);
    });
}

pub fn write_files(paths: &[String]) {
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
    });
}
