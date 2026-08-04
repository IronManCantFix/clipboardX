use crate::clipboard::{write_files, write_text};
use crate::models::{Entry, EntryType};

#[derive(Debug, PartialEq)]
pub enum Payload {
    Text(String),
    Files(Vec<String>),
}

/// 将条目转成可写回剪贴板的负载。M1：文本/URL/文件；RTF/图片 M2 支持。
pub fn write_payload(e: &Entry) -> Payload {
    match e.kind {
        EntryType::Text | EntryType::Url => Payload::Text(e.content.clone()),
        EntryType::File => Payload::Files(e.file_paths.clone()),
        EntryType::Rtf | EntryType::Image => Payload::Text(e.content.clone()),
    }
}

pub fn write_back(e: &Entry) {
    match write_payload(e) {
        Payload::Text(s) => write_text(&s),
        Payload::Files(v) => write_files(&v),
    }
}

/// 模拟 Cmd+V 粘贴到前台应用（需 macOS 辅助功能权限）。
pub fn send_cmd_v() -> Result<(), String> {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    enigo
        .key(Key::Meta, Direction::Press)
        .map_err(|e| e.to_string())?;
    enigo
        .key(Key::Unicode('v'), Direction::Press)
        .map_err(|e| e.to_string())?;
    enigo
        .key(Key::Unicode('v'), Direction::Release)
        .map_err(|e| e.to_string())?;
    enigo
        .key(Key::Meta, Direction::Release)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(kind: EntryType, content: &str) -> Entry {
        Entry {
            id: 1,
            kind,
            content: content.into(),
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
}
