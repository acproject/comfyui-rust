use comfy_core::{InputTypeSpec, IoType, NodeClassDef, NodeInputTypes};
use comfy_inference::SdImage;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::ExecutorError;
use crate::execution_context::ExecutionContext;
use crate::registry::NodeRegistry;

mod imp {
    use comfy_inference::SdImage;
    use std::path::Path;

    pub fn sd_image_to_dynamic(image: &SdImage) -> Result<image::DynamicImage, String> {
        let img = match image.channel {
            1 => {
                let buf = image::GrayImage::from_raw(
                    image.width,
                    image.height,
                    image.data.clone(),
                )
                .ok_or("Failed to create GrayImage from raw data")?;
                image::DynamicImage::ImageLuma8(buf)
            }
            3 => {
                let buf = image::RgbImage::from_raw(
                    image.width,
                    image.height,
                    image.data.clone(),
                )
                .ok_or("Failed to create RgbImage from raw data")?;
                image::DynamicImage::ImageRgb8(buf)
            }
            4 => {
                let buf = image::RgbaImage::from_raw(
                    image.width,
                    image.height,
                    image.data.clone(),
                )
                .ok_or("Failed to create RgbaImage from raw data")?;
                image::DynamicImage::ImageRgba8(buf)
            }
            _ => return Err(format!("Unsupported channel count: {}", image.channel)),
        };
        Ok(img)
    }

    pub fn dynamic_to_sd_image(img: &image::DynamicImage) -> Result<SdImage, String> {
        let rgb = img.to_rgb8();
        let width = rgb.width();
        let height = rgb.height();
        SdImage::rgb(width, height, rgb.into_raw())
            .map_err(|e| e.to_string())
    }

    pub fn load_image(path: &str) -> Result<image::DynamicImage, String> {
        let base = std::env::var("COMFY_INPUT_DIR")
            .unwrap_or_else(|_| "input".to_string());
        let full_path = Path::new(&base).join(path);
        image::open(&full_path)
            .map_err(|e| format!("Failed to open image {}: {}", full_path.display(), e))
    }

    pub fn canny_preprocess(
        img: &image::DynamicImage,
        low_threshold: f32,
        high_threshold: f32,
    ) -> Result<image::DynamicImage, String> {
        let gray = img.to_luma8();

        let edges = imageproc::edges::canny(
            &gray,
            low_threshold,
            high_threshold,
        );

        Ok(image::DynamicImage::ImageLuma8(edges))
    }

