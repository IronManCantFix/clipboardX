use crate::models::{now_ms, Entry, EntryType};
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone)]
pub struct NewEntry {
    pub kind: EntryType,
    pub content: String,
    pub content_hash: Option<i64>,
    pub rtf_data: Option<Vec<u8>>,
    pub image_path: Option<String>,
    pub thumb_path: Option<String>,
    pub file_paths: Vec<String>,
    pub source_app: Option<String>,
    pub created_at: i64,
    pub last_used_at: i64,
}

pub struct Store {
    conn: Connection,
    path: PathBuf,
}

fn entry_from_row(row: &rusqlite::Row) -> rusqlite::Result<Entry> {
    let kind: String = row.get("type")?;
    let file_paths: String = row.get("file_paths")?;
    Ok(Entry {
        id: row.get("id")?,
        kind: EntryType::from_db_str(&kind).unwrap_or(EntryType::Text),
        content: row.get("content")?,
        content_hash: row.get("content_hash")?,
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

/// 列表查询使用：跳过 rtf_data 大字段，降低每次刷新弹窗的内存与 IO 开销。
fn entry_from_row_light(row: &rusqlite::Row) -> rusqlite::Result<Entry> {
    let kind: String = row.get("type")?;
    let file_paths: String = row.get("file_paths")?;
    Ok(Entry {
        id: row.get("id")?,
        kind: EntryType::from_db_str(&kind).unwrap_or(EntryType::Text),
        content: row.get("content")?,
        content_hash: row.get("content_hash")?,
        rtf_data: None,
        image_path: row.get("image_path")?,
        thumb_path: row.get("thumb_path")?,
        file_paths: serde_json::from_str(&file_paths).unwrap_or_default(),
        source_app: row.get("source_app")?,
        pinned: row.get::<_, i64>("pinned")? != 0,
        created_at: row.get("created_at")?,
        last_used_at: row.get("last_used_at")?,
    })
}

const LIGHT_COLUMNS: &str =
    "id, type, content, content_hash, image_path, thumb_path, file_paths, source_app, pinned, created_at, last_used_at";

// ---- Schema 迁移 ----

type ApplyFn = fn(&Connection) -> rusqlite::Result<()>;

struct Migration {
    /// 迁移完成后对应的 PRAGMA user_version（严格递增，只追加不修改）
    version: i64,
    apply: ApplyFn,
}

/// 表/column 名均来自编译期字面量，拼接无注入风险。
fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    ddl: &str,
) -> rusqlite::Result<()> {
    let has_col: bool = conn
        .prepare(&format!(
            "SELECT 1 FROM pragma_table_info('{table}') WHERE name = '{column}'"
        ))?
        .exists([])?;
    if !has_col {
        conn.execute_batch(ddl)?;
    }
    Ok(())
}

/// 线性迁移列表：每次 schema 变更在末尾追加一条，不改写历史条目。
const MIGRATIONS: &[Migration] = &[
    // v1：初始 schema。所有建表/索引语句都是幂等的（IF NOT EXISTS），
    // 在 open() 里作为 bootstrap 无条件执行，这里仅确立版本基线。
    Migration {
        version: 1,
        apply: |_| Ok(()),
    },
    // v2：content_hash 列（图片/富文本内容哈希去重）。
    // 中间历史版本的库可能已用旧的散装逻辑补过该列，幂等检查保证不重复添加。
    Migration {
        version: 2,
        apply: |c| {
            add_column_if_missing(c, "entries", "content_hash",
                "ALTER TABLE entries ADD COLUMN content_hash INTEGER")
        },
    },
];

impl Store {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        // 启动时回收上次异常退出残留的 WAL 文件（新库时为空操作，开销可忽略）
        let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
        // 幂等 bootstrap：新库直接建出最新结构；旧库存在的对象全部 IF NOT EXISTS 跳过。
        // 后续增量变更走下方线性迁移，不再往里堆语句。
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                type TEXT NOT NULL,
                content TEXT NOT NULL,
                content_hash INTEGER,
                rtf_data BLOB,
                image_path TEXT,
                thumb_path TEXT,
                file_paths TEXT NOT NULL DEFAULT '[]',
                source_app TEXT,
                pinned INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                last_used_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_entries_sort ON entries(last_used_at DESC);
            CREATE INDEX IF NOT EXISTS idx_entries_type ON entries(type);",
        )?;
        // 按 user_version 线性执行迁移（如 ALTER ADD COLUMN 这类无法幂等表达的语句）
        let mut current: i64 =
            conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))?;
        for m in MIGRATIONS {
            if current >= m.version {
                continue;
            }
            (m.apply)(&conn)?;
            conn.pragma_update(None, "user_version", m.version)?;
            current = m.version;
        }
        Ok(Store {
            conn,
            path: path.to_path_buf(),
        })
    }

    /// 主数据库文件超过阈值时，在后台线程做一次 VACUUM，回收被 prune 释放的空间。
    /// 使用独立连接并重试，避开与主连接活动事务的短暂 BUSY（WAL 模式下可自愈）。
    /// 带 24h 冷却窗口：带 UP 标签的重启风暴期间不必每次启动都全量整理，
    /// 未成功（重试耗尽）时窗口过后会再次尝试。
    pub fn vacuum_if_large(&self, threshold: u64) {
        let size = std::fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0);
        if size <= threshold {
            return;
        }
        const VACUUM_COOLDOWN_MS: u64 = 24 * 60 * 60 * 1000;
        let now = now_ms().max(0) as u64;
        let last = LAST_VACUUM_ATTEMPT_MS.load(Ordering::Relaxed);
        if last != 0 && now.saturating_sub(last) < VACUUM_COOLDOWN_MS {
            println!("[clipboardx] vacuum skipped (cooldown)");
            return;
        }
        LAST_VACUUM_ATTEMPT_MS.store(now, Ordering::Relaxed);
        let path = self.path.clone();
        std::thread::spawn(move || {
            for _ in 0..6 {
                let Ok(conn) = Connection::open(&path) else {
                    break;
                };
                let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
                let done = conn.execute_batch("VACUUM;").is_ok();
                drop(conn);
                if done {
                    println!("[clipboardx] database vacuumed");
                    break;
                }
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
        });
    }

    pub fn insert(&self, e: &NewEntry) -> rusqlite::Result<i64> {
        self.conn.execute(
            "INSERT INTO entries (type, content, content_hash, rtf_data, image_path, thumb_path, file_paths, source_app, created_at, last_used_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                serde_json::to_string(&e.kind).unwrap().trim_matches('"'),
                e.content,
                e.content_hash,
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

    /// 按内容 hash 查已存在的条目（图片按 TIFF/PNG 字节 hash、RTF 按 RTF 数据 hash；
    /// hash 为 64 位，跨类型碰撞概率可忽略，故不再限定 type）。
    pub fn find_by_content_hash(&self, hash: i64) -> rusqlite::Result<Option<i64>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM entries WHERE content_hash = ?1 LIMIT 1")?;
        let mut rows = stmt.query_map(params![hash], |r| r.get(0))?;
        rows.next().transpose()
    }

    /// 返回数据库当前引用的全部图片文件路径（原图 + 缩略图）。
    pub fn all_image_paths(&self) -> rusqlite::Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT image_path, thumb_path FROM entries WHERE type = 'image'")?;
        let rows = stmt.query_map([], |r| {
            let a: Option<String> = r.get(0)?;
            let b: Option<String> = r.get(1)?;
            Ok(vec![a, b])
        })?;
        let mut out = vec![];
        for row in rows {
            for p in row?.into_iter().flatten() {
                out.push(p);
            }
        }
        Ok(out)
    }

    pub fn list(&self) -> rusqlite::Result<Vec<Entry>> {
        self.query_light(&format!(
            "SELECT {LIGHT_COLUMNS} FROM entries ORDER BY last_used_at DESC, id DESC"
        ))
    }

    pub fn list_pinned(&self) -> rusqlite::Result<Vec<Entry>> {
        self.query_light(&format!(
            "SELECT {LIGHT_COLUMNS} FROM entries WHERE pinned = 1 ORDER BY last_used_at DESC, id DESC"
        ))
    }

    /// 获取单条完整记录（含 rtf_data），粘贴时使用。
    pub fn get(&self, id: i64) -> rusqlite::Result<Option<Entry>> {
        let mut stmt = self.conn.prepare("SELECT * FROM entries WHERE id = ?1")?;
        let mut rows = stmt.query_map(params![id], entry_from_row)?;
        rows.next().transpose()
    }

    /// 按类型与内容查找已有条目（用于无 hash 类型——文本/URL/文件——的全局去重）。
    pub fn find_by_content(&self, kind: EntryType, content: &str) -> rusqlite::Result<Option<i64>> {
        let kind_str = serde_json::to_string(&kind)
            .unwrap()
            .trim_matches('"')
            .to_string();
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM entries WHERE type = ?1 AND content = ?2 LIMIT 1")?;
        let mut rows = stmt.query_map(params![kind_str, content], |r| r.get(0))?;
        rows.next().transpose()
    }

    pub fn set_pinned(&self, id: i64, pinned: bool) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE entries SET pinned = ?1 WHERE id = ?2",
            params![pinned as i64, id],
        )?;
        Ok(())
    }

    pub fn touch(&self, id: i64, at: i64) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE entries SET last_used_at = ?1 WHERE id = ?2",
            params![at, id],
        )?;
        Ok(())
    }

    pub fn count(&self) -> rusqlite::Result<i64> {
        self.conn
            .query_row("SELECT COUNT(*) FROM entries", [], |r| r.get(0))
    }

    /// 删除全部非固定条目，返回被删条目（用于清理图片文件）。
    pub fn delete_non_pinned(&self) -> rusqlite::Result<Vec<Entry>> {
        let victims = self.query_light(&format!(
            "SELECT {LIGHT_COLUMNS} FROM entries WHERE pinned = 0"
        ))?;
        self.conn
            .execute("DELETE FROM entries WHERE pinned = 0", [])?;
        Ok(victims)
    }

    /// 删除全部非固定图片条目，固定图片保留，返回被删条目（用于清理图片文件）。
    pub fn delete_images_non_pinned(&self) -> rusqlite::Result<Vec<Entry>> {
        let victims = self.query_light(&format!(
            "SELECT {LIGHT_COLUMNS} FROM entries WHERE type = 'image' AND pinned = 0"
        ))?;
        self.conn.execute(
            "DELETE FROM entries WHERE type = 'image' AND pinned = 0",
            [],
        )?;
        Ok(victims)
    }

    pub fn delete(&self, id: i64) -> rusqlite::Result<()> {
        self.conn
            .execute("DELETE FROM entries WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// 删除最旧的非固定条目，直到剩余条数 <= limit。返回被删条目（用于清理图片文件）。
    pub fn prune(&self, limit: i64) -> rusqlite::Result<Vec<Entry>> {
        // 一次删除超出 limit 的最旧非固定条目（count <= limit 时删除 0 行）。
        // 同毫秒插入的多条以 id 升序视为「更旧」，保证 FIFO 稳定。
        let sql = format!(
            "DELETE FROM entries
             WHERE id IN (
                 SELECT id FROM entries
                 WHERE pinned = 0
                 ORDER BY last_used_at ASC, id ASC
                 LIMIT (SELECT MAX(0, COUNT(*) - ?1) FROM entries WHERE pinned = 0)
             )
             RETURNING {LIGHT_COLUMNS}"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![limit], entry_from_row_light)?;
        rows.collect()
    }

    fn query_light(&self, sql: &str) -> rusqlite::Result<Vec<Entry>> {
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map([], entry_from_row_light)?;
        rows.collect()
    }
}

/// VACUUM 冷却窗口记录（毫秒时间戳），见 vacuum_if_large。
static LAST_VACUUM_ATTEMPT_MS: AtomicU64 = AtomicU64::new(0);

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn tmp_dir(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let n = TMP_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "clipboardx-{tag}-{}-{}-{}",
            std::process::id(),
            nanos,
            n
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn tmp_store() -> Store {
        Store::open(&tmp_dir("test").join("test.db")).unwrap()
    }

    fn new_entry(kind: EntryType, content: &str, last_used: i64) -> NewEntry {
        NewEntry {
            kind,
            content: content.into(),
            content_hash: None,
            rtf_data: None,
            image_path: None,
            thumb_path: None,
            file_paths: vec![],
            source_app: Some("Test".into()),
            created_at: last_used,
            last_used_at: last_used,
        }
    }

    fn text_entry(content: &str, last_used: i64) -> NewEntry {
        new_entry(EntryType::Text, content, last_used)
    }

    fn url_entry(content: &str, last_used: i64) -> NewEntry {
        new_entry(EntryType::Url, content, last_used)
    }

    fn file_entry(paths: Vec<&str>, last_used: i64) -> NewEntry {
        let mut e = new_entry(EntryType::File, &paths.join(", "), last_used);
        e.file_paths = paths.into_iter().map(String::from).collect();
        e
    }

    fn rtf_entry(data: Vec<u8>, text: &str, last_used: i64) -> NewEntry {
        let mut e = new_entry(EntryType::Rtf, text, last_used);
        e.rtf_data = Some(data);
        e.content_hash = Some(hash_of(&e.rtf_data.clone().unwrap()));
        e
    }

    fn image_entry(hash: i64, last_used: i64) -> NewEntry {
        let mut e = new_entry(EntryType::Image, "[图片]", last_used);
        e.content_hash = Some(hash);
        e.image_path = Some(format!("images/{hash}.png"));
        e.thumb_path = Some(format!("images/{hash}_thumb.png"));
        e
    }

    fn hash_of(data: &[u8]) -> i64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        data.hash(&mut h);
        h.finish() as i64
    }

    #[test]
    fn insert_and_list_sorted_by_last_used() {
        let s = tmp_store();
        s.insert(&text_entry("aaa", 100)).unwrap();
        s.insert(&text_entry("bbb", 200)).unwrap();
        let list = s.list().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].content, "bbb");
    }

    #[test]
    fn same_timestamp_ties_broken_by_id_desc() {
        let s = tmp_store();
        let a = s.insert(&text_entry("aaa", 100)).unwrap();
        let b = s.insert(&text_entry("bbb", 100)).unwrap();
        let list = s.list().unwrap();
        assert_eq!(list[0].id, b);
        assert_eq!(list[1].id, a);
    }

    #[test]
    fn pinned_not_special_ordered() {
        let s = tmp_store();
        let id_a = s.insert(&text_entry("aaa", 100)).unwrap();
        let _ = s.insert(&text_entry("bbb", 200)).unwrap();
        s.set_pinned(id_a, true).unwrap();
        let list = s.list().unwrap();
        // 固定项不置顶，按 last_used_at DESC 排序：bbb(200) 在前，aaa(100) 在后
        assert_eq!(list[0].content, "bbb");
        assert_eq!(list[1].content, "aaa");
    }

    #[test]
    fn prune_removes_oldest_non_pinned() {
        let s = tmp_store();
        let id_pinned = s.insert(&text_entry("pinned", 100)).unwrap();
        s.set_pinned(id_pinned, true).unwrap();
        s.insert(&text_entry("old", 200)).unwrap();
        s.insert(&text_entry("new", 300)).unwrap();
        // limit=1：非固定项有 2 条，删除最旧的 "old"
        let removed = s.prune(1).unwrap();
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].content, "old");
        // 固定项不受 prune 影响
        let list = s.list().unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.iter().any(|e| e.id == id_pinned));
    }

    #[test]
    fn find_by_content_dedupe_lookup() {
        let s = tmp_store();
        let id_a = s.insert(&text_entry("same", 100)).unwrap();
        let _ = s.insert(&text_entry("other", 200)).unwrap();
        let found = s.find_by_content(EntryType::Text, "same").unwrap();
        assert_eq!(found, Some(id_a));
        assert!(s.find_by_content(EntryType::Text, "nope").unwrap().is_none());
        assert!(s
            .find_by_content(EntryType::Url, "same")
            .unwrap()
            .is_none());
    }

    #[test]
    fn migration_recovers_legacy_db_without_content_hash() {
        // 模拟最早期版本的库：没有 content_hash 列、user_version=0
        let dir = tmp_dir("migration");
        let db_path = dir.join("legacy.db");
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute_batch(
                "CREATE TABLE entries (
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
                );",
            )
            .unwrap();
        }
        let s = Store::open(&db_path).unwrap();
        // 旧库迁移后可以正常读列表、写入带 hash 的新条目（不产生重复列错误）
        assert_eq!(s.list().unwrap().len(), 0);
        let img = image_entry(7, 100);
        s.insert(&img).unwrap();
        let list = s.list().unwrap();
        assert_eq!(list[0].content_hash, Some(7));
        assert_eq!(list[0].image_path.as_deref(), Some("images/7.png"));
    }

    #[test]
    fn insert_and_list_all_types() {
        let s = tmp_store();
        let id_text = s.insert(&text_entry("hello", 100)).unwrap();
        let id_url = s.insert(&url_entry("https://example.com", 200)).unwrap();
        let id_file = s.insert(&file_entry(vec!["/tmp/a.txt"], 300)).unwrap();
        let id_rtf = s
            .insert(&rtf_entry(vec![0x7b, 0x5c, 0x72, 0x74, 0x66], "正文", 400))
            .unwrap();
        let id_img = s.insert(&image_entry(123, 500)).unwrap();

        let list = s.list().unwrap();
        assert_eq!(list.len(), 5);

        let img = list.iter().find(|e| e.kind == EntryType::Image).unwrap();
        assert_eq!(img.content_hash, Some(123));
        assert_eq!(img.image_path, Some("images/123.png".to_string()));

        let file = list.iter().find(|e| e.kind == EntryType::File).unwrap();
        assert_eq!(file.file_paths, vec!["/tmp/a.txt".to_string()]);

        // RTF 条目的 content 是真实纯文本（供搜索与回退粘贴使用）
        let rtf_row = list.iter().find(|e| e.id == id_rtf).unwrap();
        assert_eq!(rtf_row.content, "正文");

        // rtf_data 属于大字段，light 查询不加载，需用 get 验证
        let rtf = s.get(id_rtf).unwrap().unwrap();
        assert_eq!(rtf.rtf_data, Some(vec![0x7b, 0x5c, 0x72, 0x74, 0x66]));

        assert!(s.get(id_text).unwrap().is_some());
        assert!(s.get(id_url).unwrap().is_some());
        assert!(s.get(id_file).unwrap().is_some());
        assert!(s.get(id_img).unwrap().is_some());
    }

    #[test]
    fn prune_keeps_pinned_across_types() {
        let s = tmp_store();
        let id_pinned = s.insert(&text_entry("pinned", 100)).unwrap();
        s.set_pinned(id_pinned, true).unwrap();
        s.insert(&image_entry(1, 200)).unwrap();
        s.insert(&url_entry("https://x.dev", 300)).unwrap();

        // limit=1：非固定项有 2 条，删除最旧的 image(200)
        let removed = s.prune(1).unwrap();
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].kind, EntryType::Image);

        // 固定项 + 剩余 1 条非固定项 = 2 条
        let list = s.list().unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.iter().any(|e| e.id == id_pinned));
    }

    #[test]
    fn dedupe_for_each_type() {
        let s = tmp_store();

        // 无 hash 类型走 find_by_content
        let id_text = s.insert(&text_entry("dup", 100)).unwrap();
        assert_eq!(
            s.find_by_content(EntryType::Text, "dup").unwrap(),
            Some(id_text)
        );
        assert!(s
            .find_by_content(EntryType::Text, "missing")
            .unwrap()
            .is_none());

        let id_url = s.insert(&url_entry("https://same", 300)).unwrap();
        assert_eq!(
            s.find_by_content(EntryType::Url, "https://same").unwrap(),
            Some(id_url)
        );

        let id_file = s.insert(&file_entry(vec!["/tmp/x.txt"], 500)).unwrap();
        assert_eq!(
            s.find_by_content(EntryType::File, "/tmp/x.txt").unwrap(),
            Some(id_file)
        );

        // 有 hash 类型（RTF/图片）走 find_by_content_hash；相同 hash 只命中同一条
        let a = s.insert(&rtf_entry(vec![1, 2, 3], "t", 700)).unwrap();
        let dup = rtf_entry(vec![1, 2, 3], "t", 800);
        assert_eq!(
            s.find_by_content_hash(dup.content_hash.unwrap()).unwrap(),
            Some(a)
        );
        assert_ne!(a, s.insert(&dup).unwrap());

        let id_img_a = s.insert(&image_entry(111, 900)).unwrap();
        assert_eq!(s.find_by_content_hash(111).unwrap(), Some(id_img_a));

        // 不同类型之间互不影响
        assert!(s.find_by_content(EntryType::Text, "[富文本]").unwrap().is_none());
        assert!(s.find_by_content(EntryType::Url, "dup").unwrap().is_none());
        assert!(s.find_by_content_hash(999).unwrap().is_none());
    }

    #[test]
    fn all_image_paths_returns_both_paths() {
        let s = tmp_store();
        s.insert(&image_entry(1, 100)).unwrap();
        s.insert(&image_entry(2, 200)).unwrap();

        let paths = s.all_image_paths().unwrap();
        assert!(paths.contains(&"images/1.png".to_string()));
        assert!(paths.contains(&"images/1_thumb.png".to_string()));
        assert!(paths.contains(&"images/2.png".to_string()));
        assert!(paths.contains(&"images/2_thumb.png".to_string()));
    }
}
