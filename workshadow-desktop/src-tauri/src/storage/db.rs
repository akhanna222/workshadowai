use rusqlite::{Connection, Result, params};
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::capture::FrameMetadata;

/// Thread-safe database handle.
#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    /// Open (or create) the SQLite database at the given path.
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;

        // Enable WAL mode for concurrent reads
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch("PRAGMA synchronous=NORMAL;")?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.initialize_schema()?;
        Ok(db)
    }

    /// Open an in-memory database (for testing).
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.initialize_schema()?;
        Ok(db)
    }

    fn initialize_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS sessions (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                started_at  INTEGER NOT NULL,
                ended_at    INTEGER,
                total_frames INTEGER DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS frames (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id      INTEGER NOT NULL REFERENCES sessions(id),
                timestamp_ms    INTEGER NOT NULL,
                segment_file    TEXT NOT NULL,
                segment_offset  INTEGER NOT NULL,
                window_title    TEXT,
                app_id          TEXT,
                browser_url     TEXT,
                ocr_text        TEXT,
                pii_flags       TEXT,
                UNIQUE(session_id, timestamp_ms)
            );

            CREATE INDEX IF NOT EXISTS idx_frames_timestamp ON frames(timestamp_ms);
            CREATE INDEX IF NOT EXISTS idx_frames_app ON frames(app_id);
            CREATE INDEX IF NOT EXISTS idx_frames_window ON frames(window_title);

            CREATE TABLE IF NOT EXISTS retention_policy (
                id              INTEGER PRIMARY KEY,
                max_days        INTEGER DEFAULT 90,
                max_gb          REAL DEFAULT 50.0
            );

            INSERT OR IGNORE INTO retention_policy (id, max_days, max_gb) VALUES (1, 90, 50.0);
            ",
        )?;
        Ok(())
    }

    /// Create a new capture session and return its ID.
    pub fn create_session(&self, started_at_ms: u64) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO sessions (started_at) VALUES (?1)",
            params![started_at_ms as i64],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// End a capture session.
    pub fn end_session(&self, session_id: i64, ended_at_ms: u64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE sessions SET ended_at = ?1 WHERE id = ?2",
            params![ended_at_ms as i64, session_id],
        )?;
        Ok(())
    }

    /// Insert a captured frame with its OCR text.
    pub fn insert_frame(
        &self,
        session_id: i64,
        metadata: &FrameMetadata,
        segment_file: &str,
        segment_offset: i32,
        ocr_text: Option<&str>,
        pii_flags: Option<&str>,
    ) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO frames (session_id, timestamp_ms, segment_file, segment_offset, window_title, app_id, browser_url, ocr_text, pii_flags)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                session_id,
                metadata.timestamp_ms as i64,
                segment_file,
                segment_offset,
                metadata.active_window_title,
                metadata.app_bundle_id,
                metadata.browser_url,
                ocr_text,
                pii_flags,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Get frames in a time range for the timeline view.
    pub fn get_frames_in_range(&self, start_ms: u64, end_ms: u64) -> Result<Vec<FrameRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, timestamp_ms, segment_file, segment_offset, window_title, app_id, browser_url, ocr_text
             FROM frames WHERE timestamp_ms >= ?1 AND timestamp_ms <= ?2
             ORDER BY timestamp_ms ASC",
        )?;

        let rows = stmt.query_map(params![start_ms as i64, end_ms as i64], |row| {
            Ok(FrameRow {
                id: row.get(0)?,
                session_id: row.get(1)?,
                timestamp_ms: row.get::<_, i64>(2)? as u64,
                segment_file: row.get(3)?,
                segment_offset: row.get(4)?,
                window_title: row.get(5)?,
                app_id: row.get(6)?,
                browser_url: row.get(7)?,
                ocr_text: row.get(8)?,
            })
        })?;

        rows.collect()
    }

    /// Get total frame count.
    pub fn total_frames(&self) -> Result<u64> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM frames", [], |row| row.get(0))?;
        Ok(count as u64)
    }

    /// Get a single frame by ID.
    pub fn get_frame_by_id(&self, frame_id: i64) -> Result<Option<FrameRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, timestamp_ms, segment_file, segment_offset, window_title, app_id, browser_url, ocr_text
             FROM frames WHERE id = ?1",
        )?;

        let mut rows = stmt.query_map(params![frame_id], |row| {
            Ok(FrameRow {
                id: row.get(0)?,
                session_id: row.get(1)?,
                timestamp_ms: row.get::<_, i64>(2)? as u64,
                segment_file: row.get(3)?,
                segment_offset: row.get(4)?,
                window_title: row.get(5)?,
                app_id: row.get(6)?,
                browser_url: row.get(7)?,
                ocr_text: row.get(8)?,
            })
        })?;

        match rows.next() {
            Some(Ok(row)) => Ok(Some(row)),
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    }

    /// Get total session count.
    pub fn total_sessions(&self) -> Result<u64> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))?;
        Ok(count as u64)
    }

    /// Get the oldest and newest frame timestamps.
    pub fn frame_timestamp_range(&self) -> Result<(Option<u64>, Option<u64>)> {
        let conn = self.conn.lock().unwrap();
        let oldest: Option<i64> = conn
            .query_row("SELECT MIN(timestamp_ms) FROM frames", [], |row| row.get(0))
            .ok();
        let newest: Option<i64> = conn
            .query_row("SELECT MAX(timestamp_ms) FROM frames", [], |row| row.get(0))
            .ok();
        Ok((oldest.map(|v| v as u64), newest.map(|v| v as u64)))
    }

    /// Get aggregated app usage for a given day (start_ms..end_ms).
    /// Returns Vec<(app_id, frame_count)> sorted by count descending.
    pub fn get_app_usage(&self, start_ms: u64, end_ms: u64) -> Result<Vec<(String, u64)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT COALESCE(app_id, 'unknown'), COUNT(*) as cnt
             FROM frames
             WHERE timestamp_ms >= ?1 AND timestamp_ms <= ?2
             GROUP BY app_id
             ORDER BY cnt DESC
             LIMIT 20",
        )?;

        let rows = stmt.query_map(params![start_ms as i64, end_ms as i64], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u64))
        })?;

        rows.collect()
    }

    /// Get top window titles for a given day.
    pub fn get_top_windows(&self, start_ms: u64, end_ms: u64) -> Result<Vec<(String, u64)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT COALESCE(window_title, 'unknown'), COUNT(*) as cnt
             FROM frames
             WHERE timestamp_ms >= ?1 AND timestamp_ms <= ?2
             GROUP BY window_title
             ORDER BY cnt DESC
             LIMIT 20",
        )?;

        let rows = stmt.query_map(params![start_ms as i64, end_ms as i64], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u64))
        })?;

        rows.collect()
    }

    /// Delete frames in a time range (for user-initiated deletion).
    pub fn delete_frames_in_range(&self, start_ms: u64, end_ms: u64) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let deleted = conn.execute(
            "DELETE FROM frames WHERE timestamp_ms >= ?1 AND timestamp_ms <= ?2",
            params![start_ms as i64, end_ms as i64],
        )?;
        Ok(deleted)
    }
}

