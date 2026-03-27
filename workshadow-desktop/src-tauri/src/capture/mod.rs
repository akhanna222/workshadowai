pub mod screen;
pub mod encoder;
pub mod metadata;
pub mod pipeline;

use serde::{Deserialize, Serialize};

/// Configuration for the screen capture engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureConfig {
    pub fps: f32,
    pub idle_fps: f32,
    pub idle_threshold_secs: u32,
    pub max_resolution: (u32, u32),
    pub segment_duration_secs: u32,
    pub multi_monitor: bool,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            fps: 1.0,
            idle_fps: 0.5,
            idle_threshold_secs: 30,
            max_resolution: (1920, 1080),
            segment_duration_secs: 300,
            multi_monitor: true,
        }
    }
}

/// Metadata attached to each captured frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameMetadata {
    pub timestamp_ms: u64,
    pub active_window_title: String,
    pub app_bundle_id: String,
    pub browser_url: Option<String>,
    pub display_index: u8,
}

/// The current state of the capture engine.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CaptureState {
    Idle,
    Recording,
    Paused,
}

/// Status snapshot for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureStatus {
    pub state: CaptureState,
    pub session_id: Option<i64>,
    pub frames_captured: u64,
    pub current_fps: f32,
    pub recording_since_ms: Option<u64>,
}

impl Default for CaptureStatus {
    fn default() -> Self {
        Self {
            state: CaptureState::Idle,
            session_id: None,
            frames_captured: 0,
            current_fps: 0.0,
            recording_since_ms: None,
        }
    }
}
