use crate::clipboard::ClipboardSnapshot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Detected {
    Text(String),
    Url(String),
    Rtf(Vec<u8>),
    /// 携带内容哈希，避免不同图片被去重误判
    Image(u64),
    Files(Vec<String>),
}

/// 分类优先级：文件 > 图片 > RTF > URL > 文本（访达复制文件会附带 TIFF 预览，必须先判文件）
pub fn classify(snap: &ClipboardSnapshot) -> Option<Detected> {
    if !snap.files.is_empty() {
        return Some(Detected::Files(snap.files.clone()));
    }
    if snap.has_image {
        let hash = snap.tiff.as_deref().map(content_hash).unwrap_or(0);
        return Some(Detected::Image(hash));
    }
    if let Some(rtf) = &snap.rtf {
        return Some(Detected::Rtf(rtf.clone()));
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

fn content_hash(data: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    data.hash(&mut h);
    h.finish()
}


use crate::clipboard::{change_count, read_clipboard};
use crate::commands::AppState;
use crate::models::EntryType;
use crate::store::NewEntry;
use std::path::Path;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 清理 images 目录中未被数据库引用的孤儿文件（早期版本去重 bug 的残留）。
pub fn cleanup_orphan_images(data_dir: &std::path::Path, store: &crate::store::Store) {
    let images_dir = data_dir.join("images");
    let Ok(entries) = store.all_image_paths() else {
        return;
    };
    let referenced: std::collections::HashSet<&str> =
        entries.iter().map(|s| s.as_str()).collect();
    let Ok(rd) = std::fs::read_dir(&images_dir) else {
        return;
    };
    let mut removed = 0u32;
    for entry in rd.flatten() {
        let path = entry.path();
        let Some(ps) = path.to_str() else { continue };
        if !referenced.contains(ps) && std::fs::remove_file(&path).is_ok() {
            removed += 1;
        }
    }
    if removed > 0 {
        println!("[clipboardx] cleaned up {removed} orphan image file(s)");
    }
}

pub fn spawn_listener(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last_count = change_count();
        let mut last_detected: Option<Detected> = None;
        loop {
            std::thread::sleep(Duration::from_millis(750));
            // 先只查 changeCount，未变化则完全跳过全量读取（省内存/CPU）
            if change_count() == last_count {
                continue;
            }
            let snap = read_clipboard();
            if snap.change_count == last_count {
                continue;
            }
            last_count = snap.change_count;
            let state = app.state::<AppState>();
            let Some(detected) = classify(&snap) else {
                continue;
            };
            if last_detected.as_ref().is_some_and(|p| same_content(p, &detected)) {
                continue;
            }
            last_detected = Some(detected.clone());
            let now = now_ts();
            let (kind, content, content_hash, rtf, files, image_path, thumb_path) = match &detected {
                Detected::Text(s) => (EntryType::Text, s.clone(), None, None, vec![], None, None),
                Detected::Url(s) => (EntryType::Url, s.clone(), None, None, vec![], None, None),
                Detected::Rtf(r) => (
                    EntryType::Rtf,
                    "[富文本]".into(),
                    None,
                    Some(r.clone()),
                    vec![],
                    None,
                    None,
                ),
                Detected::Files(f) => (EntryType::File, f.join(", "), None, None, f.clone(), None, None),
                Detected::Image(hash) => {
                    let data_dir = state.data_dir.clone();
                    let paths = snap
                        .tiff
                        .as_deref()
                        .and_then(|tiff| save_image(tiff, *hash, &data_dir).ok());
                    let (path, thumb_path) = match paths {
                        Some((p, t)) => (p, Some(t)),
                        None => continue, // 图片保存失败则跳过
                    };
                    (
                        EntryType::Image,
                        "[图片]".into(),
                        Some(*hash as i64),
                        None,
                        vec![],
                        Some(path),
                        thumb_path,
                    )
                }
            };
            let source_app = frontmost_app_name();
            let source_app_log = source_app.clone();
            let new_entry = NewEntry {
                kind,
                content,
                content_hash,
                rtf_data: rtf,
                image_path,
                thumb_path,
                file_paths: files,
                source_app,
                created_at: now,
                last_used_at: now,
            };
            let limit = state.settings.lock().unwrap().settings.history_limit;
            {
                let store = state.store.lock().unwrap();
                // 全局去重：内容已存在则移到最前，不重复插入
                let existing = match new_entry.content_hash {
                    Some(h) => store.find_by_content_hash(h).unwrap_or(None),
                    None => store.find_by_content(kind, &new_entry.content).unwrap_or(None),
                };
                match existing {
                    Some(existing_id) => {
                        let _ = store.touch(existing_id, now);
                    }
                    None => {
                        let _ = store.insert(&new_entry);
                        if let Ok(removed) = store.prune(limit) {
                            drop(store);
                            for e in &removed {
                                crate::commands::remove_image_file(e);
                            }
                        }
                    }
                }
            }
            println!(
                "[clipboardx] captured {:?} from {}",
                kind,
                source_app_log.as_deref().unwrap_or("?")
            );
            let _ = app.emit("clipboard:new-entry", ());
        }
    });
}

