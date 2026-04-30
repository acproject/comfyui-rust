use crate::error::{InferenceError, InferenceResult};
use crate::image::{SdImage, SdVideo};
use crate::params::*;
use std::future::Future;
use std::pin::Pin;

pub trait InferenceBackend: Send + Sync {
    fn supports_image_generation(&self) -> bool;
    fn supports_video_generation(&self) -> bool;

    fn generate_image(&self, params: ImageGenParams) -> InferenceResult<Vec<SdImage>>;

    fn generate_video(&self, params: VideoGenParams) -> InferenceResult<SdVideo>;

    fn upscale(&self, image: SdImage, params: UpscaleParams) -> InferenceResult<SdImage>;

    fn get_capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            supports_image_generation: self.supports_image_generation(),
            supports_video_generation: self.supports_video_generation(),
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
}

#[derive(Debug, Clone)]
pub struct BackendCapabilities {
    pub supports_image_generation: bool,
    pub supports_video_generation: bool,
}

pub struct NullBackend;

impl InferenceBackend for NullBackend {
    fn supports_image_generation(&self) -> bool {
        false
    }

    fn supports_video_generation(&self) -> bool {
        false
    }

    fn generate_image(&self, _params: ImageGenParams) -> InferenceResult<Vec<SdImage>> {
        Err(InferenceError::BackendNotAvailable("NullBackend".to_string()))
    }

    fn generate_video(&self, _params: VideoGenParams) -> InferenceResult<SdVideo> {
        Err(InferenceError::BackendNotAvailable("NullBackend".to_string()))
    }

    fn upscale(&self, _image: SdImage, _params: UpscaleParams) -> InferenceResult<SdImage> {
        Err(InferenceError::BackendNotAvailable("NullBackend".to_string()))
    }
}
