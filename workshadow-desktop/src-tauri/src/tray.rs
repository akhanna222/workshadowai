use tauri::{
    AppHandle, Manager,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
};

use crate::ipc::commands::AppState;
use crate::capture::CaptureState;
use crate::privacy::audit::AuditEvent;

/// Set up the system tray icon with a context menu.
pub fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let toggle_id = "toggle_capture";
    let show_id = "show_window";
    let quit_id = "quit";

    let toggle = MenuItemBuilder::with_id(toggle_id, "Start Capture").build(app)?;
    let show = MenuItemBuilder::with_id(show_id, "Show WorkShadow").build(app)?;
    let quit = MenuItemBuilder::with_id(quit_id, "Quit").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&toggle)
        .separator()
        .item(&show)
        .separator()
        .item(&quit)
        .build()?;

    let app_handle = app.clone();

    let _tray = TrayIconBuilder::new()
        .tooltip("WorkShadow AI")
        .menu(&menu)
        .on_menu_event(move |_tray, event| {
            match event.id().as_ref() {
                id if id == toggle_id => {
                    let state: tauri::State<AppState> = app_handle.state();
                    let status = state.pipeline.get_status();
                    match status.state {
                        CaptureState::Recording => {
                            state.pipeline.pause();
                            state
                                .audit_log
                                .log_event(AuditEvent::CapturePaused {
                                    session_id: status.session_id.unwrap_or(0),
                                })
                                .ok();
                            log::info!("Capture paused via tray");
                        }
                        CaptureState::Paused => {
                            state.pipeline.resume();
                            state
                                .audit_log
                                .log_event(AuditEvent::CaptureResumed {
                                    session_id: status.session_id.unwrap_or(0),
                                })
                                .ok();
                            log::info!("Capture resumed via tray");
                        }
                        CaptureState::Idle => {
                            log::info!("Start capture from tray — use the main window");
                        }
                    }
                }
                id if id == show_id => {
                    if let Some(window) = app_handle.get_webview_window("main") {
                        window.show().ok();
                        window.set_focus().ok();
                    }
                }
                id if id == quit_id => {
                    let state: tauri::State<AppState> = app_handle.state();
                    state.pipeline.stop();
                    app_handle.exit(0);
                }
                _ => {}
            }
        })
        .build(app)?;

    log::info!("System tray initialized");
    Ok(())
}
