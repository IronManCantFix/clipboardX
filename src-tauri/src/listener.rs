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
        };
        assert!(matches!(classify(&files), Some(Detected::Files(_))));
        let url = ClipboardSnapshot {
            change_count: 1,
            text: Some("https://x.dev".into()),
            rtf: None,
            files: vec![],
            has_image: false,
        };
        assert!(matches!(classify(&url), Some(Detected::Url(_))));
        let txt = ClipboardSnapshot {
            change_count: 1,
            text: Some("plain text".into()),
            rtf: None,
            files: vec![],
            has_image: false,
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
