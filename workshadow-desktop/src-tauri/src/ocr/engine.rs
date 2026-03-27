use super::fast::FastOcrBackend;
use super::quality::QualityOcrBackend;
use super::{OcrBackendType, OcrConfig, OcrResult};

/// Tiered OCR engine.
///
/// **Fast path** (always-on, ~1 fps): Apple Vision (macOS) or Tesseract.
/// Runs on every captured frame. Optimized for speed (<50ms/frame).
///
/// **Quality path** (on-demand): DeepSeek-OCR-2 via llama.cpp GGUF.
/// Triggered when:
/// - Fast path confidence < quality_threshold
/// - User requests re-analysis of a specific frame
/// Slower (~1-5s/frame) but significantly more accurate.
pub struct OcrEngine {
    config: OcrConfig,
    fast: FastOcrBackend,
    quality: QualityOcrBackend,
}

impl OcrEngine {
    pub fn new(config: OcrConfig) -> Self {
        let fast = FastOcrBackend::new(&config);
        let quality = QualityOcrBackend::new(&config);

        log::info!(
            "OCR engine initialized: fast={:?}, quality_available={}, threshold={}",
            fast.backend_type(),
            quality.is_available(),
            config.quality_threshold
        );

        Self {
            config,
            fast,
            quality,
        }
    }

    /// Process a frame with the fast OCR backend.
    /// If confidence is below threshold and quality backend is available,
    /// automatically triggers quality re-analysis.
    pub fn process_frame(
        &self,
        image_data: &[u8],
        width: u32,
        height: u32,
        timestamp_ms: u64,
    ) -> OcrResult {
        if !self.config.enabled {
            return OcrResult {
                frame_timestamp_ms: timestamp_ms,
                text_blocks: vec![],
                full_text: String::new(),
                avg_confidence: 0.0,
                backend: OcrBackendType::Disabled,
            };
        }

        // 1. Run fast path
        let fast_result = self.fast.process_frame(image_data, width, height, timestamp_ms);

        // 2. Check if quality re-analysis is needed
        if self.should_reanalyze(&fast_result) {
            log::debug!(
                "Fast OCR confidence {:.2} < threshold {:.2}, triggering quality re-analysis",
                fast_result.avg_confidence,
                self.config.quality_threshold
            );
            let quality_result =
                self.quality
                    .process_frame(image_data, width, height, timestamp_ms);

            // Use quality result if it produced text
            if !quality_result.full_text.is_empty() {
                return quality_result;
            }
        }

        fast_result
    }

    /// Run only the fast OCR backend (used in the capture loop for speed).
    pub fn process_frame_fast(
        &self,
        image_data: &[u8],
        width: u32,
        height: u32,
        timestamp_ms: u64,
    ) -> OcrResult {
        if !self.config.enabled {
            return OcrResult {
                frame_timestamp_ms: timestamp_ms,
                text_blocks: vec![],
                full_text: String::new(),
                avg_confidence: 0.0,
                backend: OcrBackendType::Disabled,
            };
        }
        self.fast.process_frame(image_data, width, height, timestamp_ms)
    }

    /// Run quality OCR on a frame (on-demand, e.g. user re-analysis).
    pub fn process_frame_quality(
        &self,
        image_data: &[u8],
        width: u32,
        height: u32,
        timestamp_ms: u64,
    ) -> OcrResult {
        if !self.quality.is_available() {
            log::warn!("Quality OCR requested but DeepSeek-OCR-2 is not available");
            return self.fast.process_frame(image_data, width, height, timestamp_ms);
        }
        self.quality
            .process_frame(image_data, width, height, timestamp_ms)
    }

    /// Check if a fast result should trigger quality re-analysis.
    fn should_reanalyze(&self, fast_result: &OcrResult) -> bool {
        if !self.config.quality_reanalysis {
            return false;
        }
        if !self.quality.is_available() {
            return false;
        }
        // Re-analyze if: text was found but confidence is low
        if fast_result.full_text.is_empty() {
            return false; // No text = nothing to re-analyze
        }
        fast_result.avg_confidence < self.config.quality_threshold
    }

    /// Whether the quality (DeepSeek-OCR-2) backend is available.
    pub fn is_quality_available(&self) -> bool {
        self.quality.is_available()
    }

    /// Get the active fast backend type.
    pub fn fast_backend_type(&self) -> &OcrBackendType {
        self.fast.backend_type()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disabled_ocr_returns_empty() {
        let config = OcrConfig {
            enabled: false,
            ..Default::default()
        };
        let engine = OcrEngine::new(config);
        let result = engine.process_frame(&[0u8; 100 * 100 * 4], 100, 100, 12345);
        assert!(result.full_text.is_empty());
        assert!(result.text_blocks.is_empty());
        assert_eq!(result.frame_timestamp_ms, 12345);
        assert_eq!(result.backend, OcrBackendType::Disabled);
    }

    #[test]
    fn test_engine_initializes_with_fast_backend() {
        let config = OcrConfig::default();
        let engine = OcrEngine::new(config);
        assert!(
            *engine.fast_backend_type() == OcrBackendType::Tesseract
                || *engine.fast_backend_type() == OcrBackendType::Disabled
        );
    }

    #[test]
    fn test_process_frame_fast() {
        let config = OcrConfig::default();
        let engine = OcrEngine::new(config);
        let result = engine.process_frame_fast(&vec![128u8; 320 * 240 * 4], 320, 240, 1000);
        assert_eq!(result.frame_timestamp_ms, 1000);
    }

    #[test]
    fn test_quality_not_available() {
        let config = OcrConfig::default();
        let engine = OcrEngine::new(config);
        // In test env, DeepSeek model is not present
        assert!(!engine.is_quality_available());
    }

    #[test]
    fn test_should_not_reanalyze_when_disabled() {
        let config = OcrConfig {
            quality_reanalysis: false,
            ..Default::default()
        };
        let engine = OcrEngine::new(config);
        let result = OcrResult {
            frame_timestamp_ms: 1000,
            text_blocks: vec![],
            full_text: "test".to_string(),
            avg_confidence: 0.1,
            backend: OcrBackendType::Tesseract,
        };
        assert!(!engine.should_reanalyze(&result));
    }

    #[test]
    fn test_should_not_reanalyze_empty_text() {
        let config = OcrConfig::default();
        let engine = OcrEngine::new(config);
        let result = OcrResult {
            frame_timestamp_ms: 1000,
            text_blocks: vec![],
            full_text: String::new(),
            avg_confidence: 0.0,
            backend: OcrBackendType::Tesseract,
        };
        assert!(!engine.should_reanalyze(&result));
    }
}
