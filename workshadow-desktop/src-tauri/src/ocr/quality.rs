use super::{OcrBackendType, OcrConfig, OcrResult, TextBlock};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

/// Quality OCR backend using DeepSeek-OCR-2 via llama.cpp (llama-cli).
/// Triggered on-demand for low-confidence frames or user re-analysis requests.
pub struct QualityOcrBackend {
    model_path: Option<PathBuf>,
    available: AtomicBool,
}

impl QualityOcrBackend {
    pub fn new(config: &OcrConfig) -> Self {
        let model_path = config
            .deepseek_model_path
            .as_ref()
            .map(PathBuf::from)
            .or_else(Self::find_default_model);

        let available = model_path
            .as_ref()
            .map(|p| p.exists() && is_llama_cli_available())
            .unwrap_or(false);

        if available {
            log::info!(
                "Quality OCR backend: DeepSeek-OCR-2 at {:?}",
                model_path.as_ref().unwrap()
            );
        } else {
            log::info!(
                "Quality OCR backend: not available (model={:?}, llama-cli={})",
                model_path,
                is_llama_cli_available()
            );
        }

        Self {
            model_path,
            available: AtomicBool::new(available),
        }
    }

    pub fn is_available(&self) -> bool {
        self.available.load(Ordering::Relaxed)
    }

    /// Look for the model in default locations.
    fn find_default_model() -> Option<PathBuf> {
        let candidates = [
            // ~/.workshadow/models/
            dirs::home_dir()
                .map(|h| h.join(".workshadow/models/deepseek-ocr-2.gguf")),
            // HuggingFace cache
            dirs::cache_dir()
                .map(|c| c.join("huggingface/hub/models--ggml-org--DeepSeek-OCR-GGUF/snapshots")),
            // Current directory
            Some(PathBuf::from("models/deepseek-ocr-2.gguf")),
        ];

        for candidate in candidates.iter().flatten() {
            if candidate.exists() {
                // If it's a directory (HF cache), find the .gguf file inside
                if candidate.is_dir() {
                    if let Some(gguf) = find_gguf_in_dir(candidate) {
                        return Some(gguf);
                    }
                } else {
                    return Some(candidate.clone());
                }
            }
        }

        None
    }