    pub fn sobel_preprocess(
        img: &image::DynamicImage,
        _ksize: i32,
    ) -> Result<image::DynamicImage, String> {
        let gray = img.to_luma8();
        let (width, height) = gray.dimensions();

        let grad_x = imageproc::gradients::horizontal_sobel(&gray);
        let grad_y = imageproc::gradients::vertical_sobel(&gray);

        let mut result = image::GrayImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let gx = grad_x.get_pixel(x, y)[0] as f32;
                let gy = grad_y.get_pixel(x, y)[0] as f32;
                let mag = (gx * gx + gy * gy).sqrt().min(255.0) as u8;
                result.put_pixel(x, y, image::Luma([mag]));
            }
        }

        Ok(image::DynamicImage::ImageLuma8(result))
    }

    pub fn depth_preprocess(
        img: &image::DynamicImage,
    ) -> Result<image::DynamicImage, String> {
        let gray = img.to_luma8();
        let (width, height) = gray.dimensions();

        let blurred = imageproc::filter::gaussian_blur_f32(&gray, 5.0);

        let grad_x = imageproc::gradients::horizontal_sobel(&blurred);
        let grad_y = imageproc::gradients::vertical_sobel(&blurred);

        let mut abs_grad = image::GrayImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let gx = grad_x.get_pixel(x, y)[0] as f32;
                let gy = grad_y.get_pixel(x, y)[0] as f32;
                let mag = (gx * gx + gy * gy).sqrt().min(255.0) as u8;
                abs_grad.put_pixel(x, y, image::Luma([mag]));
            }
        }

        let depth_map = imageproc::filter::gaussian_blur_f32(&abs_grad, 15.0);

        let mut normalized = image::GrayImage::new(width, height);
        let mut min_val = u8::MAX;
        let mut max_val = u8::MIN;
        for y in 0..height {
            for x in 0..width {
                let v = depth_map.get_pixel(x, y)[0];
                min_val = min_val.min(v);
                max_val = max_val.max(v);
            }
        }
        let range = (max_val as f32 - min_val as f32).max(1.0);
        for y in 0..height {
            for x in 0..width {
                let v = depth_map.get_pixel(x, y)[0];
                let norm = ((v as f32 - min_val as f32) / range * 255.0) as u8;
                normalized.put_pixel(x, y, image::Luma([norm]));
            }
        }

        Ok(image::DynamicImage::ImageLuma8(normalized))
    }

    pub fn lineart_preprocess(
        img: &image::DynamicImage,
        coarse: bool,
    ) -> Result<image::DynamicImage, String> {
        let gray = img.to_luma8();
        let (width, height) = gray.dimensions();

        let blur_sigma = if coarse { 3.0 } else { 1.0 };
        let blurred = imageproc::filter::gaussian_blur_f32(&gray, blur_sigma);

        let (low, high) = if coarse { (0.05f32, 0.2f32) } else { (0.1f32, 0.4f32) };
        let edges = imageproc::edges::canny(&blurred, low, high);

        let mut inverted = image::GrayImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let v = edges.get_pixel(x, y)[0];
                inverted.put_pixel(x, y, image::Luma([255 - v]));
            }
        }

        Ok(image::DynamicImage::ImageLuma8(inverted))
    }

    pub fn hed_preprocess(
        img: &image::DynamicImage,
        safe_steps: i32,
    ) -> Result<image::DynamicImage, String> {
        let gray = img.to_luma8();
        let (width, height) = gray.dimensions();

        let blurred = imageproc::filter::gaussian_blur_f32(&gray, 1.0);

        let grad_x = imageproc::gradients::horizontal_sobel(&blurred);
        let grad_y = imageproc::gradients::vertical_sobel(&blurred);

        let mut combined = image::GrayImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let gx = grad_x.get_pixel(x, y)[0] as f32;
                let gy = grad_y.get_pixel(x, y)[0] as f32;
                let mag = (gx * gx + gy * gy).sqrt().min(255.0) as u8;
                combined.put_pixel(x, y, image::Luma([mag]));
            }
        }

        let mut smoothed = combined.clone();
        for _ in 0..safe_steps.max(1) {
            smoothed = imageproc::filter::gaussian_blur_f32(&smoothed, 1.5);
        }

        let mut binary = image::GrayImage::new(width, height);
        let mut sum = 0u64;
        let mut count = 0u64;
        for y in 0..height {
            for x in 0..width {
                sum += smoothed.get_pixel(x, y)[0] as u64;
                count += 1;
            }
        }
        let mean = (sum / count.max(1)) as u8;
        for y in 0..height {
            for x in 0..width {
                let v = smoothed.get_pixel(x, y)[0];
                let out = if v > mean { 255 } else { 0 };
                binary.put_pixel(x, y, image::Luma([out]));
            }
        }

        Ok(image::DynamicImage::ImageLuma8(binary))
    }

    pub fn threshold_preprocess(
        img: &image::DynamicImage,
        threshold_val: f32,
        invert: bool,
    ) -> Result<image::DynamicImage, String> {
        let gray = img.to_luma8();
        let (width, height) = gray.dimensions();
        let thresh = (threshold_val * 255.0).min(255.0).max(0.0) as u8;

        let mut result = image::GrayImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let v = gray.get_pixel(x, y)[0];
                let out = if invert {
                    if v < thresh { 255 } else { 0 }
                } else {
                    if v > thresh { 255 } else { 0 }
                };
                result.put_pixel(x, y, image::Luma([out]));
            }
        }

        Ok(image::DynamicImage::ImageLuma8(result))
    }

    pub fn invert_preprocess(
        img: &image::DynamicImage,
    ) -> Result<image::DynamicImage, String> {
        let rgb = img.to_rgb8();
        let (width, height) = rgb.dimensions();

        let mut result = image::RgbImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let p = rgb.get_pixel(x, y);
                result.put_pixel(x, y, image::Rgb([255 - p[0], 255 - p[1], 255 - p[2]]));
            }
        }

        Ok(image::DynamicImage::ImageRgb8(result))
    }
}

fn load_image_from_value(image_val: &Value) -> Result<SdImage, String> {
    if let Some(path) = image_val.get("path").and_then(|v| v.as_str()) {
        let dyn_img = imp::load_image(path)?;
        imp::dynamic_to_sd_image(&dyn_img)
    } else if let Some(images) = image_val.get("images").and_then(|v| v.as_array()) {
        if let Some(first) = images.first() {
            serde_json::from_value::<SdImage>(first.clone())
                .map_err(|e| format!("Failed to deserialize SdImage: {}", e))
        } else {
            Err("Image array is empty".to_string())
        }
    } else {
        serde_json::from_value::<SdImage>(image_val.clone())
            .map_err(|e| format!("Failed to deserialize SdImage: {}", e))
    }
}

fn resolve_image(ctx: &ExecutionContext, node_id: &str, input_name: &str) -> Result<SdImage, ExecutorError> {
    let image_val = ctx.resolve_input(node_id, input_name)
        .unwrap_or_else(|_| json!(null));

    if image_val.is_null() {
        return Err(ExecutorError::NodeExecutionFailed {
            node_id: node_id.to_string(),
            message: format!("Input '{}' is required but not provided", input_name),
        });
    }

    load_image_from_value(&image_val).map_err(|e| ExecutorError::NodeExecutionFailed {
        node_id: node_id.to_string(),
        message: e,
    })
}

