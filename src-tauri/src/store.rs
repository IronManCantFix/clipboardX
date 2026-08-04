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
            "SELECT * FROM entries WHERE LOWER(content) LIKE ?1 ORDER BY pinned DESC, last_used_at DESC",
        )?;
        let rows = stmt.query_map([like], entry_from_row)?;
        rows.collect()
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

    pub fn delete_by_type(&self, kind: EntryType) -> rusqlite::Result<usize> {
        let kind_str = serde_json::to_string(&kind).unwrap().trim_matches('"').to_string();
        self.conn
            .execute("DELETE FROM entries WHERE type = ?1", params![kind_str])
    }

    pub fn delete(&self, id: i64) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM entries WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// 删除最旧的非固定条目，直到剩余条数 <= limit。返回被删条目 id。
    pub fn prune(&self, limit: i64) -> rusqlite::Result<Vec<i64>> {
        let mut out = vec![];
        loop {
            let count: i64 =
                self.conn.query_row("SELECT COUNT(*) FROM entries", [], |r| r.get(0))?;
            if count <= limit {
                break;
            }
            let victim = self.conn.query_row(
                "SELECT id FROM entries WHERE pinned = 0 ORDER BY last_used_at ASC LIMIT 1",
                [],
                |r| r.get(0),
            )?;
            self.conn
                .execute("DELETE FROM entries WHERE id = ?1", params![victim])?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_store() -> Store {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir()
            .join(format!("clipboardx-test-{}-{}", std::process::id(), nanos));
        std::fs::create_dir_all(&dir).unwrap();
        Store::open(&dir.join("test.db")).unwrap()
    }

    fn text_entry(content: &str, last_used: i64) -> NewEntry {
        NewEntry {
            kind: EntryType::Text,
            content: content.into(),
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
        assert!(removed.iter().any(|&id| id != id_old));
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
