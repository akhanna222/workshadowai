pub mod db;
pub mod segments;
pub mod retention;

use serde::{Deserialize, Serialize};

/// Storage engine configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub data_dir: String,
    pub max_retention_days: u32,
    pub max_storage_gb: f64,
    pub cleanup_interval_hours: u32,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_dir: "~/.workshadow/data".to_string(),
            max_retention_days: 90,
            max_storage_gb: 50.0,
            cleanup_interval_hours: 1,
        }
    }
}

/// Storage usage statistics for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageUsage {
    pub total_frames: u64,
    pub total_sessions: u64,
    pub disk_usage_bytes: u64,
    pub oldest_frame_ms: Option<u64>,
    pub newest_frame_ms: Option<u64>,
}