fn image_output(sd_image: SdImage) -> Result<Vec<Value>, ExecutorError> {
    let val = serde_json::to_value(&sd_image).map_err(|e| ExecutorError::NodeExecutionFailed {
        node_id: String::new(),
        message: format!("Failed to serialize image: {}", e),
    })?;
    Ok(vec![json!({
        "type": "image",
        "images": [val],
    })])
}

pub fn register_controlnet_nodes(registry: &mut NodeRegistry) {
    register_control_net_loader(registry);
    register_canny_preprocessor(registry);
    register_sobel_preprocessor(registry);
    register_depth_preprocessor(registry);
    register_lineart_preprocessor(registry);
    register_hed_preprocessor(registry);
    register_threshold_preprocessor(registry);
    register_invert_preprocessor(registry);
}

fn resolve_model_path(model_type: &str, filename: &str) -> String {
    let base = std::env::var("COMFY_MODELS_DIR").unwrap_or_else(|_| "models".to_string());
    std::path::Path::new(&base)
        .join(model_type)
        .join(filename)
        .to_string_lossy()
        .to_string()
}

fn register_control_net_loader(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "ControlNetLoader".to_string(),
        display_name: "Load ControlNet Model".to_string(),
        category: "loaders".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("control_net_name".to_string(), InputTypeSpec {
                    type_name: "COMBO".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::ControlNet],
        output_names: vec!["CONTROL_NET".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "load_control_net".to_string(),
    };

    registry.register(class_def, Arc::new(|_ctx, node, _node_id| {
        let cn_name = node.inputs.get("control_net_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let cn_path = resolve_model_path("controlnet", cn_name);
        let cn_path_owned = cn_path.clone();

        Box::pin(async move {
            if !std::path::Path::new(&cn_path_owned).exists() {
                tracing::warn!("ControlNet model not found at: {}", cn_path_owned);
            }
            Ok(vec![json!({
                "type": "controlnet",
                "path": cn_path_owned,
            })])
        })
    }));
}

fn register_canny_preprocessor(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "Canny_Preprocessor".to_string(),
        display_name: "Canny Edge Detection".to_string(),
        category: "controlnet/preprocessors".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("image".to_string(), InputTypeSpec {
                    type_name: "IMAGE".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("low_threshold".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("high_threshold".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Image],
        output_names: vec!["IMAGE".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "preprocess".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let sd_image = match resolve_image(ctx, node_id, "image") {
            Ok(img) => img,
            Err(e) => return Box::pin(async move { Err(e) }),
        };
        let low_threshold = ctx.resolve_input(node_id, "low_threshold")
            .unwrap_or_else(|_| json!(0.1))
            .as_f64()
            .unwrap_or(0.1) as f32;
        let high_threshold = ctx.resolve_input(node_id, "high_threshold")
            .unwrap_or_else(|_| json!(0.3))
            .as_f64()
            .unwrap_or(0.3) as f32;

        Box::pin(async move {
            let dyn_img = imp::sd_image_to_dynamic(&sd_image)
                .map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: e,
                })?;

            let result_img = dispatch::canny_preprocess(&dyn_img, low_threshold, high_threshold)
                .map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: e,
                })?;

            let result = imp::dynamic_to_sd_image(&result_img)
                .map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: e,
                })?;

            image_output(result)
        })
    }));
}

#[cfg(feature = "opencv")]
mod opencv_imp {
    use comfy_inference::SdImage;
    use opencv::prelude::*;

    pub fn sd_image_to_mat(image: &SdImage) -> Result<opencv::core::Mat, String> {
        let rgb_data = match image.channel {
            1 => image.data.iter().flat_map(|&g| [g, g, g]).collect(),
            3 => image.data.clone(),
            4 => image.data.chunks(4).flat_map(|px| [px[0], px[1], px[2]]).collect(),
            _ => image.data.clone(),
        };
        let mat = opencv::core::Mat::from_slice(&rgb_data)
            .map_err(|e| format!("Failed to create Mat: {}", e))?;
        let reshaped = mat.reshape(3, image.height as i32)
            .map_err(|e| format!("Failed to reshape Mat: {}", e))?;
        let mut rgb = opencv::core::Mat::default();
        opencv::core::transpose(&reshaped, &mut rgb)
            .map_err(|e| format!("Failed to transpose: {}", e))?;
        Ok(rgb)
    }

    pub fn mat_to_sd_image(mat: &opencv::core::Mat) -> Result<SdImage, String> {
        let mut rgb = opencv::core::Mat::default();
        opencv::imgproc::cvt_color_def(mat, &mut rgb, opencv::imgproc::COLOR_BGR2RGB)
            .map_err(|e| format!("cvt_color failed: {}", e))?;
        let width = rgb.cols() as u32;
        let height = rgb.rows() as u32;
        let data = rgb.data_bytes().map_err(|e| format!("Failed to get Mat data: {}", e))?;
        SdImage::rgb(width, height, data.to_vec())
            .map_err(|e| e.to_string())
    }

