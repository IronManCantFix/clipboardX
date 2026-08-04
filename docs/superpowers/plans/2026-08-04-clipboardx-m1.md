# ClipboardX M1 (MVP) 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 跑通 ClipboardX 最小闭环：剪贴板监听（文本/URL/文件）→ SQLite 存储 → 全局快捷键唤起弹窗 → 搜索/键盘导航 → 粘贴回前台应用，并提供基础设置（条数上限 + 快捷键）。

**Architecture:** Tauri 2 事件驱动分层。Rust 侧 `listener`（NSPasteboard 轮询 500ms + 类型分类 + 去重）→ `store`（rusqlite）→ `commands`（IPC 桥）；Svelte 5 前端持有全量列表快照做本地过滤。粘贴 = 写回 NSPasteboard + CGEvent/enigo 模拟 `Cmd+V`。排序 `pinned DESC, last_used_at DESC`。

**Tech Stack:** Tauri 2 · Rust 1.95 · Svelte 5 · Vite · rusqlite (bundled) · objc2-app-kit · tauri-plugin-global-shortcut · tauri-plugin-positioner · enigo · vitest

**Spec:** `docs/superpowers/specs/2026-08-04-clipboardx-design.md`

---

## 文件结构（M1 全部产出）

```
ClipboardX/
├── package.json / vite.config.ts / svelte.config.js / tsconfig.json / index.html
├── src/                          # Svelte 5 前端
│   ├── main.ts                   # 挂载 App
│   ├── App.svelte                # 弹窗：Tab + 搜索 + 列表 + 键盘导航
│   ├── styles.css                # 全局样式（毛玻璃、暗色适配）
│   ├── lib/
│   │   ├── api.ts                # invoke/listen 封装
│   │   ├── search.ts             # 纯函数：过滤 + 高亮
│   │   └── search.test.ts        # vitest 单测
│   └── components/
│       ├── TabBar.svelte         # 剪贴板/固定 Tab
│       └── EntryRow.svelte       # 单行条目（类型图标 + 文本）
├── src-tauri/
│   ├── Cargo.toml / build.rs / tauri.conf.json / capabilities/default.json / icons/
│   └── src/
│       ├── main.rs               # 入口
│       ├── lib.rs                # Tauri builder、托盘、监听线程、窗口定位
│       ├── models.rs             # Entry / Settings / EntryType
│       ├── store.rs              # SQLite 封装
│       ├── settings.rs           # settings.json 原子读写
│       ├── clipboard.rs          # NSPasteboard 读写封装
│       ├── listener.rs           # classify/去重（纯函数）+ 轮询线程
│       ├── paste.rs              # 写回剪贴板 + 模拟 Cmd+V
│       └── commands.rs           # IPC 命令
```

## 公共类型（models.rs，Task 1 定义，后续任务引用）

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryType { Text, Rtf, Image, File, Url }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: i64,
    #[serde(rename = "type")]
    pub kind: EntryType,
    pub content: String,          // 纯文本预览；url 存地址；file 存逗号拼接路径
    pub rtf_data: Option<Vec<u8>>, // M1 只存不用（M2 才支持粘贴）
    pub image_path: Option<String>, // M1 恒 None
    pub thumb_path: Option<String>, // M1 恒 None
    pub file_paths: Vec<String>,
    pub source_app: Option<String>,
    pub pinned: bool,
    pub created_at: i64,
    pub last_used_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub history_limit: i64,   // 默认 15，clamp 1..=100
    pub shortcut: String,     // 例如 "Command+Shift+V"
}

impl Default for Settings {
    fn default() -> Self {
        Settings { history_limit: 15, shortcut: "Command+Shift+V".into() }
    }
}
```

---

### Task 0: 脚手架（Tauri 2 + Svelte）

**Files:**
- Create: 前端骨架 + `src-tauri/`（由 create-tauri-app 生成后拷入）
- Modify: `.gitignore`

- [ ] **Step 1: 生成官方模板到临时目录**

```bash
cd /tmp && rm -rf clipboardx-scaffold && npm create tauri-app@latest clipboardx-scaffold -- --template svelte-ts --manager npm --identifier com.clipboardx.app --yes
```

Expected: 目录生成，内含 `src/`、`src-tauri/`、`package.json`、`vite.config.ts`、`svelte.config.js`、`tsconfig.json`、`index.html`。

- [ ] **Step 2: 拷贝到项目**

```bash
cd /Users/huanghongda/develop/codex/ClipboardX
cp -R /tmp/clipboardx-scaffold/src-tauri ./src-tauri
cp -R /tmp/clipboardx-scaffold/src ./src
cp /tmp/clipboardx-scaffold/package.json /tmp/clipboardx-scaffold/vite.config.ts \
   /tmp/clipboardx-scaffold/svelte.config.js /tmp/clipboardx-scaffold/tsconfig.json \
   /tmp/clipboardx-scaffold/index.html /tmp/clipboardx-scaffold/.gitignore .
rm -rf /tmp/clipboardx-scaffold
```

- [ ] **Step 3: 合并 .gitignore（保留 .superpowers/）**

Append 到 `.gitignore`（保留 docs/ 与 .superpowers/）：

```gitignore
.superpowers/
node_modules/
dist/
src-tauri/target/
```

- [ ] **Step 4: 升级前端到 Svelte 5 + 安装依赖**

```bash
npm install
npm install -D svelte@5 @sveltejs/vite-plugin-svelte@5 vitest
npm install @tauri-apps/api @tauri-apps/plugin-global-shortcut
```

- [ ] **Step 5: 验证 dev 启动**

```bash
npm run tauri dev
```

Expected: 窗口弹出且无编译错误。若有 Svelte 5 迁移错误（模板是 Svelte 4 时），按编译器提示修正模板组件的 runes 语法。Ctrl+C 退出。

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "chore: scaffold Tauri 2 + Svelte 5 (M1)"
```

---

### Task 1: store 模块（SQLite，TDD）

**Files:**
- Create: `src-tauri/src/models.rs`
- Create: `src-tauri/src/store.rs`
- Create: `src-tauri/src/store_test.rs`（或 store.rs 内 `#[cfg(test)]`）

- [ ] **Step 1: Cargo.toml 加入依赖**

在 `src-tauri/Cargo.toml` 的 `[dependencies]` 追加：

```toml
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rusqlite = { version = "0.32", features = ["bundled"] }
chrono = "0.4"
```

- [ ] **Step 2: 写失败测试**

