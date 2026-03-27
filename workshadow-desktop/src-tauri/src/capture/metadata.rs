use super::FrameMetadata;
use super::screen::now_ms;

/// Extract active window metadata from the OS.
pub fn get_active_window_metadata() -> FrameMetadata {
    let timestamp = now_ms();

    #[cfg(target_os = "linux")]
    {
        get_x11_active_window(timestamp)
    }

    #[cfg(target_os = "macos")]
    {
        get_macos_active_window(timestamp)
    }

    #[cfg(target_os = "windows")]
    {
        get_windows_active_window(timestamp)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        FrameMetadata {
            timestamp_ms: timestamp,
            active_window_title: String::new(),
            app_bundle_id: String::new(),
            browser_url: None,
            display_index: 0,
        }
    }
}

/// Linux: get active window info via xdotool / xprop.
#[cfg(target_os = "linux")]
fn get_x11_active_window(timestamp: u64) -> FrameMetadata {
    use std::process::Command;

    // Get active window title using xdotool
    let title = Command::new("xdotool")
        .args(["getactivewindow", "getwindowname"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    // Get window class (app identifier) using xdotool
    let app_id = Command::new("xdotool")
        .args(["getactivewindow", "getwindowclassname"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    // Try to extract browser URL if it's a known browser
    let browser_url = extract_browser_url(&app_id, &title);

    FrameMetadata {
        timestamp_ms: timestamp,
        active_window_title: title,
        app_bundle_id: app_id,
        browser_url,
        display_index: 0,
    }
}

/// macOS: get active window info.
#[cfg(target_os = "macos")]
fn get_macos_active_window(timestamp: u64) -> FrameMetadata {
    use std::process::Command;

    // Use osascript to get frontmost app name and window title
    let script = r#"
        tell application "System Events"
            set frontApp to name of first application process whose frontmost is true
            set frontWindow to ""
            try
                set frontWindow to name of front window of first application process whose frontmost is true
            end try
            return frontApp & "|" & frontWindow
        end tell
    "#;

    let output = Command::new("osascript")
        .args(["-e", script])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let parts: Vec<&str> = output.splitn(2, '|').collect();
    let app_id = parts.first().unwrap_or(&"").to_string();
    let title = parts.get(1).unwrap_or(&"").to_string();

    let browser_url = extract_browser_url(&app_id, &title);

    FrameMetadata {
        timestamp_ms: timestamp,
        active_window_title: title,
        app_bundle_id: app_id,
        browser_url,
        display_index: 0,
    }
}

/// Windows: get active window info.
#[cfg(target_os = "windows")]
fn get_windows_active_window(timestamp: u64) -> FrameMetadata {
    // TODO: Implement via GetForegroundWindow + GetWindowText + process query
    FrameMetadata {
        timestamp_ms: timestamp,
        active_window_title: String::new(),
        app_bundle_id: String::new(),
        browser_url: None,
        display_index: 0,
    }
}

/// Try to extract browser URL from the window title.
/// Many browsers include the URL or page title in their window title.
fn extract_browser_url(app_id: &str, title: &str) -> Option<String> {
    let app_lower = app_id.to_lowercase();
    let is_browser = ["chrome", "firefox", "safari", "edge", "brave", "arc", "chromium"]
        .iter()
        .any(|b| app_lower.contains(b));

    if !is_browser {
        return None;
    }

    // Many browsers show "Page Title - Browser Name" or "Page Title — Browser Name"
    // The URL itself is rarely in the title, but the page title is useful metadata.
    // For actual URL extraction, we'd need accessibility APIs.
    // For now, return the page title as a hint.
    let cleaned = title
        .rsplit_once(" - ")
        .or_else(|| title.rsplit_once(" — "))
        .map(|(page, _browser)| page.trim())
        .unwrap_or(title);

    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_browser_url_from_chrome() {
        let url = extract_browser_url("Google-chrome", "GitHub - Google Chrome");
        assert_eq!(url, Some("GitHub".to_string()));
    }

    #[test]
    fn test_extract_browser_url_non_browser() {
        let url = extract_browser_url("code", "main.rs - Visual Studio Code");
        assert_eq!(url, None);
    }

    #[test]
    fn test_extract_browser_url_empty() {
        let url = extract_browser_url("firefox", "");
        assert_eq!(url, None);
    }

    #[test]
    fn test_get_active_window_metadata_returns_timestamp() {
        let meta = get_active_window_metadata();
        assert!(meta.timestamp_ms > 0);
    }
}
