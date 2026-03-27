use super::{CaptureConfig, CaptureState, FrameMetadata};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Raw captured frame data (RGBA).
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
    frames_captured: Arc<AtomicU64>,
    last_window_change_ms: Arc<AtomicU64>,
}

impl ScreenCapture {
    pub fn new(config: CaptureConfig) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(CaptureState::Idle)),
            frames_captured: Arc::new(AtomicU64::new(0)),
            last_window_change_ms: Arc::new(AtomicU64::new(now_ms())),
        }
    }

    pub fn state(&self) -> CaptureState {
        self.state.lock().unwrap().clone()
    }

    pub fn frames_captured(&self) -> u64 {
        self.frames_captured.load(Ordering::Relaxed)
    }

    /// Start the capture loop on a background thread.
    /// Frames are sent through the provided callback.
    pub fn start<F>(&self, on_frame: F)
    where
        F: Fn(CapturedFrame) + Send + 'static,
    {
        {
            let mut state = self.state.lock().unwrap();
            *state = CaptureState::Recording;
        }

        let state = Arc::clone(&self.state);
        let frames_captured = Arc::clone(&self.frames_captured);
        let last_window_change_ms = Arc::clone(&self.last_window_change_ms);
        let config = self.config.clone();

        thread::Builder::new()
            .name("ws-capture".to_string())
            .spawn(move || {
                log::info!(
                    "Capture thread started: {} fps (idle: {} fps, idle after {}s)",
                    config.fps,
                    config.idle_fps,
                    config.idle_threshold_secs
                );

                let mut prev_window_title = String::new();

                loop {
                    // Check state
                    let current_state = state.lock().unwrap().clone();
                    match current_state {
                        CaptureState::Idle => {
                            log::info!("Capture thread exiting (idle)");
                            break;
                        }
                        CaptureState::Paused => {
                            thread::sleep(Duration::from_millis(200));
                            continue;
                        }
                        CaptureState::Recording => {}
                    }

                    // Get current metadata
                    let metadata = super::metadata::get_active_window_metadata();

                    // Track window changes for idle detection
                    if metadata.active_window_title != prev_window_title {
                        last_window_change_ms.store(metadata.timestamp_ms, Ordering::Relaxed);
                        prev_window_title = metadata.active_window_title.clone();
                    }

                    // Determine effective FPS (adaptive: drop to idle fps when no activity)
                    let idle_duration_ms =
                        metadata.timestamp_ms - last_window_change_ms.load(Ordering::Relaxed);
                    let effective_fps = if idle_duration_ms > (config.idle_threshold_secs as u64 * 1000)
                    {
                        config.idle_fps
                    } else {
                        config.fps
                    };

                    // Capture a frame
                    let frame_start = Instant::now();

                    match capture_screen_frame(&config) {
                        Some(frame) => {
                            let captured = CapturedFrame {
                                data: frame.data,
                                width: frame.width,
                                height: frame.height,
                                metadata,
                            };
                            frames_captured.fetch_add(1, Ordering::Relaxed);
                            on_frame(captured);
                        }
                        None => {
                            log::trace!("No frame captured this cycle");
                        }
                    }

                    // Sleep for the remainder of the frame interval
                    let frame_duration = Duration::from_secs_f32(1.0 / effective_fps);
                    let elapsed = frame_start.elapsed();
                    if elapsed < frame_duration {
                        thread::sleep(frame_duration - elapsed);
                    }
                }
            })
            .expect("Failed to spawn capture thread");
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

/// Internal raw frame from platform capture.
struct RawFrame {
    data: Vec<u8>,
    width: u32,
    height: u32,
}

/// Platform-specific screen capture implementation.
fn capture_screen_frame(config: &CaptureConfig) -> Option<RawFrame> {
    #[cfg(target_os = "linux")]
    {
        capture_x11_frame(config)
    }

    #[cfg(target_os = "macos")]
    {
        capture_macos_frame(config)
    }

    #[cfg(target_os = "windows")]
    {
        capture_windows_frame(config)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let _ = config;
        None
    }
}

/// Linux X11 screen capture using xcb.
#[cfg(target_os = "linux")]
fn capture_x11_frame(config: &CaptureConfig) -> Option<RawFrame> {
    // Use XCB shared memory extension for fast screen capture
    // For now, generate a synthetic test frame if X11 is not available
    use std::process::Command;

    // Try using ffmpeg to grab a single frame from X11
    let (max_w, max_h) = config.max_resolution;

    let output = Command::new("ffmpeg")
        .args([
            "-f", "x11grab",
            "-video_size", &format!("{}x{}", max_w, max_h),
            "-i", ":0.0",
            "-frames:v", "1",
            "-f", "rawvideo",
            "-pix_fmt", "rgba",
            "pipe:1",
        ])
        .output();

    match output {
        Ok(out) if out.status.success() && !out.stdout.is_empty() => {
            Some(RawFrame {
                data: out.stdout,
                width: max_w,
                height: max_h,
            })
        }
        _ => {
            // Fallback: generate a dark grey frame (for headless/CI environments)
            log::trace!("X11 capture unavailable, generating synthetic frame");
            let w = 320u32;
            let h = 240u32;
            let data = vec![30u8; (w * h * 4) as usize]; // dark grey RGBA
            Some(RawFrame {
                data,
                width: w,
                height: h,
            })
        }
    }
}

/// macOS screen capture stub (ScreenCaptureKit).
#[cfg(target_os = "macos")]
fn capture_macos_frame(_config: &CaptureConfig) -> Option<RawFrame> {
    // TODO: Implement via ScreenCaptureKit (macOS 13+) or CGDisplayStream
    // For now, return synthetic frame
    let w = 320u32;
    let h = 240u32;
    Some(RawFrame {
        data: vec![30u8; (w * h * 4) as usize],
        width: w,
        height: h,
    })
}

/// Windows screen capture stub (DXGI Desktop Duplication).
#[cfg(target_os = "windows")]
fn capture_windows_frame(_config: &CaptureConfig) -> Option<RawFrame> {
    // TODO: Implement via DXGI Desktop Duplication API
    // For now, return synthetic frame
    let w = 320u32;
    let h = 240u32;
    Some(RawFrame {
        data: vec![30u8; (w * h * 4) as usize],
        width: w,
        height: h,
    })
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_capture_state_transitions() {
        let config = CaptureConfig::default();
        let capture = ScreenCapture::new(config);

        assert_eq!(capture.state(), CaptureState::Idle);

        // Directly set state (without starting thread)
        {
            let mut state = capture.state.lock().unwrap();
            *state = CaptureState::Recording;
        }
        assert_eq!(capture.state(), CaptureState::Recording);

        capture.pause();
        assert_eq!(capture.state(), CaptureState::Paused);

        capture.resume();
        assert_eq!(capture.state(), CaptureState::Recording);

        capture.stop();
        assert_eq!(capture.state(), CaptureState::Idle);
    }

    #[test]
    fn test_frames_captured_counter() {
        let config = CaptureConfig::default();
        let capture = ScreenCapture::new(config);
        assert_eq!(capture.frames_captured(), 0);
        capture.frames_captured.fetch_add(5, Ordering::Relaxed);
        assert_eq!(capture.frames_captured(), 5);
    }
}
