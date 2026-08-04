use crate::models::{Entry, EntryType};
use rusqlite::{params, Connection};
use std::path::Path;

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
}

fn entry_from_row(row: &rusqlite::Row) -> rusqlite::Result<Entry> {
    let kind: String = row.get("type")?;
    let file_paths: String = row.get("file_paths")?;
    Ok(Entry {
        id: row.get("id")?,
        kind: serde_json::from_str(&format!("\"{kind}\"")).unwrap(),
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
        kind: serde_json::from_str(&format!("\"{kind}\"")).unwrap(),
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

impl Store {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
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
            CREATE INDEX IF NOT EXISTS idx_entries_sort ON entries(pinned DESC, last_used_at DESC);
            CREATE INDEX IF NOT EXISTS idx_entries_type ON entries(type);",
        )?;
        // 旧库迁移：补充 content_hash 列（图片去重用）
        let has_col: bool = conn
            .prepare("SELECT 1 FROM pragma_table_info('entries') WHERE name = 'content_hash'")?
            .exists([])?;
        if !has_col {
            conn.execute_batch("ALTER TABLE entries ADD COLUMN content_hash INTEGER;")?;
        }
        Ok(Store { conn })
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

    /// 按图片内容 hash 查已存在的条目（图片去重专用）。
    pub fn find_by_content_hash(&self, hash: i64) -> rusqlite::Result<Option<i64>> {
        let mut stmt =
            self.conn
                .prepare("SELECT id FROM entries WHERE type = 'image' AND content_hash = ?1 LIMIT 1")?;
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
        self.query_light(
            &format!(
                "SELECT {LIGHT_COLUMNS} FROM entries ORDER BY pinned DESC, last_used_at DESC"
            ),
        )
    }

    pub fn list_pinned(&self) -> rusqlite::Result<Vec<Entry>> {
        self.query_light(&format!(
            "SELECT {LIGHT_COLUMNS} FROM entries WHERE pinned = 1 ORDER BY last_used_at DESC"
        ))
    }

    /// 获取单条完整记录（含 rtf_data），粘贴时使用。
    pub fn get(&self, id: i64) -> rusqlite::Result<Option<Entry>> {
        let mut stmt = self.conn.prepare("SELECT * FROM entries WHERE id = ?1")?;
        let mut rows = stmt.query_map(params![id], entry_from_row)?;
        rows.next().transpose()
    }

    /// 按类型与内容查找已有条目（用于全局去重）。
    pub fn find_by_content(
        &self,
        kind: EntryType,
        content: &str,
    ) -> rusqlite::Result<Option<i64>> {
        let kind_str = serde_json::to_string(&kind).unwrap().trim_matches('"').to_string();
        let mut stmt = self.conn.prepare(
            "SELECT id FROM entries WHERE type = ?1 AND content = ?2 LIMIT 1",
        )?;
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
        self.conn.execute("DELETE FROM entries WHERE pinned = 0", [])?;
        Ok(victims)
    }

    pub fn delete_by_type(&self, kind: EntryType) -> rusqlite::Result<usize> {
        let kind_str = serde_json::to_string(&kind).unwrap().trim_matches('"').to_string();
        self.conn
            .execute("DELETE FROM entries WHERE type = ?1", params![kind_str])
    }

    pub fn delete(&self, id: i64) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM entries WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// 删除最旧的非固定条目，直到剩余条数 <= limit。返回被删条目（用于清理图片文件）。
    pub fn prune(&self, limit: i64) -> rusqlite::Result<Vec<Entry>> {
        let mut out = vec![];
        loop {
            let count: i64 =
                self.conn.query_row("SELECT COUNT(*) FROM entries", [], |r| r.get(0))?;
            if count <= limit {
                break;
            }
            let victim: i64 = self.conn.query_row(
                "SELECT id FROM entries WHERE pinned = 0 ORDER BY last_used_at ASC LIMIT 1",
                [],
                |r| r.get(0),
            )?;
            if let Some(e) = self.get(victim)? {
                out.push(e);
            }
            self.conn
                .execute("DELETE FROM entries WHERE id = ?1", params![victim])?;
        }
        Ok(out)
    }

    fn query_light(&self, sql: &str) -> rusqlite::Result<Vec<Entry>> {
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map([], entry_from_row_light)?;
        rows.collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn tmp_store() -> Store {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let n = TMP_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir()
            .join(format!("clipboardx-test-{}-{}-{}", std::process::id(), nanos, n));
        std::fs::create_dir_all(&dir).unwrap();
        Store::open(&dir.join("test.db")).unwrap()
    }

    fn text_entry(content: &str, last_used: i64) -> NewEntry {
        NewEntry {
            kind: EntryType::Text,
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
    fn pinned_comes_first() {
        let s = tmp_store();
        let id_a = s.insert(&text_entry("aaa", 100)).unwrap();
        let _ = s.insert(&text_entry("bbb", 200)).unwrap();
        s.set_pinned(id_a, true).unwrap();
        let list = s.list().unwrap();
        assert_eq!(list[0].id, id_a);
    }

    #[test]
    fn prune_removes_oldest_non_pinned() {
        let s = tmp_store();
        let id_old = s.insert(&text_entry("old", 100)).unwrap();
        let _ = s.insert(&text_entry("new", 200)).unwrap();
        s.set_pinned(id_old, true).unwrap();
        let removed = s.prune(1).unwrap();
        assert!(removed.iter().any(|e| e.id != id_old));
        assert_eq!(s.list().unwrap().len(), 1);
    }

    #[test]
    fn find_by_content_dedupe_lookup() {
        let s = tmp_store();
        let id_a = s.insert(&text_entry("same", 100)).unwrap();
        let _ = s.insert(&text_entry("other", 200)).unwrap();
        let found = s.find_by_content(EntryType::Text, "same").unwrap();
        assert_eq!(found, Some(id_a));
        assert!(s.find_by_content(EntryType::Text, "nope").unwrap().is_none());
        assert!(s.find_by_content(EntryType::Url, "same").unwrap().is_none());
    }
}

#[cfg(test)]
mod extra_tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn tmp_store() -> Store {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let n = TMP_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir()
            .join(format!("clipboardx-test-{}-{}-{}", std::process::id(), nanos, n));
        std::fs::create_dir_all(&dir).unwrap();
        Store::open(&dir.join("test.db")).unwrap()
    }

    fn text_entry(content: &str, last_used: i64) -> NewEntry {
        NewEntry {
            kind: EntryType::Text,
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

    fn url_entry(content: &str, last_used: i64) -> NewEntry {
        NewEntry {
            kind: EntryType::Url,
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

    fn file_entry(paths: Vec<&str>, last_used: i64) -> NewEntry {
        NewEntry {
            kind: EntryType::File,
            content: paths.join(", "),
            content_hash: None,
            rtf_data: None,
            image_path: None,
            thumb_path: None,
            file_paths: paths.into_iter().map(String::from).collect(),
            source_app: Some("Test".into()),
            created_at: last_used,
            last_used_at: last_used,
        }
    }

    fn rtf_entry(data: Vec<u8>, last_used: i64) -> NewEntry {
        NewEntry {
            kind: EntryType::Rtf,
            content: "[富文本]".into(),
            content_hash: None,
            rtf_data: Some(data),
            image_path: None,
            thumb_path: None,
            file_paths: vec![],
            source_app: Some("Test".into()),
            created_at: last_used,
            last_used_at: last_used,
        }
    }

    fn image_entry(hash: i64, last_used: i64) -> NewEntry {
        NewEntry {
            kind: EntryType::Image,
            content: "[图片]".into(),
            content_hash: Some(hash),
            rtf_data: None,
            image_path: Some(format!("/tmp/{hash}.png")),
            thumb_path: Some(format!("/tmp/{hash}_thumb.png")),
            file_paths: vec![],
            source_app: Some("Test".into()),
            created_at: last_used,
            last_used_at: last_used,
        }
    }

    #[test]
    fn insert_and_list_all_types() {
        let s = tmp_store();
        let id_text = s.insert(&text_entry("hello", 100)).unwrap();
        let id_url = s.insert(&url_entry("https://example.com", 200)).unwrap();
        let id_file = s.insert(&file_entry(vec!["/tmp/a.txt"], 300)).unwrap();
        let id_rtf = s.insert(&rtf_entry(vec![0x7b, 0x5c, 0x72, 0x74, 0x66], 400)).unwrap();
        let id_img = s.insert(&image_entry(123, 500)).unwrap();

        let list = s.list().unwrap();
        assert_eq!(list.len(), 5);

        let img = list.iter().find(|e| e.kind == EntryType::Image).unwrap();
        assert_eq!(img.content_hash, Some(123));
        assert_eq!(img.image_path, Some("/tmp/123.png".to_string()));

        let file = list.iter().find(|e| e.kind == EntryType::File).unwrap();
        assert_eq!(file.file_paths, vec!["/tmp/a.txt".to_string()]);

        // rtf_data 属于大字段，light 查询不加载，需用 get 验证
        let rtf = s.get(id_rtf).unwrap().unwrap();
        assert_eq!(rtf.rtf_data, Some(vec![0x7b, 0x5c, 0x72, 0x74, 0x66]));

        // 其余 ID 均为有效记录
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

        let removed = s.prune(1).unwrap();
        assert!(removed.iter().any(|e| e.kind == EntryType::Image || e.kind == EntryType::Url));

        let list = s.list().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, id_pinned);
    }

    #[test]
    fn dedupe_for_each_type() {
        let s = tmp_store();

        // store.insert 本身不 dedupe；这里验证 find_by_content 能正确查找
        let id_text = s.insert(&text_entry("dup", 100)).unwrap();
        assert_eq!(s.find_by_content(EntryType::Text, "dup").unwrap(), Some(id_text));
        assert!(s.find_by_content(EntryType::Text, "missing").unwrap().is_none());

        let id_url = s.insert(&url_entry("https://same", 300)).unwrap();
        assert_eq!(s.find_by_content(EntryType::Url, "https://same").unwrap(), Some(id_url));

        let id_file = s.insert(&file_entry(vec!["/tmp/x.txt"], 500)).unwrap();
        assert_eq!(s.find_by_content(EntryType::File, "/tmp/x.txt").unwrap(), Some(id_file));

        let id_rtf = s.insert(&rtf_entry(vec![1, 2, 3], 700)).unwrap();
        assert_eq!(s.find_by_content(EntryType::Rtf, "[富文本]").unwrap(), Some(id_rtf));

        // 各类型之间 content 相同也不应互相匹配
        assert!(s.find_by_content(EntryType::Text, "[富文本]").unwrap().is_none());
        assert!(s.find_by_content(EntryType::Url, "dup").unwrap().is_none());
    }

    #[test]
    fn dedupe_image_by_content_hash() {
        let s = tmp_store();

        let id_a = s.insert(&image_entry(111, 100)).unwrap();
        s.find_by_content_hash(111).unwrap();
        assert_eq!(s.find_by_content_hash(111).unwrap(), Some(id_a));

        let id_b = s.insert(&image_entry(222, 300)).unwrap();
        assert_eq!(s.find_by_content_hash(222).unwrap(), Some(id_b));
        assert!(s.find_by_content_hash(333).unwrap().is_none());
    }

    #[test]
    fn all_image_paths_returns_both_paths() {
        let s = tmp_store();
        s.insert(&image_entry(1, 100)).unwrap();
        s.insert(&image_entry(2, 200)).unwrap();

        let paths = s.all_image_paths().unwrap();
        assert!(paths.contains(&"/tmp/1.png".to_string()));
        assert!(paths.contains(&"/tmp/1_thumb.png".to_string()));
        assert!(paths.contains(&"/tmp/2.png".to_string()));
        assert!(paths.contains(&"/tmp/2_thumb.png".to_string()));
    }
}
