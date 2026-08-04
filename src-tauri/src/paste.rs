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

#[repr(C)]
struct CGEvent(std::ffi::c_void);

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventSourceCreate(state_id: u32) -> *mut CGEvent;
    fn CGEventCreateKeyboardEvent(
        source: *mut CGEvent,
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

// CGEvent flags — 与 enigo 完全一致的值
const FLAG_NON_COALESCED: u64 = 0x100;
const FLAG_UNKNOWN: u64 = 0x2000_0000; // enigo 注释：正确的事件都有此 flag
const FLAG_COMMAND: u64 = 0x100_000; // CGEventFlagCommand
const NX_DEVICELCMDKEYMASK: u64 = 0x0000_0008; // 设备级左 Command 键标志

const HID_EVENT_TAP: u32 = 0;
const SOURCE_STATE_COMBINED: u32 = 1;
const KEY_COMMAND: u16 = 55;
const KEY_V: u16 = 9;

/// 模拟 Cmd+V 粘贴到前台应用（需 macOS 辅助功能权限）。
/// 使用与 enigo 完全一致的 flags 和按键序列。
/// 权限不足时 CGEvent 静默不生效——内容已写入剪贴板，用户可手动 Cmd+V。
pub fn send_cmd_v() {
    // 基础 flags（enigo 初始化时设置）
    let base_flags = FLAG_NON_COALESCED | FLAG_UNKNOWN;
    // 按下 Command 后的 flags
    let cmd_flags = base_flags | FLAG_COMMAND | NX_DEVICELCMDKEYMASK;

    unsafe {
        let source = CGEventSourceCreate(SOURCE_STATE_COMBINED);
        // Command↓
        let cmd_down = CGEventCreateKeyboardEvent(source, KEY_COMMAND, true);
        if !cmd_down.is_null() {
            CGEventSetFlags(cmd_down, cmd_flags);
            CGEventPost(HID_EVENT_TAP, cmd_down);
            CFRelease(cmd_down);
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
        // V↓（带 Command flag）
        let v_down = CGEventCreateKeyboardEvent(source, KEY_V, true);
        if !v_down.is_null() {
            CGEventSetFlags(v_down, cmd_flags);
            CGEventPost(HID_EVENT_TAP, v_down);
            CFRelease(v_down);
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
        // V↑（带 Command flag）
        let v_up = CGEventCreateKeyboardEvent(source, KEY_V, false);
        if !v_up.is_null() {
            CGEventSetFlags(v_up, cmd_flags);
            CGEventPost(HID_EVENT_TAP, v_up);
            CFRelease(v_up);
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
        // Command↑
        let cmd_up = CGEventCreateKeyboardEvent(source, KEY_COMMAND, false);
        if !cmd_up.is_null() {
            CGEventSetFlags(cmd_up, base_flags);
            CGEventPost(HID_EVENT_TAP, cmd_up);
            CFRelease(cmd_up);
        }
        if !source.is_null() {
            CFRelease(source);
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
