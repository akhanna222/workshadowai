use super::{CaptureConfig, CaptureState, FrameMetadata};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Raw captured frame data.
pub struct CapturedFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub metadata: FrameMetadata,
}

/// Platform-agnostic screen capture handle.
pub struct ScreenCapture {
    config: CaptureConfig,
    state: Arc<Mutex<CaptureState>>,
}

impl ScreenCapture {
    pub fn new(config: CaptureConfig) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(CaptureState::Idle)),
        }
    }

    pub fn state(&self) -> CaptureState {
        self.state.lock().unwrap().clone()
    }

    /// Start the capture loop. Frames are sent through the provided callback.
    pub fn start<F>(&self, _on_frame: F)
    where
        F: Fn(CapturedFrame) + Send + 'static,
    {
        let mut state = self.state.lock().unwrap();
        *state = CaptureState::Recording;
        log::info!(
            "Screen capture started at {} fps (idle: {} fps)",
            self.config.fps,
            self.config.idle_fps
        );

        // TODO: Platform-specific capture implementation
        // macOS: CGDisplayStream / ScreenCaptureKit
        // Windows: DXGI Desktop Duplication API
        // Linux: PipeWire screen capture portal
    }

    pub fn pause(&self) {
        let mut state = self.state.lock().unwrap();
        *state = CaptureState::Paused;
        log::info!("Screen capture paused");
    }

    pub fn resume(&self) {
        let mut state = self.state.lock().unwrap();
        *state = CaptureState::Recording;
        log::info!("Screen capture resumed");
    }

    pub fn stop(&self) {
        let mut state = self.state.lock().unwrap();
        *state = CaptureState::Idle;
        log::info!("Screen capture stopped");
    }
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
