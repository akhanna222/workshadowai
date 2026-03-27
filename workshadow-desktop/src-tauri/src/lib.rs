pub mod capture;
pub mod config;
pub mod ipc;
pub mod ocr;
pub mod privacy;
pub mod search;
pub mod storage;

use config::AppConfig;
use ipc::commands::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    let config = AppConfig::load();
    log::info!("WorkShadow AI v{} starting", env!("CARGO_PKG_VERSION"));

    let app_state = AppState {
        config: std::sync::Mutex::new(config),
        capture_status: std::sync::Mutex::new(capture::CaptureStatus::default()),
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
            get_capture_status,
            get_settings,
            update_settings,
            get_storage_usage,
            get_daily_summary,
        ])
        .run(tauri::generate_context!())
        .expect("error while running WorkShadow AI");
}
