use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Debug, Clone)]
pub struct SdImage {
    pub width: u32,
    pub height: u32,
    pub channel: u32,
    pub data: Vec<u8>,
}

impl Serialize for SdImage {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("SdImage", 4)?;
        state.serialize_field("width", &self.width)?;
        state.serialize_field("height", &self.height)?;
        state.serialize_field("channel", &self.channel)?;
        state.serialize_field("data", &base64_encode(&self.data))?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for SdImage {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct SdImageHelper {
            width: u32,
            height: u32,
            channel: u32,
            data: String,
        }
        let helper = SdImageHelper::deserialize(deserializer)?;
        let data = base64_decode(&helper.data).map_err(serde::de::Error::custom)?;
        SdImage::from_raw(helper.width, helper.height, helper.channel, data)
            .map_err(serde::de::Error::custom)
    }
}

impl SdImage {
    pub fn new(width: u32, height: u32, channel: u32) -> Self {
        Self {
            width,
            height,
            channel,
            data: vec![0u8; (width * height * channel) as usize],
        }
    }

    pub fn from_raw(width: u32, height: u32, channel: u32, data: Vec<u8>) -> Result<Self, ImageError> {
        let expected_len = (width * height * channel) as usize;
        if data.len() != expected_len {
            return Err(ImageError::SizeMismatch {
                expected: expected_len,
                actual: data.len(),
            });
        }
        Ok(Self { width, height, channel, data })
    }

    pub fn rgb(width: u32, height: u32, data: Vec<u8>) -> Result<Self, ImageError> {
        Self::from_raw(width, height, 3, data)
    }

    pub fn rgba(width: u32, height: u32, data: Vec<u8>) -> Result<Self, ImageError> {
        Self::from_raw(width, height, 4, data)
    }

    pub fn grayscale(width: u32, height: u32, data: Vec<u8>) -> Result<Self, ImageError> {
        Self::from_raw(width, height, 1, data)
    }

    pub fn pixel_count(&self) -> usize {
        (self.width * self.height) as usize
    }

    pub fn byte_len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    pub fn to_png_bytes(&self) -> Result<Vec<u8>, ImageError> {
        encode_png(self)
    }

    pub fn from_png_bytes(bytes: &[u8]) -> Result<Self, ImageError> {
        decode_png(bytes)
    }

    pub fn to_base64_png(&self) -> Result<String, ImageError> {
        let png_bytes = self.to_png_bytes()?;
        Ok(base64_encode(&png_bytes))
    }

    /// Create a solid color image
    pub fn solid(width: u32, height: u32, r: u8, g: u8, b: u8) -> Self {
        let data_size = (width * height * 3) as usize;
        let mut data = vec![0u8; data_size];
        for i in 0..(width * height) as usize {
            data[i * 3] = r;
            data[i * 3 + 1] = g;
            data[i * 3 + 2] = b;
        }
        Self { width, height, channel: 3, data }
    }

    /// Create a black frame
    pub fn black(width: u32, height: u32) -> Self {
        Self::solid(width, height, 0, 0, 0)
    }

    /// Alpha blend this image on top of another (both must be RGB, same size)
    pub fn blend_over(&self, base: &SdImage, opacity: f32) -> SdImage {
        if self.width != base.width || self.height != base.height {
            return base.clone();
        }
        let opacity = opacity.clamp(0.0, 1.0);
        if opacity <= 0.0 {
            return base.clone();
        }
        if opacity >= 1.0 {
            return self.clone();
        }
        let w = self.width;
        let h = self.height;
        let channels = 3;
        let mut data = vec![0u8; (w * h * channels) as usize];
        for i in 0..data.len() {
            let fg = self.data[i] as f32;
            let bg = base.data[i] as f32;
            data[i] = ((fg * opacity) + (bg * (1.0 - opacity))) as u8;
        }
        SdImage { width: w, height: h, channel: channels, data }
    }

    /// Resize to target dimensions using simple bilinear interpolation
    pub fn resize(&self, w: u32, h: u32) -> SdImage {
        if self.width == w && self.height == h {
            return self.clone();
        }
        if w == 0 || h == 0 {
            return SdImage::black(1, 1);
        }
        let mut data = vec![0u8; (w * h * 3) as usize];
        let x_ratio = (self.width as f32 - 1.0) / w as f32;
        let y_ratio = (self.height as f32 - 1.0) / h as f32;

        for y in 0..h {
            for x in 0..w {
                let src_x = x as f32 * x_ratio;
                let src_y = y as f32 * y_ratio;
                let x0 = src_x.floor() as u32;
                let y0 = src_y.floor() as u32;
                let x1 = (x0 + 1).min(self.width - 1);
                let y1 = (y0 + 1).min(self.height - 1);
                let x_frac = src_x - x0 as f32;
                let y_frac = src_y - y0 as f32;

                for c in 0..3 {
                    let idx = |px: u32, py: u32| -> usize {
                        ((py * self.width + px) * 3 + c) as usize
                    };
                    let v00 = self.data[idx(x0, y0)] as f32;
                    let v10 = self.data[idx(x1, y0)] as f32;
                    let v01 = self.data[idx(x0, y1)] as f32;
                    let v11 = self.data[idx(x1, y1)] as f32;

                    let v0 = v00 * (1.0 - x_frac) + v10 * x_frac;
                    let v1 = v01 * (1.0 - x_frac) + v11 * x_frac;
                    let v = v0 * (1.0 - y_frac) + v1 * y_frac;

                    let tgt_idx = ((y * w + x) * 3 + c) as usize;
                    data[tgt_idx] = v.clamp(0.0, 255.0) as u8;
                }
            }
        }
        SdImage { width: w, height: h, channel: 3, data }
    }
}

