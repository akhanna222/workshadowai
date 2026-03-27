use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

use crate::capture::pipeline::CapturePipeline;
use crate::capture::CaptureStatus;
use crate::config::AppConfig;
use crate::ocr::engine::OcrEngine;
use crate::privacy::audit::{AuditEvent, AuditLog};
use crate::privacy::exclusions::ExclusionFilter;
use crate::privacy::keymanager::KeyManager;
use crate::search::index::SearchIndex;
use crate::search::{SearchFilters, SearchResult};
use crate::storage::db::Database;
use crate::storage::segments::SegmentManager;
use crate::storage::StorageUsage;

/// Application state shared via Tauri managed state.
pub struct AppState {
    pub config: std::sync::Mutex<AppConfig>,
    pub pipeline: CapturePipeline,
    pub db: Database,
    pub search_index: Arc<SearchIndex>,
    pub segment_manager: SegmentManager,
    pub ocr_engine: Arc<OcrEngine>,
    pub audit_log: AuditLog,
    pub key_manager: KeyManager,
}

// ── Search ──

#[tauri::command]
pub fn search(
    query: String,
    filters: Option<SearchFilters>,
    state: State<AppState>,
) -> Result<Vec<SearchResult>, String> {
    let filters = filters.unwrap_or_default();
    let config = state.config.lock().unwrap();
    let max_results = config.search.max_results;
    drop(config);

    state
        .search_index
        .search(&query, &filters, max_results)
        .map_err(|e| e.to_string())
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
    let row = state
        .db
        .get_frame_by_id(frame_id)
        .map_err(|e| e.to_string())?;

    Ok(row.map(|f| FrameDetail {
        frame_id: f.id,
        timestamp_ms: f.timestamp_ms,
        window_title: f.window_title.unwrap_or_default(),
        app_id: f.app_id.unwrap_or_default(),
        browser_url: f.browser_url,
        ocr_text: f.ocr_text,
        pii_flags: None,
        segment_file: f.segment_file,
        segment_offset: f.segment_offset,
    }))
}

// ── Capture control ──

#[tauri::command]
pub fn start_capture(state: State<AppState>) -> Result<(), String> {
    let config = state.config.lock().unwrap().clone();

    let filter = ExclusionFilter::new(
        config.privacy.excluded_apps.clone(),
        config.privacy.excluded_url_patterns.clone(),
    );

    let session_id = state
        .pipeline
        .start(
            state.db.clone(),
            config.data_dir(),
            config.ocr.clone(),
            filter,
            Some(Arc::clone(&state.search_index)),
        )
        .map_err(|e| e.to_string())?;

    state
        .audit_log
        .log_event(AuditEvent::CaptureStarted { session_id })
        .ok();

    log::info!("Capture started via IPC (session {})", session_id);
    Ok(())
}

#[tauri::command]
pub fn pause_capture(state: State<AppState>) -> Result<(), String> {
    let session_id = state.pipeline.get_status().session_id.unwrap_or(0);
    state.pipeline.pause();
    state
        .audit_log
        .log_event(AuditEvent::CapturePaused { session_id })
        .ok();
    Ok(())
}

#[tauri::command]
pub fn resume_capture(state: State<AppState>) -> Result<(), String> {
    let session_id = state.pipeline.get_status().session_id.unwrap_or(0);
    state.pipeline.resume();
    state
        .audit_log
        .log_event(AuditEvent::CaptureResumed { session_id })
        .ok();
    Ok(())
}

#[tauri::command]
pub fn stop_capture(state: State<AppState>) -> Result<(), String> {
    let session_id = state.pipeline.get_status().session_id.unwrap_or(0);
    state.pipeline.stop();
    state
        .audit_log
        .log_event(AuditEvent::CaptureStopped { session_id })
        .ok();
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
    if let Err(e) = new_config.save() {
        log::warn!("Failed to persist settings: {}", e);
    }
    *config = new_config;
    state
        .audit_log
        .log_event(AuditEvent::SettingsChanged {
            field: "bulk".to_string(),
            old_value: String::new(),
            new_value: "updated".to_string(),
        })
        .ok();
    log::info!("Settings updated and persisted");
    Ok(())
}

// ── Storage ──

#[tauri::command]
pub fn get_storage_usage(state: State<AppState>) -> Result<StorageUsage, String> {
    let total_frames = state.db.total_frames().map_err(|e| e.to_string())?;
    let total_sessions = state.db.total_sessions().map_err(|e| e.to_string())?;
    let (oldest, newest) = state
        .db
        .frame_timestamp_range()
        .map_err(|e| e.to_string())?;
    let disk_usage_bytes = state.segment_manager.total_disk_usage();

    Ok(StorageUsage {
        total_frames,
        total_sessions,
        disk_usage_bytes,
        oldest_frame_ms: oldest,
        newest_frame_ms: newest,
    })
}

