pub mod engine;
pub mod fast;
pub mod quality;
pub mod dedup;
pub mod pii;

use serde::{Deserialize, Serialize};

/// Result of OCR processing on a single frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    pub frame_timestamp_ms: u64,
    pub text_blocks: Vec<TextBlock>,
    pub full_text: String,
    /// Average confidence across all text blocks (0.0–1.0).
    pub avg_confidence: f32,
    /// Which backend produced this result.
    pub backend: OcrBackendType,
}

/// A single text region detected by OCR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextBlock {
    pub text: String,
    pub confidence: f32,
    /// Bounding box as (x, y, width, height) normalized 0.0–1.0.
    pub bbox: (f32, f32, f32, f32),
}

/// Identifies which OCR backend produced a result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OcrBackendType {
    AppleVision,
    Tesseract,
    DeepSeekOcr,
    Disabled,
}

/// OCR pipeline configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrConfig {
    pub enabled: bool,
    pub language: String,
    pub dedup_threshold: f32,
    pub pii_detection: bool,
    /// Confidence threshold below which quality OCR is triggered.
    #[serde(default = "default_quality_threshold")]
    pub quality_threshold: f32,
    /// Path to DeepSeek-OCR-2 GGUF model file (optional).
    #[serde(default)]
    pub deepseek_model_path: Option<String>,
    /// Enable quality re-analysis for low-confidence frames.
    #[serde(default = "default_true")]
    pub quality_reanalysis: bool,
}

fn default_quality_threshold() -> f32 {
    0.5
}

fn default_true() -> bool {
    true
}

impl Default for OcrConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            language: "en".to_string(),
            dedup_threshold: 0.9,
            pii_detection: true,
            quality_threshold: 0.5,
            deepseek_model_path: None,
            quality_reanalysis: true,
        }
    }
}