#[derive(Debug, Clone)]
pub struct SdAudio {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u32,
}

impl SdAudio {
    pub fn new(samples: Vec<f32>, sample_rate: u32, channels: u32) -> Self {
        Self { samples, sample_rate, channels }
    }

    pub fn from_wav_bytes(bytes: &[u8]) -> Result<Self, ImageError> {
        // Simple WAV parser (PCM 16-bit mono/stereo)
        if bytes.len() < 44 {
            return Err(ImageError::Base64Error("WAV too short".to_string()));
        }
        // Check RIFF header
        if &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return Err(ImageError::Base64Error("Invalid WAV header".to_string()));
        }
        // Parse fmt chunk
        let channels = u16::from_le_bytes([bytes[22], bytes[23]]) as u32;
        let sample_rate = u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);
        let bits_per_sample = u16::from_le_bytes([bytes[34], bytes[35]]);
        // Find data chunk
        let mut offset = 36;
        while offset + 8 <= bytes.len() {
            let chunk_id = &bytes[offset..offset+4];
            let chunk_size = u32::from_le_bytes([
                bytes[offset+4], bytes[offset+5], bytes[offset+6], bytes[offset+7]
            ]) as usize;
            if chunk_id == b"data" {
                let data_start = offset + 8;
                let data_end = data_start + chunk_size;
                if data_end > bytes.len() {
                    return Err(ImageError::Base64Error("WAV data truncated".to_string()));
                }
                let mut samples = Vec::with_capacity(chunk_size / 2);
                match bits_per_sample {
                    16 => {
                        for i in (0..chunk_size).step_by(2) {
                            let idx = data_start + i;
                            if idx + 1 < data_end {
                                let s = i16::from_le_bytes([bytes[idx], bytes[idx+1]]);
                                samples.push(s as f32 / 32768.0);
                            }
                        }
                    }
                    _ => {
                        return Err(ImageError::Base64Error(
                            format!("Unsupported bits per sample: {}", bits_per_sample)
                        ));
                    }
                }
                return Ok(Self { samples, sample_rate, channels });
            }
            offset += 8 + chunk_size;
        }
        Err(ImageError::Base64Error("No data chunk in WAV".to_string()))
    }

    pub fn duration_sec(&self) -> f32 {
        if self.channels == 0 || self.sample_rate == 0 {
            return 0.0;
        }
        self.samples.len() as f32 / (self.channels as f32 * self.sample_rate as f32)
    }

    pub fn to_wav_bytes(&self) -> Vec<u8> {
        let bits_per_sample: u16 = 16;
        let byte_rate = self.sample_rate * self.channels * (bits_per_sample / 8) as u32;
        let block_align = self.channels * (bits_per_sample / 8) as u32;
        let data_size = (self.samples.len() * 2) as u32;
        let file_size = 36 + data_size;

        let mut buf = Vec::with_capacity((44 + data_size) as usize);
        // RIFF header
        buf.extend_from_slice(b"RIFF");
        buf.extend_from_slice(&file_size.to_le_bytes());
        buf.extend_from_slice(b"WAVE");
        // fmt chunk
        buf.extend_from_slice(b"fmt ");
        buf.extend_from_slice(&16u32.to_le_bytes()); // chunk size
        buf.extend_from_slice(&1u16.to_le_bytes());  // PCM format
        buf.extend_from_slice(&(self.channels as u16).to_le_bytes());
        buf.extend_from_slice(&self.sample_rate.to_le_bytes());
        buf.extend_from_slice(&byte_rate.to_le_bytes());
        buf.extend_from_slice(&(block_align as u16).to_le_bytes());
        buf.extend_from_slice(&bits_per_sample.to_le_bytes());
        // data chunk
        buf.extend_from_slice(b"data");
        buf.extend_from_slice(&data_size.to_le_bytes());
        for &sample in &self.samples {
            let s = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
            buf.extend_from_slice(&s.to_le_bytes());
        }
        buf
    }

    /// Trim audio to start_sec..end_sec (in seconds)
    pub fn trim(&self, start_sec: f32, end_sec: f32) -> Self {
        if self.samples.is_empty() || self.sample_rate == 0 || self.channels == 0 {
            return self.clone();
        }
        let total_dur = self.duration_sec();
        let start = start_sec.max(0.0);
        let end = end_sec.min(total_dur);
        if start >= end {
            return Self::new(vec![], self.sample_rate, self.channels);
        }
        let start_sample = (start * self.sample_rate as f32 * self.channels as f32) as usize;
        let end_sample = (end * self.sample_rate as f32 * self.channels as f32) as usize;
        let start_sample = start_sample.min(self.samples.len());
        let end_sample = end_sample.min(self.samples.len());
        Self::new(self.samples[start_sample..end_sample].to_vec(), self.sample_rate, self.channels)
    }

    /// Adjust volume by multiplier (1.0 = original, 0.0 = mute, 2.0 = double)
    pub fn adjust_volume(&self, multiplier: f32) -> Self {
        let samples: Vec<f32> = self.samples.iter()
            .map(|&s| (s * multiplier).clamp(-1.0, 1.0))
            .collect();
        Self::new(samples, self.sample_rate, self.channels)
    }

    /// Apply fade-in effect over duration_sec seconds
    pub fn fade_in(&self, duration_sec: f32) -> Self {
        if duration_sec <= 0.0 || self.samples.is_empty() {
            return self.clone();
        }
        let fade_samples = (duration_sec * self.sample_rate as f32 * self.channels as f32) as usize;
        let fade_samples = fade_samples.min(self.samples.len());
        let mut samples = self.samples.clone();
        for i in 0..fade_samples {
            let gain = i as f32 / fade_samples as f32;
            samples[i] = (samples[i] * gain).clamp(-1.0, 1.0);
        }
        Self::new(samples, self.sample_rate, self.channels)
    }

    /// Apply fade-out effect over duration_sec seconds
    pub fn fade_out(&self, duration_sec: f32) -> Self {
        if duration_sec <= 0.0 || self.samples.is_empty() {
            return self.clone();
        }
        let fade_samples = (duration_sec * self.sample_rate as f32 * self.channels as f32) as usize;
        let fade_samples = fade_samples.min(self.samples.len());
        let mut samples = self.samples.clone();
        let total = self.samples.len();
        for i in 0..fade_samples {
            let idx = total - fade_samples + i;
            let gain = 1.0 - (i as f32 / fade_samples as f32);
            samples[idx] = (samples[idx] * gain).clamp(-1.0, 1.0);
        }
        Self::new(samples, self.sample_rate, self.channels)
    }

    /// Mix two audio tracks together. If sample rates differ, resamples other to self's rate.
    pub fn mix(&self, other: &SdAudio, other_volume: f32) -> Self {
        if self.samples.is_empty() {
            return other.adjust_volume(other_volume);
        }
        if other.samples.is_empty() {
            return self.clone();
        }

        // Resample other if sample rates differ (simple linear interpolation)
        let other_resampled = if other.sample_rate != self.sample_rate || other.channels != self.channels {
            other.resample_to(self.sample_rate, self.channels)
        } else {
            other.clone()
        };

        let max_len = self.samples.len().max(other_resampled.samples.len());
        let mut mixed = vec![0.0f32; max_len];
        for i in 0..self.samples.len() {
            mixed[i] += self.samples[i];
        }
        for i in 0..other_resampled.samples.len() {
            mixed[i] += other_resampled.samples[i] * other_volume;
        }
        // Clamp to [-1.0, 1.0]
        for s in &mut mixed {
            *s = s.clamp(-1.0, 1.0);
        }
        Self::new(mixed, self.sample_rate, self.channels)
    }

    /// Create silent audio of given duration
    pub fn silence(duration_sec: f32, sample_rate: u32, channels: u32) -> Self {
        let num_samples = (duration_sec * sample_rate as f32 * channels as f32) as usize;
        Self::new(vec![0.0f32; num_samples], sample_rate, channels)
    }

    /// Resample audio to target sample rate and channels (simple linear interpolation)
    fn resample_to(&self, target_rate: u32, target_channels: u32) -> Self {
        if self.sample_rate == target_rate && self.channels == target_channels {
            return self.clone();
        }
        if self.samples.is_empty() {
            return Self::new(vec![], target_rate, target_channels);
        }

        let src_duration = self.duration_sec();
        let target_samples = (src_duration * target_rate as f32 * target_channels as f32) as usize;
        let mut resampled = vec![0.0f32; target_samples];

        let src_frames = self.samples.len() / self.channels as usize;
        let tgt_frames = target_samples / target_channels as usize;

        for tgt_frame in 0..tgt_frames {
            let src_frame_pos = tgt_frame as f64 * src_frames as f64 / tgt_frames as f64;
            let src_frame_idx = src_frame_pos.floor() as usize;
            let frac = src_frame_pos - src_frame_idx as f64;

            for tgt_ch in 0..target_channels as usize {
                // Simple channel mapping: for upmixing, duplicate; for downmixing, average
                let src_ch = (tgt_ch * self.channels as usize) / target_channels as usize;
                let src_ch = src_ch.min(self.channels as usize - 1);

                let idx0 = (src_frame_idx * self.channels as usize + src_ch).min(self.samples.len() - 1);
                let idx1 = ((src_frame_idx + 1) * self.channels as usize + src_ch).min(self.samples.len() - 1);

                let s0 = self.samples[idx0];
                let s1 = self.samples[idx1];
                let s = s0 + (s1 - s0) * frac as f32;

                let tgt_idx = tgt_frame * target_channels as usize + tgt_ch;
                if tgt_idx < resampled.len() {
                    resampled[tgt_idx] = s;
                }
            }
        }

        Self::new(resampled, target_rate, target_channels)
    }

    /// Concatenate another audio after this one (resamples if needed)
    pub fn concat(&self, other: &SdAudio) -> Self {
        if self.samples.is_empty() {
            return other.clone();
        }
        if other.samples.is_empty() {
            return self.clone();
        }
        let other_resampled = if other.sample_rate != self.sample_rate || other.channels != self.channels {
            other.resample_to(self.sample_rate, self.channels)
        } else {
            other.clone()
        };
        let mut samples = self.samples.clone();
        samples.extend_from_slice(&other_resampled.samples);
        Self::new(samples, self.sample_rate, self.channels)
    }
}

