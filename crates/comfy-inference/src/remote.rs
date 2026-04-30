use crate::backend::InferenceBackend;
use crate::error::{InferenceError, InferenceResult};
use crate::image::{SdImage, SdVideo};
use crate::params::*;

#[derive(Debug, Clone)]
pub struct RemoteConfig {
    pub base_url: String,
    pub api_prefix: String,
    pub timeout_secs: u64,
}

impl Default for RemoteConfig {
    fn default() -> Self {
        Self {
            base_url: "http://127.0.0.1:1234".to_string(),
            api_prefix: "sdcpp/v1".to_string(),
            timeout_secs: 300,
        }
    }
}

impl RemoteConfig {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            ..Default::default()
        }
    }

    pub fn with_api_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.api_prefix = prefix.into();
        self
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{}/{}", self.base_url, self.api_prefix, path)
    }
}

pub struct RemoteBackend {
    config: RemoteConfig,
    client: reqwest::Client,
    capabilities: std::sync::Mutex<Option<CapabilitiesCache>>,
}

struct CapabilitiesCache {
    supports_img: bool,
    supports_vid: bool,
}

impl RemoteBackend {
    pub fn new(config: RemoteConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .unwrap_or_default();

        Self {
            config,
            client,
            capabilities: std::sync::Mutex::new(None),
        }
    }

    pub fn config(&self) -> &RemoteConfig {
        &self.config
    }

    async fn fetch_capabilities(&self) -> InferenceResult<(bool, bool)> {
        let cache = self.capabilities.lock().unwrap();
        if let Some(caps) = &*cache {
            return Ok((caps.supports_img, caps.supports_vid));
        }
        drop(cache);

        let url = self.config.url("capabilities");
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| InferenceError::NetworkError(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(InferenceError::RemoteError {
                status: resp.status().as_u16(),
                message: "Failed to fetch capabilities".to_string(),
            });
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| InferenceError::NetworkError(e.to_string()))?;

        let modes = body["supported_modes"].as_array();
        let supports_img = modes
            .map(|m| m.iter().any(|v| v.as_str() == Some("img_gen")))
            .unwrap_or(false);
        let supports_vid = modes
            .map(|m| m.iter().any(|v| v.as_str() == Some("vid_gen")))
            .unwrap_or(false);

        let mut cache = self.capabilities.lock().unwrap();
        *cache = Some(CapabilitiesCache {
            supports_img,
            supports_vid,
        });

        Ok((supports_img, supports_vid))
    }

    pub async fn generate_image_async(
        &self,
        params: ImageGenParams,
    ) -> InferenceResult<Vec<SdImage>> {
        let url = self.config.url("img_gen");
        let body = serde_json::to_value(&params)
            .map_err(|e| InferenceError::InvalidParameter(e.to_string()))?;

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| InferenceError::NetworkError(e.to_string()))?;

