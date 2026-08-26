use crate::clipboard::{write_files, write_image_png, write_rtf_with_text, write_text};
use crate::models::{Entry, EntryType};

#[derive(Debug, PartialEq)]
pub enum Payload {
    Text(String),
    /// RTF 数据 + 纯文本回退：目标应用只认 String 时也有内容可粘
    Rtf(Vec<u8>, String),
    Files(Vec<String>),
    Image(String),
}

/// 将条目转成可写回剪贴板的负载。RTF 恢复为富文本数据并附纯文本。
pub fn write_payload(e: &Entry) -> Payload {
    match e.kind {
        EntryType::Text | EntryType::Url => Payload::Text(e.content.clone()),
        EntryType::File => Payload::Files(e.file_paths.clone()),
        EntryType::Image => Payload::Image(e.image_path.clone().unwrap_or_default()),
        // 旧库记录 content 固定为「[富文本]」占位，此时不把它当真实文本写回
        EntryType::Rtf => Payload::Rtf(
            e.rtf_data.clone().unwrap_or_default(),
            e.content.clone(),
        ),
    }
}

pub fn write_back(e: &Entry) -> Result<(), String> {
    match write_payload(e) {
        Payload::Text(s) => {
            write_text(&s);
            Ok(())
        }
        Payload::Rtf(data, text) => {
            let plain = if text == "[富文本]" { "" } else { text.as_str() };
            write_rtf_with_text(&data, plain);
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
    fn CFRelease(cf: *const std::ffi::c_void);
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
            CFRelease(cmd_down as *const std::ffi::c_void);
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
        // V↓（带 Command flag）
        let v_down = CGEventCreateKeyboardEvent(source, KEY_V, true);
        if !v_down.is_null() {
            CGEventSetFlags(v_down, cmd_flags);
            CGEventPost(HID_EVENT_TAP, v_down);
            CFRelease(v_down as *const std::ffi::c_void);
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
        // V↑（带 Command flag）
        let v_up = CGEventCreateKeyboardEvent(source, KEY_V, false);
        if !v_up.is_null() {
            CGEventSetFlags(v_up, cmd_flags);
            CGEventPost(HID_EVENT_TAP, v_up);
            CFRelease(v_up as *const std::ffi::c_void);
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
        // Command↑
        let cmd_up = CGEventCreateKeyboardEvent(source, KEY_COMMAND, false);
        if !cmd_up.is_null() {
            CGEventSetFlags(cmd_up, base_flags);
            CGEventPost(HID_EVENT_TAP, cmd_up);
            CFRelease(cmd_up as *const std::ffi::c_void);
        }
        if !source.is_null() {
            CFRelease(source as *const std::ffi::c_void);
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
    fn payload_for_rtf_carries_plain_text_fallback() {
        let mut e = entry(EntryType::Rtf, "实际正文");
        e.rtf_data = Some(vec![0x7b, 0x5c, 0x72, 0x74, 0x66]);
        assert_eq!(
            write_payload(&e),
            Payload::Rtf(vec![0x7b, 0x5c, 0x72, 0x74, 0x66], "实际正文".into())
        );
    }

    #[test]
    fn payload_for_legacy_rtf_placeholder_content() {
        // 旧版库记录 content 为占位文案；写回时不把占位当正文
        let mut e = entry(EntryType::Rtf, "[富文本]");
        e.rtf_data = Some(vec![1, 2]);
        match write_payload(&e) {
            Payload::Rtf(data, text) => {
                assert_eq!(data, vec![1, 2]);
                assert_eq!(text, "[富文本]");
            }
            other => panic!("expected Rtf payload, got {other:?}"),
        }
    }
}
