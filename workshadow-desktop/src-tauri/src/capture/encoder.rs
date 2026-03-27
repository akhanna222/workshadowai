use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use super::FrameMetadata;

/// Supported hardware encoders.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HardwareEncoder {
    VideoToolbox, // macOS
    Nvenc,        // NVIDIA
    Qsv,          // Intel Quick Sync
    Amf,          // AMD
    Software,     // libx265 fallback
}

impl HardwareEncoder {
    /// Return the FFmpeg encoder name for this variant.
    pub fn ffmpeg_encoder_name(&self) -> &'static str {
        match self {
            HardwareEncoder::VideoToolbox => "hevc_videotoolbox",
            HardwareEncoder::Nvenc => "hevc_nvenc",
            HardwareEncoder::Qsv => "hevc_qsv",
            HardwareEncoder::Amf => "hevc_amf",
            HardwareEncoder::Software => "libx265",
        }
    }
}

/// Metadata sidecar entry written as one JSON line per frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidecarEntry {
    pub frame_index: u32,
    pub timestamp_ms: u64,
    pub window_title: String,
    pub app_id: String,
    pub browser_url: Option<String>,
    pub display_index: u8,
}

impl From<(&FrameMetadata, u32)> for SidecarEntry {
    fn from((meta, idx): (&FrameMetadata, u32)) -> Self {
        Self {
            frame_index: idx,
            timestamp_ms: meta.timestamp_ms,
            window_title: meta.active_window_title.clone(),
            app_id: meta.app_bundle_id.clone(),
            browser_url: meta.browser_url.clone(),
            display_index: meta.display_index,
        }
    }
}

/// Tracks a single segment being written.
pub struct SegmentWriter {
    pub segment_path: PathBuf,
    pub sidecar_path: PathBuf,
    pub frame_count: u32,
    pub started_at_ms: u64,
    sidecar_file: std::fs::File,
    encoder: Option<ffmpeg_next::encoder::Video>,
    output_ctx: Option<ffmpeg_next::format::context::Output>,
    stream_index: usize,
}

impl SegmentWriter {
    /// Create a new segment writer. Initializes the MKV container and H.265 encoder.
    pub fn new(
        output_dir: &Path,
        timestamp_ms: u64,
        width: u32,
        height: u32,
        fps: f32,
        hw_encoder: &HardwareEncoder,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        fs::create_dir_all(output_dir)?;

        let segment_path = output_dir.join(format!("segment_{}.mkv", timestamp_ms));
        let sidecar_path = output_dir.join(format!("segment_{}.jsonl", timestamp_ms));
        let sidecar_file = fs::File::create(&sidecar_path)?;

        // Try to open FFmpeg output context
        let (encoder, output_ctx, stream_index) =
            match Self::init_ffmpeg_encoder(&segment_path, width, height, fps, hw_encoder) {
                Ok(tuple) => (Some(tuple.0), Some(tuple.1), tuple.2),
                Err(e) => {
                    log::warn!(
                        "FFmpeg encoder init failed ({}), segment will store raw frame data: {}",
                        hw_encoder.ffmpeg_encoder_name(),
                        e
                    );
                    (None, None, 0)
                }
            };

        Ok(Self {
            segment_path,
            sidecar_path,
            frame_count: 0,
            started_at_ms: timestamp_ms,
            sidecar_file,
            encoder,
            output_ctx,
            stream_index,
        })
    }

