use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEvent {
    CaptureStarted { session_id: i64 },
    CaptureStopped { session_id: i64 },
    CapturePaused { session_id: i64 },
    CaptureResumed { session_id: i64 },
    DataDeleted { start_ms: u64, end_ms: u64, frames_deleted: usize },
    SettingsChanged { field: String, old_value: String, new_value: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    timestamp: String,
    event: AuditEvent,
}

/// Append-only audit log for capture and data events.
pub struct AuditLog {
    log_path: PathBuf,
}

impl AuditLog {
    pub fn new(log_dir: &Path) -> Self {
        fs::create_dir_all(log_dir).ok();
        Self {
            log_path: log_dir.join("audit.jsonl"),
        }
    }

    /// Append an event to the audit log.
    pub fn log_event(&self, event: AuditEvent) -> std::io::Result<()> {
        let entry = AuditLogEntry {
            timestamp: Utc::now().to_rfc3339(),
            event,
        };

        let json = serde_json::to_string(&entry)?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;

        writeln!(file, "{}", json)?;
        Ok(())
    }

    /// Read all audit log entries.
    pub fn read_all(&self) -> std::io::Result<Vec<AuditLogEntry>> {
        let content = fs::read_to_string(&self.log_path)?;
        let entries: Vec<AuditLogEntry> = content
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();
        Ok(entries)
    }
}