#[derive(Debug, Clone)]
pub struct SdVideo {
    pub frames: Vec<SdImage>,
    pub fps: i32,
    pub audio: Option<SdAudio>,
}

impl Serialize for SdVideo {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let field_count = if self.audio.is_some() { 4 } else { 3 };
        let mut state = serializer.serialize_struct("SdVideo", field_count)?;
        state.serialize_field("frames", &self.frames)?;
        state.serialize_field("fps", &self.fps)?;
        state.serialize_field("frame_count", &self.frame_count())?;
        if let Some(ref audio) = self.audio {
            state.serialize_field("audio", audio)?;
        }
        state.end()
    }
}

impl<'de> Deserialize<'de> for SdVideo {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct SdVideoHelper {
            frames: Vec<SdImage>,
            fps: i32,
            audio: Option<SdAudio>,
        }
        let helper = SdVideoHelper::deserialize(deserializer)?;
        Ok(SdVideo::new(helper.frames, helper.fps, helper.audio))
    }
}

impl Serialize for SdAudio {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("SdAudio", 3)?;
        state.serialize_field("sample_rate", &self.sample_rate)?;
        state.serialize_field("channels", &self.channels)?;
        state.serialize_field("num_samples", &self.samples.len())?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for SdAudio {
    fn deserialize<D: Deserializer<'de>>(_deserializer: D) -> Result<Self, D::Error> {
        // Audio is not deserialized from frames; use from_wav_bytes instead
        Err(serde::de::Error::custom("SdAudio deserialization not supported directly, use from_wav_bytes"))
    }
}