    pub fn canny_preprocess(
        img: &image::DynamicImage,
        low_threshold: f64,
        high_threshold: f64,
    ) -> Result<image::DynamicImage, String> {
        let sd_img = super::imp::dynamic_to_sd_image(img)?;
        let mat = sd_image_to_mat(&sd_img)?;
        let mut gray = opencv::core::Mat::default();
        opencv::imgproc::cvt_color_def(&mat, &mut gray, opencv::imgproc::COLOR_RGB2GRAY)
            .map_err(|e| format!("cvt_color to gray failed: {}", e))?;
        let mut edges = opencv::core::Mat::default();
        opencv::imgproc::canny_def(&gray, &mut edges, low_threshold, high_threshold)
            .map_err(|e| format!("Canny failed: {}", e))?;
        let mut rgb_result = opencv::core::Mat::default();
        opencv::imgproc::cvt_color_def(&edges, &mut rgb_result, opencv::imgproc::COLOR_GRAY2RGB)
            .map_err(|e| format!("cvt_color to rgb failed: {}", e))?;
        let result = mat_to_sd_image(&rgb_result)?;
        super::imp::sd_image_to_dynamic(&result)
    }

    pub fn sobel_preprocess(
        img: &image::DynamicImage,
        _ksize: i32,
    ) -> Result<image::DynamicImage, String> {
        let sd_img = super::imp::dynamic_to_sd_image(img)?;
        let mat = sd_image_to_mat(&sd_img)?;
        let mut gray = opencv::core::Mat::default();
        opencv::imgproc::cvt_color_def(&mat, &mut gray, opencv::imgproc::COLOR_RGB2GRAY)
            .map_err(|e| format!("cvt_color to gray failed: {}", e))?;
        let mut grad_x = opencv::core::Mat::default();
        let mut grad_y = opencv::core::Mat::default();
        opencv::imgproc::sobel_def(&gray, &mut grad_x, opencv::core::CV_32F, 1, 0)
            .map_err(|e| format!("Sobel X failed: {}", e))?;
        opencv::imgproc::sobel_def(&gray, &mut grad_y, opencv::core::CV_32F, 0, 1)
            .map_err(|e| format!("Sobel Y failed: {}", e))?;
        let mut abs_grad_x = opencv::core::Mat::default();
        let mut abs_grad_y = opencv::core::Mat::default();
        opencv::core::convert_scale_abs(&grad_x, &mut abs_grad_x, 1.0, 0.0)
            .map_err(|e| format!("convert_scale_abs X failed: {}", e))?;
        opencv::core::convert_scale_abs(&grad_y, &mut abs_grad_y, 1.0, 0.0)
            .map_err(|e| format!("convert_scale_abs Y failed: {}", e))?;
        let mut combined = opencv::core::Mat::default();
        opencv::core::add_weighted(&abs_grad_x, 0.5, &abs_grad_y, 0.5, 0.0, &mut combined, opencv::core::CV_8U)
            .map_err(|e| format!("add_weighted failed: {}", e))?;
        let mut rgb_result = opencv::core::Mat::default();
        opencv::imgproc::cvt_color_def(&combined, &mut rgb_result, opencv::imgproc::COLOR_GRAY2RGB)
            .map_err(|e| format!("cvt_color to rgb failed: {}", e))?;
        let result = mat_to_sd_image(&rgb_result)?;
        super::imp::sd_image_to_dynamic(&result)
    }

