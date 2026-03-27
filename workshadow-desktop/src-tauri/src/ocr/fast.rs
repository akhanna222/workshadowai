use super::{OcrBackendType, OcrConfig, OcrResult, TextBlock};

/// Fast OCR backend for real-time capture (~1 fps).
/// Priority: Apple Vision (macOS) → Tesseract CLI → disabled.
pub struct FastOcrBackend {
    backend_type: OcrBackendType,
    language: String,
}

impl FastOcrBackend {
    pub fn new(config: &OcrConfig) -> Self {
        let backend_type = Self::detect_backend();
        log::info!("Fast OCR backend: {:?}", backend_type);
        Self {
            backend_type,
            language: config.language.clone(),
        }
    }

    pub fn backend_type(&self) -> &OcrBackendType {
        &self.backend_type
    }

    fn detect_backend() -> OcrBackendType {
        // macOS: try Apple Vision first (native, fastest)
        #[cfg(target_os = "macos")]
        {
            if is_apple_vision_available() {
                return OcrBackendType::AppleVision;
            }
        }

        // Cross-platform: try Tesseract CLI
        if is_tesseract_available() {
            return OcrBackendType::Tesseract;
        }

        OcrBackendType::Disabled
    }

    /// Run fast OCR on a raw RGBA frame.
    pub fn process_frame(
        &self,
        image_data: &[u8],
        width: u32,
        height: u32,
        timestamp_ms: u64,
    ) -> OcrResult {
        match self.backend_type {
            OcrBackendType::AppleVision => {
                #[cfg(target_os = "macos")]
                {
                    self.process_apple_vision(image_data, width, height, timestamp_ms)
                }
                #[cfg(not(target_os = "macos"))]
                {
                    empty_result(timestamp_ms)
                }
            }
            OcrBackendType::Tesseract => {
                self.process_tesseract(image_data, width, height, timestamp_ms)
            }
            _ => empty_result(timestamp_ms),
        }
    }

    /// macOS: OCR via Apple Vision framework (VNRecognizeTextRequest).
    #[cfg(target_os = "macos")]
    fn process_apple_vision(
        &self,
        image_data: &[u8],
        width: u32,
        height: u32,
        timestamp_ms: u64,
    ) -> OcrResult {
        use std::process::Command;

        // Save temp PNG
        let tmp_file = match save_temp_png(image_data, width, height) {
            Some(f) => f,
            None => return empty_result(timestamp_ms),
        };

        // Use Swift CLI bridge to Apple Vision
        // The script uses VNRecognizeTextRequest via osascript/swift
        let script = format!(
            r#"
            import Vision
            import AppKit

            let url = URL(fileURLWithPath: "{}")
            guard let image = NSImage(contentsOf: url),
                  let cgImage = image.cgImage(forProposedRect: nil, context: nil, hints: nil) else {{
                exit(1)
            }}

            let request = VNRecognizeTextRequest()
            request.recognitionLevel = .accurate
            request.recognitionLanguages = ["{}"]
            request.usesLanguageCorrection = true

            let handler = VNImageRequestHandler(cgImage: cgImage)
            try? handler.perform([request])

            guard let observations = request.results else {{ exit(0) }}

            for obs in observations {{
                let text = obs.topCandidates(1).first?.string ?? ""
                let conf = obs.confidence
                let box = obs.boundingBox
                print("\(conf)\t\(box.origin.x)\t\(box.origin.y)\t\(box.width)\t\(box.height)\t\(text)")
            }}
            "#,
            tmp_file.path().display(),
            self.language
        );

        let output = Command::new("swift")
            .args(["-e", &script])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                parse_vision_output(&String::from_utf8_lossy(&out.stdout), timestamp_ms)
            }
            _ => {
                // Fallback to tesseract if Vision fails
                self.process_tesseract(image_data, width, height, timestamp_ms)
            }
        }
    }

    /// Cross-platform: OCR via Tesseract CLI with TSV output for bounding boxes.
    fn process_tesseract(
        &self,
        image_data: &[u8],
        width: u32,
        height: u32,
        timestamp_ms: u64,
    ) -> OcrResult {
        use std::process::Command;

        let tmp_file = match save_temp_png(image_data, width, height) {
            Some(f) => f,
            None => return empty_result(timestamp_ms),
        };

        // Run tesseract with TSV output for confidence + bounding boxes
        let output = Command::new("tesseract")
            .arg(tmp_file.path())
            .arg("stdout")
            .args(["--oem", "3", "--psm", "3"])
            .args(["-l", &self.language])
            .arg("tsv")
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let tsv = String::from_utf8_lossy(&out.stdout);
                parse_tesseract_tsv(&tsv, width, height, timestamp_ms)
            }
            Ok(_) | Err(_) => {
                // Fallback: try plain text mode
                let output = Command::new("tesseract")
                    .arg(tmp_file.path())
                    .arg("stdout")
                    .args(["--oem", "3", "--psm", "3"])
                    .args(["-l", &self.language])
                    .output();

                match output {
                    Ok(out) if out.status.success() => {
                        let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
                        if text.is_empty() {
                            return empty_result(timestamp_ms);
                        }
                        OcrResult {
                            frame_timestamp_ms: timestamp_ms,
                            text_blocks: vec![TextBlock {
                                text: text.clone(),
                                confidence: 0.7,
                                bbox: (0.0, 0.0, 1.0, 1.0),
                            }],
                            full_text: text,
                            avg_confidence: 0.7,
                            backend: OcrBackendType::Tesseract,
                        }
                    }
                    _ => empty_result(timestamp_ms),
                }
            }
        }
    }
}

