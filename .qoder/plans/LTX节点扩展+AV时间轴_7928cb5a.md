# LTX 节点扩展 + 音视频时间轴编辑器

## 现状分析

**后端已注册的 LTX 相关节点** (在 `builtin_nodes.rs` 中):
- LTXLoader, LTXVideoSampler, LTXVAudioVAELoader, LTXAVTextEncoderLoader
- LTXVConditioning, EmptyLTXVLatentVideo, LTXVEmptyLatentAudio
- LTXVImgToVideoInplace, LTXVPreprocess, LTXVCropGuides
- LTXVConcatAVLatent, LTXVSeparateAVLatent
- LTXVAudioVAEEncode, LTXVAudioVAEDecode
- LTXVLatentUpsampler, LatentUpscaleModelLoader, LoraLoaderModelOnly

**需要新增的节点** (参考 ComfyUI-LTXVideo `__init__.py`):

## Task 1: 后端 - 扩展 IoType 枚举

在 `crates/comfy-core/src/graph/node.rs` 的 `IoType` 枚举中新增:
- `GuiderParameters` - 用于 Guider 参数传递
- `Lora` - LoRA 模型类型（如需要）

同步更新 `io_type_str()` 和 `from_io_type_str()` 方法。

## Task 2: 后端 - 新增 AV 编排节点

在 `crates/comfy-executor/src/builtin_nodes.rs` 中新增:

**2a. MultimodalGuider 节点** (参考 `guiders/multimodal_guider.py`)
- class_type: `"MultimodalGuider"`
- 输入: model, positive, negative, parameters(GUIDER_PARAMETERS), skip_blocks(STRING)
- 输出: GUIDER
- 将音视频联合引导配置打包为 JSON 传递给后端

**2b. GuiderParameters 节点** (参考 `guiders/parameters.py`)
- class_type: `"GuiderParameters"`
- 输入: modality(COMBO: VIDEO/AUDIO), cfg(FLOAT), stg(FLOAT), perturb_attn(BOOLEAN), rescale(FLOAT), modality_scale(FLOAT), skip_step(INT), cross_attn(BOOLEAN), parameters(可选链式)
- 输出: GUIDER_PARAMETERS
- 支持链式连接，将 VIDEO 和 AUDIO 两组参数合并

**2c. STG 相关节点** (参考 `stg.py`)
- `LTXVApplySTG` - 应用 STG 到模型
- `STGGuiderNode` / `STGGuiderAdvancedNode` - STG 引导器
- `STGAdvancedPresetsNode` - STG 高级预设

## Task 3: 后端 - 新增采样器节点

**3a. 基础采样器系列** (参考 `easy_samplers.py`)
- `LTXVBaseSampler` - 基础视频采样
- `LTXVExtendSampler` - 扩展采样（续写视频）
- `LTXVInContextSampler` - 上下文采样
- `LTXVNormalizingSampler` - 归一化采样
- `LinearOverlapLatentTransition` - 潜空间过渡

**3b. 高级采样器**
- `LTXVLoopingSampler` (参考 `looping_sampler.py`) - 循环视频采样
- `LTXVTiledSampler` (参考 `tiled_sampler.py`) - 分块采样
- `LTXVTiledVAEDecode` (参考 `tiled_vae_decode.py`) - 分块 VAE 解码

## Task 4: 后端 - 新增 IC-LoRA 节点

参考 `iclora.py` 和 `iclora_attention.py`:
- `LTXAddVideoICLoRAGuide` - 添加 IC-LoRA 引导
- `LTXAddVideoICLoRAGuideAdvanced` - 高级 IC-LoRA 引导
- `LTXICLoRALoaderModelOnly` - IC-LoRA 模型加载
- `LTXVSetAudioRefTokens` - 设置音频参考 token（Lipdub 用）

## Task 5: 后端 - 新增 Latent 操作与辅助节点

**5a. Latent 归一化** (参考 `latent_norm.py`)
- `LTXVAdainLatent` - AdaIN 归一化
- `LTXVStatNormLatent` - 统计归一化
- `LTXVPerStepAdainPatcher` - 逐步骤 AdaIN
- `LTXVPerStepStatNormPatcher` - 逐步骤统计归一化