    pub fn depth_preprocess(
        img: &image::DynamicImage,
    ) -> Result<image::DynamicImage, String> {
        let sd_img = super::imp::dynamic_to_sd_image(img)?;
        let mat = sd_image_to_mat(&sd_img)?;
        let mut gray = opencv::core::Mat::default();
        opencv::imgproc::cvt_color_def(&mat, &mut gray, opencv::imgproc::COLOR_RGB2GRAY)
            .map_err(|e| format!("cvt_color to gray failed: {}", e))?;

        let mut blurred = opencv::core::Mat::default();
        opencv::imgproc::gaussian_blur_def(&gray, &mut blurred, opencv::core::Size::new(7, 7), 0.0)
            .map_err(|e| format!("GaussianBlur failed: {}", e))?;

        let mut laplacian = opencv::core::Mat::default();
        opencv::imgproc::laplacian_def(&blurred, &mut laplacian, opencv::core::CV_32F)
            .map_err(|e| format!("Laplacian failed: {}", e))?;

        let mut abs_lap = opencv::core::Mat::default();
        opencv::core::convert_scale_abs(&laplacian, &mut abs_lap, 1.0, 0.0)
            .map_err(|e| format!("convert_scale_abs failed: {}", e))?;

        let mut depth_coarse = opencv::core::Mat::default();
        opencv::imgproc::gaussian_blur_def(&abs_lap, &mut depth_coarse, opencv::core::Size::new(31, 31), 0.0)
            .map_err(|e| format!("GaussianBlur coarse failed: {}", e))?;

        let mut depth_medium = opencv::core::Mat::default();
        opencv::imgproc::gaussian_blur_def(&abs_lap, &mut depth_medium, opencv::core::Size::new(15, 15), 0.0)
            .map_err(|e| format!("GaussianBlur medium failed: {}", e))?;

        let mut depth_fine = opencv::core::Mat::default();
        opencv::imgproc::gaussian_blur_def(&abs_lap, &mut depth_fine, opencv::core::Size::new(5, 5), 0.0)
            .map_err(|e| format!("GaussianBlur fine failed: {}", e))?;

        let mut combined = opencv::core::Mat::default();
        opencv::core::add_weighted(&depth_coarse, 0.5, &depth_medium, 0.3, 0.0, &mut combined, opencv::core::CV_8U)
            .map_err(|e| format!("add_weighted 1 failed: {}", e))?;
        let mut depth_map = opencv::core::Mat::default();
        opencv::core::add_weighted(&combined, 0.8, &depth_fine, 0.2, 0.0, &mut depth_map, opencv::core::CV_8U)
            .map_err(|e| format!("add_weighted 2 failed: {}", e))?;

        let mut inverted = opencv::core::Mat::default();
        opencv::core::bitwise_not_def(&depth_map, &mut inverted)
            .map_err(|e| format!("bitwise_not failed: {}", e))?;

        let mut normalized = opencv::core::Mat::default();
        opencv::core::normalize(&inverted, &mut normalized, 0.0, 255.0, opencv::core::NORM_MINMAX, opencv::core::CV_8U, &opencv::core::no_array())
            .map_err(|e| format!("normalize failed: {}", e))?;

        let mut rgb_result = opencv::core::Mat::default();
        opencv::imgproc::cvt_color_def(&normalized, &mut rgb_result, opencv::imgproc::COLOR_GRAY2RGB)
            .map_err(|e| format!("cvt_color to rgb failed: {}", e))?;
        let result = mat_to_sd_image(&rgb_result)?;
        super::imp::sd_image_to_dynamic(&result)
    }

    pub fn lineart_preprocess(
        img: &image::DynamicImage,
        coarse: bool,
    ) -> Result<image::DynamicImage, String> {
        let sd_img = super::imp::dynamic_to_sd_image(img)?;
        let mat = sd_image_to_mat(&sd_img)?;
        let mut gray = opencv::core::Mat::default();
        opencv::imgproc::cvt_color_def(&mat, &mut gray, opencv::imgproc::COLOR_RGB2GRAY)
            .map_err(|e| format!("cvt_color to gray failed: {}", e))?;

        let mut blurred = opencv::core::Mat::default();
        let blur_size = if coarse { 5 } else { 3 };
        opencv::imgproc::gaussian_blur_def(&gray, &mut blurred, opencv::core::Size::new(blur_size, blur_size), 0.0)
            .map_err(|e| format!("GaussianBlur failed: {}", e))?;

        let mut binary = opencv::core::Mat::default();
        let block_size = if coarse { 31 } else { 15 };
        opencv::imgproc::adaptive_threshold(
            &blurred, &mut binary, 255.0,
            opencv::imgproc::ADAPTIVE_THRESH_GAUSSIAN_C,
            opencv::imgproc::THRESH_BINARY_INV,
            block_size, 5.0,
        ).map_err(|e| format!("adaptive_threshold failed: {}", e))?;

        let kernel_size = if coarse { 3 } else { 1 };
        let kernel = opencv::imgproc::get_structuring_element_def(
            opencv::imgproc::MORPH_ELLIPSE,
            opencv::core::Size::new(kernel_size, kernel_size),
        ).map_err(|e| format!("get_structuring_element failed: {}", e))?;

        let mut cleaned = opencv::core::Mat::default();
        opencv::imgproc::morphology_ex_def(&binary, &mut cleaned, opencv::imgproc::MORPH_CLOSE, &kernel)
            .map_err(|e| format!("morphology failed: {}", e))?;

        let mut rgb_result = opencv::core::Mat::default();
        opencv::imgproc::cvt_color_def(&cleaned, &mut rgb_result, opencv::imgproc::COLOR_GRAY2RGB)
            .map_err(|e| format!("cvt_color to rgb failed: {}", e))?;
        let result = mat_to_sd_image(&rgb_result)?;
        super::imp::sd_image_to_dynamic(&result)
    }