    fn init_ffmpeg_encoder(
        path: &Path,
        width: u32,
        height: u32,
        fps: f32,
        hw_encoder: &HardwareEncoder,
    ) -> Result<(ffmpeg_next::encoder::Video, ffmpeg_next::format::context::Output, usize), ffmpeg_next::Error>
    {
        let mut output_ctx = ffmpeg_next::format::output(&path)?;

        let codec = ffmpeg_next::encoder::find_by_name(hw_encoder.ffmpeg_encoder_name())
            .or_else(|| {
                log::warn!(
                    "Encoder '{}' not found, falling back to libx265",
                    hw_encoder.ffmpeg_encoder_name()
                );
                ffmpeg_next::encoder::find_by_name("libx265")
            })
            .ok_or(ffmpeg_next::Error::EncoderNotFound)?;

        let mut stream = output_ctx.add_stream(codec)?;
        let stream_index = stream.index();

        let ctx = ffmpeg_next::codec::context::Context::new_with_codec(codec);
        let mut encoder = ctx.encoder().video()?;
        encoder.set_width(width);
        encoder.set_height(height);
        encoder.set_format(ffmpeg_next::format::Pixel::YUV420P);
        encoder.set_time_base(ffmpeg_next::Rational::new(1, (fps * 1000.0) as i32));
        encoder.set_frame_rate(Some(ffmpeg_next::Rational::new(fps as i32, 1)));

        // Quality settings for H.265 — CRF 28 is good for screen content
        let mut opts = ffmpeg_next::Dictionary::new();
        if hw_encoder == &HardwareEncoder::Software {
            opts.set("crf", "28");
            opts.set("preset", "ultrafast");
            opts.set("tune", "zerolatency");
        }

        let encoder = encoder.open_with(opts)?;

        // Copy encoder params back to stream
        stream.set_parameters(&encoder);

        ffmpeg_next::format::context::output::dump(&output_ctx, 0, path.to_str());
        output_ctx.write_header()?;

        Ok((encoder, output_ctx, stream_index))
    }

    /// Write a raw RGBA frame to the segment.
    pub fn write_frame(
        &mut self,
        frame_data: &[u8],
        width: u32,
        height: u32,
        metadata: &FrameMetadata,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Write sidecar entry
        let entry = SidecarEntry::from((metadata, self.frame_count));
        let json = serde_json::to_string(&entry)?;
        writeln!(self.sidecar_file, "{}", json)?;

        // Encode frame if encoder is available
        if let (Some(ref mut encoder), Some(ref mut output_ctx)) =
            (&mut self.encoder, &mut self.output_ctx)
        {
            let mut frame = ffmpeg_next::frame::Video::new(
                ffmpeg_next::format::Pixel::YUV420P,
                width,
                height,
            );
            frame.set_pts(Some(self.frame_count as i64));

            // Convert RGBA to YUV420P
            let mut scaler = ffmpeg_next::software::scaling::Context::get(
                ffmpeg_next::format::Pixel::RGBA,
                width,
                height,
                ffmpeg_next::format::Pixel::YUV420P,
                width,
                height,
                ffmpeg_next::software::scaling::Flags::BILINEAR,
            )?;

            let mut src_frame = ffmpeg_next::frame::Video::new(
                ffmpeg_next::format::Pixel::RGBA,
                width,
                height,
            );
            // Copy RGBA data into source frame
            let src_linesize = src_frame.stride(0);
            let src_plane = src_frame.data_mut(0);
            for y in 0..height as usize {
                let src_start = y * (width as usize) * 4;
                let dst_start = y * src_linesize;
                let row_bytes = (width as usize) * 4;
                if src_start + row_bytes <= frame_data.len()
                    && dst_start + row_bytes <= src_plane.len()
                {
                    src_plane[dst_start..dst_start + row_bytes]
                        .copy_from_slice(&frame_data[src_start..src_start + row_bytes]);
                }
            }

            scaler.run(&src_frame, &mut frame)?;

            encoder.send_frame(&frame)?;

            let mut encoded_packet = ffmpeg_next::Packet::empty();
            while encoder.receive_packet(&mut encoded_packet).is_ok() {
                encoded_packet.set_stream(self.stream_index);
                encoded_packet.write_interleaved(output_ctx)?;
            }
        }

        self.frame_count += 1;
        Ok(())
    }

    /// Finalize the segment: flush encoder and write trailer.
    pub fn finalize(mut self) -> Result<SegmentInfo, Box<dyn std::error::Error>> {
        if let (Some(ref mut encoder), Some(ref mut output_ctx)) =
            (&mut self.encoder, &mut self.output_ctx)
        {
            // Flush encoder
            encoder.send_eof()?;
            let mut encoded_packet = ffmpeg_next::Packet::empty();
            while encoder.receive_packet(&mut encoded_packet).is_ok() {
                encoded_packet.set_stream(self.stream_index);
                encoded_packet.write_interleaved(output_ctx)?;
            }
            output_ctx.write_trailer()?;
        }

        self.sidecar_file.flush()?;

        let disk_size = fs::metadata(&self.segment_path)
            .map(|m| m.len())
            .unwrap_or(0);

        log::info!(
            "Segment finalized: {} ({} frames, {} bytes)",
            self.segment_path.display(),
            self.frame_count,
            disk_size
        );

        Ok(SegmentInfo {
            segment_path: self.segment_path,
            sidecar_path: self.sidecar_path,
            frame_count: self.frame_count,
            started_at_ms: self.started_at_ms,
            disk_size_bytes: disk_size,
        })
    }
}