/// A row from the frames table.
#[derive(Debug, Clone)]
pub struct FrameRow {
    pub id: i64,
    pub session_id: i64,
    pub timestamp_ms: u64,
    pub segment_file: String,
    pub segment_offset: i32,
    pub window_title: Option<String>,
    pub app_id: Option<String>,
    pub browser_url: Option<String>,
    pub ocr_text: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_metadata() -> FrameMetadata {
        FrameMetadata {
            timestamp_ms: 1000,
            active_window_title: "Test Window".to_string(),
            app_bundle_id: "com.test.app".to_string(),
            browser_url: None,
            display_index: 0,
        }
    }

    #[test]
    fn test_create_session_and_insert_frame() {
        let db = Database::open_in_memory().unwrap();
        let session_id = db.create_session(1000).unwrap();
        assert!(session_id > 0);

        let frame_id = db
            .insert_frame(session_id, &test_metadata(), "segment_1000.mkv", 0, Some("hello world"), None)
            .unwrap();
        assert!(frame_id > 0);
    }

    #[test]
    fn test_get_frames_in_range() {
        let db = Database::open_in_memory().unwrap();
        let session_id = db.create_session(1000).unwrap();

        for i in 0..10 {
            let mut meta = test_metadata();
            meta.timestamp_ms = 1000 + i * 100;
            db.insert_frame(session_id, &meta, "seg.mkv", i as i32, Some("text"), None)
                .unwrap();
        }

        let frames = db.get_frames_in_range(1000, 1500).unwrap();
        assert_eq!(frames.len(), 6); // timestamps 1000,1100,1200,1300,1400,1500
    }

    #[test]
    fn test_delete_frames() {
        let db = Database::open_in_memory().unwrap();
        let session_id = db.create_session(1000).unwrap();

        for i in 0..5 {
            let mut meta = test_metadata();
            meta.timestamp_ms = 1000 + i * 100;
            db.insert_frame(session_id, &meta, "seg.mkv", i as i32, None, None)
                .unwrap();
        }

        let deleted = db.delete_frames_in_range(1000, 1200).unwrap();
        assert_eq!(deleted, 3);
        assert_eq!(db.total_frames().unwrap(), 2);
    }
}
