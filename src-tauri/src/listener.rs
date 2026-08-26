use crate::clipboard::ClipboardSnapshot;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Detected {
    Text(String),
    Url(String),
    /// 富文本数据 + 同屏捕获的纯文本（供搜索与粘贴回退）
    Rtf {
        rtf: Vec<u8>,
        text: String,
    },
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
        return Some(Detected::Rtf {
            rtf: rtf.clone(),
            text: snap.text.clone().unwrap_or_default(),
        });
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
use crate::commands::{lock_settings, lock_store};
use crate::models::{now_ms, EntryType};
use crate::store::{NewEntry, Store};
use std::sync::mpsc;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::OnceLock;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

/// 清理 images 目录中未被数据库引用的孤儿文件（早期版本去重 bug 的残留）。
/// 引用比对按文件名（basename）进行：新版库内存的是相对路径，旧版库可能是绝对路径，
/// basename 归一对两种历史形态都成立；data_dir 整体迁移后孤儿误判的风险也随之消除。
pub fn cleanup_orphan_images(data_dir: &std::path::Path, store: &crate::store::Store) {
    let images_dir = data_dir.join("images");
    let Ok(entries) = store.all_image_paths() else {
        return;
    };
    let referenced: std::collections::HashSet<&str> = entries
        .iter()
        .filter_map(|p| Path::new(p).file_name().and_then(|n| n.to_str()))
        .collect();
    let Ok(rd) = std::fs::read_dir(&images_dir) else {
        return;
    };
    let mut removed = 0u32;
    for entry in rd.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if !referenced.contains(name) && std::fs::remove_file(&path).is_ok() {
            removed += 1;
        }
    }
    if removed > 0 {
        println!("[clipboardx] cleaned up {removed} orphan image file(s)");
        crate::logging::info(format!("cleaned up {removed} orphan image file(s)"));
    }
}

/// 监听线程心跳（毫秒时间戳）。看门狗检测到心跳停滞（panic/死锁）后自动重启监听线程。
static LAST_HEARTBEAT: AtomicI64 = AtomicI64::new(0);

/// 去重 + 落库 + 剪枝清理 + 通知前端。由监听线程与图片工作线程共同调用。
fn commit_entry(app: &AppHandle, new_entry: NewEntry) {
    let state = app.state::<crate::commands::AppState>();
    let kind = new_entry.kind;
    let now = new_entry.last_used_at;
    let limit = lock_settings(&state).settings.history_limit;
    let data_dir = state.data_dir.clone();
    {
        let store = lock_store(&state);
        // 全局去重：内容已存在则移到最前，不重复插入
        let existing = match new_entry.content_hash {
            Some(h) => lock_store_find_hash(&store, h),
            None => store.find_by_content(kind, &new_entry.content).ok().flatten(),
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
                        crate::commands::remove_image_file(e, &data_dir);
                    }
                }
            }
        }
    }
    println!(
        "[clipboardx] captured {:?} from {}",
        kind,
        new_entry.source_app.as_deref().unwrap_or("?")
    );
    let _ = app.emit("clipboard:new-entry", ());
}

fn lock_store_find_hash(store: &Store, hash: i64) -> Option<i64> {
    store.find_by_content_hash(hash).unwrap_or(None)
}