**5b. Latent 操作** (参考 `latents.py`, `pyramid_blending.py`)
- `LTXVAddLatentGuide` - 添加潜空间引导
- `LTXVImgToVideoConditionOnly` - 图生视频条件
- `LTXVSelectLatents` - 选择潜空间
- `LTXVSetVideoLatentNoiseMasks` - 设置噪声掩码
- `LTXVLaplacianPyramidBlend` - 拉普拉斯金字塔混合

**5c. 辅助工具节点**
- `LTXVPromptEnhancer` / `LTXVPromptEnhancerLoader` (参考 `prompt_enhancer_nodes.py`)
- `LTXVGemmaCLIPModelLoader` / `LTXVGemmaEnhancePrompt` (参考 `gemma_encoder.py`)
- `GemmaAPITextEncode` (参考 `gemma_api_conditioning.py`)
- `DynamicConditioning` (参考 `dynamic_conditioning.py`)
- `LTXVHDRDecodePostprocess` (参考 `hdr.py`)
- `LTXVDilateVideoMask` / `LTXVInpaintPreprocess` (参考 `vanish_nodes.py`)
- `FloatToInt` / `ImageToCPU` (参考 `utiltily_nodes.py`)
- `LTXVPatcherVAE` (参考 `vae_patcher.py`)
- `LTXVQ8Patch` / `LTXVQ8LoraModelLoader` (参考 `q8_nodes.py`)
- `DecoderNoise` (参考 `decoder_noise.py`)
- `LTXVDrawTracks` / `LTXVSparseTrackEditor` (参考 `sparse_tracks.py`)
- `LTXVLoadConditioning` / `LTXVSaveConditioning`
- `LTXVMultiPromptProvider` (参考 `looping_sampler.py`)
- `LTXVAddGuideAdvanced` / `LTXVAddGuideAdvancedAttention` (参考 `guide.py`)

## Task 6: 后端 - 新增 SaveVideoWithAudio 输出节点

新增 `SaveVideoWithAudio` 节点:
- 输入: video(VIDEO), audio(AUDIO), filename_prefix(STRING), format(COMBO: mp4/webm)
- 输出: VIDEO (is_output_node=true)
- 使用 ffmpeg 将视频帧和音频合并编码
- 输出结果包含视频文件路径和音频文件路径，供前端时间轴使用

## Task 7: 后端 - 在 register_all 中注册所有新节点

更新 `builtin_nodes.rs` 的 `register_all()` 函数，调用所有新增的 `register_xxx` 函数。

## Task 8: 前端 - 音视频时间轴编辑器组件

在 `comfy-ui/src/components/` 下新增时间轴编辑器:

**8a. 新建 `timeline/AudioVideoTimeline.tsx`**
- 双轨道时间轴：视频轨（显示关键帧缩略图）+ 音频轨（显示波形）
- 拖拽调整音频/视频轨道的起始偏移量
- 时间标尺和播放指针
- 缩放/平移控制
- 裁剪区间手柄（左右边界）

**8b. 新建 `timeline/WaveformVisualizer.tsx`**
- 使用 Web Audio API 或 canvas 绘制音频波形
- 支持缩放级别自适应

**8c. 新建 `timeline/TimelineTypes.ts`**
- 定义 `TimelineTrack`, `TimelineClip`, `TimelineState` 等类型

## Task 9: 前端 - 集成到 SaveVideoWithAudio 节点

在 `ComfyNode.tsx` 中:
- 识别 `SaveVideoWithAudio` 节点类型
- 渲染 `AudioVideoTimeline` 组件，替代简单的视频/音频预览
- 将时间轴偏移量作为节点参数传递到后端

## Task 10: 前端 - 更新类型定义和 Store

- 在 `types/api.ts` 中新增 `GUIDER_PARAMETERS` 等 IoType
- 在 `store/workflow.ts` 中新增 `outputVideos` 状态（存储含音频的视频输出）
- 在 `nodeColors.ts` 中为新增类型添加颜色

## Task 11: 验证与测试

- 确保所有新节点在 `/object_info` API 中正确返回
- 确保前端可以正确添加和连接新节点
- 验证时间轴编辑器的拖拽交互和数据传递