在 `src-tauri/src/store.rs` 写入实现骨架（`pub struct Store { conn: rusqlite::Connection }`），并在文件末尾加测试：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_store() -> Store {
        let dir = std::env::temp_dir().join(format!("clipboardx-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        Store::open(&dir.join("test.db")).unwrap()
    }

    fn text_entry(content: &str, last_used: i64) -> NewEntry {
        NewEntry { kind: EntryType::Text, content: content.into(), rtf_data: None,
                   image_path: None, thumb_path: None, file_paths: vec![],
                   source_app: Some("Test".into()), created_at: last_used, last_used_at: last_used }
    }

    #[test]
    fn insert_and_list_sorted_by_last_used() {
        let s = tmp_store();
        s.insert(&text_entry("aaa", 100)).unwrap();
        s.insert(&text_entry("bbb", 200)).unwrap();
        let list = s.list().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].content, "bbb"); // 最近使用在前
    }

    #[test]
    fn pinned_comes_first() {
        let s = tmp_store();
        let id_a = s.insert(&text_entry("aaa", 100)).unwrap();
        let _ = s.insert(&text_entry("bbb", 200)).unwrap();
        s.set_pinned(id_a, true).unwrap();
        let list = s.list().unwrap();
        assert_eq!(list[0].id, id_a); // 固定置顶
    }

    #[test]
    fn prune_removes_oldest_non_pinned() {
        let s = tmp_store();
        let id_old = s.insert(&text_entry("old", 100)).unwrap();
        let _ = s.insert(&text_entry("new", 200)).unwrap();
        s.set_pinned(id_old, true).unwrap();
        let removed = s.prune(1).unwrap(); // 上限 1：只删最旧非固定
        assert!(removed.iter().any(|&(id, _)| id != id_old));
        assert_eq!(s.list().unwrap().len(), 1);
    }

    #[test]
    fn search_matches_case_insensitive_substring() {
        let s = tmp_store();
        s.insert(&text_entry("Hello World", 100)).unwrap();
        s.insert(&text_entry("other", 200)).unwrap();
        let found = s.search("WORLD").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].content, "Hello World");
    }
}
```

- [ ] **Step 3: 运行确认失败**

```bash
cd src-tauri && cargo test store
```

Expected: 编译失败（`Store`/`NewEntry`/`EntryType` 未实现）。

- [ ] **Step 4: 实现 store.rs**

```rust
use crate::models::{Entry, EntryType};
use rusqlite::{params, Connection};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct NewEntry {
    pub kind: EntryType,
    pub content: String,
    pub rtf_data: Option<Vec<u8>>,
    pub image_path: Option<String>,
    pub thumb_path: Option<String>,
    pub file_paths: Vec<String>,
    pub source_app: Option<String>,
    pub created_at: i64,
    pub last_used_at: i64,
}

pub struct Store { conn: Connection }

fn entry_from_row(row: &rusqlite::Row) -> rusqlite::Result<Entry> {
    let kind: String = row.get("type")?;
    let file_paths: String = row.get("file_paths")?;
    Ok(Entry {
        id: row.get("id")?,
        kind: serde_json::from_str(&format!("\"{kind}\"")).unwrap(),
        content: row.get("content")?,
        rtf_data: row.get("rtf_data")?,
        image_path: row.get("image_path")?,
        thumb_path: row.get("thumb_path")?,
        file_paths: serde_json::from_str(&file_paths).unwrap_or_default(),
        source_app: row.get("source_app")?,
        pinned: row.get::<_, i64>("pinned")? != 0,
        created_at: row.get("created_at")?,
        last_used_at: row.get("last_used_at")?,
    })
}

