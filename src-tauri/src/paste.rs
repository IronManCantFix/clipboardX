use crate::clipboard::{write_files, write_image_png, write_rtf, write_text};
use crate::models::{Entry, EntryType};

#[derive(Debug, PartialEq)]
pub enum Payload {
    Text(String),
    Rtf(Vec<u8>),
    Files(Vec<String>),
    Image(String),
}

/// 将条目转成可写回剪贴板的负载。RTF 恢复为富文本数据。
pub fn write_payload(e: &Entry) -> Payload {
    match e.kind {
        EntryType::Text | EntryType::Url => Payload::Text(e.content.clone()),
        EntryType::File => Payload::Files(e.file_paths.clone()),
        EntryType::Image => Payload::Image(e.image_path.clone().unwrap_or_default()),
        EntryType::Rtf => Payload::Rtf(e.rtf_data.clone().unwrap_or_default()),
    }
}

pub fn write_back(e: &Entry) -> Result<(), String> {
    match write_payload(e) {
        Payload::Text(s) => {
            write_text(&s);
            Ok(())
        }
        Payload::Rtf(data) => {
            write_rtf(&data);
            Ok(())
        }
        Payload::Files(v) => {
            write_files(&v);
            Ok(())
        }
        Payload::Image(path) => write_image_png(&path),
    }
}

/// 模拟 Cmd+V 粘贴到前台应用（需 macOS 辅助功能权限）。

#[repr(C)]
struct CGEvent(std::ffi::c_void);

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventCreateKeyboardEvent(
        source: *const std::ffi::c_void,
        virtual_key: u16,
        key_down: bool,
    ) -> *mut CGEvent;
    fn CGEventSetFlags(event: *mut CGEvent, flags: u64);
    fn CGEventPost(tap: u32, event: *mut CGEvent);
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(cf: *mut CGEvent);
}

const FLAG_COMMAND: u64 = 0x100_000;
const HID_EVENT_TAP: u32 = 0;
const KEY_V: u16 = 9;

/// 模拟 Cmd+V 粘贴到前台应用（需 macOS 辅助功能权限）。
/// 直接调用 CoreGraphics CGEvent，不依赖 enigo。
/// 权限不足时 CGEvent 静默不生效——内容已写入剪贴板，用户可手动 Cmd+V。
pub fn send_cmd_v() {
    unsafe {
        let down = CGEventCreateKeyboardEvent(std::ptr::null(), KEY_V, true);
        if !down.is_null() {
            CGEventSetFlags(down, FLAG_COMMAND);
            CGEventPost(HID_EVENT_TAP, down);
            CFRelease(down);
        }
        let up = CGEventCreateKeyboardEvent(std::ptr::null(), KEY_V, false);
        if !up.is_null() {
            CGEventSetFlags(up, FLAG_COMMAND);
            CGEventPost(HID_EVENT_TAP, up);
            CFRelease(up);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(kind: EntryType, content: &str) -> Entry {
        Entry {
            id: 1,
            kind,
            content: content.into(),
            content_hash: None,
            rtf_data: None,
            image_path: None,
            thumb_path: None,
            file_paths: vec![],
            source_app: None,
            pinned: false,
            created_at: 0,
            last_used_at: 0,
        }
    }

    #[test]
    fn payload_for_url_is_text() {
        let e = entry(EntryType::Url, "https://x.dev");
        assert_eq!(write_payload(&e), Payload::Text("https://x.dev".into()));
    }

    #[test]
    fn payload_for_file_uses_paths() {
        let mut e = entry(EntryType::File, "");
        e.file_paths = vec!["/a/b.txt".into()];
        assert_eq!(write_payload(&e), Payload::Files(vec!["/a/b.txt".into()]));
    }

    #[test]
    fn payload_for_text_is_text() {
        let e = entry(EntryType::Text, "hello");
        assert_eq!(write_payload(&e), Payload::Text("hello".into()));
    }

    #[test]
    fn payload_for_image_uses_path() {
        let mut e = entry(EntryType::Image, "[图片]");
        e.image_path = Some("/a/b.png".into());
        assert_eq!(write_payload(&e), Payload::Image("/a/b.png".into()));
    }

    #[test]
    fn payload_for_rtf_uses_rtf_data() {
        let mut e = entry(EntryType::Rtf, "[富文本]");
        e.rtf_data = Some(vec![0x7b, 0x5c, 0x72, 0x74, 0x66]);
        assert_eq!(
            write_payload(&e),
            Payload::Rtf(vec![0x7b, 0x5c, 0x72, 0x74, 0x66])
        );
    }
}
