pub mod capture;
pub mod config;
pub mod ipc;
pub mod ocr;
pub mod privacy;
pub mod search;
pub mod storage;

use std::sync::Arc;

use capture::pipeline::CapturePipeline;
use config::AppConfig;
use ipc::commands::*;
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
    log::info!("Database opened at {:?}", db_path);

    // Initialize search index
    let search_index = Arc::new(
        SearchIndex::open(&config.index_dir()).expect("Failed to open search index"),
    );
    log::info!("Search index opened at {:?}", config.index_dir());

    // Initialize segment manager
    let segment_manager = SegmentManager::new(config.data_dir());

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
                log::info!(
                    "Retention scheduler started (every {}h, max {}d / {}GB)",
                    cleanup_hours,
                    max_days,
                    max_gb
                );
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
                        Ok(_) => log::debug!("Retention cleanup: nothing to remove"),
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
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running WorkShadow AI");
}