    /// Run quality OCR on a frame image. This is slower but more accurate.
    pub fn process_frame(
        &self,
        image_data: &[u8],
        width: u32,
        height: u32,
        timestamp_ms: u64,
    ) -> OcrResult {
        if !self.is_available() {
            return empty_result(timestamp_ms);
        }

        let model_path = match &self.model_path {
            Some(p) => p,
            None => return empty_result(timestamp_ms),
        };

        // Save temp image
        let tmp_file = match save_temp_png(image_data, width, height) {
            Some(f) => f,
            None => return empty_result(timestamp_ms),
        };

        // Run llama-cli with the DeepSeek-OCR-2 model
        // The model expects an image input and produces OCR text output
        let output = Command::new("llama-cli")
            .args([
                "-m",
                model_path.to_str().unwrap_or(""),
                "--image",
                tmp_file.path().to_str().unwrap_or(""),
                "-p",
                "Extract all visible text from this screenshot. Output only the text content, preserving layout where possible.",
                "-n",
                "2048",
                "--temp",
                "0.1",
                "-ngl",
                "99", // Offload all layers to GPU/Metal
            ])
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
                        confidence: 0.95, // LLM-based OCR is generally high confidence
                        bbox: (0.0, 0.0, 1.0, 1.0),
                    }],
                    full_text: text,
                    avg_confidence: 0.95,
                    backend: OcrBackendType::DeepSeekOcr,
                }
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                log::warn!("DeepSeek-OCR failed: {}", stderr.chars().take(200).collect::<String>());
                empty_result(timestamp_ms)
            }
            Err(e) => {
                log::warn!("DeepSeek-OCR exec failed: {}", e);
                self.available.store(false, Ordering::Relaxed);
                empty_result(timestamp_ms)
            }
        }
    }

    /// Download the DeepSeek-OCR-2 GGUF model to the default location.
    /// Returns the path to the downloaded model.
    pub fn download_model() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let model_dir = dirs::home_dir()
            .ok_or("No home directory")?
            .join(".workshadow/models");
        std::fs::create_dir_all(&model_dir)?;

        let model_path = model_dir.join("deepseek-ocr-2-Q8_0.gguf");
        if model_path.exists() {
            log::info!("Model already exists at {:?}", model_path);
            return Ok(model_path);
        }

        log::info!("Downloading DeepSeek-OCR-2 GGUF model...");

        // Use huggingface-cli or curl to download
        let status = Command::new("huggingface-cli")
            .args([
                "download",
                "ggml-org/DeepSeek-OCR-GGUF",
                "DeepSeek-OCR-2-Q8_0.gguf",
                "--local-dir",
                model_dir.to_str().unwrap_or(""),
            ])
            .status();

        match status {
            Ok(s) if s.success() => {
                // huggingface-cli may save with original name
                let alt_path = model_dir.join("DeepSeek-OCR-2-Q8_0.gguf");
                if alt_path.exists() && !model_path.exists() {
                    std::fs::rename(&alt_path, &model_path)?;
                }
                log::info!("Model downloaded to {:?}", model_path);
                Ok(model_path)
            }
            _ => {
                // Fallback to curl
                let url = "https://huggingface.co/ggml-org/DeepSeek-OCR-GGUF/resolve/main/DeepSeek-OCR-2-Q8_0.gguf";
                let status = Command::new("curl")
                    .args(["-L", "-o", model_path.to_str().unwrap_or(""), url])
                    .status()?;

                if status.success() {
                    Ok(model_path)
                } else {
                    Err("Failed to download model".into())
                }
            }
        }
    }
}

fn save_temp_png(image_data: &[u8], width: u32, height: u32) -> Option<tempfile::NamedTempFile> {
    let tmp_file = tempfile::Builder::new()
        .suffix(".png")
        .tempfile()
        .ok()?;
    let img = image::RgbaImage::from_raw(width, height, image_data.to_vec())?;
    img.save(tmp_file.path()).ok()?;
    Some(tmp_file)
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

fn is_llama_cli_available() -> bool {
    Command::new("llama-cli")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn find_gguf_in_dir(dir: &Path) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .flat_map(|e| {
            let p = e.path();
            if p.is_dir() {
                // Search one level deeper (HF cache structure)
                std::fs::read_dir(&p)
                    .ok()
                    .into_iter()
                    .flatten()
                    .filter_map(|e2| e2.ok())
                    .map(|e2| e2.path())
                    .collect::<Vec<_>>()
            } else {
                vec![p]
            }
        })
        .find(|p| p.extension().map_or(false, |ext| ext == "gguf"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_backend_initializes() {
        let config = OcrConfig::default();
        let backend = QualityOcrBackend::new(&config);
        // Will be unavailable in test env (no model file)
        // Just verify it doesn't panic
        let _ = backend.is_available();
    }

    #[test]
    fn test_quality_backend_unavailable_returns_empty() {
        let config = OcrConfig {
            deepseek_model_path: Some("/nonexistent/model.gguf".to_string()),
            ..Default::default()
        };
        let backend = QualityOcrBackend::new(&config);
        let result = backend.process_frame(&vec![0u8; 100 * 100 * 4], 100, 100, 1000);
        assert!(result.full_text.is_empty());
        assert_eq!(result.backend, OcrBackendType::Disabled);
    }

    #[test]
    fn test_find_default_model_returns_none() {
        // In test env, no model should exist
        // This just verifies the function doesn't panic
        let _ = QualityOcrBackend::find_default_model();
    }
}
