use crate::clipboard::ClipboardSnapshot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Detected {
    Text(String),
    Url(String),
    Rtf(Vec<u8>),
    Image,
    Files(Vec<String>),
}

/// 分类优先级：图片 > RTF > 文件 > URL > 文本
pub fn classify(snap: &ClipboardSnapshot) -> Option<Detected> {
    if snap.has_image {
        return Some(Detected::Image);
    }
    if let Some(rtf) = &snap.rtf {
        return Some(Detected::Rtf(rtf.clone()));
    }
    if !snap.files.is_empty() {
        return Some(Detected::Files(snap.files.clone()));
    }
    let text = snap.text.as_deref()?;
    if looks_like_url(text) {
        Some(Detected::Url(text.to_string()))
    } else {
        Some(Detected::Text(text.to_string()))
    }
}

pub fn looks_like_url(s: &str) -> bool {
    let t = s.trim();
    t.starts_with("http://")
        || t.starts_with("https://")
        || t.starts_with("ftp://")
        || t.starts_with("www.")
}

pub fn same_content(a: &Detected, b: &Detected) -> bool {
    a == b
}


use crate::clipboard::read_clipboard;
use crate::commands::AppState;
use crate::models::EntryType;
use crate::store::NewEntry;
use chrono::Utc;
use std::path::Path;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

pub fn spawn_listener(app: AppHandle, state: std::sync::Arc<AppState>) {
    std::thread::spawn(move || {
        let mut last_count = read_clipboard().change_count;
        let mut last_detected: Option<Detected> = None;
        loop {
            std::thread::sleep(Duration::from_millis(500));
            let snap = read_clipboard();
            if snap.change_count == last_count {
                continue;
            }
            last_count = snap.change_count;
            let Some(detected) = classify(&snap) else {
                continue;
            };
            if last_detected.as_ref().is_some_and(|p| same_content(p, &detected)) {
                continue;
            }
            last_detected = Some(detected.clone());
            let now = Utc::now().timestamp();
            let (kind, content, rtf, files, image_path) = match &detected {
                Detected::Text(s) => (EntryType::Text, s.clone(), None, vec![], None),
                Detected::Url(s) => (EntryType::Url, s.clone(), None, vec![], None),
                Detected::Rtf(r) => (EntryType::Rtf, "[富文本]".into(), Some(r.clone()), vec![], None),
                Detected::Files(f) => (EntryType::File, f.join(", "), None, f.clone(), None),
                Detected::Image => {
                    let data_dir = state.data_dir.clone();
                    let path = snap.tiff.as_deref().and_then(|tiff| save_image(tiff, &data_dir).ok());
                    let path = match path {
                        Some(p) => p,
                        None => continue, // 图片保存失败则跳过
                    };
                    (EntryType::Image, "[图片]".into(), None, vec![], Some(path))
                }
            };
            let source_app = frontmost_app_name();
            let new_entry = NewEntry {
                kind,
                content,
                rtf_data: rtf,
                image_path,
                thumb_path: None,
                file_paths: files,
                source_app,
                created_at: now,
                last_used_at: now,
            };
            let limit = state.settings.lock().unwrap().settings.history_limit;
            {
                let store = state.store.lock().unwrap();
                let _ = store.insert(&new_entry);
                let _ = store.prune(limit);
            }
            let _ = app.emit("clipboard:new-entry", ());
        }
    });
}

/// 将 TIFF 数据转为 PNG 存入 images 目录，返回文件路径。
fn save_image(tiff: &[u8], data_dir: &Path) -> Result<String, String> {
    let img = image::load_from_memory_with_format(tiff, image::ImageFormat::Tiff)
        .map_err(|e| format!("tiff decode failed: {e}"))?;
    let images_dir = data_dir.join("images");
    std::fs::create_dir_all(&images_dir).map_err(|e| e.to_string())?;
    let path = images_dir.join(format!("{}.png", Utc::now().timestamp_millis()));
    img.save_with_format(&path, image::ImageFormat::Png)
        .map_err(|e| format!("png save failed: {e}"))?;
    Ok(path.to_string_lossy().into_owned())
}

/// 前台应用名（objc2 NSWorkspace；失败返回 None）
fn frontmost_app_name() -> Option<String> {
    objc2::rc::autoreleasepool(|_| {
        let ws = objc2_app_kit::NSWorkspace::sharedWorkspace();
        let app = ws.frontmostApplication()?;
        Some(app.localizedName()?.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_detection() {
        assert!(looks_like_url("https://github.com/foo"));
        assert!(looks_like_url("www.example.com/path"));
        assert!(!looks_like_url("hello world"));
        assert!(!looks_like_url("git push origin main"));
    }

    #[test]
    fn classify_priority_image_over_text() {
        let snap = ClipboardSnapshot {
            change_count: 1,
            text: Some("hello".into()),
            rtf: None,
            files: vec![],
            has_image: true,
            tiff: Some(vec![1, 2, 3]),
        };
        assert!(matches!(classify(&snap), Some(Detected::Image)));
    }

    #[test]
    fn classify_rtf_over_text() {
        let snap = ClipboardSnapshot {
            change_count: 1,
            text: Some("hello".into()),
            rtf: Some(vec![1, 2, 3]),
            files: vec![],
            has_image: false,
            tiff: None,
        };
        assert!(matches!(classify(&snap), Some(Detected::Rtf(_))));
    }

    #[test]
    fn classify_files_and_url_and_text() {
        let files = ClipboardSnapshot {
            change_count: 1,
            text: None,
            rtf: None,
            files: vec!["/a/b.txt".into()],
            has_image: false,
            tiff: None,
        };
        assert!(matches!(classify(&files), Some(Detected::Files(_))));
        let url = ClipboardSnapshot {
            change_count: 1,
            text: Some("https://x.dev".into()),
            rtf: None,
            files: vec![],
            has_image: false,
            tiff: None,
        };
        assert!(matches!(classify(&url), Some(Detected::Url(_))));
        let txt = ClipboardSnapshot {
            change_count: 1,
            text: Some("plain text".into()),
            rtf: None,
            files: vec![],
            has_image: false,
            tiff: None,
        };
        assert!(matches!(classify(&txt), Some(Detected::Text(_))));
    }

    #[test]
    fn dedupe_same_content() {
        let a = Detected::Text("abc".into());
        let b = Detected::Text("abc".into());
        assert!(same_content(&a, &b));
        assert!(!same_content(&a, &Detected::Text("abd".into())));
    }
}