        if resp.status() == reqwest::StatusCode::ACCEPTED {
            return self.poll_job_result(resp).await;
        }

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let msg = resp.text().await.unwrap_or_default();
            return Err(InferenceError::RemoteError { status, message: msg });
        }

        let result: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| InferenceError::NetworkError(e.to_string()))?;

        parse_image_response(&result)
    }

    pub async fn generate_video_async(
        &self,
        params: VideoGenParams,
    ) -> InferenceResult<SdVideo> {
        let url = self.config.url("vid_gen");
        let body = serde_json::to_value(&params)
            .map_err(|e| InferenceError::InvalidParameter(e.to_string()))?;

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| InferenceError::NetworkError(e.to_string()))?;

        if resp.status() == reqwest::StatusCode::ACCEPTED {
            return self.poll_video_job_result(resp).await;
        }

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let msg = resp.text().await.unwrap_or_default();
            return Err(InferenceError::RemoteError { status, message: msg });
        }

        Err(InferenceError::UnsupportedOperation(
            "Direct video response not yet supported".to_string(),
        ))
    }

    async fn poll_job_result(&self, resp: reqwest::Response) -> InferenceResult<Vec<SdImage>> {
        let job: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| InferenceError::NetworkError(e.to_string()))?;

        let poll_url = job["poll_url"]
            .as_str()
            .ok_or_else(|| InferenceError::RemoteError {
                status: 202,
                message: "No poll_url in job response".to_string(),
            })?;

        let full_url = format!("{}{}", self.config.base_url, poll_url);

        loop {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            let status_resp = self
                .client
                .get(&full_url)
                .send()
                .await
                .map_err(|e| InferenceError::NetworkError(e.to_string()))?;

            let status_body: serde_json::Value = status_resp
                .json()
                .await
                .map_err(|e| InferenceError::NetworkError(e.to_string()))?;

            match status_body["status"].as_str() {
                Some("completed") => {
                    return parse_image_response(&status_body);
                }
                Some("failed") => {
                    return Err(InferenceError::GenerationFailed(
                        status_body["error_message"]
                            .as_str()
                            .unwrap_or("Unknown error")
                            .to_string(),
                    ));
                }
                Some("cancelled") => {
                    return Err(InferenceError::GenerationFailed(
                        "Job was cancelled".to_string(),
                    ));
                }
                _ => continue,
            }
        }
    }

    async fn poll_video_job_result(&self, resp: reqwest::Response) -> InferenceResult<SdVideo> {
        let job: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| InferenceError::NetworkError(e.to_string()))?;

        let poll_url = job["poll_url"]
            .as_str()
            .ok_or_else(|| InferenceError::RemoteError {
                status: 202,
                message: "No poll_url in job response".to_string(),
            })?;

        let full_url = format!("{}{}", self.config.base_url, poll_url);

        loop {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            let status_resp = self
                .client
                .get(&full_url)
                .send()
                .await
                .map_err(|e| InferenceError::NetworkError(e.to_string()))?;

            let status_body: serde_json::Value = status_resp
                .json()
                .await
                .map_err(|e| InferenceError::NetworkError(e.to_string()))?;

            match status_body["status"].as_str() {
                Some("completed") => {
                    let frames = parse_image_response(&status_body)?;
                    let fps = status_body["result_fps"].as_i64().unwrap_or(16) as i32;
                    return Ok(SdVideo::new(frames, fps));
                }
                Some("failed") => {
                    return Err(InferenceError::GenerationFailed(
                        status_body["error_message"]
                            .as_str()
                            .unwrap_or("Unknown error")
                            .to_string(),
                    ));
                }
                Some("cancelled") => {
                    return Err(InferenceError::GenerationFailed(
                        "Job was cancelled".to_string(),
                    ));
                }
                _ => continue,
            }
        }
    }
}

fn parse_image_response(body: &serde_json::Value) -> InferenceResult<Vec<SdImage>> {
    let images_array = body["images"]
        .as_array()
        .ok_or_else(|| InferenceError::ImageDecodeError("No images in response".to_string()))?;

    let mut images = Vec::new();
    for img_val in images_array {
        if let Some(b64) = img_val.as_str() {
            let bytes = base64_decode(b64)
                .map_err(|e| InferenceError::ImageDecodeError(e.to_string()))?;
            let img = SdImage::from_png_bytes(&bytes)
                .map_err(|e| InferenceError::ImageDecodeError(e.to_string()))?;
            images.push(img);
        }
    }

    Ok(images)
}

fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let mut decoder = base64_decode_engine(input);
    let mut decoded = Vec::new();
    decoder
        .read_to_end(&mut decoded)
        .map_err(|e| e.to_string())?;
    Ok(decoded)
}

fn base64_decode_engine(input: &str) -> impl std::io::Read + '_ {
    let trimmed = input.trim();
    base64::read::DecoderReader::new(trimmed.as_bytes(), &base64::engine::general_purpose::STANDARD)
}

impl InferenceBackend for RemoteBackend {
    fn supports_image_generation(&self) -> bool {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async { self.fetch_capabilities().await.map(|(img, _)| img).unwrap_or(false) })
    }

    fn supports_video_generation(&self) -> bool {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async { self.fetch_capabilities().await.map(|(_, vid)| vid).unwrap_or(false) })
    }

    fn generate_image(&self, params: ImageGenParams) -> InferenceResult<Vec<SdImage>> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(self.generate_image_async(params))
    }

    fn generate_video(&self, params: VideoGenParams) -> InferenceResult<SdVideo> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(self.generate_video_async(params))
    }

    fn upscale(&self, _image: SdImage, _params: UpscaleParams) -> InferenceResult<SdImage> {
        Err(InferenceError::UnsupportedOperation(
            "Upscale not supported via remote backend".to_string(),
        ))
    }
}
