use super::{OcrConfig, OcrResult, TextBlock};

/// OCR engine with pluggable backends.
/// Priority: ONNX PaddleOCR → Tesseract CLI → disabled.
pub struct OcrEngine {
    config: OcrConfig,
    backend: OcrBackend,
}

enum OcrBackend {
    /// PaddleOCR via ONNX Runtime (future: requires model files).
    #[allow(dead_code)]
    OnnxPaddleOcr,
    /// Tesseract CLI fallback — works if `tesseract` is installed.
    TesseractCli,
    /// No OCR available.
    Disabled,
}

impl OcrEngine {
    pub fn new(config: OcrConfig) -> Self {
        if !config.enabled {
            log::info!("OCR disabled by configuration");
            return Self {
                config,
                backend: OcrBackend::Disabled,
            };
        }

        // Try to detect available OCR backend
        let backend = Self::detect_backend(&config);
        match &backend {
            OcrBackend::TesseractCli => log::info!("OCR backend: Tesseract CLI"),
            OcrBackend::OnnxPaddleOcr => log::info!("OCR backend: PaddleOCR (ONNX)"),
            OcrBackend::Disabled => log::warn!("No OCR backend available — OCR will be skipped"),
        }

        Self { config, backend }
    }

    fn detect_backend(_config: &OcrConfig) -> OcrBackend {
        // TODO: Check for ONNX model files first
        // let model_dir = dirs::data_dir().join("workshadow/models/paddleocr");
        // if model_dir.join("det.onnx").exists() { return OcrBackend::OnnxPaddleOcr; }

        // Check for tesseract CLI
        if is_tesseract_available() {
            return OcrBackend::TesseractCli;
        }

        OcrBackend::Disabled
    }

    /// Run OCR on a raw RGBA frame image.
    pub fn process_frame(
        &self,
        image_data: &[u8],
        width: u32,
        height: u32,
        timestamp_ms: u64,
    ) -> OcrResult {
        match self.backend {
            OcrBackend::Disabled => OcrResult {
                frame_timestamp_ms: timestamp_ms,
                text_blocks: vec![],
                full_text: String::new(),
            },
            OcrBackend::TesseractCli => {
                self.process_with_tesseract(image_data, width, height, timestamp_ms)
            }
            OcrBackend::OnnxPaddleOcr => {
                // TODO: ONNX inference pipeline
                self.process_with_tesseract(image_data, width, height, timestamp_ms)
            }
        }
    }

    /// Run OCR via Tesseract CLI.
    /// Writes a temp PNG, runs `tesseract`, parses output.
    fn process_with_tesseract(
        &self,
        image_data: &[u8],
        width: u32,
        height: u32,
        timestamp_ms: u64,
    ) -> OcrResult {
        use std::process::Command;

        // Convert RGBA to PNG in a temp file
        let tmp_result = tempfile::Builder::new()
            .suffix(".png")
            .tempfile();

        let tmp_file = match tmp_result {
            Ok(f) => f,
            Err(e) => {
                log::error!("Failed to create temp file for OCR: {}", e);
                return empty_result(timestamp_ms);
            }
        };

        let img = match image::RgbaImage::from_raw(width, height, image_data.to_vec()) {
            Some(img) => img,
            None => return empty_result(timestamp_ms),
        };

        // Downscale for OCR speed (1280x720 max)
        let ocr_img = if width > 1280 || height > 720 {
            let scale = (1280.0 / width as f64).min(720.0 / height as f64);
            let nw = (width as f64 * scale) as u32;
            let nh = (height as f64 * scale) as u32;
            image::imageops::resize(&img, nw, nh, image::imageops::FilterType::Triangle)
        } else {
            img
        };

        if let Err(e) = ocr_img.save(tmp_file.path()) {
            log::error!("Failed to save temp PNG for OCR: {}", e);
            return empty_result(timestamp_ms);
        }

        // Run tesseract: input.png stdout --oem 3 --psm 3 -l <lang>
        let output = Command::new("tesseract")
            .arg(tmp_file.path())
            .arg("stdout")
            .args(["--oem", "3", "--psm", "3"])
            .args(["-l", &self.config.language])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let full_text = String::from_utf8_lossy(&out.stdout).trim().to_string();

                // Tesseract CLI doesn't give bounding boxes in plain text mode.
                // For bounding boxes, we'd use TSV output. Keep it simple for now.
                let text_blocks = if full_text.is_empty() {
                    vec![]
                } else {
                    vec![TextBlock {
                        text: full_text.clone(),
                        confidence: 0.8, // Tesseract default confidence estimate
                        bbox: (0.0, 0.0, 1.0, 1.0),
                    }]
                };

                OcrResult {
                    frame_timestamp_ms: timestamp_ms,
                    text_blocks,
                    full_text,
                }
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                log::debug!("Tesseract failed: {}", stderr);
                empty_result(timestamp_ms)
            }
            Err(e) => {
                log::debug!("Tesseract exec failed: {}", e);
                empty_result(timestamp_ms)
            }
        }
    }
}

fn empty_result(timestamp_ms: u64) -> OcrResult {
    OcrResult {
        frame_timestamp_ms: timestamp_ms,
        text_blocks: vec![],
        full_text: String::new(),
    }
}

/// Check if tesseract CLI is installed and accessible.
fn is_tesseract_available() -> bool {
    std::process::Command::new("tesseract")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
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
    }

    #[test]
    fn test_ocr_engine_initializes() {
        let config = OcrConfig::default();
        let engine = OcrEngine::new(config);
        // Should not panic
        let result = engine.process_frame(&vec![128u8; 320 * 240 * 4], 320, 240, 1000);
        assert_eq!(result.frame_timestamp_ms, 1000);
    }
}
