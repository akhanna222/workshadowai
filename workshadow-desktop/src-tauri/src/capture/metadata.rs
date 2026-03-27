use super::FrameMetadata;
use super::screen::now_ms;

/// Extract active window metadata from the OS.
pub fn get_active_window_metadata() -> FrameMetadata {
    // TODO: Platform-specific implementation
    // macOS: NSWorkspace.shared.frontmostApplication + accessibility APIs
    // Windows: GetForegroundWindow + GetWindowText
    // Linux: X11/Wayland window info

    FrameMetadata {
        timestamp_ms: now_ms(),
        active_window_title: String::new(),
        app_bundle_id: String::new(),
        browser_url: None,
        display_index: 0,
    }
}

/// Extract browser URL from accessibility APIs (if active window is a browser).
pub fn extract_browser_url(_app_id: &str) -> Option<String> {
    // TODO: Use accessibility APIs to read URL bar content
    // Supported browsers: Chrome, Firefox, Safari, Edge, Arc
    None
}