    pub fn threshold_preprocess(
        img: &image::DynamicImage,
        threshold_val: f32,
        invert: bool,
    ) -> Result<image::DynamicImage, String> {
        let sd_img = super::imp::dynamic_to_sd_image(img)?;
        let mat = sd_image_to_mat(&sd_img)?;
        let mut gray = opencv::core::Mat::default();
        opencv::imgproc::cvt_color_def(&mat, &mut gray, opencv::imgproc::COLOR_RGB2GRAY)
            .map_err(|e| format!("cvt_color to gray failed: {}", e))?;
        let thresh = (threshold_val * 255.0).min(255.0) as f64;
        let thresh_type = if invert {
            opencv::imgproc::THRESH_BINARY_INV
        } else {
            opencv::imgproc::THRESH_BINARY
        };
        let mut result = opencv::core::Mat::default();
        opencv::imgproc::threshold(&gray, &mut result, thresh, 255.0, thresh_type)
            .map_err(|e| format!("threshold failed: {}", e))?;
        let mut rgb_result = opencv::core::Mat::default();
        opencv::imgproc::cvt_color_def(&result, &mut rgb_result, opencv::imgproc::COLOR_GRAY2RGB)
            .map_err(|e| format!("cvt_color to rgb failed: {}", e))?;
        let sd_result = mat_to_sd_image(&rgb_result)?;
        super::imp::sd_image_to_dynamic(&sd_result)
    }

    pub fn invert_preprocess(
        img: &image::DynamicImage,
    ) -> Result<image::DynamicImage, String> {
        let sd_img = super::imp::dynamic_to_sd_image(img)?;
        let mat = sd_image_to_mat(&sd_img)?;
        let mut result = opencv::core::Mat::default();
        opencv::core::bitwise_not_def(&mat, &mut result)
            .map_err(|e| format!("bitwise_not failed: {}", e))?;
        let sd_result = mat_to_sd_image(&result)?;
        super::imp::sd_image_to_dynamic(&sd_result)
    }

    pub fn hed_preprocess(
        img: &image::DynamicImage,
        safe_steps: i32,
    ) -> Result<image::DynamicImage, String> {
        let sd_img = super::imp::dynamic_to_sd_image(img)?;
        let mat = sd_image_to_mat(&sd_img)?;
        let mut gray = opencv::core::Mat::default();
        opencv::imgproc::cvt_color_def(&mat, &mut gray, opencv::imgproc::COLOR_RGB2GRAY)
            .map_err(|e| format!("cvt_color to gray failed: {}", e))?;

        let mut gray_f32 = opencv::core::Mat::default();
        gray.convert_to(&mut gray_f32, opencv::core::CV_32F, 1.0 / 255.0, 0.0)
            .map_err(|e| format!("convert_to f32 failed: {}", e))?;

        let sigma_c = 0.8;
        let sigma_s = sigma_c * (1.6 + 0.3 * safe_steps as f64);

        let mut gauss_c = opencv::core::Mat::default();
        let mut gauss_s = opencv::core::Mat::default();
        opencv::imgproc::gaussian_blur_def(&gray_f32, &mut gauss_c, opencv::core::Size::new(0, 0), sigma_c)
            .map_err(|e| format!("GaussianBlur c failed: {}", e))?;
        opencv::imgproc::gaussian_blur_def(&gray_f32, &mut gauss_s, opencv::core::Size::new(0, 0), sigma_s)
            .map_err(|e| format!("GaussianBlur s failed: {}", e))?;

        let rows = gray_f32.rows();
        let cols = gray_f32.cols();
        let gc_data = gauss_c.data_bytes().map_err(|e| format!("gauss_c data: {}", e))?;
        let gs_data = gauss_s.data_bytes().map_err(|e| format!("gauss_s data: {}", e))?;

        let mut xdog_data = vec![0u8; (rows * cols) as usize];
        let tau: f32 = 0.98;
        let epsilon: f32 = 10.0;

        for i in 0..(rows * cols) as usize {
            let gc = f32::from_le_bytes(gc_data[i * 4..i * 4 + 4].try_into().unwrap_or([0u8; 4]));
            let gs = f32::from_le_bytes(gs_data[i * 4..i * 4 + 4].try_into().unwrap_or([0u8; 4]));
            let diff = gc - tau * gs;
            let val = if gc > 0.0 { diff / gc } else { 0.0 };
            let result = if val > epsilon { 1.0 } else { 1.0 + f32::tanh((val - epsilon) * 5.0) };
            xdog_data[i] = (result.min(1.0).max(0.0) * 255.0) as u8;
        }

        let xdog_mat = opencv::core::Mat::from_slice(&xdog_data)
            .map_err(|e| format!("from_slice failed: {}", e))?;
        let reshaped = xdog_mat.reshape(1, rows)
            .map_err(|e| format!("reshape failed: {}", e))?;

        let mut rgb_result = opencv::core::Mat::default();
        opencv::imgproc::cvt_color_def(&reshaped, &mut rgb_result, opencv::imgproc::COLOR_GRAY2RGB)
            .map_err(|e| format!("cvt_color to rgb failed: {}", e))?;
        let result = mat_to_sd_image(&rgb_result)?;
        super::imp::sd_image_to_dynamic(&result)
    }
}