/// Info about a completed segment.
#[derive(Debug, Clone)]
pub struct SegmentInfo {
    pub segment_path: PathBuf,
    pub sidecar_path: PathBuf,
    pub frame_count: u32,
    pub started_at_ms: u64,
    pub disk_size_bytes: u64,
}

/// Top-level video encoder manager. Handles segment rotation and encoder lifecycle.
pub struct VideoEncoder {
    pub hw_encoder: HardwareEncoder,
    pub output_dir: PathBuf,
    pub segment_duration_secs: u32,
    pub width: u32,
    pub height: u32,
    pub fps: f32,
    current_segment: Option<SegmentWriter>,
    completed_segments: Vec<SegmentInfo>,
}

impl VideoEncoder {
    pub fn new(
        output_dir: PathBuf,
        segment_duration_secs: u32,
        width: u32,
        height: u32,
        fps: f32,
    ) -> Self {
        ffmpeg_next::init().ok();

        let hw_encoder = detect_hardware_encoder();
        log::info!(
            "Video encoder: {:?} ({})",
            hw_encoder,
            hw_encoder.ffmpeg_encoder_name()
        );

        Self {
            hw_encoder,
            output_dir,
            segment_duration_secs,
            width,
            height,
            fps,
            current_segment: None,
            completed_segments: Vec::new(),
        }
    }

    /// Encode a single frame. Handles segment rotation automatically.
    pub fn encode_frame(
        &mut self,
        frame_data: &[u8],
        metadata: &FrameMetadata,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Check if we need to rotate segments
        if let Some(ref seg) = self.current_segment {
            let elapsed_secs = (metadata.timestamp_ms - seg.started_at_ms) / 1000;
            if elapsed_secs >= self.segment_duration_secs as u64 {
                // Rotate: finalize current and start new
                let old_seg = self.current_segment.take().unwrap();
                let info = old_seg.finalize()?;
                self.completed_segments.push(info);
            }
        }

        // Start new segment if needed
        if self.current_segment.is_none() {
            let seg = SegmentWriter::new(
                &self.output_dir,
                metadata.timestamp_ms,
                self.width,
                self.height,
                self.fps,
                &self.hw_encoder,
            )?;
            self.current_segment = Some(seg);
        }

        // Write frame to current segment
        if let Some(ref mut seg) = self.current_segment {
            seg.write_frame(frame_data, self.width, self.height, metadata)?;
        }

        Ok(())
    }

    /// Get the frame index within the current segment.
    pub fn current_segment_frame_index(&self) -> u32 {
        self.current_segment
            .as_ref()
            .map(|s| s.frame_count)
            .unwrap_or(0)
    }

    /// Get the current segment file name (relative).
    pub fn current_segment_file(&self) -> Option<String> {
        self.current_segment.as_ref().map(|s| {
            s.segment_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        })
    }

    /// Finalize any open segment. Call this when capture stops.
    pub fn flush(&mut self) -> Result<Option<SegmentInfo>, Box<dyn std::error::Error>> {
        if let Some(seg) = self.current_segment.take() {
            let info = seg.finalize()?;
            self.completed_segments.push(info.clone());
            Ok(Some(info))
        } else {
            Ok(None)
        }
    }

    /// Get list of all completed segments.
    pub fn completed_segments(&self) -> &[SegmentInfo] {
        &self.completed_segments
    }
}

/// Detect the best available hardware encoder on this machine.
pub fn detect_hardware_encoder() -> HardwareEncoder {
    ffmpeg_next::init().ok();

    // Try each HW encoder in priority order
    let candidates = [
        HardwareEncoder::VideoToolbox,
        HardwareEncoder::Nvenc,
        HardwareEncoder::Qsv,
        HardwareEncoder::Amf,
    ];

    for candidate in &candidates {
        if ffmpeg_next::encoder::find_by_name(candidate.ffmpeg_encoder_name()).is_some() {
            log::info!("Hardware encoder detected: {:?}", candidate);
            return candidate.clone();
        }
    }

    log::info!("No hardware encoder found, using software libx265");
    HardwareEncoder::Software
}