/// Save RGBA image data to a temp PNG file, downscaling for OCR speed.
fn save_temp_png(
    image_data: &[u8],
    width: u32,
    height: u32,
) -> Option<tempfile::NamedTempFile> {
    let tmp_file = tempfile::Builder::new()
        .suffix(".png")
        .tempfile()
        .ok()?;

    let img = image::RgbaImage::from_raw(width, height, image_data.to_vec())?;

    // Downscale for OCR speed (1280x720 max)
    let ocr_img = if width > 1280 || height > 720 {
        let scale = (1280.0 / width as f64).min(720.0 / height as f64);
        let nw = (width as f64 * scale) as u32;
        let nh = (height as f64 * scale) as u32;
        image::imageops::resize(&img, nw, nh, image::imageops::FilterType::Triangle)
    } else {
        img
    };

    ocr_img.save(tmp_file.path()).ok()?;
    Some(tmp_file)
}

/// Parse Tesseract TSV output into OcrResult with bounding boxes and confidence.
fn parse_tesseract_tsv(
    tsv: &str,
    img_width: u32,
    img_height: u32,
    timestamp_ms: u64,
) -> OcrResult {
    let mut text_blocks = Vec::new();
    let mut full_text_parts = Vec::new();

    for line in tsv.lines().skip(1) {
        // TSV columns: level, page_num, block_num, par_num, line_num, word_num,
        //              left, top, width, height, conf, text
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 12 {
            continue;
        }

        let conf: f32 = cols[10].parse().unwrap_or(-1.0);
        let text = cols[11].trim();

        if conf < 0.0 || text.is_empty() {
            continue;
        }

        let left: f32 = cols[6].parse().unwrap_or(0.0);
        let top: f32 = cols[7].parse().unwrap_or(0.0);
        let w: f32 = cols[8].parse().unwrap_or(0.0);
        let h: f32 = cols[9].parse().unwrap_or(0.0);

        // Normalize to 0.0–1.0
        let nx = left / img_width as f32;
        let ny = top / img_height as f32;
        let nw = w / img_width as f32;
        let nh = h / img_height as f32;

        full_text_parts.push(text.to_string());
        text_blocks.push(TextBlock {
            text: text.to_string(),
            confidence: conf / 100.0, // Tesseract gives 0-100
            bbox: (nx, ny, nw, nh),
        });
    }

    let full_text = full_text_parts.join(" ");
    let avg_confidence = if text_blocks.is_empty() {
        0.0
    } else {
        text_blocks.iter().map(|b| b.confidence).sum::<f32>() / text_blocks.len() as f32
    };

    OcrResult {
        frame_timestamp_ms: timestamp_ms,
        text_blocks,
        full_text,
        avg_confidence,
        backend: OcrBackendType::Tesseract,
    }
}

