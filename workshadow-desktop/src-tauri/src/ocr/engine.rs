use super::{OcrConfig, OcrResult};

/// OCR engine backed by PaddleOCR-Lite via ONNX Runtime.
pub struct OcrEngine {
    config: OcrConfig,
    // TODO: ort::Session for ONNX inference
}

impl OcrEngine {
    pub fn new(config: OcrConfig) -> Self {
        log::info!("Initializing OCR engine (language: {})", config.language);
        // TODO: Load PaddleOCR ONNX model files
        // - detection model
        // - recognition model
        // - classification model (optional)
        Self { config }
    }

    /// Run OCR on a raw frame image.
    /// The image should be pre-downscaled to 1280x720 for performance.
    pub fn process_frame(&self, _image_data: &[u8], _width: u32, _height: u32, timestamp_ms: u64) -> OcrResult {
        if !self.config.enabled {
            return OcrResult {
                frame_timestamp_ms: timestamp_ms,
                text_blocks: vec![],
                full_text: String::new(),
            };
        }

        // TODO: Actual ONNX inference pipeline
        // 1. Preprocess image (resize, normalize)
        // 2. Run text detection model → bounding boxes
        // 3. Run text recognition model → text per box
        // 4. Assemble OcrResult

        OcrResult {
            frame_timestamp_ms: timestamp_ms,
            text_blocks: vec![],
            full_text: String::new(),
        }
    }
}
