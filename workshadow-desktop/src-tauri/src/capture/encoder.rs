use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Supported hardware encoders.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HardwareEncoder {
    VideoToolbox, // macOS
    Nvenc,        // NVIDIA
    Qsv,          // Intel Quick Sync
    Amf,          // AMD
    Software,     // libx265 fallback
}

/// Manages H.265 video encoding for captured frames.
pub struct VideoEncoder {
    pub encoder: HardwareEncoder,
    pub output_dir: PathBuf,
    pub segment_duration_secs: u32,
    current_segment: Option<SegmentWriter>,
}

struct SegmentWriter {
    pub path: PathBuf,
    pub frame_count: u32,
    #[allow(dead_code)]
    pub started_at_ms: u64,
}

impl VideoEncoder {
    pub fn new(output_dir: PathBuf, segment_duration_secs: u32) -> Self {
        let encoder = Self::detect_hardware_encoder();
        log::info!("Video encoder selected: {:?}", encoder);

        Self {
            encoder,
            output_dir,
            segment_duration_secs,
            current_segment: None,
        }
    }

    /// Detect the best available hardware encoder on the current platform.
    fn detect_hardware_encoder() -> HardwareEncoder {
        // TODO: Probe for hardware encoder availability
        // macOS: check for VideoToolbox
        // Windows: check NVENC → QSV → AMF
        // Fallback: software (libx265)
        log::warn!("Hardware encoder detection not yet implemented, using software fallback");
        HardwareEncoder::Software
    }

    /// Encode a raw frame and write it to the current segment.
    pub fn encode_frame(&mut self, _frame_data: &[u8], _width: u32, _height: u32, timestamp_ms: u64) {
        // TODO: Integrate ffmpeg-next for actual H.265 encoding
        // 1. Check if current segment has exceeded duration → rotate
        // 2. Encode frame via hardware/software encoder
        // 3. Write to .mkv container
        if let Some(ref mut seg) = self.current_segment {
            seg.frame_count += 1;
        } else {
            let path = self.output_dir.join(format!("segment_{}.mkv", timestamp_ms));
            self.current_segment = Some(SegmentWriter {
                path,
                frame_count: 1,
                started_at_ms: timestamp_ms,
            });
        }
    }

    /// Finalize and close the current segment.
    pub fn flush_segment(&mut self) -> Option<PathBuf> {
        self.current_segment.take().map(|seg| {
            log::info!("Segment finalized: {} ({} frames)", seg.path.display(), seg.frame_count);
            seg.path
        })
    }
}