/// 像素上限：超过则跳过保存，避免超大图（如 8K 截图）全量解码造成内存尖峰。
const MAX_IMAGE_PIXELS: u64 = 24_000_000;

/// 将 TIFF 数据转为 PNG 存入 images 目录，返回 (原图路径, 缩略图路径)。
/// 文件名基于内容 hash：同一张图重复复制时直接复用已有文件，不重复保存。
fn save_image(tiff: &[u8], hash: u64, data_dir: &Path) -> Result<(String, String), String> {
    let images_dir = data_dir.join("images");
    std::fs::create_dir_all(&images_dir).map_err(|e| e.to_string())?;
    let name = format!("{hash}");
    let path = images_dir.join(format!("{name}.png"));
    let thumb_path = images_dir.join(format!("{name}_thumb.png"));
    if path.exists() && thumb_path.exists() {
        return Ok((
            path.to_string_lossy().into_owned(),
            thumb_path.to_string_lossy().into_owned(),
        ));
    }
    // 先只读 TIFF 头部获取尺寸，超大图直接拒绝，不进入全量解码
    let reader =
        image::ImageReader::with_format(std::io::Cursor::new(tiff), image::ImageFormat::Tiff);
    let (w, h) = reader
        .into_dimensions()
        .map_err(|e| format!("tiff header read failed: {e}"))?;
    if (w as u64) * (h as u64) > MAX_IMAGE_PIXELS {
        return Err(format!("image too large ({w}x{h}), skipped"));
    }
    let img = image::load_from_memory_with_format(tiff, image::ImageFormat::Tiff)
        .map_err(|e| format!("tiff decode failed: {e}"))?;
    // 原图：最长边超过 1600px 时缩小，控制磁盘占用
    let main = downscale(&img, 1600);
    main.save_with_format(&path, image::ImageFormat::Png)
        .map_err(|e| format!("png save failed: {e}"))?;
    // 缩略图：最长边 360px，弹窗列表展示用，降低渲染内存
    let thumb = downscale(&img, 360);
    thumb
        .save_with_format(&thumb_path, image::ImageFormat::Png)
        .map_err(|e| format!("thumb save failed: {e}"))?;
    Ok((
        path.to_string_lossy().into_owned(),
        thumb_path.to_string_lossy().into_owned(),
    ))
}

fn downscale(img: &image::DynamicImage, max: u32) -> image::DynamicImage {
    let (w, h) = (img.width(), img.height());
    if w.max(h) <= max {
        return img.clone();
    }
    let scale = max as f32 / w.max(h) as f32;
    let nw = ((w as f32 * scale).round() as u32).max(1);
    let nh = ((h as f32 * scale).round() as u32).max(1);
    img.resize(nw, nh, image::imageops::FilterType::Lanczos3)
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
        assert!(matches!(classify(&snap), Some(Detected::Image(_))));
    }

    #[test]
    fn classify_files_over_image() {
        // 访达复制文件时同时有 TIFF 预览和文件路径，应判定为 Files
        let snap = ClipboardSnapshot {
            change_count: 1,
            text: None,
            rtf: None,
            files: vec!["/Users/test/file.png".into()],
            has_image: true,
            tiff: Some(vec![1, 2, 3]),
        };
        assert!(matches!(classify(&snap), Some(Detected::Files(_))));
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

    #[test]
    fn dedupe_distinguishes_different_images() {
        let a = Detected::Image(111);
        let b = Detected::Image(222);
        assert!(!same_content(&a, &b));
        assert!(same_content(&a, &Detected::Image(111)));
    }

    #[test]
    fn cleanup_removes_only_orphan_files() {
        let dir = std::env::temp_dir().join(format!(
            "clipboardx-orphan-test-{}-{}",
            std::process::id(),
            now_ms()
        ));
        let images = dir.join("images");
        std::fs::create_dir_all(&images).unwrap();
        let orphan = images.join("orphan.png");
        let kept = images.join("kept.png");
        std::fs::write(&orphan, b"x").unwrap();
        std::fs::write(&kept, b"x").unwrap();

        let store = crate::store::Store::open(&dir.join("test.db")).unwrap();
        let new_entry = crate::store::NewEntry {
            kind: crate::models::EntryType::Image,
            content: "[图片]".into(),
            content_hash: Some(1),
            rtf_data: None,
            image_path: Some(kept.to_string_lossy().into_owned()),
            thumb_path: None,
            file_paths: vec![],
            source_app: None,
            created_at: 1,
            last_used_at: 1,
        };
        store.insert(&new_entry).unwrap();

        cleanup_orphan_images(&dir, &store);
        assert!(!orphan.exists(), "orphan file should be removed");
        assert!(kept.exists(), "referenced file should be kept");
    }
}
