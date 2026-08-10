use crate::error::{InferenceError, InferenceResult};
use crate::image::{SdImage, SdVideo};
use crate::params::*;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

pub trait InferenceBackend: Send + Sync {
    fn supports_image_generation(&self) -> bool;
    fn supports_video_generation(&self) -> bool;
    fn supports_3d_generation(&self) -> bool {
        false
    }
    fn supports_audio_video_generation(&self) -> bool {
        false
    }
    fn supports_context_ir(&self) -> bool {
        false
    }

    fn generate_image(&self, params: ImageGenParams) -> InferenceResult<Vec<SdImage>>;

    fn generate_video(&self, params: VideoGenParams) -> InferenceResult<SdVideo>;

    fn upscale(&self, image: SdImage, params: UpscaleParams) -> InferenceResult<SdImage>;

    fn generate_3d_gaussian(&self, _params: Gaussian3DParams) -> InferenceResult<Gaussian3DOutput> {
        Err(InferenceError::BackendNotAvailable("3D generation not implemented".to_string()))
    }

    /// H3 音视频联合生成 (T2VA/Ref2VA/I2VA/MR2VA/SFX/Audio)
    fn generate_av(&self, _params: H3Params) -> InferenceResult<SdVideo> {
        Err(InferenceError::BackendNotAvailable("Audio-Video generation not available on this backend".to_string()))
    }

    /// Context-IR 多模态上下文理解 (复用已加载的VLM/text_encoder)
    fn context_ir(&self, _params: ContextIrParams) -> InferenceResult<H3Context> {
        Err(InferenceError::BackendNotAvailable("Context-IR not available on this backend".to_string()))
    }

    fn decode_video_latent(&self, _latent: &Value, _params: &VideoGenParams) -> InferenceResult<SdVideo> {
        Err(InferenceError::BackendNotAvailable("decode_video_latent not implemented".to_string()))
    }

    fn get_capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            supports_image_generation: self.supports_image_generation(),
            supports_video_generation: self.supports_video_generation(),
            supports_3d_generation: self.supports_3d_generation(),
            supports_audio_video_generation: self.supports_audio_video_generation(),
            supports_context_ir: self.supports_context_ir(),
        }
    }
}

pub trait AsyncInferenceBackend: Send + Sync {
    fn generate_image_async(
        &self,
        params: ImageGenParams,
    ) -> Pin<Box<dyn Future<Output = InferenceResult<Vec<SdImage>>> + Send + '_>>;

    fn generate_video_async(
        &self,
        params: VideoGenParams,
    ) -> Pin<Box<dyn Future<Output = InferenceResult<SdVideo>> + Send + '_>>;

    fn generate_av_async(
        &self,
        params: H3Params,
    ) -> Pin<Box<dyn Future<Output = InferenceResult<SdVideo>> + Send + '_>>;

    fn context_ir_async(
        &self,
        params: ContextIrParams,
    ) -> Pin<Box<dyn Future<Output = InferenceResult<H3Context>> + Send + '_>>;
}

impl<B: InferenceBackend> AsyncInferenceBackend for B {
    fn generate_image_async(
        &self,
        params: ImageGenParams,
    ) -> Pin<Box<dyn Future<Output = InferenceResult<Vec<SdImage>>> + Send + '_>> {
        let result = self.generate_image(params);
        Box::pin(async move { result })
    }

    fn generate_video_async(
        &self,
        params: VideoGenParams,
    ) -> Pin<Box<dyn Future<Output = InferenceResult<SdVideo>> + Send + '_>> {
        let result = self.generate_video(params);
        Box::pin(async move { result })
    }

    fn generate_av_async(
        &self,
        params: H3Params,
    ) -> Pin<Box<dyn Future<Output = InferenceResult<SdVideo>> + Send + '_>> {
        let result = self.generate_av(params);
        Box::pin(async move { result })
    }

    fn context_ir_async(
        &self,
        params: ContextIrParams,
    ) -> Pin<Box<dyn Future<Output = InferenceResult<H3Context>> + Send + '_>> {
        let result = self.context_ir(params);
        Box::pin(async move { result })
    }
}

#[derive(Debug, Clone)]
pub struct BackendCapabilities {
    pub supports_image_generation: bool,
    pub supports_video_generation: bool,
    pub supports_3d_generation: bool,
    pub supports_audio_video_generation: bool,
    pub supports_context_ir: bool,
}

pub struct NullBackend;

impl InferenceBackend for NullBackend {
    fn supports_image_generation(&self) -> bool {
        false
    }

    fn supports_video_generation(&self) -> bool {
        false
    }

    fn supports_audio_video_generation(&self) -> bool {
        false
    }

    fn supports_context_ir(&self) -> bool {
        false
    }

    fn generate_image(&self, _params: ImageGenParams) -> InferenceResult<Vec<SdImage>> {
        Err(InferenceError::BackendNotAvailable("NullBackend".to_string()))
    }

    fn generate_video(&self, _params: VideoGenParams) -> InferenceResult<SdVideo> {
        Err(InferenceError::BackendNotAvailable("NullBackend".to_string()))
    }

    fn generate_av(&self, _params: H3Params) -> InferenceResult<SdVideo> {
        Err(InferenceError::BackendNotAvailable("NullBackend".to_string()))
    }

    fn context_ir(&self, _params: ContextIrParams) -> InferenceResult<H3Context> {
        Err(InferenceError::BackendNotAvailable("NullBackend".to_string()))
    }

    fn upscale(&self, _image: SdImage, _params: UpscaleParams) -> InferenceResult<SdImage> {
        Err(InferenceError::BackendNotAvailable("NullBackend".to_string()))
    }
}
