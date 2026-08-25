//! 轻量文件日志 + 全局 panic 钩子。
//!
//! 目的：排查「应用有时自动退出」的问题。
//! - 所有日志（UTC 时间戳）追加写入 `<app_data_dir>/clipboardx.log`
//!   （macOS 上为 `~/Library/Application Support/com.clipboardx.app/clipboardx.log`）。
//! - 全局 panic 钩子把任何 panic 的位置、消息与回溯写入日志。配合 release 的
//!   `panic = "unwind"`，后台线程 panic 也能留下完整记录（abort 模式下进程直接
//!   消失、无迹可查，很可能是「自动退出」的元凶）。
//! - `init` 前仅输出到 stderr（dev 终端可见）。

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

/// 全局日志文件路径。未 `init` 之前为 `None`（此时只输出到 stderr）。
static LOG_FILE: Mutex<Option<PathBuf>> = Mutex::new(None);

/// 日志文件超过该大小后，下次启动时轮转（clipboardx.log -> clipboardx.log.old）。
const ROTATE_BYTES: u64 = 10 * 1024 * 1024;

/// 当前 UTC 时间字符串（秒级 + 毫秒）。用 UTC 便于与 macOS 崩溃报告时间戳对照。
fn now_str() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let total_secs = now.as_secs();
    let millis = now.subsec_millis();
    // civil_from_days（Howard Hinnant 算法）：把 Unix 秒换算成 UTC 日历时间，零依赖
    let z = total_secs.div_euclid(86_400);
    let rem = total_secs.rem_euclid(86_400);
    let (hh, mm, ss) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    format!("{y:04}-{mo:02}-{d:02} {hh:02}:{mm:02}:{ss:02}.{millis:03} UTC")
}

fn log_path() -> Option<PathBuf> {
    LOG_FILE.lock().unwrap_or_else(|p| p.into_inner()).clone()
}

/// 写一行日志：文件 + stderr 双写。任何一步失败都忽略（日志不应影响主流程）。
fn write_line(level: &str, msg: &str) {
    let line = format!("[{level}] {} {msg}\n", now_str());
    eprint!("{line}");
    if let Some(path) = log_path() {
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
            let _ = f.write_all(line.as_bytes());
            let _ = f.flush();
        }
    }
}

pub fn info(msg: impl AsRef<str>) {
    write_line("INFO", msg.as_ref());
}

pub fn error(msg: impl AsRef<str>) {
    write_line("ERROR", msg.as_ref());
}

/// 初始化日志文件（同时安装 panic 钩子）。在 setup 阶段调用一次。
pub fn init(path: PathBuf) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    // 轮转：上次日志过大时改名保留
    if std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0) > ROTATE_BYTES {
        let _ = std::fs::rename(&path, path.with_extension("log.old"));
    }
    *LOG_FILE.lock().unwrap_or_else(|p| p.into_inner()) = Some(path);
    install_panic_hook();
}

/// 安装全局 panic 钩子：把 panic 位置、消息与回溯写入日志。幂等，可重复调用。
/// 应在任何可能 panic 的代码之前尽早调用（`run()` 开头 + `init` 内各一次）。
pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let loc = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "?".to_string());
        let payload = info.payload();
        let msg = if let Some(s) = payload.downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = payload.downcast_ref::<String>() {
            s.clone()
        } else {
            "non-string panic payload".to_string()
        };
        // force_capture 不依赖 RUST_BACKTRACE 环境变量；hook 内不可再 panic
        let bt = std::backtrace::Backtrace::force_capture();
        error(&format!("PANIC at {loc}: {msg}\n{bt}"));
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);
    // LOG_FILE 是全局的，串行化依赖它的测试避免并行竞态
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn tmp_log_path() -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let n = TMP_COUNTER.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!(
            "clipboardx-log-test-{}-{}-{}.log",
            std::process::id(),
            nanos,
            n
        ))
    }

    #[test]
    fn init_writes_and_appends() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let path = tmp_log_path();
        init(path.clone());
        info("first line");
        info("second line");
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("first line"));
        assert!(content.contains("second line"));
        assert!(content.contains("INFO"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn panic_hook_writes_to_log() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let path = tmp_log_path();
        init(path.clone());
        // panic 发生在子线程，不影响测试进程本身
        let handle = std::thread::spawn(|| {
            panic!("test panic for log");
        });
        assert!(handle.join().is_err());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("PANIC at"));
        assert!(content.contains("test panic for log"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn now_str_is_formatted() {
        let s = now_str();
        assert!(s.len() >= 23, "unexpected format: {s}");
        assert!(s.ends_with("UTC"));
        // "YYYY-MM-DD HH:MM:SS.mmm UTC"
        assert_eq!(&s[4..5], "-", "year-month separator: {s}");
        assert_eq!(&s[7..8], "-", "month-day separator: {s}");
        assert_eq!(&s[10..11], " ", "date-time separator: {s}");
        assert_eq!(&s[13..14], ":", "hour-minute separator: {s}");
    }
}