/// 监听线程单次迭代：检测剪贴板变化并捕获条目。任何 panic 由调用方捕获，不影响线程存活。
/// 返回是否观察到剪贴板活动（供自适应轮询退避）。
fn listener_iteration(
    app: &AppHandle,
    last_count: &mut isize,
    last_detected: &mut Option<Detected>,
) -> bool {
    // 先只查 changeCount，未变化则完全跳过全量读取（省内存/CPU）
    if change_count() == *last_count {
        return false;
    }
    let mut snap = read_clipboard();
    if snap.change_count == *last_count {
        return false;
    }
    *last_count = snap.change_count;
    let state = app.state::<crate::commands::AppState>();
    let Some(detected) = classify(&snap) else {
        return true;
    };
    if last_detected
        .as_ref()
        .is_some_and(|p| same_content(p, &detected))
    {
        return true;
    }
    *last_detected = Some(detected.clone());
    let now = now_ms();
    let source_app = frontmost_app_name();
    if let Detected::Image(hash) = &detected {
        // 图片保存（TIFF/PNG 解码 + PNG 编码）较慢：交给单一工作线程串行处理，
        // 避免阻塞监听，也避免连截多图时无上限地并发解码造成内存尖峰。
        let Some(data) = snap.tiff.take() else {
            return true;
        };
        enqueue_image_job(ImageJob {
            app: app.clone(),
            data_dir: state.data_dir.clone(),
            data,
            hash: *hash,
            source_app,
            created_at: now,
        });
        return true;
    }
    let (kind, content, content_hash, rtf, files) = match &detected {
        Detected::Text(s) => (EntryType::Text, s.clone(), None, None, vec![]),
        Detected::Url(s) => (EntryType::Url, s.clone(), None, None, vec![]),
        Detected::Rtf { rtf, text } => (
            EntryType::Rtf,
            // content 存真实纯文本：列表可读、可搜索；为空才回退占位文案
            if text.is_empty() {
                "[富文本]".to_string()
            } else {
                text.clone()
            },
            Some(content_hash(rtf) as i64),
            Some(rtf.clone()),
            vec![],
        ),
        Detected::Files(f) => (EntryType::File, f.join(", "), None, None, f.clone()),
        Detected::Image(_) => unreachable!("image handled above"),
    };
    let new_entry = NewEntry {
        kind,
        content,
        content_hash,
        rtf_data: rtf,
        image_path: None,
        thumb_path: None,
        file_paths: files,
        source_app,
        created_at: now,
        last_used_at: now,
    };
    commit_entry(app, new_entry);
    true
}

// ---- 图片处理工作线程：单消费者队列 ----

struct ImageJob {
    app: AppHandle,
    data_dir: PathBuf,
    /// 原始图片字节（TIFF 或 PNG，解码端按魔数自适应）
    data: Vec<u8>,
    hash: u64,
    source_app: Option<String>,
    created_at: i64,
}

static IMAGE_TX: OnceLock<mpsc::Sender<ImageJob>> = OnceLock::new();

/// 把图片任务送入单工作线程串行处理；首次调用时惰性启动线程。
fn enqueue_image_job(job: ImageJob) {
    let tx = IMAGE_TX.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<ImageJob>();
        std::thread::Builder::new()
            .name("clipboardx-image-worker".into())
            .spawn(move || image_worker_loop(rx))
            .expect("spawn image worker");
        tx
    });
    if tx.send(job).is_err() {
        // 接收端永不关闭（OnceLock 持有 Sender 克隆），此分支仅为防御性兜底
        crate::logging::error("image worker channel closed unexpectedly; image dropped");
    }
}

fn image_worker_loop(rx: mpsc::Receiver<ImageJob>) {
    while let Ok(job) = rx.recv() {
        // 单个任务的 panic 不拖垮整个工作线程
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| process_image_job(&job)));
        if result.is_err() {
            eprintln!("[clipboardx] image worker task panicked (recovered)");
            crate::logging::error("image worker task panicked (recovered)");
        }
    }
}

fn process_image_job(job: &ImageJob) {
    match save_image(&job.data, job.hash, &job.data_dir) {
        Ok((path, thumb_path)) => {
            let new_entry = NewEntry {
                kind: EntryType::Image,
                content: "[图片]".into(),
                content_hash: Some(job.hash as i64),
                rtf_data: None,
                image_path: Some(path),
                thumb_path: Some(thumb_path),
                file_paths: vec![],
                source_app: job.source_app.clone(),
                created_at: job.created_at,
                last_used_at: job.created_at,
            };
            commit_entry(&job.app, new_entry);
        }
        Err(reason) => {
            println!("[clipboardx] image save skipped: {reason}");
        }
    }
}

/// 监听线程是否已在运行。
///
/// 看门狗发现心跳停滞时会尝试「重启」监听线程——但心跳停滞最常见的真实原因是
/// 旧线程仍然活着、只是阻塞在剪贴板读取上（如系统休眠唤醒后 pasteboard 服务
/// 忙碌、其它应用长时间占用剪贴板）。此时绝不能重复启动新线程：两个线程并发
/// 访问 `NSPasteboard::generalPasteboard` 会竞争其内部类型缓存，直接 SIGSEGV
/// 崩溃（详见 clipboard.rs 中 PASTEBOARD_LOCK 的注释）。只有旧线程确实退出后
/// （本标记回到 false）才允许重新启动。
static WORKER_ACTIVE: AtomicBool = AtomicBool::new(false);

