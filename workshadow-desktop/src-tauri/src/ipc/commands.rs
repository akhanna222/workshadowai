use serde::{Deserialize, Serialize};
use tauri::State;

use crate::capture::pipeline::CapturePipeline;
use crate::capture::CaptureStatus;
use crate::config::AppConfig;
use crate::privacy::exclusions::ExclusionFilter;
use crate::search::{SearchFilters, SearchResult};
use crate::storage::db::Database;
use crate::storage::StorageUsage;

/// Application state shared via Tauri managed state.
pub struct AppState {
    pub config: std::sync::Mutex<AppConfig>,
    pub pipeline: CapturePipeline,
    pub db: Database,
}

// ── Search ──

#[tauri::command]
pub fn search(
    query: String,
    filters: Option<SearchFilters>,
    _state: State<AppState>,
) -> Result<Vec<SearchResult>, String> {
    let _filters = filters.unwrap_or_default();
    // TODO: Wire to SearchIndex
    log::info!("Search query: {}", query);
    Ok(vec![])
}

// ── Timeline ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameSummary {
    pub frame_id: i64,
    pub timestamp_ms: u64,
    pub window_title: String,
    pub app_id: String,
    pub thumbnail_path: Option<String>,
}

#[tauri::command]
pub fn get_timeline_range(
    start_ms: u64,
    end_ms: u64,
    state: State<AppState>,
) -> Result<Vec<FrameSummary>, String> {
    let frames = state
        .db
        .get_frames_in_range(start_ms, end_ms)
        .map_err(|e| e.to_string())?;

    Ok(frames
        .into_iter()
        .map(|f| FrameSummary {
            frame_id: f.id,
            timestamp_ms: f.timestamp_ms,
            window_title: f.window_title.unwrap_or_default(),
            app_id: f.app_id.unwrap_or_default(),
            thumbnail_path: None,
        })
        .collect())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameDetail {
    pub frame_id: i64,
    pub timestamp_ms: u64,
    pub window_title: String,
    pub app_id: String,
    pub browser_url: Option<String>,
    pub ocr_text: Option<String>,
    pub pii_flags: Option<Vec<String>>,
    pub segment_file: String,
    pub segment_offset: i32,
}

#[tauri::command]
pub fn get_frame(frame_id: i64, state: State<AppState>) -> Result<Option<FrameDetail>, String> {
    // Query frame by getting a range of 1 around the expected timestamp
    // For a proper implementation, we'd add a get_frame_by_id method to Database
    let _ = (frame_id, &state);
    log::info!("Get frame: {}", frame_id);
    Ok(None)
}

// ── Capture control ──

#[tauri::command]
pub fn start_capture(state: State<AppState>) -> Result<(), String> {
    let config = state.config.lock().unwrap().clone();

    let filter = ExclusionFilter::new(
        config.privacy.excluded_apps.clone(),
        config.privacy.excluded_url_patterns.clone(),
    );

    state
        .pipeline
        .start(
            state.db.clone(),
            config.data_dir(),
            config.ocr.clone(),
            filter,
        )
        .map_err(|e| e.to_string())?;

    log::info!("Capture started via IPC");
    Ok(())
}

#[tauri::command]
pub fn pause_capture(state: State<AppState>) -> Result<(), String> {
    state.pipeline.pause();
    log::info!("Capture paused via IPC");
    Ok(())
}

#[tauri::command]
pub fn resume_capture(state: State<AppState>) -> Result<(), String> {
    state.pipeline.resume();
    log::info!("Capture resumed via IPC");
    Ok(())
}

#[tauri::command]
pub fn stop_capture(state: State<AppState>) -> Result<(), String> {
    state.pipeline.stop();
    log::info!("Capture stopped via IPC");
    Ok(())
}

#[tauri::command]
pub fn get_capture_status(state: State<AppState>) -> Result<CaptureStatus, String> {
    Ok(state.pipeline.get_status())
}

// ── Settings ──

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<AppConfig, String> {
    let config = state.config.lock().unwrap();
    Ok(config.clone())
}

#[tauri::command]
pub fn update_settings(new_config: AppConfig, state: State<AppState>) -> Result<(), String> {
    let mut config = state.config.lock().unwrap();
    // Persist to disk
    if let Err(e) = new_config.save() {
        log::warn!("Failed to persist settings: {}", e);
    }
    *config = new_config;
    log::info!("Settings updated");
    Ok(())
}

// ── Storage ──

#[tauri::command]
pub fn get_storage_usage(state: State<AppState>) -> Result<StorageUsage, String> {
    let total_frames = state.db.total_frames().map_err(|e| e.to_string())?;
    Ok(StorageUsage {
        total_frames,
        total_sessions: 0, // TODO: add session count query
        disk_usage_bytes: 0, // TODO: wire to SegmentManager
        oldest_frame_ms: None,
        newest_frame_ms: None,
    })
}

// ── Daily summary ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailySummary {
    pub date: String,
    pub total_frames: u64,
    pub hours_by_app: Vec<(String, f64)>,
    pub top_urls: Vec<(String, u64)>,
    pub top_windows: Vec<(String, u64)>,
}

#[tauri::command]
pub fn get_daily_summary(date: String, _state: State<AppState>) -> Result<DailySummary, String> {
    // TODO: Aggregate from database
    Ok(DailySummary {
        date,
        total_frames: 0,
        hours_by_app: vec![],
        top_urls: vec![],
        top_windows: vec![],
    })
}