#[cfg(feature = "opencv")]
mod dispatch {
    pub fn canny_preprocess(img: &image::DynamicImage, low: f32, high: f32) -> Result<image::DynamicImage, String> {
        super::opencv_imp::canny_preprocess(img, low as f64, high as f64)
    }
    pub fn sobel_preprocess(img: &image::DynamicImage, ksize: i32) -> Result<image::DynamicImage, String> {
        super::opencv_imp::sobel_preprocess(img, ksize)
    }
    pub fn depth_preprocess(img: &image::DynamicImage) -> Result<image::DynamicImage, String> {
        super::opencv_imp::depth_preprocess(img)
    }
    pub fn lineart_preprocess(img: &image::DynamicImage, coarse: bool) -> Result<image::DynamicImage, String> {
        super::opencv_imp::lineart_preprocess(img, coarse)
    }
    pub fn hed_preprocess(img: &image::DynamicImage, safe_steps: i32) -> Result<image::DynamicImage, String> {
        super::opencv_imp::hed_preprocess(img, safe_steps)
    }
    pub fn threshold_preprocess(img: &image::DynamicImage, thresh: f32, invert: bool) -> Result<image::DynamicImage, String> {
        super::opencv_imp::threshold_preprocess(img, thresh, invert)
    }
    pub fn invert_preprocess(img: &image::DynamicImage) -> Result<image::DynamicImage, String> {
        super::opencv_imp::invert_preprocess(img)
    }
}

#[cfg(not(feature = "opencv"))]
mod dispatch {
    pub fn canny_preprocess(img: &image::DynamicImage, low: f32, high: f32) -> Result<image::DynamicImage, String> {
        super::imp::canny_preprocess(img, low, high)
    }
    pub fn sobel_preprocess(img: &image::DynamicImage, ksize: i32) -> Result<image::DynamicImage, String> {
        super::imp::sobel_preprocess(img, ksize)
    }
    pub fn depth_preprocess(img: &image::DynamicImage) -> Result<image::DynamicImage, String> {
        super::imp::depth_preprocess(img)
    }
    pub fn lineart_preprocess(img: &image::DynamicImage, coarse: bool) -> Result<image::DynamicImage, String> {
        super::imp::lineart_preprocess(img, coarse)
    }
    pub fn hed_preprocess(img: &image::DynamicImage, safe_steps: i32) -> Result<image::DynamicImage, String> {
        super::imp::hed_preprocess(img, safe_steps)
    }
    pub fn threshold_preprocess(img: &image::DynamicImage, thresh: f32, invert: bool) -> Result<image::DynamicImage, String> {
        super::imp::threshold_preprocess(img, thresh, invert)
    }
    pub fn invert_preprocess(img: &image::DynamicImage) -> Result<image::DynamicImage, String> {
        super::imp::invert_preprocess(img)
    }
}

fn register_sobel_preprocessor(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "Sobel_Preprocessor".to_string(),
        display_name: "Sobel Edge Detection".to_string(),
        category: "controlnet/preprocessors".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("image".to_string(), InputTypeSpec {
                    type_name: "IMAGE".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("ksize".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Image],
        output_names: vec!["IMAGE".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "preprocess".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let sd_image = match resolve_image(ctx, node_id, "image") {
            Ok(img) => img,
            Err(e) => return Box::pin(async move { Err(e) }),
        };
        let ksize = ctx.resolve_input(node_id, "ksize")
            .unwrap_or_else(|_| json!(3))
            .as_i64()
            .unwrap_or(3) as i32;

        Box::pin(async move {
            let dyn_img = imp::sd_image_to_dynamic(&sd_image)
                .map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: e,
                })?;

            let result_img = dispatch::sobel_preprocess(&dyn_img, ksize)
                .map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: e,
                })?;

            let result = imp::dynamic_to_sd_image(&result_img)
                .map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: e,
                })?;

            image_output(result)
        })
    }));
}

fn register_depth_preprocessor(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "Depth_Preprocessor".to_string(),
        display_name: "Depth Map (Gradient)".to_string(),
        category: "controlnet/preprocessors".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("image".to_string(), InputTypeSpec {
                    type_name: "IMAGE".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Image],
        output_names: vec!["IMAGE".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "preprocess".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let sd_image = match resolve_image(ctx, node_id, "image") {
            Ok(img) => img,
            Err(e) => return Box::pin(async move { Err(e) }),
        };

        Box::pin(async move {
            let dyn_img = imp::sd_image_to_dynamic(&sd_image)
                .map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: e,
                })?;

            let result_img = dispatch::depth_preprocess(&dyn_img)
                .map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: e,
                })?;

            let result = imp::dynamic_to_sd_image(&result_img)
                .map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: e,
                })?;

            image_output(result)
        })
    }));
}