fn spawn_listener_worker(app: AppHandle) {
    // swap 保证即使看门狗与其它路径同时尝试启动，也只会有一个线程胜出
    if WORKER_ACTIVE.swap(true, Ordering::SeqCst) {
        println!("[clipboardx] listener worker already running; skip duplicate spawn");
        crate::logging::error(
            "listener worker already running; duplicate spawn skipped (old worker may be stuck in pasteboard read)",
        );
        return;
    }
    std::thread::Builder::new()
        .name("clipboardx-listener".into())
        .spawn(move || {
            // 任何退出路径（包括初始化阶段 panic）都要释放「运行中」标记，
            // 否则看门狗将永远无法恢复监听
            struct ClearActive;
            impl Drop for ClearActive {
                fn drop(&mut self) {
                    WORKER_ACTIVE.store(false, Ordering::SeqCst);
                }
            }
            let _clear_on_exit = ClearActive;
            let mut last_count = change_count();
            let mut last_detected: Option<Detected> = None;
            // 空闲迭代计数：剪贴板长期不变时逐步放慢轮询，降低空转功耗；
            // 任一次活动（或有异常）都会让计数增长，有变化则立即清零回快速档。
            let mut idle_iters: u32 = 0;
            loop {
                LAST_HEARTBEAT.store(now_ms(), Ordering::Relaxed);
                std::thread::sleep(Duration::from_millis(poll_interval_ms(idle_iters)));
                // 捕获单次迭代的任何 panic：失败只跳过这一次，监听线程保持存活
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    listener_iteration(&app, &mut last_count, &mut last_detected)
                }));
                match result {
                    Ok(true) => idle_iters = 0,
                    Ok(false) => idle_iters = idle_iters.saturating_add(1),
                    Err(_) => {
                        idle_iters = idle_iters.saturating_add(1);
                        eprintln!("[clipboardx] listener iteration panicked (recovered)");
                        crate::logging::error("listener iteration panicked (recovered)");
                    }
                }
            }
        })
        .expect("spawn listener worker");
}

/// 自适应轮询间隔：空闲越久轮询越慢（300ms → 700ms → 1400ms 上限）。
/// 用户感知层面几乎无差别（唤起弹窗走快捷键，不依赖这一轮询），
/// 而常驻功耗显著下降；看门狗 10s 心跳阈值远大于最大间隔，互不影响。
fn poll_interval_ms(idle_iters: u32) -> u64 {
    match idle_iters {
        0..=19 => 300,
        20..=49 => 700,
        _ => 1400,
    }
}

/// 启动剪贴板监听：工作线程 + 看门狗（心跳停滞超过 10s 尝试重启；若原线程仍存活
/// 则跳过，见 spawn_listener_worker 的 WORKER_ACTIVE 守卫）。
pub fn spawn_listener(app: AppHandle) {
    let watchdog_app = app.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(5));
        let last = LAST_HEARTBEAT.load(Ordering::Relaxed);
        if last != 0 && now_ms().saturating_sub(last) > 10_000 {
            println!("[clipboardx] listener heartbeat stale; restarting listener");
            crate::logging::info("listener heartbeat stale; trying to restart listener");
            LAST_HEARTBEAT.store(now_ms(), Ordering::Relaxed);
            spawn_listener_worker(watchdog_app.clone());
        }
    });
    spawn_listener_worker(app);
}

/// 像素上限：超过则跳过保存，避免超大图（如 8K 截图）全量解码造成内存尖峰。
const MAX_IMAGE_PIXELS: u64 = 24_000_000;