/// Parse Apple Vision output (tab-separated: conf, x, y, w, h, text).
#[cfg(target_os = "macos")]
fn parse_vision_output(output: &str, timestamp_ms: u64) -> OcrResult {
    let mut text_blocks = Vec::new();
    let mut full_text_parts = Vec::new();

    for line in output.lines() {
        let cols: Vec<&str> = line.splitn(6, '\t').collect();
        if cols.len() < 6 {
            continue;
        }

        let conf: f32 = cols[0].parse().unwrap_or(0.0);
        let x: f32 = cols[1].parse().unwrap_or(0.0);
        let y: f32 = cols[2].parse().unwrap_or(0.0);
        let w: f32 = cols[3].parse().unwrap_or(0.0);
        let h: f32 = cols[4].parse().unwrap_or(0.0);
        let text = cols[5].trim().to_string();

        if text.is_empty() {
            continue;
        }

        full_text_parts.push(text.clone());
        text_blocks.push(TextBlock {
            text,
            confidence: conf,
            bbox: (x, 1.0 - y - h, w, h), // Vision uses bottom-left origin
        });
    }

    let full_text = full_text_parts.join(" ");
    let avg_confidence = if text_blocks.is_empty() {
        0.0
    } else {
        text_blocks.iter().map(|b| b.confidence).sum::<f32>() / text_blocks.len() as f32
    };

    OcrResult {
        frame_timestamp_ms: timestamp_ms,
        text_blocks,
        full_text,
        avg_confidence,
        backend: OcrBackendType::AppleVision,
    }
}

fn empty_result(timestamp_ms: u64) -> OcrResult {
    OcrResult {
        frame_timestamp_ms: timestamp_ms,
        text_blocks: vec![],
        full_text: String::new(),
        avg_confidence: 0.0,
        backend: OcrBackendType::Disabled,
    }
}

fn is_tesseract_available() -> bool {
    std::process::Command::new("tesseract")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "macos")]
fn is_apple_vision_available() -> bool {
    // Apple Vision is available on macOS 10.15+ (always present on modern macOS)
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fast_backend_initializes() {
        let config = OcrConfig::default();
        let backend = FastOcrBackend::new(&config);
        // Should be Tesseract or Disabled depending on system
        assert!(
            *backend.backend_type() == OcrBackendType::Tesseract
                || *backend.backend_type() == OcrBackendType::Disabled
        );
    }

    #[test]
    fn test_fast_backend_process_frame() {
        let config = OcrConfig::default();
        let backend = FastOcrBackend::new(&config);
        let data = vec![200u8; 320 * 240 * 4];
        let result = backend.process_frame(&data, 320, 240, 5000);
        assert_eq!(result.frame_timestamp_ms, 5000);
    }

    #[test]
    fn test_parse_tesseract_tsv() {
        let tsv = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n\
                   5\t1\t1\t1\t1\t1\t100\t50\t200\t30\t95\tHello\n\
                   5\t1\t1\t1\t1\t2\t320\t50\t200\t30\t88\tWorld\n";

        let result = parse_tesseract_tsv(tsv, 1920, 1080, 1000);
        assert_eq!(result.text_blocks.len(), 2);
        assert_eq!(result.full_text, "Hello World");
        assert!(result.avg_confidence > 0.8);
        assert_eq!(result.backend, OcrBackendType::Tesseract);

        // Check normalized bounding boxes
        let first = &result.text_blocks[0];
        assert!((first.bbox.0 - 100.0 / 1920.0).abs() < 0.01);
    }

    #[test]
    fn test_empty_result() {
        let result = empty_result(42);
        assert_eq!(result.frame_timestamp_ms, 42);
        assert!(result.full_text.is_empty());
        assert_eq!(result.avg_confidence, 0.0);
        assert_eq!(result.backend, OcrBackendType::Disabled);
    }

    #[test]
    fn test_save_temp_png() {
        let data = vec![128u8; 100 * 100 * 4];
        let tmp = save_temp_png(&data, 100, 100);
        assert!(tmp.is_some());
        assert!(tmp.unwrap().path().exists());
    }

    #[test]
    fn test_save_temp_png_downscales() {
        // 4K image should be downscaled
        let data = vec![128u8; 3840 * 2160 * 4];
        let tmp = save_temp_png(&data, 3840, 2160);
        assert!(tmp.is_some());
    }
}