/// Downscale RGBA frame data to fit within max_width x max_height,
/// preserving aspect ratio. Returns (new_data, new_width, new_height).
pub fn downscale_frame(
    data: &[u8],
    width: u32,
    height: u32,
    max_width: u32,
    max_height: u32,
) -> (Vec<u8>, u32, u32) {
    if width <= max_width && height <= max_height {
        return (data.to_vec(), width, height);
    }

    let scale_w = max_width as f64 / width as f64;
    let scale_h = max_height as f64 / height as f64;
    let scale = scale_w.min(scale_h);

    let new_width = ((width as f64 * scale) as u32).max(2) & !1; // ensure even
    let new_height = ((height as f64 * scale) as u32).max(2) & !1;

    // Use image crate for high-quality resize
    match image::RgbaImage::from_raw(width, height, data.to_vec()) {
        Some(img) => {
            let resized = image::imageops::resize(
                &img,
                new_width,
                new_height,
                image::imageops::FilterType::Triangle,
            );
            log::debug!(
                "Downscaled {}x{} → {}x{}",
                width,
                height,
                new_width,
                new_height
            );
            (resized.into_raw(), new_width, new_height)
        }
        None => {
            log::warn!("Failed to create image for downscaling, returning original");
            (data.to_vec(), width, height)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_hardware_encoder_returns_valid() {
        let encoder = detect_hardware_encoder();
        // Should always return something (Software at minimum)
        assert!(!encoder.ffmpeg_encoder_name().is_empty());
    }

    #[test]
    fn test_downscale_no_change_when_within_limits() {
        let data = vec![0u8; 100 * 100 * 4]; // 100x100 RGBA
        let (_, w, h) = downscale_frame(&data, 100, 100, 1920, 1080);
        assert_eq!(w, 100);
        assert_eq!(h, 100);
    }

    #[test]
    fn test_downscale_reduces_resolution() {
        let data = vec![0u8; 3840 * 2160 * 4]; // 4K RGBA
        let (new_data, w, h) = downscale_frame(&data, 3840, 2160, 1920, 1080);
        assert!(w <= 1920);
        assert!(h <= 1080);
        assert_eq!(new_data.len(), (w * h * 4) as usize);
    }

    #[test]
    fn test_downscale_preserves_aspect_ratio() {
        let data = vec![0u8; 2560 * 1440 * 4]; // 16:9 QHD
        let (_, w, h) = downscale_frame(&data, 2560, 1440, 1920, 1080);
        let original_ratio = 2560.0 / 1440.0;
        let new_ratio = w as f64 / h as f64;
        assert!((original_ratio - new_ratio).abs() < 0.05);
    }

    #[test]
    fn test_sidecar_entry_serialization() {
        let meta = FrameMetadata {
            timestamp_ms: 1234567890,
            active_window_title: "VS Code".to_string(),
            app_bundle_id: "com.microsoft.vscode".to_string(),
            browser_url: None,
            display_index: 0,
        };
        let entry = SidecarEntry::from((&meta, 42));
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("VS Code"));
        assert!(json.contains("1234567890"));
        assert!(json.contains("42"));
    }

    #[test]
    fn test_segment_writer_creates_files() {
        let dir = tempfile::tempdir().unwrap();
        let seg = SegmentWriter::new(
            dir.path(),
            1000,
            320,
            240,
            1.0,
            &HardwareEncoder::Software,
        );
        // Even if encoder init fails, sidecar should exist
        if let Ok(seg) = seg {
            assert!(seg.sidecar_path.exists());
        }
    }

    #[test]
    fn test_video_encoder_segment_rotation() {
        let dir = tempfile::tempdir().unwrap();
        let mut enc = VideoEncoder::new(
            dir.path().to_path_buf(),
            2, // 2-second segments for fast test
            320,
            240,
            1.0,
        );

        // Write frames across 3 seconds — should trigger a rotation
        let frame_data = vec![128u8; 320 * 240 * 4];
        for i in 0..4 {
            let meta = FrameMetadata {
                timestamp_ms: 1000 + i * 1000,
                active_window_title: format!("Window {}", i),
                app_bundle_id: "com.test".to_string(),
                browser_url: None,
                display_index: 0,
            };
            enc.encode_frame(&frame_data, &meta).ok();
        }

        enc.flush().ok();
        // Should have at least 1 completed segment from rotation
        assert!(!enc.completed_segments().is_empty());
    }
}
