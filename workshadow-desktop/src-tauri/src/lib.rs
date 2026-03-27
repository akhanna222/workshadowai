pub mod capture;
pub mod config;
pub mod ipc;
pub mod ocr;
pub mod privacy;
pub mod search;
pub mod storage;
pub mod tray;

use std::sync::Arc;

use capture::pipeline::CapturePipeline;
use config::AppConfig;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState, hotkey::HotKey};
use ipc::commands::*;
use tauri::Manager;
use ocr::engine::OcrEngine;
use privacy::audit::AuditLog;
use privacy::keymanager::KeyManager;
use search::index::SearchIndex;
use storage::db::Database;
use storage::segments::SegmentManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();
    ffmpeg_next::init().expect("Failed to initialize FFmpeg");

    let config = AppConfig::load();
    log::info!("WorkShadow AI v{} starting", env!("CARGO_PKG_VERSION"));

    // Initialize database
    let db_path = config.data_dir().join("workshadow.db");
    std::fs::create_dir_all(config.data_dir()).ok();
    let db = Database::open(&db_path).expect("Failed to open database");

    // Initialize search index
    let search_index = Arc::new(
        SearchIndex::open(&config.index_dir()).expect("Failed to open search index"),
    );

    // Initialize segment manager
    let segment_manager = SegmentManager::new(config.data_dir());

    // Initialize OCR engine (tiered: fast + quality)
    let ocr_engine = Arc::new(OcrEngine::new(config.ocr.clone()));

    // Initialize privacy components
    let audit_log = AuditLog::new(&config.data_dir());
    let key_manager = KeyManager::new(&config.data_dir());
    log::info!(
        "Encryption key loaded (storage: {:?})",
        key_manager.storage()
    );

    // Encrypt any unencrypted segments from previous sessions
    {
        let key = *key_manager.key();
        let data_dir = config.data_dir();
        match storage::encrypted::encrypt_segments_in_dir(&data_dir, &key) {
            Ok(0) => {}
            Ok(n) => log::info!("Encrypted {} unencrypted segment files from previous session", n),
            Err(e) => log::warn!("Failed to encrypt existing segments: {}", e),
        }
    }

    // Initialize capture pipeline
    let pipeline = CapturePipeline::new(config.capture.clone());

    // Start retention cleanup scheduler
    {
        let db_clone = db.clone();
        let seg_clone = SegmentManager::new(config.data_dir());
        let max_days = config.storage.max_retention_days;
        let max_gb = config.storage.max_storage_gb;
        let cleanup_hours = config.storage.cleanup_interval_hours;

        std::thread::Builder::new()
            .name("ws-retention".to_string())
            .spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(
                        cleanup_hours as u64 * 3600,
                    ));
                    let manager = storage::retention::RetentionManager::new(
                        db_clone.clone(),
                        SegmentManager::new(seg_clone.data_dir().to_path_buf()),
                        max_days,
                        max_gb,
                    );
                    match manager.run_cleanup() {
                        Ok(deleted) if deleted > 0 => {
                            log::info!("Retention cleanup: removed {} items", deleted);
                        }
                        Ok(_) => {}
                        Err(e) => log::error!("Retention cleanup error: {}", e),
                    }
                }
            })
            .expect("Failed to spawn retention thread");
    }

    let app_state = AppState {
        config: std::sync::Mutex::new(config),
        pipeline,
        db,
        search_index,
        segment_manager,
        ocr_engine,
        audit_log,
        key_manager,
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .setup(|app| {
            let handle = app.handle().clone();

            // Set up system tray
            if let Err(e) = tray::setup_tray(&handle) {
                log::warn!("Failed to set up system tray: {}", e);
            }

            // Register global hotkey
            setup_global_hotkey(handle);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            search,
            get_timeline_range,
            get_frame,
            start_capture,
            pause_capture,
            resume_capture,
            stop_capture,
            get_capture_status,
            get_settings,
            update_settings,
            get_storage_usage,
            get_daily_summary,
            get_ocr_status,
            reanalyze_frame,
            download_quality_model,
            delete_time_range,
            get_audit_log,
            get_privacy_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running WorkShadow AI");
}

/// Set up the global hotkey for pause/resume (Ctrl+Shift+P / Cmd+Shift+P).
fn setup_global_hotkey(app_handle: tauri::AppHandle) {
    let manager = match GlobalHotKeyManager::new() {
        Ok(m) => m,
        Err(e) => {
            log::warn!("Failed to initialize global hotkey manager: {}", e);
            return;
        }
    };

    let hotkey = match "ctrl+shift+KeyP".parse::<HotKey>() {
        Ok(hk) => hk,
        Err(e) => {
            log::warn!("Failed to parse hotkey: {}", e);
            return;
        }
    };

    let hotkey_id = hotkey.id();

    if let Err(e) = manager.register(hotkey) {
        log::warn!("Failed to register global hotkey Ctrl+Shift+P: {}", e);
        return;
    }

    log::info!("Global hotkey registered: Ctrl+Shift+P (pause/resume)");

    std::thread::Builder::new()
        .name("ws-hotkey".to_string())
        .spawn(move || {
            let _manager = manager;
            let receiver = GlobalHotKeyEvent::receiver();

            loop {
                if let Ok(event) = receiver.recv() {
                    if event.id() == hotkey_id && event.state() == HotKeyState::Pressed {
                        let state: tauri::State<AppState> = app_handle.state();
                        let status = state.pipeline.get_status();

                        match status.state {
                            capture::CaptureState::Recording => {
                                state.pipeline.pause();
                                state
                                    .audit_log
                                    .log_event(privacy::audit::AuditEvent::CapturePaused {
                                        session_id: status.session_id.unwrap_or(0),
                                    })
                                    .ok();
                                log::info!("Capture paused via hotkey");
                            }
                            capture::CaptureState::Paused => {
                                state.pipeline.resume();
                                state
                                    .audit_log
                                    .log_event(privacy::audit::AuditEvent::CaptureResumed {
                                        session_id: status.session_id.unwrap_or(0),
                                    })
                                    .ok();
                                log::info!("Capture resumed via hotkey");
                            }
                            capture::CaptureState::Idle => {}
                        }
                    }
                }
            }
        })
        .expect("Failed to spawn hotkey thread");
}