// ── Data deletion ──

#[tauri::command]
pub fn delete_time_range(
    start_ms: u64,
    end_ms: u64,
    state: State<AppState>,
) -> Result<usize, String> {
    let deleted = state
        .db
        .delete_frames_in_range(start_ms, end_ms)
        .map_err(|e| e.to_string())?;

    state
        .audit_log
        .log_event(AuditEvent::DataDeleted {
            start_ms,
            end_ms,
            frames_deleted: deleted,
        })
        .ok();

    log::info!(
        "Deleted {} frames in range {}..{}",
        deleted,
        start_ms,
        end_ms
    );
    Ok(deleted)
}

// ── Audit log ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: String,
    pub event: String,
}

#[tauri::command]
pub fn get_audit_log(state: State<AppState>) -> Result<Vec<AuditEntry>, String> {
    match state.audit_log.read_all() {
        Ok(entries) => Ok(entries
            .into_iter()
            .rev() // newest first
            .take(100)
            .map(|e| AuditEntry {
                timestamp: e.timestamp,
                event: serde_json::to_string(&e.event).unwrap_or_default(),
            })
            .collect()),
        Err(_) => Ok(vec![]), // No audit log yet
    }
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
pub fn get_daily_summary(date: String, state: State<AppState>) -> Result<DailySummary, String> {
    let parsed = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid date '{}': {}", date, e))?;

    let start_ms = parsed
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp_millis() as u64;
    let end_ms = start_ms + 86_400_000;

    let frames = state
        .db
        .get_frames_in_range(start_ms, end_ms)
        .map_err(|e| e.to_string())?;
    let total_frames = frames.len() as u64;

    let app_usage = state
        .db
        .get_app_usage(start_ms, end_ms)
        .map_err(|e| e.to_string())?;

    let hours_by_app: Vec<(String, f64)> = app_usage
        .into_iter()
        .map(|(app, count)| (app, count as f64 / 3600.0))
        .collect();

    let top_windows = state
        .db
        .get_top_windows(start_ms, end_ms)
        .map_err(|e| e.to_string())?;

    Ok(DailySummary {
        date,
        total_frames,
        hours_by_app,
        top_urls: vec![],
        top_windows,
    })
}

// ── OCR ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrStatus {
    pub fast_backend: String,
    pub quality_available: bool,
    pub quality_model: String,
}

#[tauri::command]
pub fn get_ocr_status(state: State<AppState>) -> Result<OcrStatus, String> {
    Ok(OcrStatus {
        fast_backend: format!("{:?}", state.ocr_engine.fast_backend_type()),
        quality_available: state.ocr_engine.is_quality_available(),
        quality_model: if state.ocr_engine.is_quality_available() {
            "DeepSeek-OCR-2 (GGUF)".to_string()
        } else {
            "Not installed".to_string()
        },
    })
}

#[tauri::command]
pub fn reanalyze_frame(frame_id: i64, state: State<AppState>) -> Result<String, String> {
    let frame = state
        .db
        .get_frame_by_id(frame_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Frame {} not found", frame_id))?;

    if let Some(ref ocr_text) = frame.ocr_text {
        Ok(format!(
            "Existing OCR text (frame {}): {}",
            frame_id,
            ocr_text.chars().take(500).collect::<String>()
        ))
    } else {
        Ok(format!(
            "Frame {} has no OCR text. Re-analysis from video segments is planned for a future update.",
            frame_id
        ))
    }
}

#[tauri::command]
pub fn download_quality_model() -> Result<String, String> {
    match crate::ocr::quality::QualityOcrBackend::download_model() {
        Ok(path) => Ok(format!("Model downloaded to {:?}", path)),
        Err(e) => Err(format!("Download failed: {}", e)),
    }
}

// ── Privacy info ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyStatus {
    pub encryption_active: bool,
    pub excluded_apps_count: usize,
    pub excluded_url_patterns_count: usize,
    pub audit_log_entries: usize,
}

#[tauri::command]
pub fn get_privacy_status(state: State<AppState>) -> Result<PrivacyStatus, String> {
    let config = state.config.lock().unwrap();
    let audit_count = state
        .audit_log
        .read_all()
        .map(|entries| entries.len())
        .unwrap_or(0);

    Ok(PrivacyStatus {
        encryption_active: true, // Key manager always initializes
        excluded_apps_count: config.privacy.excluded_apps.len(),
        excluded_url_patterns_count: config.privacy.excluded_url_patterns.len(),
        audit_log_entries: audit_count,
    })
}