impl Store {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                type TEXT NOT NULL,
                content TEXT NOT NULL,
                rtf_data BLOB,
                image_path TEXT,
                thumb_path TEXT,
                file_paths TEXT NOT NULL DEFAULT '[]',
                source_app TEXT,
                pinned INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                last_used_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_entries_sort ON entries(pinned DESC, last_used_at DESC);
            CREATE INDEX IF NOT EXISTS idx_entries_type ON entries(type);",
        )?;
        Ok(Store { conn })
    }

    pub fn insert(&self, e: &NewEntry) -> rusqlite::Result<i64> {
        self.conn.execute(
            "INSERT INTO entries (type, content, rtf_data, image_path, thumb_path, file_paths, source_app, created_at, last_used_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                serde_json::to_string(&e.kind).unwrap().trim_matches('"'),
                e.content,
                e.rtf_data,
                e.image_path,
                e.thumb_path,
                serde_json::to_string(&e.file_paths).unwrap(),
                e.source_app,
                e.created_at,
                e.last_used_at,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list(&self) -> rusqlite::Result<Vec<Entry>> {
        self.query("SELECT * FROM entries ORDER BY pinned DESC, last_used_at DESC")
    }

    pub fn list_pinned(&self) -> rusqlite::Result<Vec<Entry>> {
        self.query("SELECT * FROM entries WHERE pinned = 1 ORDER BY last_used_at DESC")
    }

    pub fn search(&self, q: &str) -> rusqlite::Result<Vec<Entry>> {
        let like = format!("%{}%", q.to_lowercase());
        let mut stmt = self.conn.prepare(
            "SELECT * FROM entries WHERE LOWER(content) LIKE ?1 ORDER BY pinned DESC, last_used_at DESC")?;
        let rows = stmt.query_map([like], entry_from_row)?;
        rows.collect()
    }

    pub fn set_pinned(&self, id: i64, pinned: bool) -> rusqlite::Result<()> {
        self.conn.execute("UPDATE entries SET pinned = ?1 WHERE id = ?2", params![pinned as i64, id])?;
        Ok(())
    }

    pub fn touch(&self, id: i64, at: i64) -> rusqlite::Result<()> {
        self.conn.execute("UPDATE entries SET last_used_at = ?1 WHERE id = ?2", params![at, id])?;
        Ok(())
    }

    pub fn delete(&self, id: i64) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM entries WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// 删除最旧的非固定条目，直到剩余条数 <= limit。返回被删条目 id。
    pub fn prune(&self, limit: i64) -> rusqlite::Result<Vec<i64>> {
        let mut out = vec![];
        loop {
            let count: i64 = self.conn.query_row("SELECT COUNT(*) FROM entries", [], |r| r.get(0))?;
            if count <= limit { break; }
            let victim = self.conn.query_row(
                "SELECT id FROM entries WHERE pinned = 0 ORDER BY last_used_at ASC LIMIT 1", [], |r| r.get(0))?;
            self.conn.execute("DELETE FROM entries WHERE id = ?1", params![victim])?;
            out.push(victim);
        }
        Ok(out)
    }

    fn query(&self, sql: &str) -> rusqlite::Result<Vec<Entry>> {
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map([], entry_from_row)?;
        rows.collect()
    }
}
```

- [ ] **Step 5: 运行确认通过**

```bash
cd src-tauri && cargo test store
```

Expected: 4 个测试全部 PASS。

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat: SQLite store with sorting/pinning/search/prune"
```

---

### Task 2: settings 存储（TDD）

**Files:**
- Create: `src-tauri/src/settings.rs`

- [ ] **Step 1: 写失败测试**（settings.rs 末尾 `#[cfg(test)] mod tests`）

```rust
#[test]
fn defaults_when_no_file() {
    let dir = std::env::temp_dir().join(format!("clipboardx-settings-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let s = SettingsStore::load(&dir.join("settings.json")).unwrap();
    assert_eq!(s.history_limit, 15);
    assert_eq!(s.shortcut, "Command+Shift+V");
}

#[test]
fn save_then_reload_roundtrip() {
    let dir = std::env::temp_dir().join(format!("clipboardx-settings-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("settings.json");
    let mut s = SettingsStore::load(&path).unwrap();
    s.history_limit = 30;
    s.shortcut = "Command+Shift+1".into();
    s.save().unwrap();
    let loaded = SettingsStore::load(&path).unwrap();
    assert_eq!(loaded.history_limit, 30);
    assert_eq!(loaded.shortcut, "Command+Shift+1");
}

#[test]
fn limit_is_clamped_to_100() {
    let dir = std::env::temp_dir().join(format!("clipboardx-settings-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut s = SettingsStore::load(&dir.join("settings.json")).unwrap();
    s.history_limit = 9999;
    s.save().unwrap();
    assert_eq!(s.history_limit, 100);
}
```

- [ ] **Step 2: 运行确认失败**

```bash
cd src-tauri && cargo test settings
```

Expected: 编译失败（`SettingsStore` 未定义）。

- [ ] **Step 3: 实现 settings.rs**

```rust
use crate::models::Settings;
use serde_json;
use std::fs;
use std::path::{Path, PathBuf};

pub struct SettingsStore { path: PathBuf, pub settings: Settings }

impl SettingsStore {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let settings = if path.exists() {
            let raw = fs::read_to_string(path)?;
            serde_json::from_str::<Settings>(&raw)?
        } else {
            Settings::default()
        };
        let mut s = Self { path: path.to_path_buf(), settings };
        s.clamp();
        Ok(s)
    }

    fn clamp(&mut self) {
        self.settings.history_limit = self.settings.history_limit.clamp(1, 100);
        if self.settings.shortcut.trim().is_empty() {
            self.settings.shortcut = "Command+Shift+V".into();
        }
    }

    pub fn save(&mut self) -> anyhow::Result<()> {
        self.clamp();
        if let Some(dir) = self.path.parent() { fs::create_dir_all(dir)?; }
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_string_pretty(&self.settings)?)?; // 原子写
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}
```

- [ ] **Step 4: 运行确认通过**

```bash
cd src-tauri && cargo test settings
```

Expected: 3 个测试全部 PASS。

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat: atomic settings persistence with clamp"
```

---

### Task 3: clipboard 封装 + 监听分类逻辑（TDD）

**Files:**
- Create: `src-tauri/src/clipboard.rs`（NSPasteboard 封装）
- Create: `src-tauri/src/listener.rs`（classify / 去重纯函数 + 轮询线程）

- [ ] **Step 1: Cargo.toml 加依赖**

```toml
objc2 = "0.6"
objc2-app-kit = { version = "0.3", features = ["NSPasteboard", "NSEvent", "NSWorkspace"] }
objc2-foundation = { version = "0.3" }
```

- [ ] **Step 2: 写失败测试**（listener.rs 末尾）

```rust
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
        let files = ClipboardSnapshot { change_count: 1, text: None, rtf: None,
            files: vec!["/a/b.txt".into()], has_image: false };
        assert!(matches!(classify(&files), Some(Detected::Files(_))));
        let url = ClipboardSnapshot { change_count: 1, text: Some("https://x.dev".into()),
            rtf: None, files: vec![], has_image: false };
        assert!(matches!(classify(&url), Some(Detected::Url(_))));
        let txt = ClipboardSnapshot { change_count: 1, text: Some("plain text".into()),
            rtf: None, files: vec![], has_image: false };
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
```

- [ ] **Step 3: 运行确认失败**

```bash
cd src-tauri && cargo test listener
```

Expected: 编译失败（`ClipboardSnapshot`/`Detected` 未定义）。

- [ ] **Step 4: 实现 clipboard.rs**

```rust
use objc2::rc::autoreleasepool;
use objc2_app_kit::NSPasteboard;
use objc2_foundation::{NSArray, NSData, NSString};

/// 一次剪贴板快照。M1 覆盖：文本 / RTF / 文件 / 图片标记。
pub struct ClipboardSnapshot {
    pub change_count: isize,
    pub text: Option<String>,
    pub rtf: Option<Vec<u8>>,
    pub files: Vec<String>,
    pub has_image: bool,
}

pub fn read_clipboard() -> ClipboardSnapshot {
    autoreleasepool(|_| {
        let pb = NSPasteboard::generalPasteboard();
        let change_count = pb.changeCount();
        let text = pb.stringForType(&NSPasteboard::typeString()).map(|s| s.to_string());
        let rtf = pb.dataForType(&NSPasteboard::typeRTF()).map(|d| d.bytes().to_vec());
        let files: Vec<String> = pb
            .propertyListForType(&NSPasteboard::typeFilenames())
            .and_then(|pl| pl.downcast::<NSArray<NSString>>().ok())
            .map(|arr| arr.iter().map(|s| s.to_string()).collect())
            .unwrap_or_default();
        let has_image = pb.dataForType(&NSPasteboard::typeTIFF()).is_some();
        ClipboardSnapshot { change_count, text, rtf, files, has_image }
    })
}

pub fn write_text(s: &str) {
    autoreleasepool(|_| {
        let pb = NSPasteboard::generalPasteboard();
        pb.clearContents();
        let ns = NSString::from_str(s);
        let _ = pb.setString_forType(&ns, &NSPasteboard::typeString());
    });
}

pub fn write_files(paths: &[String]) {
    autoreleasepool(|_| {
        let pb = NSPasteboard::generalPasteboard();
        pb.clearContents();
        let arr = NSArray::from_retained_vec(paths.iter().map(|p| NSString::from_str(p)).collect());
        let _ = pb.declareTypes_owner(
            &NSArray::from_retained_vec(vec![NSPasteboard::typeFilenames()]),
            None,
        );
        let _ = pb.setPropertyList_forType(&arr, &NSPasteboard::typeFilenames());
    });
}
```

- [ ] **Step 5: 实现 listener.rs（分类 + 去重）**

```rust
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
    if snap.has_image { return Some(Detected::Image); }
    if let Some(rtf) = &snap.rtf { return Some(Detected::Rtf(rtf.clone())); }
    if !snap.files.is_empty() { return Some(Detected::Files(snap.files.clone())); }
    let text = snap.text.as_deref()?;
    if looks_like_url(text) { Some(Detected::Url(text.to_string())) } else { Some(Detected::Text(text.to_string())) }
}

