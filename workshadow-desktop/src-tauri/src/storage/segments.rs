use std::fs;
use std::path::{Path, PathBuf};

/// Manages video segment files on disk.
pub struct SegmentManager {
    data_dir: PathBuf,
}

impl SegmentManager {
    pub fn new(data_dir: PathBuf) -> Self {
        fs::create_dir_all(&data_dir).ok();
        Self { data_dir }
    }

    /// List all segment files sorted by name (chronological).
    pub fn list_segments(&self) -> Vec<PathBuf> {
        let mut segments: Vec<PathBuf> = fs::read_dir(&self.data_dir)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map_or(false, |ext| ext == "mkv"))
            .collect();
        segments.sort();
        segments
    }

    /// Get total disk usage of all segments in bytes.
    pub fn total_disk_usage(&self) -> u64 {
        self.list_segments()
            .iter()
            .filter_map(|p| fs::metadata(p).ok())
            .map(|m| m.len())
            .sum()
    }

    /// Delete a specific segment file.
    pub fn delete_segment(&self, path: &Path) -> std::io::Result<()> {
        // Also delete the sidecar .jsonl if it exists
        let sidecar = path.with_extension("jsonl");
        if sidecar.exists() {
            fs::remove_file(&sidecar)?;
        }
        fs::remove_file(path)
    }

    /// Get the data directory path.
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }
}
