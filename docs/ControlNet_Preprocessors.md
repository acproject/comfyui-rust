# ControlNet 预处理算法差异化改造

## 问题背景

在启用 `controlnet-opencv` 特性之前，所有 ControlNet 预处理器（Depth、Lineart、HED 等）都基于 `imageproc` 库的 Sobel 梯度算子实现，导致不同预处理器的输出效果高度相似，无法为不同的 ControlNet 模型提供有区分度的条件图像。

## 改造方案

利用 OpenCV 的丰富图像处理算子，为每个预处理器实现具有明显差异化的算法。

## 算法对比表

| 预处理器 | 旧算法 | 新算法 (OpenCV) | 效果特征 | 适用 ControlNet 模型 |
|----------|--------|-----------------|----------|---------------------|
| **Canny** | Sobel → Canny | OpenCV Canny | 细边缘线，黑白分明 | ControlNet Canny |
| **Sobel** | Sobel 梯度幅度 | OpenCV Sobel | 梯度强度图，灰度连续 | 通用边缘引导 |
| **Depth** | Sobel + 模糊 + 归一化 | Laplacian + 多尺度高斯金字塔 + 反转 | 模拟深度图，近亮远暗，平滑渐变 | ControlNet Depth |
| **Lineart** | Canny + 反色 | 自适应阈值 + 形态学闭运算 | 细腻连续线条，类似手绘线稿 | ControlNet Lineart |
| **HED** | Sobel + 模糊 + 阈值 | XDoG (eXtended Difference of Gaussians) | 柔和边缘，艺术风格线条 | ControlNet HED |
| **Threshold** | 简单阈值 | OpenCV 阈值 | 二值化 | 通用二值引导 |
| **Invert** | 像素反转 | OpenCV bitwise_not | 颜色反转 | 辅助处理 |

## 各算法详细说明

### 1. Depth Preprocessor (深度预处理)

**算法流程：**
1. 灰度化 + 高斯模糊 (7x7)
2. Laplacian 算子检测纹理细节
3. 多尺度高斯金字塔融合：
   - 粗尺度 (31x31)：捕捉大尺度深度变化
   - 中尺度 (15x15)：中等尺度结构
   - 细尺度 (5x5)：保留细节
   - 权重：0.5 : 0.3 : 0.2
4. 反转 (bitwise_not)：使近处亮、远处暗
5. 归一化到 [0, 255]

**效果特点：**
- 产生类似 MiDaS 深度估计的平滑渐变效果
- 近处物体较亮，远处较暗
- 比旧算法的 Sobel 梯度图更具深度感

### 2. Lineart Preprocessor (线稿预处理)

**算法流程：**
1. 灰度化 + 高斯模糊 (coarse: 5x5, fine: 3x3)
2. 自适应阈值 (Adaptive Gaussian Threshold)：
   - coarse: block_size=31, C=5
   - fine: block_size=15, C=5
3. 形态学闭运算 (MORPH_CLOSE)：
   - coarse: 3x3 椭圆核
   - fine: 1x1 核
4. 灰度转 RGB 输出

**效果特点：**
- 自适应阈值能根据局部亮度调整阈值，产生更均匀的线条
- 形态学闭运算连接断裂的线条，消除小噪点
- 比旧算法的 Canny 反色更接近手绘线稿效果

### 3. HED Preprocessor (HED 边缘检测)

**算法流程：**
1. 灰度化 + 转浮点 [0, 1]
2. 双尺度高斯模糊：
   - σ_c = 0.8 (中心尺度)
   - σ_s = σ_c × (1.6 + 0.3 × safe_steps) (周围尺度)
3. XDoG 计算：
   - diff = gauss_c - τ × gauss_s (τ = 0.98)
   - val = diff / gauss_c (归一化)
   - result = tanh 平滑映射
4. 输出二值化边缘图

**效果特点：**
- XDoG (eXtended Difference of Gaussians) 模拟 HED 神经网络的边缘检测效果
- 通过 safe_steps 参数控制边缘粗细
- 比旧算法的 Sobel + 阈值更柔和、更具艺术感

### 4. Canny Preprocessor (Canny 边缘检测)

**算法流程：**
1. 灰度化
2. OpenCV Canny 算子 (low_threshold, high_threshold)
3. 灰度转 RGB 输出

**效果特点：**
- 使用 OpenCV 优化的 Canny 实现，性能更好
- 支持 CUDA 加速

### 5. Sobel Preprocessor (Sobel 边缘检测)

**算法流程：**
1. 灰度化
2. Sobel X 和 Sobel Y 梯度计算
3. 梯度幅度合成 (add_weighted)
4. 灰度转 RGB 输出

**效果特点：**
- 输出梯度强度图，灰度连续
- 适合需要平滑梯度信息的场景

### 6. Threshold Preprocessor (阈值预处理)

**算法流程：**
1. 灰度化
2. OpenCV 阈值 (THRESH_BINARY / THRESH_BINARY_INV)
3. 灰度转 RGB 输出

**效果特点：**
- 支持可配置阈值和反转
- 产生清晰的二值化图像

### 7. Invert Preprocessor (反转预处理)

**算法流程：**
1. OpenCV bitwise_not 按位取反
2. BGR 转 RGB 输出

**效果特点：**
- 简单的颜色反转操作

## 代码结构

```
crates/comfy-executor/src/controlnet.rs
── imp::                          # 基础实现 (imageproc)
│   ├── canny_preprocess()
│   ├── sobel_preprocess()
│   ├── depth_preprocess()
│   ├── lineart_preprocess()
│   ├── hed_preprocess()
│   ├── threshold_preprocess()
│   └── invert_preprocess()
├── opencv_imp::                   # OpenCV 加速实现 (feature = "opencv")
│   ├── sd_image_to_mat()
│   ├── mat_to_sd_image()
│   ├── canny_preprocess()
│   ├── sobel_preprocess()
│   ├── depth_preprocess()         # 新算法: Laplacian + 多尺度金字塔
│   ├── lineart_preprocess()       # 新算法: 自适应阈值 + 形态学
│   ├── hed_preprocess()           # 新算法: XDoG
│   ├── threshold_preprocess()
│   └── invert_preprocess()
└── dispatch::                     # 分发模块
    ├── #[cfg(feature = "opencv")] # 优先使用 OpenCV
    └── #[cfg(not(feature = "opencv"))] # 回退到 imageproc
```

## 启用方式

在 `start.sh` 中启用 `controlnet-opencv` 特性：

```bash
cargo build --release --features "controlnet-opencv,local-ffi"
```

## 性能对比

| 实现 | 优势 | 劣势 |
|------|------|------|
| imageproc (旧) | 纯 Rust，无外部依赖 | 算法简单，效果单一 |
| OpenCV (新) | 算法丰富，支持 CUDA 加速 | 需要 OpenCV 库依赖 |

## 注意事项

1. OpenCV 实现需要系统安装 OpenCV 库（含 CUDA 支持）
2. `mmproj-*.gguf` 辅助文件会自动过滤，不显示在 LLM 模型下拉框中
3. LLM 模型支持子目录组织，如 `llm/Qwen3.5-4B-GGUF/Qwen3.5-4B-Q4_0.gguf`
4. 自动检测同目录下的 `mmproj-*.gguf` 文件并添加 `--mmproj` 参数
