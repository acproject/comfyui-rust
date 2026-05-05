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
}

#[derive(Debug, Clone)]
pub struct SdVideo {
    pub frames: Vec<SdImage>,
    pub fps: i32,
}

impl Serialize for SdVideo {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("SdVideo", 3)?;
        state.serialize_field("frames", &self.frames)?;
        state.serialize_field("fps", &self.fps)?;
        state.serialize_field("frame_count", &self.frame_count())?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for SdVideo {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct SdVideoHelper {
            frames: Vec<SdImage>,
            fps: i32,
        }
        let helper = SdVideoHelper::deserialize(deserializer)?;
        Ok(SdVideo::new(helper.frames, helper.fps))
    }
}

impl SdVideo {
    pub fn new(frames: Vec<SdImage>, fps: i32) -> Self {
        Self { frames, fps }
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