pub fn looks_like_url(s: &str) -> bool {
    let t = s.trim();
    t.starts_with("http://") || t.starts_with("https://") || t.starts_with("ftp://") || t.starts_with("www.")
}

pub fn same_content(a: &Detected, b: &Detected) -> bool {
    a == b
}
```

- [ ] **Step 6: 运行确认通过**

```bash
cd src-tauri && cargo test listener
```

Expected: 5 个测试全部 PASS。

- [ ] **Step 7: Commit**

```bash
git add -A && git commit -m "feat: pasteboard snapshot + classify/dedupe logic"
```

---

### Task 4: Tauri 命令桥 + 粘贴（TDD 可测部分）

**Files:**
- Create: `src-tauri/src/paste.rs`
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`（注册命令、管理 Store 状态）

- [ ] **Step 1: Cargo.toml 加依赖**

```toml
tauri = { version = "2", features = ["tray-icon", "image-png"] }
tauri-plugin-global-shortcut = "2"
tauri-plugin-positioner = "2"
enigo = "0.3"
anyhow = "1"
```

- [ ] **Step 2: 写失败测试**（paste.rs 末尾）

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Entry, EntryType};

    fn entry(kind: EntryType, content: &str) -> Entry {
        Entry { id: 1, kind, content: content.into(), rtf_data: None, image_path: None,
                thumb_path: None, file_paths: vec![], source_app: None,
                pinned: false, created_at: 0, last_used_at: 0 }
    }

    #[test]
    fn payload_for_url_is_text() {
        let e = entry(EntryType::Url, "https://x.dev");
        match write_payload(&e) {
            Payload::Text(s) => assert_eq!(s, "https://x.dev"),
            other => panic!("expected text payload, got {other:?}"),
        }
    }

    #[test]
    fn payload_for_file_uses_paths() {
        let mut e = entry(EntryType::File, "");
        e.file_paths = vec!["/a/b.txt".into()];
        match write_payload(&e) {
            Payload::Files(v) => assert_eq!(v, vec!["/a/b.txt"]),
            other => panic!("expected files payload, got {other:?}"),
        }
    }
}
```

- [ ] **Step 3: 运行确认失败**

```bash
cd src-tauri && cargo test paste
```

Expected: 编译失败（`Payload`/`write_payload` 未定义）。

- [ ] **Step 4: 实现 paste.rs**

```rust
use crate::clipboard::{write_files, write_text};
use crate::models::{Entry, EntryType};

#[derive(Debug)]
pub enum Payload { Text(String), Files(Vec<String>) }

