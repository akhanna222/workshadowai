pub mod engine;
pub mod dedup;
pub mod pii;

use serde::{Deserialize, Serialize};

/// Result of OCR processing on a single frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    pub frame_timestamp_ms: u64,
    pub text_blocks: Vec<TextBlock>,
    pub full_text: String,
}

/// A single text region detected by OCR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextBlock {
    pub text: String,
    pub confidence: f32,
    /// Bounding box as (x, y, width, height) normalized 0.0–1.0.
    pub bbox: (f32, f32, f32, f32),
}

/// OCR pipeline configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrConfig {
    pub enabled: bool,
    pub language: String,
    pub dedup_threshold: f32,
    pub pii_detection: bool,
}

impl Default for OcrConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            language: "en".to_string(),
            dedup_threshold: 0.9,
            pii_detection: true,
        }
    }
}