impl SdVideo {
    pub fn new(frames: Vec<SdImage>, fps: i32, audio: Option<SdAudio>) -> Self {
        Self { frames, fps, audio }
    }

    pub fn new_without_audio(frames: Vec<SdImage>, fps: i32) -> Self {
        Self { frames, fps, audio: None }
    }

    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    pub fn to_gif_bytes(&self) -> Result<Vec<u8>, ImageError> {
        let mut buf = Vec::new();
        {
            let mut encoder = gif::Encoder::new(&mut buf, self.frames[0].width as u16, self.frames[0].height as u16, &[]).map_err(|e| ImageError::PngEncodeError(e.to_string()))?;
            encoder.set_repeat(gif::Repeat::Infinite).map_err(|e| ImageError::PngEncodeError(e.to_string()))?;
            for frame in &self.frames {
                let rgb_data = match frame.channel {
                    1 => {
                        frame.data.iter().flat_map(|&g| [g, g, g]).collect()
                    }
                    3 => frame.data.clone(),
                    4 => frame.data.chunks(4).flat_map(|px| [px[0], px[1], px[2]]).collect(),
                    _ => frame.data.clone(),
                };
                let gif_frame = gif::Frame::from_rgb(frame.width as u16, frame.height as u16, &rgb_data);
                encoder.write_frame(&gif_frame).map_err(|e| ImageError::PngEncodeError(e.to_string()))?;
            }
        }
        Ok(buf)
    }

    pub fn encode_with_ffmpeg(&self, output_path: &std::path::Path, fps: i32, crf: i32) -> Result<(), ImageError> {
        if self.frames.is_empty() {
            return Err(ImageError::PngEncodeError("No frames to encode".to_string()));
        }

        // Try full-featured ffmpeg with rawvideo pipe first
        if let Some(ffmpeg_path) = Self::find_full_ffmpeg() {
            return self.encode_with_ffmpeg_rawvideo(&ffmpeg_path, output_path, fps, crf);
        }

        // Fallback: use PNG-based encoding (works with minimal ffmpeg that has png decoder + image2 demuxer)
        self.encode_with_ffmpeg_png(output_path, fps, crf)
    }

