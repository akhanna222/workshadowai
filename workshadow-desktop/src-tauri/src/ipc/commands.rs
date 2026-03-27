use serde::{Deserialize, Serialize};
use tauri::State;

use crate::capture::CaptureStatus;
use crate::config::AppConfig;
use crate::search::{SearchFilters, SearchResult};
use crate::storage::StorageUsage;

/// Application state shared via Tauri managed state.
pub struct AppState {
    pub config: std::sync::Mutex<AppConfig>,
    pub capture_status: std::sync::Mutex<CaptureStatus>,
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
    _state: State<AppState>,
) -> Result<Vec<FrameSummary>, String> {
    // TODO: Wire to Database
    log::info!("Timeline range: {}..{}", start_ms, end_ms);
    Ok(vec![])
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
pub fn get_frame(frame_id: i64, _state: State<AppState>) -> Result<Option<FrameDetail>, String> {
    // TODO: Wire to Database
    log::info!("Get frame: {}", frame_id);
    Ok(None)
}

// ── Capture control ──

#[tauri::command]
pub fn start_capture(state: State<AppState>) -> Result<(), String> {
    let mut status = state.capture_status.lock().unwrap();
    status.state = crate::capture::CaptureState::Recording;
    log::info!("Capture started");
    // TODO: Actually start the capture engine
    Ok(())
}

#[tauri::command]
pub fn pause_capture(state: State<AppState>) -> Result<(), String> {
    let mut status = state.capture_status.lock().unwrap();
    status.state = crate::capture::CaptureState::Paused;
    log::info!("Capture paused");
    Ok(())
}

#[tauri::command]
pub fn get_capture_status(state: State<AppState>) -> Result<CaptureStatus, String> {
    let status = state.capture_status.lock().unwrap();
    Ok(status.clone())
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
    *config = new_config;
    // TODO: Persist to config.toml
    log::info!("Settings updated");
    Ok(())
}

// ── Storage ──

#[tauri::command]
pub fn get_storage_usage(_state: State<AppState>) -> Result<StorageUsage, String> {
    // TODO: Wire to Database + SegmentManager
    Ok(StorageUsage {
        total_frames: 0,
        total_sessions: 0,
        disk_usage_bytes: 0,
        oldest_frame_ms: None,
        newest_frame_ms: None,
    })
}

// ── Daily summary (stretch) ──

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