/// 将剪贴板图片字节转为 PNG 存入 images 目录，返回相对 data_dir 的存储路径
/// （"images/{hash}.png"，见 Entry::image_path 注释的迁移说明）。
/// 原图保持原始分辨率（超过 MAX_IMAGE_PIXELS 的图在头部检查时已拒绝），
/// 保证粘贴回退的是完整原图；缩略图单独降采样供弹窗列表展示。
/// 文件名基于内容 hash：同一张图重复复制时直接复用已有文件，不重复保存。
/// 输入格式不限定：TIFF 与 PNG 字节流都按魔数自适应解码（read_clipboard 已做
/// TIFF 优先、PNG 回退的采集）。
fn save_image(data: &[u8], hash: u64, data_dir: &Path) -> Result<(String, String), String> {
    let images_dir = data_dir.join("images");
    std::fs::create_dir_all(&images_dir).map_err(|e| e.to_string())?;
    let name = format!("{hash}");
    let abs_path = images_dir.join(format!("{name}.png"));
    let abs_thumb = images_dir.join(format!("{name}_thumb.png"));
    let rel_path = format!("images/{name}.png");
    let rel_thumb = format!("images/{name}_thumb.png");
    if abs_path.exists() && abs_thumb.exists() {
        return Ok((rel_path, rel_thumb));
    }
    // 先只读头部获取尺寸，超大图直接拒绝，不进入全量解码（格式按魔数猜测）
    let cursor = std::io::Cursor::new(data);
    let reader = image::ImageReader::new(cursor)
        .with_guessed_format()
        .map_err(|e| format!("image header read failed: {e}"))?;
    let (w, h) = reader
        .into_dimensions()
        .map_err(|e| format!("image header read failed: {e}"))?;
    if (w as u64) * (h as u64) > MAX_IMAGE_PIXELS {
        return Err(format!("image too large ({w}x{h}), skipped"));
    }
    let img = image::load_from_memory(data).map_err(|e| format!("image decode failed: {e}"))?;
    // 原图：保持原始分辨率保存，粘贴回退时得到完整原图
    img.save_with_format(&abs_path, image::ImageFormat::Png)
        .map_err(|e| format!("png save failed: {e}"))?;
    // 缩略图：最长边 360px，弹窗列表展示用，降低渲染内存
    let thumb = downscale(&img, 360);
    thumb
        .save_with_format(&abs_thumb, image::ImageFormat::Png)
        .map_err(|e| format!("thumb save failed: {e}"))?;
    Ok((rel_path, rel_thumb))
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
    fn classify_rtf_over_text_and_carries_plain_text() {
        let snap = ClipboardSnapshot {
            change_count: 1,
            text: Some("hello".into()),
            rtf: Some(vec![1, 2, 3]),
            files: vec![],
            has_image: false,
            tiff: None,
        };
        match classify(&snap) {
            Some(Detected::Rtf { rtf, text }) => {
                assert_eq!(rtf, vec![1, 2, 3]);
                assert_eq!(text, "hello");
            }
            other => panic!("expected Rtf, got {other:?}"),
        }
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
        // RTF 相同数据 + 相同文本视为同一内容
        let ra = Detected::Rtf {
            rtf: vec![9],
            text: "hi".into(),
        };
        assert!(same_content(&ra, &ra.clone()));
    }

    #[test]
    fn dedupe_distinguishes_different_images() {
        let a = Detected::Image(111);
        let b = Detected::Image(222);
        assert!(!same_content(&a, &b));
        assert!(same_content(&a, &Detected::Image(111)));
    }

    #[test]
    fn poll_interval_steps_up_when_idle() {
        assert_eq!(poll_interval_ms(0), 300);
        assert_eq!(poll_interval_ms(19), 300);
        assert_eq!(poll_interval_ms(20), 700);
        assert_eq!(poll_interval_ms(49), 700);
        assert_eq!(poll_interval_ms(50), 1400);
        assert_eq!(poll_interval_ms(10_000), 1400);
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
        // 新版库存相对路径；同时塞一条旧版绝对路径记录，验证 basename 归一兼容
        let new_entry = crate::store::NewEntry {
            kind: crate::models::EntryType::Image,
            content: "[图片]".into(),
            content_hash: Some(1),
            rtf_data: None,
            image_path: Some("images/kept.png".into()),
            thumb_path: None,
            file_paths: vec![],
            source_app: None,
            created_at: 1,
            last_used_at: 1,
        };
        store.insert(&new_entry).unwrap();
        let legacy_entry = crate::store::NewEntry {
            kind: crate::models::EntryType::Image,
            content: "[图片]".into(),
            content_hash: Some(2),
            rtf_data: None,
            image_path: Some(dir.join("images/legacy.png").to_string_lossy().into_owned()),
            thumb_path: None,
            file_paths: vec![],
            source_app: None,
            created_at: 1,
            last_used_at: 1,
        };
        store.insert(&legacy_entry).unwrap();
        std::fs::write(images.join("legacy.png"), b"x").unwrap();

        cleanup_orphan_images(&dir, &store);
        assert!(!orphan.exists(), "orphan file should be removed");
        assert!(kept.exists(), "relative-referenced file should be kept");
        assert!(
            images.join("legacy.png").exists(),
            "absolute-path reference should be matched by basename"
        );
    }
}