    /// Encode video by piping raw frames to ffmpeg (requires rawvideo decoder)
    fn encode_with_ffmpeg_rawvideo(&self, ffmpeg_path: &str, output_path: &std::path::Path, fps: i32, _crf: i32) -> Result<(), ImageError> {
        let first = &self.frames[0];
        let w = first.width;
        let h = first.height;
        let ch = first.channel;
        let pix_fmt = if ch == 4 { "rgba" } else { "rgb24" };

        // If audio is present, write to a temp WAV file for ffmpeg input
        let audio_tmp = if let Some(ref audio) = self.audio {
            let tmp_path = std::env::temp_dir().join(format!("comfyui_audio_{}.wav", std::process::id()));
            std::fs::write(&tmp_path, audio.to_wav_bytes())
                .map_err(|e| ImageError::PngEncodeError(format!("Failed to write temp audio: {}", e)))?;
            Some(tmp_path)
        } else {
            None
        };

        let has_audio = audio_tmp.is_some();

        tracing::info!("encode_with_ffmpeg: using {} to pipe {} frames ({}x{}, {}){} via stdin",
            ffmpeg_path, self.frames.len(), w, h, pix_fmt,
            if has_audio { " with audio" } else { "" });

        // Build ffmpeg command args
        let size_str = format!("{}x{}", w, h);
        let fps_str = fps.to_string();
        let output_str = output_path.to_str().unwrap_or("").to_string();

        let mut args = vec![
            "-y",
            "-f", "rawvideo",
            "-pix_fmt", pix_fmt,
            "-s", &size_str,
            "-framerate", &fps_str,
            "-i", "pipe:0",
        ];

        let audio_path_str_owned;
        if let Some(ref audio_path) = audio_tmp {
            audio_path_str_owned = audio_path.to_str().unwrap_or("").to_string();
            args.extend_from_slice(&[
                "-i", &audio_path_str_owned,
                "-c:v", "libx264",
                "-pix_fmt", "yuv420p",
                "-preset", "medium",
                "-c:a", "aac",
                "-b:a", "192k",
                "-shortest",
                "-movflags", "+faststart",
            ]);
        } else {
            args.extend_from_slice(&[
                "-c:v", "libx264",
                "-pix_fmt", "yuv420p",
                "-preset", "medium",
                "-movflags", "+faststart",
            ]);
        }

        args.push(&output_str);

        let mut child = std::process::Command::new(ffmpeg_path)
            .args(&args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| ImageError::PngEncodeError(format!("Failed to spawn ffmpeg: {}", e)))?;

        // Take stderr handle to read in a separate thread
        let stderr_handle = child.stderr.take();

        // Spawn thread to collect stderr output
        let stderr_thread = stderr_handle.map(|stderr| {
            std::thread::spawn(move || {
                use std::io::Read;
                let mut output = String::new();
                let mut reader = stderr;
                let _ = reader.read_to_string(&mut output);
                output
            })
        });

        // Write raw frame data to ffmpeg stdin
        if let Some(ref mut stdin) = child.stdin {
            use std::io::Write;
            for (i, frame) in self.frames.iter().enumerate() {
                let expected = (w * h * ch) as usize;
                if frame.data.len() != expected {
                    return Err(ImageError::PngEncodeError(format!(
                        "Frame {} has {} bytes, expected {}", i, frame.data.len(), expected
                    )));
                }
                if let Err(e) = stdin.write_all(&frame.data) {
                    // Wait for stderr to get error details
                    let stderr_output = stderr_thread
                        .and_then(|t| t.join().ok())
                        .unwrap_or_default();
                    return Err(ImageError::PngEncodeError(format!(
                        "Failed to write frame {} to ffmpeg: {}. ffmpeg stderr: {}",
                        i, e, stderr_output
                    )));
                }
            }
        }
        // Drop stdin to signal EOF
        drop(child.stdin.take());

        let output = child.wait_with_output()
            .map_err(|e| ImageError::PngEncodeError(format!("Failed to wait for ffmpeg: {}", e)))?;

        // Clean up temp audio file
        if let Some(ref tmp) = audio_tmp {
            let _ = std::fs::remove_file(tmp);
        }

        if !output.status.success() {
            let stderr = stderr_thread
                .and_then(|t| t.join().ok())
                .unwrap_or_else(|| String::from_utf8_lossy(&output.stderr).to_string());
            return Err(ImageError::PngEncodeError(format!(
                "ffmpeg exited with code {:?}: {}", output.status.code(), stderr
            )));
        }

        Ok(())
    }

    /// Encode video using PNG frames piped to ffmpeg (works with minimal ffmpeg builds)
    fn encode_with_ffmpeg_png(&self, output_path: &std::path::Path, fps: i32, _crf: i32) -> Result<(), ImageError> {
        let ffmpeg_path = Self::find_any_ffmpeg()
            .ok_or_else(|| ImageError::PngEncodeError("No ffmpeg found".to_string()))?;

        tracing::info!("encode_with_ffmpeg_png: using {} (PNG pipe mode) for {} frames{}",
            ffmpeg_path, self.frames.len(),
            if self.audio.is_some() { " with audio" } else { "" });

        // Write PNG frames to temp directory
        let tmp_dir = std::env::temp_dir().join(format!("comfyui_ffmpeg_mp4_{}", std::process::id()));
        std::fs::create_dir_all(&tmp_dir)
            .map_err(|e| ImageError::PngEncodeError(format!("Failed to create temp dir: {}", e)))?;

        for (i, frame) in self.frames.iter().enumerate() {
            let png_path = tmp_dir.join(format!("frame_{:06}.png", i));
            let png_bytes = frame.to_png_bytes()
                .map_err(|e| ImageError::PngEncodeError(format!("Failed to encode frame {}: {}", i, e)))?;
            std::fs::write(&png_path, png_bytes)
                .map_err(|e| ImageError::PngEncodeError(format!("Failed to write frame {}: {}", i, e)))?;
        }

        // If audio is present, write to a temp WAV file
        let audio_tmp = if let Some(ref audio) = self.audio {
            let tmp_path = tmp_dir.join("audio.wav");
            std::fs::write(&tmp_path, audio.to_wav_bytes())
                .map_err(|e| ImageError::PngEncodeError(format!("Failed to write temp audio: {}", e)))?;
            Some(tmp_path)
        } else {
            None
        };

        let input_pattern = tmp_dir.join("frame_%06d.png");
        let input_pattern_str = input_pattern.to_str().unwrap_or("").to_string();
        let fps_str = fps.to_string();
        let output_str = output_path.to_str().unwrap_or("").to_string();

        let status = if let Some(ref audio_path) = audio_tmp {
            let audio_path_str = audio_path.to_str().unwrap_or("").to_string();
            std::process::Command::new(&ffmpeg_path)
                .args([
                    "-y",
                    "-framerate", &fps_str,
                    "-i", &input_pattern_str,
                    "-i", &audio_path_str,
                    "-c:v", "libx264",
                    "-pix_fmt", "yuv420p",
                    "-preset", "medium",
                    "-c:a", "aac",
                    "-b:a", "192k",
                    "-shortest",
                    "-movflags", "+faststart",
                    &output_str,
                ])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map_err(|e| ImageError::PngEncodeError(format!("Failed to run ffmpeg: {}", e)))?
        } else {
            std::process::Command::new(&ffmpeg_path)
                .args([
                    "-y",
                    "-framerate", &fps_str,
                    "-i", &input_pattern_str,
                    "-c:v", "libx264",
                    "-pix_fmt", "yuv420p",
                    "-preset", "medium",
                    "-movflags", "+faststart",
                    &output_str,
                ])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map_err(|e| ImageError::PngEncodeError(format!("Failed to run ffmpeg: {}", e)))?
        };

        let _ = std::fs::remove_dir_all(&tmp_dir);

        if !status.success() {
            return Err(ImageError::PngEncodeError(format!(
                "ffmpeg exited with code {:?}", status.code()
            )));
        }

        Ok(())
    }