fn register_lineart_preprocessor(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "Lineart_Preprocessor".to_string(),
        display_name: "Lineart".to_string(),
        category: "controlnet/preprocessors".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("image".to_string(), InputTypeSpec {
                    type_name: "IMAGE".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("coarse".to_string(), InputTypeSpec {
                    type_name: "BOOLEAN".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Image],
        output_names: vec!["IMAGE".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "preprocess".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let sd_image = match resolve_image(ctx, node_id, "image") {
            Ok(img) => img,
            Err(e) => return Box::pin(async move { Err(e) }),
        };
        let coarse = ctx.resolve_input(node_id, "coarse")
            .unwrap_or_else(|_| json!(false))
            .as_bool()
            .unwrap_or(false);

        Box::pin(async move {
            let dyn_img = imp::sd_image_to_dynamic(&sd_image)
                .map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: e,
                })?;

            let result_img = dispatch::lineart_preprocess(&dyn_img, coarse)
                .map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: e,
                })?;

            let result = imp::dynamic_to_sd_image(&result_img)
                .map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: e,
                })?;

            image_output(result)
        })
    }));
}

fn register_hed_preprocessor(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "HED_Preprocessor".to_string(),
        display_name: "HED Edge Detection".to_string(),
        category: "controlnet/preprocessors".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("image".to_string(), InputTypeSpec {
                    type_name: "IMAGE".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("safe_steps".to_string(), InputTypeSpec {
                    type_name: "INT".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Image],
        output_names: vec!["IMAGE".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "preprocess".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let sd_image = match resolve_image(ctx, node_id, "image") {
            Ok(img) => img,
            Err(e) => return Box::pin(async move { Err(e) }),
        };
        let safe_steps = ctx.resolve_input(node_id, "safe_steps")
            .unwrap_or_else(|_| json!(2))
            .as_i64()
            .unwrap_or(2) as i32;

        Box::pin(async move {
            let dyn_img = imp::sd_image_to_dynamic(&sd_image)
                .map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: e,
                })?;

            let result_img = dispatch::hed_preprocess(&dyn_img, safe_steps)
                .map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: e,
                })?;

            let result = imp::dynamic_to_sd_image(&result_img)
                .map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: e,
                })?;

            image_output(result)
        })
    }));
}

fn register_threshold_preprocessor(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "Threshold_Preprocessor".to_string(),
        display_name: "Binary Threshold".to_string(),
        category: "controlnet/preprocessors".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("image".to_string(), InputTypeSpec {
                    type_name: "IMAGE".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: {
                let mut m = HashMap::new();
                m.insert("threshold".to_string(), InputTypeSpec {
                    type_name: "FLOAT".to_string(),
                    extra: HashMap::new(),
                });
                m.insert("invert".to_string(), InputTypeSpec {
                    type_name: "BOOLEAN".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Image],
        output_names: vec!["IMAGE".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "preprocess".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let sd_image = match resolve_image(ctx, node_id, "image") {
            Ok(img) => img,
            Err(e) => return Box::pin(async move { Err(e) }),
        };
        let thresh_val = ctx.resolve_input(node_id, "threshold")
            .unwrap_or_else(|_| json!(0.5))
            .as_f64()
            .unwrap_or(0.5) as f32;
        let invert = ctx.resolve_input(node_id, "invert")
            .unwrap_or_else(|_| json!(false))
            .as_bool()
            .unwrap_or(false);

        Box::pin(async move {
            let dyn_img = imp::sd_image_to_dynamic(&sd_image)
                .map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: e,
                })?;

            let result_img = dispatch::threshold_preprocess(&dyn_img, thresh_val, invert)
                .map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: e,
                })?;

            let result = imp::dynamic_to_sd_image(&result_img)
                .map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: e,
                })?;

            image_output(result)
        })
    }));
}

fn register_invert_preprocessor(registry: &mut NodeRegistry) {
    let class_def = NodeClassDef {
        class_type: "Invert_Preprocessor".to_string(),
        display_name: "Invert Image".to_string(),
        category: "controlnet/preprocessors".to_string(),
        input_types: NodeInputTypes {
            required: {
                let mut m = HashMap::new();
                m.insert("image".to_string(), InputTypeSpec {
                    type_name: "IMAGE".to_string(),
                    extra: HashMap::new(),
                });
                m
            },
            optional: HashMap::new(),
            hidden: HashMap::new(),
        },
        output_types: vec![IoType::Image],
        output_names: vec!["IMAGE".to_string()],
        output_is_list: vec![false],
        is_output_node: false,
        has_intermediate_output: false,
        is_changed: None,
        not_idempotent: false,
        function_name: "preprocess".to_string(),
    };

    registry.register(class_def, Arc::new(|ctx, _node, node_id| {
        let sd_image = match resolve_image(ctx, node_id, "image") {
            Ok(img) => img,
            Err(e) => return Box::pin(async move { Err(e) }),
        };

        Box::pin(async move {
            let dyn_img = imp::sd_image_to_dynamic(&sd_image)
                .map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: e,
                })?;

            let result_img = dispatch::invert_preprocess(&dyn_img)
                .map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: e,
                })?;

            let result = imp::dynamic_to_sd_image(&result_img)
                .map_err(|e| ExecutorError::NodeExecutionFailed {
                    node_id: node_id.to_string(),
                    message: e,
                })?;

            image_output(result)
        })
    }));
}
