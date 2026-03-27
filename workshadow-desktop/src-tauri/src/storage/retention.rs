use super::db::Database;
use super::segments::SegmentManager;
use std::time::{SystemTime, UNIX_EPOCH};

/// Enforces retention policies by deleting old data.
pub struct RetentionManager {
    db: Database,
    segments: SegmentManager,
    max_days: u32,
    max_bytes: u64,
}

impl RetentionManager {
    pub fn new(db: Database, segments: SegmentManager, max_days: u32, max_storage_gb: f64) -> Self {
        Self {
            db,
            segments,
            max_days,
            max_bytes: (max_storage_gb * 1_073_741_824.0) as u64,
        }
    }

    /// Run cleanup: delete oldest data if age or size limits are exceeded.
    /// Returns the number of frames deleted.
    pub fn run_cleanup(&self) -> Result<usize, Box<dyn std::error::Error>> {
        let mut total_deleted = 0;

        // 1. Delete frames older than max_days
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_millis() as u64;
        let cutoff_ms = now_ms - (self.max_days as u64 * 86_400_000);

        let deleted = self.db.delete_frames_in_range(0, cutoff_ms)?;
        total_deleted += deleted;
        if deleted > 0 {
            log::info!("Retention: deleted {} frames older than {} days", deleted, self.max_days);
        }

        // 2. Delete oldest segments if disk usage exceeds max
        let disk_usage = self.segments.total_disk_usage();
        if disk_usage > self.max_bytes {
            let excess = disk_usage - self.max_bytes;
            log::info!(
                "Retention: disk usage {}MB exceeds limit {}MB, need to free {}MB",
                disk_usage / 1_048_576,
                self.max_bytes / 1_048_576,
                excess / 1_048_576
            );

            let mut freed: u64 = 0;
            for segment_path in self.segments.list_segments() {
                if freed >= excess {
                    break;
                }
                if let Ok(meta) = std::fs::metadata(&segment_path) {
                    freed += meta.len();
                    self.segments.delete_segment(&segment_path).ok();
                    total_deleted += 1;
                }
            }
        }

        Ok(total_deleted)
    }
}