    /// Find any available ffmpeg binary
    fn find_any_ffmpeg() -> Option<String> {
        let candidates = [
            "/usr/local/bin/ffmpeg",
            "/usr/bin/ffmpeg",
            "/bin/ffmpeg",
        ];

        for path in &candidates {
            if std::path::Path::new(path).exists() {
                return Some(path.to_string());
            }
        }

        // Check PATH
        if std::process::Command::new("ffmpeg")
            .arg("-version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok()
        {
            return Some("ffmpeg".to_string());
        }

        None
    }

    /// Find a full-featured ffmpeg that supports rawvideo demuxer and pipe protocol
    fn find_full_ffmpeg() -> Option<String> {
        // Check common locations for full ffmpeg
        let candidates = [
            "/usr/bin/ffmpeg",
            "/usr/local/bin/ffmpeg",
            "/bin/ffmpeg",
        ];

        for path in &candidates {
            if std::path::Path::new(path).exists() {
                // Check if it supports rawvideo demuxer
                // ffmpeg -demuxers outputs to stdout
                let output = std::process::Command::new(path)
                    .args(["-demuxers"])
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::null())
                    .output()
                    .ok()?;

                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.contains("rawvideo") {
                    tracing::info!("Found full-featured ffmpeg at {}", path);
                    return Some(path.to_string());
                }
            }
        }

        // Fallback to PATH
        let output = std::process::Command::new("ffmpeg")
            .args(["-demuxers"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .output()
            .ok()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("rawvideo") {
            tracing::info!("Using ffmpeg from PATH");
            return Some("ffmpeg".to_string());
        }

        None
    }

    pub fn encode_webm_with_ffmpeg(&self, output_path: &std::path::Path, fps: i32, crf: i32) -> Result<(), ImageError> {
        if self.frames.is_empty() {
            return Err(ImageError::PngEncodeError("No frames to encode".to_string()));
        }

        let tmp_dir = std::env::temp_dir().join(format!("comfyui_ffmpeg_webm_{}", std::process::id()));
        std::fs::create_dir_all(&tmp_dir)
            .map_err(|e| ImageError::PngEncodeError(format!("Failed to create temp dir: {}", e)))?;

        for (i, frame) in self.frames.iter().enumerate() {
            let png_path = tmp_dir.join(format!("frame_{:06}.png", i));
            let png_bytes = frame.to_png_bytes()
                .map_err(|e| ImageError::PngEncodeError(format!("Failed to encode frame {}: {}", i, e)))?;
            std::fs::write(&png_path, png_bytes)
                .map_err(|e| ImageError::PngEncodeError(format!("Failed to write frame {}: {}", i, e)))?;
        }

        let input_pattern = tmp_dir.join("frame_%06d.png");
        let status = std::process::Command::new("ffmpeg")
            .args([
                "-y",
                "-framerate", &fps.to_string(),
                "-i", input_pattern.to_str().unwrap_or(""),
                "-c:v", "libvpx-vp9",
                "-pix_fmt", "yuv420p",
                "-crf", &crf.to_string(),
                "-b:v", "0",
                output_path.to_str().unwrap_or(""),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|e| ImageError::PngEncodeError(format!("Failed to run ffmpeg: {}", e)))?;

        let _ = std::fs::remove_dir_all(&tmp_dir);

        if !status.success() {
            return Err(ImageError::PngEncodeError(format!(
                "ffmpeg exited with code {:?}", status.code()
            )));
        }

        Ok(())
    }

    pub fn decode_with_ffmpeg(video_path: &std::path::Path, fps: i32) -> Result<Self, ImageError> {
        let tmp_dir = std::env::temp_dir().join(format!("comfyui_ffmpeg_decode_{}", std::process::id()));
        std::fs::create_dir_all(&tmp_dir)
            .map_err(|e| ImageError::PngEncodeError(format!("Failed to create temp dir: {}", e)))?;

        let output_pattern = tmp_dir.join("frame_%06d.png");
        let status = std::process::Command::new("ffmpeg")
            .args([
                "-i", video_path.to_str().unwrap_or(""),
                "-vf", &format!("fps={}", fps),
                output_pattern.to_str().unwrap_or(""),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|e| ImageError::PngEncodeError(format!("Failed to run ffmpeg: {}", e)))?;

        if !status.success() {
            let _ = std::fs::remove_dir_all(&tmp_dir);
            return Err(ImageError::PngEncodeError(format!(
                "ffmpeg decode exited with code {:?}", status.code()
            )));
        }

        let mut frames = Vec::new();
        let mut idx = 1;
        loop {
            let frame_path = tmp_dir.join(format!("frame_{:06}.png", idx));
            if !frame_path.exists() {
                break;
            }
            match std::fs::read(&frame_path) {
                Ok(data) => {
                    match SdImage::from_png_bytes(&data) {
                        Ok(img) => frames.push(img),
                        Err(_) => {}
                    }
                }
                Err(_) => break,
            }
            idx += 1;
        }

        let _ = std::fs::remove_dir_all(&tmp_dir);

        Ok(SdVideo::new_without_audio(frames, fps))
    }

    pub fn is_ffmpeg_available() -> bool {
        Self::find_any_ffmpeg().is_some()
    }

    /// Get video duration in seconds
    pub fn duration_sec(&self) -> f32 {
        if self.fps <= 0 || self.frames.is_empty() {
            return 0.0;
        }
        self.frames.len() as f32 / self.fps as f32
    }

    /// Trim video to start_sec..end_sec (in seconds)
    pub fn trim(&self, start_sec: f32, end_sec: f32) -> Self {
        if self.frames.is_empty() || self.fps <= 0 {
            return self.clone();
        }
        let total_dur = self.duration_sec();
        let start = start_sec.max(0.0);
        let end = end_sec.min(total_dur);
        if start >= end {
            return Self::new(vec![], self.fps, None);
        }
        let start_frame = (start * self.fps as f32) as usize;
        let end_frame = (end * self.fps as f32) as usize;
        let start_frame = start_frame.min(self.frames.len());
        let end_frame = end_frame.min(self.frames.len());

        let trimmed_frames: Vec<SdImage> = self.frames[start_frame..end_frame].to_vec();
        let trimmed_audio = self.audio.as_ref().map(|a| a.trim(start, end));

        Self::new(trimmed_frames, self.fps, trimmed_audio)
    }

    /// Concatenate another video after this one. If fps differ, other is resampled.
    pub fn concat(&self, other: &SdVideo) -> Self {
        if self.frames.is_empty() {
            return other.clone();
        }
        if other.frames.is_empty() {
            return self.clone();
        }

        // Resample other video if fps or resolution differ by resizing frames
        let other_resampled = if other.fps != self.fps ||
            (!other.frames.is_empty() && !self.frames.is_empty() &&
             (other.frames[0].width != self.frames[0].width || other.frames[0].height != self.frames[0].height)) {
            other.resample_to(self.fps, self.frames[0].width, self.frames[0].height)
        } else {
            other.clone()
        };

        let mut frames = self.frames.clone();
        frames.extend_from_slice(&other_resampled.frames);

        let audio = match (&self.audio, &other_resampled.audio) {
            (Some(a), Some(b)) => Some(a.concat(b)),
            (Some(a), None) => {
                let silence_dur = other_resampled.duration_sec();
                let silence = SdAudio::silence(silence_dur, a.sample_rate, a.channels);
                Some(a.concat(&silence))
            }
            (None, Some(b)) => {
                let silence_dur = self.duration_sec();
                let silence = SdAudio::silence(silence_dur, b.sample_rate, b.channels);
                Some(silence.concat(b))
            }
            (None, None) => None,
        };

        Self::new(frames, self.fps, audio)
    }

    /// Adjust video volume (multiplier: 0.0 = mute, 1.0 = original)
    pub fn adjust_volume(&self, multiplier: f32) -> Self {
        let audio = self.audio.as_ref().map(|a| a.adjust_volume(multiplier));
        Self::new(self.frames.clone(), self.fps, audio)
    }

    /// Apply audio fade-in
    pub fn audio_fade_in(&self, duration_sec: f32) -> Self {
        let audio = self.audio.as_ref().map(|a| a.fade_in(duration_sec));
        Self::new(self.frames.clone(), self.fps, audio)
    }

    /// Apply audio fade-out
    pub fn audio_fade_out(&self, duration_sec: f32) -> Self {
        let audio = self.audio.as_ref().map(|a| a.fade_out(duration_sec));
        Self::new(self.frames.clone(), self.fps, audio)
    }

    /// Replace audio track with new audio
    pub fn replace_audio(&self, new_audio: Option<SdAudio>) -> Self {
        Self::new(self.frames.clone(), self.fps, new_audio)
    }

    /// Mix additional audio into the video
    pub fn mix_audio(&self, other: &SdAudio, volume: f32) -> Self {
        let audio = match &self.audio {
            Some(a) => Some(a.mix(other, volume)),
            None => Some(other.adjust_volume(volume)),
        };
        Self::new(self.frames.clone(), self.fps, audio)
    }

    /// Resample video to target fps, width, height using simple nearest-neighbor
    fn resample_to(&self, target_fps: i32, target_width: u32, target_height: u32) -> Self {
        if self.frames.is_empty() {
            return self.clone();
        }

        let src_duration = self.duration_sec();
        let target_frame_count = (src_duration * target_fps as f32) as usize;
        let mut resampled_frames = Vec::with_capacity(target_frame_count);

        for tgt_idx in 0..target_frame_count {
            let src_pos = tgt_idx as f64 * self.frames.len() as f64 / target_frame_count as f64;
            let src_idx = src_pos.floor() as usize;
            let src_idx = src_idx.min(self.frames.len() - 1);

            let src_frame = &self.frames[src_idx];
            let resized = if src_frame.width != target_width || src_frame.height != target_height {
                // Simple nearest-neighbor resize
                Self::resize_frame(src_frame, target_width, target_height)
            } else {
                src_frame.clone()
            };
            resampled_frames.push(resized);
        }

        let audio = self.audio.as_ref().map(|a| {
            // Keep audio duration the same by trimming/padding to match new video
            let new_duration = resampled_frames.len() as f32 / target_fps as f32;
            let a_dur = a.duration_sec();
            if a_dur >= new_duration {
                a.trim(0.0, new_duration)
            } else {
                let silence = SdAudio::silence(new_duration - a_dur, a.sample_rate, a.channels);
                a.concat(&silence)
            }
        });

        Self::new(resampled_frames, target_fps, audio)
    }

    /// Simple nearest-neighbor resize for a frame
    fn resize_frame(frame: &SdImage, w: u32, h: u32) -> SdImage {
        if frame.width == w && frame.height == h {
            return frame.clone();
        }
        let mut data = vec![0u8; (w * h * frame.channel) as usize];
        let x_ratio = frame.width as f64 / w as f64;
        let y_ratio = frame.height as f64 / h as f64;

        for y in 0..h {
            for x in 0..w {
                let src_x = (x as f64 * x_ratio) as u32;
                let src_y = (y as f64 * y_ratio) as u32;
                let src_x = src_x.min(frame.width - 1);
                let src_y = src_y.min(frame.height - 1);

                for c in 0..frame.channel {
                    let src_idx = ((src_y * frame.width + src_x) * frame.channel + c) as usize;
                    let tgt_idx = ((y * w + x) * frame.channel + c) as usize;
                    if src_idx < frame.data.len() && tgt_idx < data.len() {
                        data[tgt_idx] = frame.data[src_idx];
                    }
                }
            }
        }
        SdImage::from_raw(w, h, frame.channel, data).unwrap_or_else(|_| frame.clone())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    #[error("Size mismatch: expected {expected} bytes, got {actual}")]
    SizeMismatch { expected: usize, actual: usize },
    #[error("PNG encode error: {0}")]
    PngEncodeError(String),
    #[error("PNG decode error: {0}")]
    PngDecodeError(String),
    #[error("Base64 error: {0}")]
    Base64Error(String),
}

impl fmt::Display for SdImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SdImage({}x{}x{})",
            self.width, self.height, self.channel
        )
    }
}

fn encode_png(image: &SdImage) -> Result<Vec<u8>, ImageError> {
    let mut buf = Vec::new();
    {
        let mut png_encoder = png::Encoder::new(&mut buf, image.width, image.height);
        let color_type = match image.channel {
            1 => png::ColorType::Grayscale,
            3 => png::ColorType::Rgb,
            4 => png::ColorType::Rgba,
            _ => png::ColorType::Rgb,
        };
        png_encoder.set_color(color_type);
        png_encoder.set_depth(png::BitDepth::Eight);

        let mut writer = png_encoder
            .write_header()
            .map_err(|e| ImageError::PngEncodeError(e.to_string()))?;
        writer
            .write_image_data(&image.data)
            .map_err(|e| ImageError::PngEncodeError(e.to_string()))?;
        writer.finish().map_err(|e| ImageError::PngEncodeError(e.to_string()))?;
    }
    Ok(buf)
}

fn decode_png(bytes: &[u8]) -> Result<SdImage, ImageError> {
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder
        .read_info()
        .map_err(|e| ImageError::PngDecodeError(e.to_string()))?;

    let info = reader.info().clone();
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let output_info = reader
        .next_frame(&mut buf)
        .map_err(|e| ImageError::PngDecodeError(e.to_string()))?;

    buf.truncate(output_info.buffer_size());

    let channel = match info.color_type {
        png::ColorType::Grayscale => 1,
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        png::ColorType::GrayscaleAlpha => 2,
        _ => 3,
    };

    SdImage::from_raw(info.width, info.height, channel, buf)
}

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    let chunks = data.chunks(3);
    for chunk in chunks {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;

        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

fn base64_decode(input: &str) -> Result<Vec<u8>, ImageError> {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let input = input.trim();
    let mut result = Vec::with_capacity(input.len() * 3 / 4);

    let chars: Vec<u8> = input
        .bytes()
        .filter(|&b| b != b'\n' && b != b'\r')
        .collect();

    let chunks = chars.chunks(4);
    for chunk in chunks {
        if chunk.len() < 2 {
            break;
        }
        let mut acc: u32 = 0;
        let mut bits = 0u32;
        for &b in chunk.iter().take_while(|&&b| b != b'=') {
            let val = TABLE
                .iter()
                .position(|&c| c == b)
                .ok_or_else(|| ImageError::Base64Error("Invalid base64 character".to_string()))?;
            acc = (acc << 6) | val as u32;
            bits += 6;
        }
        while bits >= 8 {
            bits -= 8;
            result.push((acc >> bits) as u8);
        }
    }
    Ok(result)
}
