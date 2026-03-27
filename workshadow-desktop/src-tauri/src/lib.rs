pub mod capture;
pub mod config;
pub mod ipc;
pub mod ocr;
pub mod privacy;
pub mod search;
pub mod storage;

use capture::pipeline::CapturePipeline;
use config::AppConfig;
use ipc::commands::*;
use storage::db::Database;

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

    // Initialize capture pipeline
    let pipeline = CapturePipeline::new(config.capture.clone());

    let app_state = AppState {
        config: std::sync::Mutex::new(config),
        pipeline,
        db,
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
