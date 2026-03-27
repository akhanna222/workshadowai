use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use super::encoder::{downscale_frame, VideoEncoder};
use super::screen::{CapturedFrame, ScreenCapture};
use super::{CaptureConfig, CaptureState, CaptureStatus};
use crate::ocr::dedup::TextDeduplicator;
use crate::ocr::engine::OcrEngine;
use crate::ocr::pii::detect_pii;
use crate::ocr::OcrConfig;
use crate::privacy::exclusions::ExclusionFilter;
use crate::search::index::SearchIndex;
use crate::storage::db::Database;

/// The full capture pipeline: screen → downscale → encode → OCR → store → index.
pub struct CapturePipeline {
    screen: ScreenCapture,
    config: CaptureConfig,
    status: Arc<Mutex<CaptureStatus>>,
    running: Arc<AtomicBool>,
}

impl CapturePipeline {
    pub fn new(config: CaptureConfig) -> Self {
        Self {
            screen: ScreenCapture::new(config.clone()),
            config,
            status: Arc::new(Mutex::new(CaptureStatus::default())),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start the full pipeline. Returns immediately; work happens on background threads.
    pub fn start(
        &self,
        db: Database,
        data_dir: PathBuf,
        ocr_config: OcrConfig,
        exclusion_filter: ExclusionFilter,
        search_index: Option<Arc<SearchIndex>>,
    ) -> Result<i64, Box<dyn std::error::Error>> {
        if self.running.load(Ordering::Relaxed) {
            return Err("Pipeline already running".into());
        }

        self.running.store(true, Ordering::Relaxed);

        // Create capture session
        let session_id = db.create_session(super::screen::now_ms())?;

        // Update status
        {
            let mut status = self.status.lock().unwrap();
            status.state = CaptureState::Recording;
            status.session_id = Some(session_id);
            status.recording_since_ms = Some(super::screen::now_ms());
            status.frames_captured = 0;
            status.current_fps = self.config.fps;
        }

        // Set up shared state for the frame callback
        let status = Arc::clone(&self.status);
        let running = Arc::clone(&self.running);
        let config = self.config.clone();
        let (max_w, max_h) = config.max_resolution;

        // Initialize encoder
        let encoder = Arc::new(Mutex::new(VideoEncoder::new(
            data_dir.clone(),
            config.segment_duration_secs,
            max_w,
            max_h,
            config.fps,
        )));

        // Initialize OCR
        let ocr_engine = Arc::new(OcrEngine::new(ocr_config.clone()));
        let dedup = Arc::new(Mutex::new(TextDeduplicator::new(ocr_config.dedup_threshold)));

        let frame_counter = Arc::new(AtomicU64::new(0));

        // Start capture with frame callback
        self.screen.start(move |frame: CapturedFrame| {
            if !running.load(Ordering::Relaxed) {
                return;
            }

            // Check exclusion filter
            if exclusion_filter.is_app_excluded(&frame.metadata.app_bundle_id) {
                log::trace!("Skipping excluded app: {}", frame.metadata.app_bundle_id);
                return;
            }
            if let Some(ref url) = frame.metadata.browser_url {
                if exclusion_filter.is_url_excluded(url) {
                    log::trace!("Skipping excluded URL: {}", url);
                    return;
                }
            }

            // Downscale if needed
            let (scaled_data, scaled_w, scaled_h) =
                downscale_frame(&frame.data, frame.width, frame.height, max_w, max_h);

            // Encode frame
            let segment_file;
            let segment_offset;
            {
                let mut enc = encoder.lock().unwrap();
                segment_offset = enc.current_segment_frame_index() as i32;
                if let Err(e) = enc.encode_frame(&scaled_data, &frame.metadata) {
                    log::error!("Encode error: {}", e);
                }
                segment_file = enc.current_segment_file().unwrap_or_default();
            }

            // OCR — fast path only in capture loop for real-time performance.
            // Quality re-analysis happens async for low-confidence frames.
            let ocr_result = ocr_engine.process_frame_fast(
                &scaled_data,
                scaled_w,
                scaled_h,
                frame.metadata.timestamp_ms,
            );

            // Dedup check
            let should_store = {
                let mut dd = dedup.lock().unwrap();
                !dd.is_duplicate(&ocr_result.full_text)
            };

            if should_store || ocr_result.full_text.is_empty() {
                // PII detection
                let pii_flags = if ocr_config.pii_detection && !ocr_result.full_text.is_empty() {
                    let detections = detect_pii(&ocr_result.full_text);
                    if detections.is_empty() {
                        None
                    } else {
                        let types: Vec<String> = detections
                            .iter()
                            .map(|d| format!("{:?}", d.pii_type))
                            .collect();
                        Some(serde_json::to_string(&types).unwrap_or_default())
                    }
                } else {
                    None
                };

                // Store in database
                let ocr_text = if ocr_result.full_text.is_empty() {
                    None
                } else {
                    Some(ocr_result.full_text.as_str())
                };

                match db.insert_frame(
                    session_id,
                    &frame.metadata,
                    &segment_file,
                    segment_offset,
                    ocr_text,
                    pii_flags.as_deref(),
                ) {
                    Ok(frame_id) => {
                        // Index in Tantivy search
                        if let Some(ref idx) = search_index {
                            if let Err(e) = idx.add_frame(
                                frame_id,
                                frame.metadata.timestamp_ms,
                                ocr_text.unwrap_or(""),
                                &frame.metadata.active_window_title,
                                &frame.metadata.app_bundle_id,
                                frame.metadata.browser_url.as_deref(),
                            ) {
                                log::error!("Search index error: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("DB insert error: {}", e);
                    }
                }
            }

            // Update status
            let count = frame_counter.fetch_add(1, Ordering::Relaxed) + 1;
            if let Ok(mut s) = status.lock() {
                s.frames_captured = count;
            }
        });

        log::info!("Capture pipeline started (session {})", session_id);
        Ok(session_id)
    }

    /// Pause the pipeline.
    pub fn pause(&self) {
        self.screen.pause();
        if let Ok(mut status) = self.status.lock() {
            status.state = CaptureState::Paused;
        }
    }

    /// Resume the pipeline.
    pub fn resume(&self) {
        self.screen.resume();
        if let Ok(mut status) = self.status.lock() {
            status.state = CaptureState::Recording;
        }
    }

    /// Stop the pipeline.
    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
        self.screen.stop();
        if let Ok(mut status) = self.status.lock() {
            status.state = CaptureState::Idle;
        }
        log::info!("Capture pipeline stopped");
    }

    /// Get current pipeline status.
    pub fn get_status(&self) -> CaptureStatus {
        self.status.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_status_defaults() {
        let config = CaptureConfig::default();
        let pipeline = CapturePipeline::new(config);
        let status = pipeline.get_status();
        assert_eq!(status.state, CaptureState::Idle);
        assert_eq!(status.frames_captured, 0);
        assert!(status.session_id.is_none());
    }

    #[test]
    fn test_pipeline_start_creates_session() {
        let config = CaptureConfig::default();
        let pipeline = CapturePipeline::new(config);
        let db = Database::open_in_memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let ocr_config = OcrConfig::default();
        let filter = ExclusionFilter::new(vec![], vec![]);

        let session_id = pipeline
            .start(db, dir.path().to_path_buf(), ocr_config, filter, None)
            .unwrap();
        assert!(session_id > 0);

        let status = pipeline.get_status();
        assert_eq!(status.state, CaptureState::Recording);
        assert_eq!(status.session_id, Some(session_id));

        std::thread::sleep(std::time::Duration::from_millis(100));
        pipeline.stop();
        assert_eq!(pipeline.get_status().state, CaptureState::Idle);
    }

    #[test]
    fn test_pipeline_with_search_index() {
        let config = CaptureConfig::default();
        let pipeline = CapturePipeline::new(config);
        let db = Database::open_in_memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let index_dir = dir.path().join("index");
        let ocr_config = OcrConfig::default();
        let filter = ExclusionFilter::new(vec![], vec![]);
        let search_index = Arc::new(SearchIndex::open(&index_dir).unwrap());

        let session_id = pipeline
            .start(
                db,
                dir.path().join("data"),
                ocr_config,
                filter,
                Some(search_index),
            )
            .unwrap();
        assert!(session_id > 0);

        std::thread::sleep(std::time::Duration::from_millis(200));
        pipeline.stop();
    }
}