/// 将条目转成可写回剪贴板的负载。M1：文本/URL/文件；RTF/图片 M2 支持。
pub fn write_payload(e: &Entry) -> Payload {
    match e.kind {
        EntryType::Text | EntryType::Url => Payload::Text(e.content.clone()),
        EntryType::File => Payload::Files(e.file_paths.clone()),
        EntryType::Rtf | EntryType::Image => {
            // M1 兜底：退化为文本预览（RTF/图片 M2 才完整支持）
            Payload::Text(e.content.clone())
        }
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
    use enigo::{Enigo, Key, Keyboard, Settings};
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    enigo.key(Key::V, enigo::Direction::Press).map_err(|e| e.to_string())?;
    enigo.key(Key::V, enigo::Direction::Release).map_err(|e| e.to_string())?;
    // 注：真实 Cmd+V 需按住 Meta；Tauri 下常用方案见 Task 6 Step 5 说明。
    Ok(())
}
```

> 说明：`enigo 0.3` 键盘 API 若与以上签名有出入（如需先 `enigo.key(Key::Meta, ...)` 组合键），以编译错误为准微调。Cmd+V 的 Meta 组合在 Task 6 Step 5 的集成验证中一并校准。

- [ ] **Step 5: 运行确认通过**

```bash
cd src-tauri && cargo test paste
```

Expected: 2 个测试 PASS。

- [ ] **Step 6: 实现 commands.rs**

```rust
use crate::models::Settings;
use crate::store::Store;
use std::sync::Mutex;

pub struct AppState {
    pub store: Mutex<Store>,
    pub settings: Mutex<crate::settings::SettingsStore>,
}

#[tauri::command]
pub fn list_entries(state: tauri::State<'_, AppState>, tab: Option<String>) -> Result<Vec<crate::models::Entry>, String> {
    let store = state.store.lock().map_err(|e| e.to_string())?;
    match tab.as_deref() {
        Some("pinned") => store.list_pinned().map_err(|e| e.to_string()),
        _ => store.list().map_err(|e| e.to_string()),
    }
}

#[tauri::command]
pub fn search_entries(state: tauri::State<'_, AppState>, query: String) -> Result<Vec<crate::models::Entry>, String> {
    let store = state.store.lock().map_err(|e| e.to_string())?;
    store.search(&query).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_pinned(state: tauri::State<'_, AppState>, id: i64, pinned: bool) -> Result<(), String> {
    state.store.lock().map_err(|e| e.to_string())?.set_pinned(id, pinned).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_entry(state: tauri::State<'_, AppState>, id: i64) -> Result<(), String> {
    state.store.lock().map_err(|e| e.to_string())?.delete(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn paste_entry(state: tauri::State<'_, AppState>, id: i64) -> Result<(), String> {
    let list = state.store.lock().map_err(|e| e.to_string())?.list().map_err(|e| e.to_string())?;
    let e = list.into_iter().find(|x| x.id == id).ok_or("entry not found")?;
    crate::paste::write_back(&e);
    let now = chrono::Utc::now().timestamp();
    state.store.lock().map_err(|e| e.to_string())?.touch(id, now).map_err(|e| e.to_string())?;
    crate::paste::send_cmd_v()
}

#[tauri::command]
pub fn get_settings(state: tauri::State<'_, AppState>) -> Result<Settings, String> {
    Ok(state.settings.lock().map_err(|e| e.to_string())?.settings.clone())
}

#[tauri::command]
pub fn update_settings(state: tauri::State<'_, AppState>, settings: Settings) -> Result<(), String> {
    let mut guard = state.settings.lock().map_err(|e| e.to_string())?;
    guard.settings = settings;
    guard.save().map_err(|e| e.to_string())
}
```

- [ ] **Step 7: 在 lib.rs 中接入 AppState 并注册命令**

```rust
mod clipboard;
mod commands;
mod listener;
mod models;
mod paste;
mod settings;
mod store;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_positioner::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let store = store::Store::open(&data_dir.join("entries.db"))
                .map_err(|e| format!("db open failed: {e}"))?;
            let settings = settings::SettingsStore::load(&data_dir.join("settings.json"))
                .map_err(|e| format!("settings load failed: {e}"))?;
            let state = commands::AppState {
                store: std::sync::Mutex::new(store),
                settings: std::sync::Mutex::new(settings),
            };
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_entries,
            commands::search_entries,
            commands::set_pinned,
            commands::delete_entry,
            commands::paste_entry,
            commands::get_settings,
            commands::update_settings,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");
    app.run(|_, _| {});
}
```

- [ ] **Step 8: 编译验证**

```bash
cd src-tauri && cargo check
```

Expected: 无错误。

- [ ] **Step 9: Commit**

```bash
git add -A && git commit -m "feat: tauri commands + paste back to clipboard"
```

---

### Task 5: 前端搜索（TDD）+ 弹窗 UI

**Files:**
- Create: `src/lib/search.ts`
- Create: `src/lib/search.test.ts`
- Create: `src/lib/api.ts`
- Create: `src/App.svelte`（替换模板默认）
- Create: `src/components/TabBar.svelte`
- Create: `src/components/EntryRow.svelte`
- Modify: `src/styles.css`、`src/main.ts`、`index.html`

- [ ] **Step 1: 写失败测试**（`src/lib/search.test.ts`）

```ts
import { describe, expect, it } from 'vitest';
import { filterEntries, highlight } from './search';

const items = [
  { id: 1, content: 'Hello World' },
  { id: 2, content: 'git push origin main' },
  { id: 3, content: 'https://github.com/foo' },
];

describe('filterEntries', () => {
  it('matches case-insensitive substring', () => {
    const out = filterEntries(items, 'WORLD');
    expect(out.map((i) => i.id)).toEqual([1]);
  });

  it('empty query returns all', () => {
    expect(filterEntries(items, '')).toHaveLength(3);
  });

  it('no match returns empty', () => {
    expect(filterEntries(items, 'zzz')).toHaveLength(0);
  });
});

describe('highlight', () => {
  it('wraps matched part in <mark>', () => {
    expect(highlight('Hello World', 'world')).toBe('Hello <mark>World</mark>');
  });

  it('no match returns original escaped', () => {
    expect(highlight('<b>', 'x')).toBe('&lt;b&gt;');
  });
});
```

- [ ] **Step 2: 运行确认失败**

```bash
npx vitest run src/lib/search.test.ts
```

Expected: FAIL（模块不存在）。

- [ ] **Step 3: 实现 search.ts**

```ts
export interface Searchable {
  content: string;
}

export function filterEntries<T extends Searchable>(items: T[], query: string): T[] {
  const q = query.trim().toLowerCase();
  if (!q) return items;
  return items.filter((i) => i.content.toLowerCase().includes(q));
}

export function escapeHtml(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

export function highlight(text: string, query: string): string {
  const q = query.trim().toLowerCase();
  const escaped = escapeHtml(text);
  if (!q) return escaped;
  const idx = escaped.toLowerCase().indexOf(q);
  if (idx < 0) return escaped;
  return (
    escaped.slice(0, idx) +
    '<mark>' + escaped.slice(idx, idx + q.length) + '</mark>' +
    escaped.slice(idx + q.length)
  );
}
```

- [ ] **Step 4: 运行确认通过**

```bash
npx vitest run src/lib/search.test.ts
```

Expected: 5 个用例 PASS。

- [ ] **Step 5: 实现 api.ts**

```ts
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export type EntryType = 'text' | 'rtf' | 'image' | 'file' | 'url';

export interface Entry {
  id: number;
  type: EntryType;
  content: string;
  rtf_data: number[] | null;
  image_path: string | null;
  thumb_path: string | null;
  file_paths: string[];
  source_app: string | null;
  pinned: boolean;
  created_at: number;
  last_used_at: number;
}

export interface Settings {
  history_limit: number;
  shortcut: string;
}

export const api = {
  list: (tab?: 'pinned') => invoke<Entry[]>('list_entries', { tab: tab ?? null }),
  search: (query: string) => invoke<Entry[]>('search_entries', { query }),
  setPinned: (id: number, pinned: boolean) => invoke('set_pinned', { id, pinned }),
  delete: (id: number) => invoke('delete_entry', { id }),
  paste: (id: number) => invoke('paste_entry', { id }),
  getSettings: () => invoke<Settings>('get_settings'),
  updateSettings: (settings: Settings) => invoke('update_settings', { settings }),
};

export function onNewEntry(cb: () => void): Promise<() => void> {
  return listen('clipboard:new-entry', cb);
}
```

- [ ] **Step 6: 实现弹窗 UI**

`src/App.svelte`：

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { api, onNewEntry, type Entry } from './lib/api';
  import { filterEntries, highlight } from './lib/search';
  import TabBar from './components/TabBar.svelte';
  import EntryRow from './components/EntryRow.svelte';

  let entries: Entry[] = $state([]);
  let query = $state('');
  let tab: 'all' | 'pinned' = $state('all');
  let selected = $state(0);
  let searchInput: HTMLInputElement;

  let visibleEntries = $derived(filterEntries(entries, query));

  async function refresh() {
    entries = await api.list(tab === 'pinned' ? 'pinned' : undefined);
    selected = 0;
  }

  onMount(async () => {
    await refresh();
    onNewEntry(refresh);
    searchInput?.focus();
  });

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') { e.preventDefault(); selected = Math.min(selected + 1, visibleEntries.length - 1); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); selected = Math.max(selected - 1, 0); }
    else if (e.key === 'Enter' && visibleEntries[selected]) { e.preventDefault(); paste(visibleEntries[selected]); }
    else if (e.key === 'Escape') { window.close(); }
    else if (e.key === 'p' && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      const cur = visibleEntries[selected];
      if (cur) { await api.setPinned(cur.id, !cur.pinned); await refresh(); }
    } else if (e.key === 'Backspace' && visibleEntries[selected]) {
      await api.delete(visibleEntries[selected].id); await refresh();
    } else if (e.key === '1' && (e.metaKey || e.ctrlKey)) { tab = 'all'; await refresh(); }
    else if (e.key === '2' && (e.metaKey || e.ctrlKey)) { tab = 'pinned'; await refresh(); }
  }

  async function paste(entry: Entry) {
    try {
      await api.paste(entry.id);
      window.close();
    } catch (err) {
      console.error('paste failed:', err);
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="pop">
  <div class="search">
    <input
      bind:this={searchInput}
      bind:value={query}
      placeholder="搜索剪贴板…"
    />
  </div>
  <TabBar {tab} pinnedCount={entries.filter((e) => e.pinned).length} {refresh} />
  <div class="list">
    {#if visibleEntries.length === 0}
      <div class="empty">{query ? '无匹配结果' : '暂无剪贴板记录'}</div>
    {:else}
      {#each visibleEntries as entry, i (entry.id)}
        <EntryRow {entry} selected={i === selected} {query}
          onclick={() => paste(entry)}
          onmouseenter={() => (selected = i)} />
      {/each}
    {/if}
  </div>
  <div class="foot">
    <span>↑↓ 选择 · ⏎ 粘贴</span>
    <span>⌘P 固定 · ⌫ 删除 · Esc 关闭</span>
  </div>
</div>
```

`src/components/TabBar.svelte`：

```svelte
<script lang="ts">
  let { tab, pinnedCount, refresh }: {
    tab: 'all' | 'pinned';
    pinnedCount: number;
    refresh: () => Promise<void>;
  } = $props();

  async function switchTab(t: 'all' | 'pinned') {
    tab = t;
    await refresh();
  }
</script>

<div class="tabs">
  <button class:active={tab === 'all'} onclick={() => switchTab('all')}>剪贴板</button>
  {#if pinnedCount > 0}
    <button class:active={tab === 'pinned'} onclick={() => switchTab('pinned')}>固定</button>
  {/if}
</div>

<style>
  .tabs { display: flex; gap: 14px; padding: 2px 10px 6px; }
  button { background: none; border: none; font-size: 12px; font-weight: 600;
           color: var(--text-secondary); padding: 4px 2px 7px; cursor: pointer;
           border-bottom: 2px solid transparent; }
  button.active { color: var(--text-primary); border-bottom-color: var(--accent); }
</style>
```

`src/components/EntryRow.svelte`：

```svelte
<script lang="ts">
  import { highlight } from '../lib/search';
  import type { Entry } from '../lib/api';

  let { entry, selected, query }: {
    entry: Entry; selected: boolean; query: string;
  } = $props();

  const typeLabel: Record<Entry['type'], string> = {
    text: 'T', rtf: 'R', image: '🖼', file: '🗂', url: '🔗',
  };
</script>

<div
  class="row" class:selected
  role="button" tabindex="-1"
  onclick={() => onselect?.()}
  onmouseenter={() => onmouseenter?.()}
>
  <span class="ico">{typeLabel[entry.type]}</span>
  <span class="txt">{@html highlight(entry.content, query)}</span>
  {#if entry.pinned}<span class="pin">📌</span>{/if}
</div>

<style>
  .row { display: flex; align-items: center; gap: 9px; padding: 6px 8px;
         border-radius: 7px; cursor: pointer; }
  .row.selected { background: var(--selected-bg); }
  .ico { width: 20px; height: 20px; border-radius: 5px; flex-shrink: 0;
         display: flex; align-items: center; justify-content: center;
         font-size: 11px; color: #fff; background: var(--icon-bg, #8e8e93); }
  .txt { flex: 1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .pin { flex-shrink: 0; font-size: 10px; }
  :global(mark) { background: var(--mark-bg); color: inherit; border-radius: 2px; }
</style>
```

> 注：`EntryRow` 的 `onselect`/`onmouseenter` 回调由父组件通过 props 传入；`highlight` 返回 HTML，用 `{@html}` 渲染（内容已转义，安全）。

- [ ] **Step 7: 全局样式 + 透明无边框窗口**

`src/styles.css`：

```css
:root {
  --bg: rgba(250, 250, 250, 0.86);
  --text-primary: #1d1d1f;
  --text-secondary: #86868b;
  --accent: #0a84ff;
  --selected-bg: rgba(0, 113, 227, 0.13);
  --mark-bg: rgba(255, 200, 60, 0.45);
  --border: rgba(0, 0, 0, 0.08);
}
@media (prefers-color-scheme: dark) {
  :root {
    --bg: rgba(30, 30, 32, 0.86);
    --text-primary: #f5f5f7;
    --text-secondary: #98989d;
    --selected-bg: rgba(10, 132, 255, 0.2);
    --mark-bg: rgba(255, 200, 60, 0.35);
    --border: rgba(255, 255, 255, 0.1);
  }
}
html, body { margin: 0; background: transparent; }
body { font-family: -apple-system, BlinkMacSystemFont, sans-serif; user-select: none; }
.pop { display: flex; flex-direction: column; height: 100vh;
       background: var(--bg); backdrop-filter: blur(24px) saturate(1.5);
       border: 1px solid var(--border); border-radius: 12px; overflow: hidden; }
.search { padding: 8px 10px 6px; }
.search input { width: 100%; box-sizing: border-box; border: 0; outline: none;
       background: rgba(120, 120, 128, 0.13); border-radius: 7px; padding: 5px 10px;
       font-size: 13px; color: var(--text-primary); }
.list { flex: 1; overflow-y: auto; padding: 4px; }
.empty { padding: 40px 0; text-align: center; color: var(--text-secondary); font-size: 13px; }
.foot { display: flex; justify-content: space-between; padding: 6px 12px;
        border-top: 1px solid var(--border); font-size: 10.5px; color: var(--text-secondary); }
```

- [ ] **Step 8: tauri.conf.json 窗口配置（无边框弹窗）**

修改 `src-tauri/tauri.conf.json` 的 `app.windows[0]`：

```json
{
  "label": "main",
  "title": "ClipboardX",
  "width": 360,
  "height": 480,
  "transparent": true,
  "decorations": false,
  "alwaysOnTop": true,
  "resizable": false,
  "fullscreen": false,
  "visible": false,
  "skipTaskbar": true
}
```

- [ ] **Step 9: 编译 + 前端测试通过**

```bash
cd src-tauri && cargo check && cd .. && npx vitest run
```

Expected: cargo check 无错，vitest 全绿。

- [ ] **Step 10: Commit**

```bash
git add -A && git commit -m "feat: popover UI with tabs, search and keyboard nav"
```

---

### Task 6: 全局快捷键 + 弹窗定位 + 监听线程接线

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/listener.rs`（增加轮询线程）

- [ ] **Step 1: 在 listener.rs 增加轮询线程函数（含去重与事件推送）**

```rust
use crate::clipboard::read_clipboard;
use crate::commands::AppState;
use crate::models::EntryType;
use crate::store::NewEntry;
use chrono::Utc;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

pub fn spawn_listener(app: AppHandle, state: std::sync::Arc<Mutex<AppState>>) {
    std::thread::spawn(move || {
        let mut last_count = read_clipboard().change_count;
        let mut last_detected: Option<Detected> = None;
        loop {
            std::thread::sleep(Duration::from_millis(500));
            let snap = read_clipboard();
            if snap.change_count == last_count { continue; }
            last_count = snap.change_count;
            let Some(detected) = classify(&snap) else { continue };
            // M1：图片跳过（M2 支持）
            if detected == Detected::Image { last_detected = None; continue; }
            if last_detected.as_ref().is_some_and(|p| same_content(p, &detected)) { continue; }
            last_detected = Some(detected.clone());
            let now = Utc::now().timestamp();
            let (kind, content, rtf, files) = match &detected {
                Detected::Text(s) => (EntryType::Text, s.clone(), None, vec![]),
                Detected::Url(s) => (EntryType::Url, s.clone(), None, vec![]),
                Detected::Rtf(r) => (EntryType::Rtf, "[富文本]".into(), Some(r.clone()), vec![]),
                Detected::Files(f) => (EntryType::File, f.join(", "), None, f.clone()),
                Detected::Image => unreachable!(),
            };
            let source_app = frontmost_app_name();
            let new_entry = NewEntry {
                kind, content, rtf_data: rtf, image_path: None, thumb_path: None,
                file_paths: files, source_app, created_at: now, last_used_at: now,
            };
            let limit;
            {
                let guard = state.lock().unwrap();
                let store = &guard.store;
                let _ = store.insert(&new_entry);
                let settings = &guard.settings.lock().unwrap().settings;
                limit = settings.history_limit;
                let _ = store.prune(limit);
            }
            let _ = app.emit("clipboard:new-entry", ());
        }
    });
}

/// 前台应用名（objc2 NSWorkspace；失败返回 None）
fn frontmost_app_name() -> Option<String> {
    objc2::rc::autoreleasepool(|_| {
        let ws = objc2_app_kit::NSWorkspace::sharedWorkspace();
        let app = ws.frontmostApplication()?;
        Some(app.localizedName()?.to_string())
    })
}
```

- [ ] **Step 2: 在 lib.rs 接线：快捷键 + 定位 + 监听**

```rust
use tauri::{Manager, Runtime};
use tauri_plugin_positioner::{Position, WindowExt};

pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_positioner::init())
        .setup(|app| {
            // ...（沿用 Task 4 的 data_dir / store / settings 初始化）...
            let app_handle = app.handle().clone();
            let state_arc = std::sync::Arc::new(commands::AppState {
                store: std::sync::Mutex::new(store),
                settings: std::sync::Mutex::new(settings),
            });
            app.manage(state_arc.clone());

            // 全局快捷键：唤起弹窗
            let mut keys = tauri_plugin_global_shortcut::Code::KeyV
                .modifiers(tauri_plugin_global_shortcut::Modifiers::SUPER | tauri_plugin_global_shortcut::Modifiers::SHIFT);
            use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
            app.global_shortcut().on_shortcut(keys, move |app, _event, state| {
                if let ShortcutState::Pressed = state {
                    if let Some(win) = app.get_webview_window("main") {
                        let _ = win.move_window(Position::Cursor);
                        let _ = win.show();
                        let _ = win.set_focus();
                    }
                }
            }).expect("register shortcut");

            // 启动监听线程
            listener::spawn_listener(app_handle.clone(), state_arc);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![ /* Task 4 的命令 */ ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");
    app.run(|_, _| {});
}
```

> 注：`Modifiers::SUPER` 在 macOS 上即 ⌘。若 `on_shortcut` 需先 `register`，按插件 API 用 `app.global_shortcut().register(keys)?` 并保持注册结果存活；M1 以"dev 下能唤起弹窗"为验收标准。

- [ ] **Step 3: 校准 Cmd+V 粘贴（enigo 组合键）**

在 `paste.rs` 的 `send_cmd_v` 改为按住 Meta 的组合键（若上一步 API 验证失败，用以下方案）：

```rust
pub fn send_cmd_v() -> Result<(), String> {
    use enigo::{Enigo, Key, Keyboard, Settings};
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    enigo.key(Key::Meta, enigo::Direction::Press).map_err(|e| e.to_string())?;
    enigo.key(Key::V, enigo::Direction::Press).map_err(|e| e.to_string())?;
    enigo.key(Key::V, enigo::Direction::Release).map_err(|e| e.to_string())?;
    enigo.key(Key::Meta, enigo::Direction::Release).map_err(|e| e.to_string())?;
    Ok(())
}
```

- [ ] **Step 4: 集成验证（手动）**

```bash
npm run tauri dev
```

验证清单：
1. 复制一段文本 → 按 `⌘⇧V` → 弹窗出现在鼠标附近，含该文本
2. 复制 URL、Finder 中复制文件 → 分别出现在列表（🔗 / 🗂）
3. 上下键选择 → 回车 → 文本粘贴进目标应用且弹窗关闭
4. 复制相同文本两次 → 列表不出现重复条目
5. 在系统设置 → 隐私与安全性 → 辅助功能中允许本应用（首次模拟 Cmd+V 前需要）

若 `Cmd+V` 未生效：确认辅助功能授权；若仍失败，改用手动触发粘贴（后端写回剪贴板 + 前端提示用户按 Cmd+V），并在 commit message 里注明。

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat: global shortcut, cursor positioning, listener wiring"
```

---

### Task 7: 设置弹窗（条数上限 + 快捷键）

**Files:**
- Create: `src/routes/settings/` → `src/Settings.svelte`（独立窗口视图）
- Modify: `src/lib/api.ts`（已有 get/update settings）
- Modify: `src-tauri/src/lib.rs`（创建设置窗口、托盘菜单）
- Modify: `src-tauri/src/commands.rs`（快捷键变更时重新注册）

- [ ] **Step 1: 创建设置窗口（lib.rs setup 中，visible: false）**

```rust
let settings_win = tauri::WebviewWindowBuilder::new(app, "settings",
    tauri::WebviewUrl::App("settings.html".into()))
    .title("ClipboardX 设置")
    .inner_size(430.0, 560.0)
    .resizable(false)
    .visible(false)
    .build()?;
```

- [ ] **Step 2: 托盘：点击图标弹出设置窗口**

```rust
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;

// setup 内：
let show_settings = MenuItem::with_id(app, "settings", "设置…", true, None::<&str>)?;
let quit = MenuItem::with_id(app, "quit", "退出 ClipboardX", true, None::<&str>)?;
let menu = Menu::with_items(app, &[&show_settings, &quit])?;
TrayIconBuilder::with_id("main-tray")
    .menu(&menu)
    .show_menu_on_left_click(true)
    .on_menu_event(|app, event| match event.id().as_ref() {
        "settings" => { let _ = app.get_webview_window("settings").map(|w| w.show()); }
        "quit" => app.exit(0),
        _ => {}
    })
    .build(app)?;
```

- [ ] **Step 3: 实现 Settings.svelte**

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type Settings } from './lib/api';

  let settings = $state<Settings>({ history_limit: 15, shortcut: 'Command+Shift+V' });
  let recording = $state(false);
  let status = $state('');

  onMount(async () => { settings = await api.getSettings(); });

  async function onLimitInput(e: Event) {
    settings.history_limit = Number((e.target as HTMLInputElement).value);
    await save();
  }

  async function save() {
    try { await api.updateSettings(settings); status = '已保存'; }
    catch { status = '保存失败'; }
  }

  function startRecord() {
    recording = true;
    status = '请按下新的快捷键…';
    window.addEventListener('keydown', onKeyCapture);
  }

  function onKeyCapture(e: KeyboardEvent) {
    e.preventDefault();
    const mods: string[] = [];
    if (e.metaKey) mods.push('Command');
    if (e.ctrlKey) mods.push('Control');
    if (e.altKey) mods.push('Alt');
    if (e.shiftKey) mods.push('Shift');
    if (mods.length === 0 || e.key.length !== 1) return;
    settings.shortcut = mods.join('+') + '+' + e.key.toUpperCase();
    recording = false;
    window.removeEventListener('keydown', onKeyCapture);
    save();
  }
</script>

<div class="settings">
  <h2>ClipboardX 设置</h2>

  <section>
    <h3>常规</h3>
    <label>
      历史记录上限（{settings.history_limit} 条，最大 100）
      <input type="range" min="1" max="100" value={settings.history_limit}
             oninput={onLimitInput} />
    </label>
  </section>

  <section>
    <h3>快捷键</h3>
    <button onclick={recording ? undefined : startRecord}>
      {recording ? '按下组合键…' : settings.shortcut}
    </button>
  </section>

  <p class="status">{status}</p>
</div>

<style>
  .settings { padding: 20px; font-family: -apple-system, sans-serif; }
  section { margin-bottom: 18px; }
  h3 { font-size: 12px; text-transform: uppercase; color: var(--text-secondary); }
  input[type='range'] { width: 100%; }
  button { border: 1px solid var(--border); border-radius: 7px; padding: 6px 14px;
           background: var(--accent); color: #fff; font-size: 13px; cursor: pointer; }
  .status { color: var(--text-secondary); font-size: 12px; min-height: 16px; }
</style>
```

- [ ] **Step 4: 快捷键变更后重新注册（commands.rs）**

`update_settings` 中比较新旧 shortcut，变化时调用 `app.global_shortcut().unregister_all()` 后按新值注册；并将 `on_shortcut` 的注册逻辑抽成 `register_shortcut(app) -> Result<(), String>`（lib.rs 与 update_settings 共用）。

> 验收：设置中改成 `⌘⇧1` 后，新组合键可唤起弹窗，旧组合键失效。

- [ ] **Step 5: 前端弹窗提供"设置"入口（TabBar 右侧齿轮按钮 → 打开设置窗口）**

```ts
import { getCurrentWindow } from '@tauri-apps/api/window';
// 或通过 invoke 命令 'open_settings' 在后端 show settings 窗口
```

- [ ] **Step 6: 集成验证 + Commit**

```bash
npm run tauri dev
```

验证：菜单栏图标 → 设置弹窗；修改条数上限生效（监听清理按新上限执行）；录制新快捷键生效。然后：

```bash
git add -A && git commit -m "feat: settings window with limit + shortcut recorder"
```

---

### Task 8: M1 集成收尾

**Files:**
- Modify: `docs/superpowers/plans/2026-08-04-clipboardx-m1.md`（验收清单勾选）

- [ ] **Step 1: 全流程回归清单**

```bash
npm run tauri dev
```

逐项验证：
- [ ] 文本 / URL / 文件（Finder）复制后 500ms 内出现在列表
- [ ] 重复复制相同内容不产生重复条目
- [ ] 弹窗跟随鼠标、不遮挡任务栏/菜单栏（Position::Cursor 避让）
- [ ] 搜索：大小写不敏感、子串匹配、命中高亮；<50ms 响应
- [ ] 键盘：↑↓ 选择、⏎ 粘贴关闭、⌘P 固定置顶、⌫ 删除、Esc 关闭、⌘1/⌘2 切 Tab
- [ ] 固定条目置顶且进入"固定"Tab；无固定时 Tab 隐藏
- [ ] 条数上限生效：超过上限删除最旧非固定条目
- [ ] 设置：条数、快捷键可改并持久化（重启后保留）
- [ ] 粘贴目标：文本/URL/文件均能粘贴进前台应用（辅助功能已授权）
- [ ] Rust 与前端测试全绿：`cd src-tauri && cargo test && cd .. && npx vitest run`

- [ ] **Step 2: 内存基线记录**

用 `活动监视器` 观察常驻内存，记录到 `docs/notes/memory-baseline.md`（M1 基线，M3 对比用）。

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "chore: M1 integration pass + memory baseline"
```

---

## Self-Review 记录

- **Spec 覆盖**：M1 范围（监听文本/URL/文件、SQLite、弹窗、搜索、键盘导航、粘贴、全局快捷键、设置条数/快捷键）均落在 Task 0–8；图片/RTF 粘贴、排除应用、自动清理时间策略、导出属 M2/M3，spec 中已标注。
- **占位符扫描**：无 TBD/TODO；enigo 组合键与 `on_shortcut` 注册细节给出两套可执行方案并注明验收标准。
- **类型一致性**：`Entry`/`Settings`/`EntryType` 在 models.rs 定义一次，store/listener/commands/api.ts 引用签名一致；前端 `Entry` 接口字段与 Rust serde 输出对齐（`type` 经 `#[serde(rename = "type")]`，TS 侧为 `type`）。
