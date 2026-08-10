# H3 生态超级节点 + FlashAttn 驱动后端 + V100 GEMM Backend 开发计划

> 目标：将 `flash_attn_v100` 项目（Python/CUDA）作为底层推理驱动接入 ComfyUI-Rust，
> 新增 MiniMax-H3 全生态（音视频同步生成）超级节点与 Context-IR 多模态编排 MVP，
> 最后实现 V100 Quantized GEMM Backend 以加速量化推理。

---

## 0. 架构总览

### 0.1 现状

```
ComfyUI-Rust (Rust)                     flash_attn_v100 (Python/CUDA)
─────────────────────                   ─────────────────────────────
comfy-ui (React/XYFlow)                flash_attn_llm/
  └─ 节点面板 / 时间线 / 属性面板          ├─ kernels/linear.py (QuantizedLinear)
comfy-executor                           ├─ quantization/ (INT8/4bit/GPTQ/AWQ)
  ├─ NodeRegistry (class_def + exec_fn)  ├─ models/ (attention/causal_lm)
  ├─ builtin_nodes.rs (80+ 节点)         ├─ engine/ (LLMEngine/scheduler)
  └─ ExecutionContext (resolve/backend)  └─ server/server.py (FastAPI)
comfy-inference
  ├─ InferenceBackend trait             examples/inference_minimax_h3/
  ├─ LocalBackend (FFI → sd.cpp)         └─ quantize_load.py
  ├─ CliBackend (subprocess → sd-cli)       ├─ load_pipeline_4bit_multi()
  └─ RemoteBackend (HTTP)                  ├─ test_generate_t2va()
comfy-api (HTTP/WS server)                 └─ test_generate_ref2va()
  └─ state.rs: create_state() 按 config 选后端
```

### 0.2 目标架构

新增 **FlashAttn Python Bridge**（FastAPI 常驻服务）作为 Rust 与 Python 之间的桥梁，
Rust 侧实现 `FlashAttnBackend`（类比现有 `RemoteBackend`）通过 HTTP 调用。

```
┌──────────────── ComfyUI-Rust ────────────────┐     ┌──────── flash_attn_v100 ────────┐
│ comfy-ui                                     │     │                                  │
│  ├─ NodePanel (H3 超级节点自动出现)           │     │  flash_attn_bridge/ (新增)       │
│  └─ ComfyNode → H3DirectorTimeline (新增)    │     │   ├─ server.py  (FastAPI)        │
│ comfy-executor                               │     │   ├─ context_ir.py (Context-IR)  │
│  ├─ register_h3_nodes() (新增模块)           │ HTTP │   ├─ pipeline_runner.py          │
│  └─ ExecutionContext.backend()               │◄───►│   └─ schemas.py                  │
│ comfy-inference                              │ JSON │  examples/inference_minimax_h3/ │
│  └─ FlashAttnBackend (新增, 实现 trait)      │     │   └─ quantize_load.py (复用)     │
│ comfy-api                                    │     │  flash_attn_llm/                 │
│  └─ create_state(): "flash-attn" → 新后端    │     │   └─ kernels/linear.py (GEMM)    │
└──────────────────────────────────────────────┘     └──────────────────────────────────┘
```

### 0.3 关键设计决策

| 决策点 | 选择 | 理由 |
|--------|------|------|
| Rust↔Python 集成方式 | **HTTP Bridge**（FastAPI 常驻服务） | 模型巨大不能每请求重载；进程隔离避免 GIL 阻塞 Rust；可复用现有 `RemoteBackend` 模式；独立可测；支持 SSE 进度 |
| 不选 FFI/PyO3 | — | GIL 与异步运行时冲突、调试困难、二进制膨胀、CUDA 上下文管理复杂 |
| 不选 CLI 子进程 | — | 每次生成需重新加载 30GB+ 模型，不可接受 |
| Context-IR VLM | Qwen3-VL（或已有 LLM） | 零额外显存（复用已加载的 text_encoder/LLM），符合 project_memory 约定 |
| 超级节点前端 | 内嵌时间线组件（仿 LTXDirector） | 复用 AudioVideoTimeline 基础设施，保持 UX 一致 |
| 节点类型扩展 | 新增 `IoType::H3Context` | 类型安全区分 H3 三字段 prompt 与普通 STRING |

---

## Phase 1: FlashAttn Python Bridge（驱动层基础）

> 优先级：P0（后续所有功能的基础）
> 预计工时：3-4 天

### 1.1 新建 Python Bridge 服务

**新建目录**：`flash_attn_bridge/`（位于项目根目录，与 `flash_attn_llm/` 平级）

```
flash_attn_bridge/
├── __init__.py
├── server.py            # FastAPI 应用 + 启动入口
├── schemas.py           # Pydantic 请求/响应模型
├── pipeline_runner.py   # 封装 load_pipeline_4bit_multi + 生成
├── context_ir.py        # Context-IR MVP（Phase 2 实现）
├── progress.py          # SSE 进度回调桥接
└── config.py            # 环境变量/配置（设备、模型路径、量化）
```

### 1.2 server.py 接口定义

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/health` | 健康检查 + 模型加载状态 |
| POST | `/load` | 预加载 MiniMax-H3 pipeline（异步，返回 job_id） |
| GET | `/load/status/{job_id}` | 加载进度（SSE） |
| POST | `/generate/t2va` | 文生音视频 |
| POST | `/generate/ref2va` | 参考图生音视频 |
| POST | `/context-ir/parse` | Context-IR 多模态解析（Phase 2） |
| GET | `/generate/stream/{job_id}` | SSE 生成进度（step/total、预览帧） |

**关键请求模型**（`schemas.py`）：
```python
class T2VARequest(BaseModel):
    prompt: str                    # 三字段格式或自然语言
    negative_prompt: str = ""
    width: int = 960
    height: int = 544
    num_frames: int = 124          # 必须 17*n+5, 120-360
    fps: int = 24
    num_inference_steps: int = 10
    seed: int = 42
    guidance_scale: float = 7.0
    workflow: str = "t2va"         # t2va | ref2va

class Ref2VARequest(T2VARequest):
    reference_images: list[str]    # base64 或文件路径
    workflow: str = "ref2va"

class GenerationResponse(BaseModel):
    job_id: str
    video_path: str                # 编码后的 mp4 路径
    audio_path: str                # wav 路径
    duration_sec: float
    fps: int
    width: int
    height: int
```

### 1.3 pipeline_runner.py

- 封装 `load_pipeline_4bit_multi()`，全局单例（首次请求触发加载或启动时预加载）。
- 封装 `pipe(...)` 调用，固定 `output_type="np"`, `output=["videos","audio","sampling_rate"]`。
- 生成完成后用 ffmpeg 编码 mp4 + 导出 wav（复用 quantize_load.py 的 `_export()` 逻辑）。
- 进度回调：通过 diffusers callback 机制推送到 SSE 队列。
- 严格遵守 project_memory 中的硬约束：
  - `PYTORCH_CUDA_ALLOC_CONF=expandable_segments:True`
  - text_encoder 用 `@torch.no_grad()`
  - 跨设备 tensor 经 CPU 中转
  - VAE dtype 自动检测转换
  - AdaLN 预计算缓存

### 1.4 启动方式

```bash
# 独立启动（开发/调试）
python -m flash_attn_bridge.server --model-path /path/to/h3 --quantization int8

# 或由 comfy-server 自动拉起（子进程托管，见 1.6）
```

### 1.5 Rust 侧：FlashAttnBackend

**新建文件**：`crates/comfy-inference/src/flash_attn.rs`

```rust
pub struct FlashAttnBackend {
    client: reqwest::Client,
    base_url: String,
    config: FlashAttnConfig,
    // 可选：child process handle（自动拉起 Python 服务）
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FlashAttnConfig {
    pub bridge_url: String,           // 默认 http://127.0.0.1:8190
    pub auto_start: bool,             // 是否自动拉起 Python 进程
    pub python_path: Option<String>,
    pub model_path: Option<String>,
    pub quantization: String,         // int8 | 4bit
    pub transformer_devices: Vec<String>,
    pub vae_device: String,
}
```

实现 `InferenceBackend` trait：
- `supports_video_generation()` → `true`
- `generate_video(params)` → POST `/generate/t2va` 或 `/generate/ref2va`，拉取结果文件，构造 `SdVideo`（含音频元数据）
- `generate_image()` → `Err(BackendNotAvailable)`（H3 不支持纯图）
- 新增 `generate_av()` 方法（返回 video+audio）——通过扩展 trait 或 downcast

**问题**：现有 `InferenceBackend::generate_video` 只返回 `SdVideo`（无音频）。
**方案**：在 `SdVideo` 中新增 `audio: Option<SdAudio>` 字段（见 1.7），保持向后兼容。

### 1.6 后端注册与配置

修改文件：
- `crates/comfy-inference/src/lib.rs`：新增 `pub mod flash_attn;`（feature = "flash-attn"）
- `crates/comfy-inference/Cargo.toml`：新增 `flash-attn = ["remote"]` feature
- `crates/comfy-api/src/state.rs`：`create_state()` 增加 `"flash-attn"` 分支
- `crates/comfy-api/src/config.rs`：`InferenceConfig` 增加 `flash_attn: Option<FlashAttnConfig>`
- `crates/comfy-api/src/bin/comfy-server.rs`：feature gate

后端选择逻辑（`comfy-server.rs`）：
```rust
match config.inference.backend.as_str() {
    "cli" => AppState::with_cli_backend(...),
    "flash-attn" => AppState::with_flash_attn_backend(...),  // 新增
    _ => AppState::with_local_backend(...),
}
```

config.json 示例：
```json
{
  "inference": {
    "backend": "flash-attn",
    "flash_attn": {
      "bridge_url": "http://127.0.0.1:8190",
      "auto_start": true,
      "model_path": "/models/MiniMax-H3",
      "quantization": "int8",
      "transformer_devices": ["cuda:0", "cuda:1"],
      "vae_device": "cuda:0"
    }
  }
}
```

### 1.7 SdVideo 扩展音频支持

修改 `crates/comfy-inference/src/image.rs`：
```rust
#[derive(Debug, Clone)]
pub struct SdAudio {
    pub samples: Vec<f32>,    // PCM interleaved
    pub sample_rate: u32,
    pub channels: u32,
}

pub struct SdVideo {
    pub frames: Vec<SdImage>,
    pub fps: i32,
    pub audio: Option<SdAudio>,   // 新增
}
```
同步更新 Serialize/Deserialize。`SdVideo::encode_with_ffmpeg` 增加音频轨道合并。

### 1.8 验收标准

- [ ] `curl http://127.0.0.1:8190/health` 返回 `{"status":"ok","model_loaded":true}`
- [ ] Rust `FlashAttnBackend::generate_video()` 能调用 bridge 并返回含音频的 `SdVideo`
- [ ] config 设置 `backend: "flash-attn"` 时 comfy-server 自动连接/拉起 bridge
- [ ] SSE 进度能从 Python 传到 Rust（日志可见 step/total）

---

## Phase 2: Context-IR MVP（多模态编排）

> 优先级：P0（用户明确要求先做）
> 预计工时：2-3 天
> 依赖：Phase 1 的 bridge 骨架（但可并行开发，用 mock）

### 2.1 Context-IR 职责

H3-Context-IR 负责将用户的多模态输入（自然语言 + 参考图 + 参考视频/音频描述）
解析为 MiniMax-H3 要求的三字段结构化 prompt：

```
integrated_multimodal_description: <视觉+听觉的统一叙事描述>
overall_soundscape: <环境音/音效层>
non_diegetic_music: <配乐层>
```

### 2.2 实现：`flash_attn_bridge/context_ir.py`

```python
class ContextIRRequest(BaseModel):
    raw_prompt: str
    images: list[str] | None = None        # base64 图片路径
    reference_video_desc: str | None = None
    duration_sec: float = 5.0
    style_hint: str | None = None

class ContextIRResponse(BaseModel):
    integrated_multimodal_description: str
    overall_soundscape: str
    non_diegetic_music: str
    formatted_prompt: str                  # 三字段拼接，直接传给 pipe
    confidence: float

def parse_multimodal(req: ContextIRRequest) -> ContextIRResponse:
    # 1. 若有 images → 用 Qwen3-VL 生成图片描述（复用已加载模型，零额外显存）
    # 2. 构造 LLM prompt，要求输出严格 JSON 三字段
    # 3. 调用 LLM（flash_attn_llm engine 或 diffusers text_encoder）
    # 4. 解析、校验、拼接 formatted_prompt
```

**关键约束**（project_memory）：
- VLM/LLM 推理必须 `@torch.no_grad()`
- 复用已加载的 text_encoder，不单独加载新模型
- 输出字段非空时才拼入 formatted_prompt

### 2.3 Rust 侧：ContextIR 节点

**新建文件**：`crates/comfy-executor/src/h3_nodes.rs`（将 H3 相关节点集中管理）

节点 1：`H3ContextIR`
| 项 | 值 |
|----|-----|
| class_type | `H3ContextIR` |
| display_name | "H3 Context-IR Parser" |
| category | "H3/context" |
| 输入 | `raw_prompt`(STRING, multiline), `images`(IMAGE, optional), `duration`(FLOAT), `style_hint`(STRING, optional) |
| 输出 | `H3_CONTEXT`(新 IoType), `formatted_prompt`(STRING) |
| is_output_node | false |

执行逻辑：
1. 收集输入（图片从上游节点解析为文件路径/base64）
2. 调用 `FlashAttnBackend::context_ir_parse()`（新增 trait 方法或直接 HTTP）
3. 输出 JSON 值 `{"integrated_multimodal_description":..., "overall_soundscape":..., "non_diegetic_music":...}`

### 2.4 IoType 扩展

修改 `crates/comfy-core/src/graph/node.rs`：
```rust
pub enum IoType {
    // ... existing
    H3Context,   // 新增：H3 三字段结构化上下文
}
```
同步 `io_type_str()` → `"H3_CONTEXT"`, `from_io_type_str()` 反向映射。

### 2.5 后端 trait 扩展

在 `InferenceBackend` trait 中增加带默认实现的方法：
```rust
fn context_ir_parse(&self, _req: ContextIrParams) -> InferenceResult<H3Context> {
    Err(InferenceError::BackendNotAvailable("context_ir not supported".into()))
}
```
`FlashAttnBackend` 覆盖实现，POST `/context-ir/parse`。

新增参数类型（`params.rs`）：
```rust
pub struct ContextIrParams {
    pub raw_prompt: String,
    pub images: Vec<SdImage>,
    pub reference_video_desc: Option<String>,
    pub duration_sec: f32,
    pub style_hint: Option<String>,
}
pub struct H3Context {
    pub integrated_multimodal_description: String,
    pub overall_soundscape: String,
    pub non_diegetic_music: String,
}
impl H3Context { pub fn formatted_prompt(&self) -> String { ... } }
```

### 2.6 验收标准

- [ ] `H3ContextIR` 节点出现在节点面板 "H3/context" 分类下
- [ ] 输入自然语言 "一只红狐在雪地里走" → 输出三字段结构化 prompt
- [ ] 输入参考图时，VLM 能描述画面内容并融入三字段
- [ ] 输出可直接连接到 H3 超级节点的 prompt 输入

---

## Phase 3: H3 音视频超级节点（编排层）

> 优先级：P0（用户核心需求）
> 预计工时：4-5 天
> 依赖：Phase 1（后端）、Phase 2（Context-IR，可选依赖）

### 3.1 后端节点设计

在 `crates/comfy-executor/src/h3_nodes.rs` 注册以下节点：

#### 节点 2：`H3Director`（超级节点，核心）

| 项 | 值 |
|----|-----|
| class_type | `H3Director` |
| display_name | "H3 Omni Director" |
| category | "H3/generation" |
| is_output_node | true |
| is_resizable | true（前端内嵌时间线） |

**输入**：
| 名称 | 类型 | 必选 | 说明 |
|------|------|------|------|
| `h3_context` | H3_CONTEXT | 可选 | Context-IR 输出（与 prompt 二选一） |
| `prompt` | STRING | 可选 | 直接三字段 prompt（h3_context 优先） |
| `negative_prompt` | STRING | 可选 | |
| `reference_images` | IMAGE | 可选 | ref2va 参考图（列表） |
| `width` | INT | 必选 | 默认 960 |
| `height` | INT | 必选 | 默认 544 |
| `num_frames` | INT | 必选 | 默认 124，约束 17*n+5 |
| `fps` | INT | 必选 | 默认 24 |
| `steps` | INT | 必选 | 默认 10 |
| `seed` | INT | 必选 | 默认 42 |
| `guidance_scale` | FLOAT | 必选 | 默认 7.0 |
| `workflow` | COMBO | 必选 | t2va / ref2va（自动根据 reference_images 判断） |
| `audio_offset` | FLOAT | 可选 | 音频偏移（秒），时间线联动 |
| `video_offset` | FLOAT | 可选 | 视频偏移（秒），时间线联动 |

**输出**：
| 序号 | 名称 | 类型 |
|------|------|------|
| 0 | `video` | VIDEO |
| 1 | `audio` | AUDIO |
| 2 | `duration` | FLOAT |

**执行逻辑**：
1. 解析 prompt：若 `h3_context` 有连接，用 `H3Context::formatted_prompt()`；否则用 `prompt` 字符串
2. 根据 `reference_images` 是否存在自动设置 workflow（t2va/ref2va）
3. 校验 `num_frames` 满足 `17*n+5` 且 120-360（不满足则自动对齐并 warning）
4. 构造 `VideoGenParams`（扩展支持 audio），调用 `backend.generate_video()`
5. 设置 `NodeOutput::with_ui()` 传递预览帧路径给前端
6. 输出 VIDEO + AUDIO + duration

#### 节点 3：`H3ModelLoader`

| 项 | 值 |
|----|-----|
| class_type | `H3ModelLoader` |
| category | "H3/loaders" |
| 输入 | `model_path`(STRING), `quantization`(COMBO: int8/4bit), `transformer_devices`(STRING), `vae_device`(STRING) |
| 输出 | `H3_MODEL`(新 IoType 或复用 MODEL) |

实际作用：配置 FlashAttnBackend 的模型参数（触发预加载）。输出一个配置 token，供 H3Director 校验。
MVP 阶段可简化：H3Director 直接读全局 config，不强制连接 H3ModelLoader。

#### 节点 4：`H3SaveVideoAudio`（可选，复用 SaveVideoWithAudio）

现有 `SaveVideoWithAudio` 节点已支持视频+音频合并保存，H3Director 输出的 VIDEO/AUDIO
可直接连入该节点。无需重复开发。

### 3.2 节点注册

修改 `crates/comfy-executor/src/lib.rs`：
```rust
pub mod h3_nodes;
pub use h3_nodes::register_h3_nodes;
```

修改 `builtin_nodes.rs::register_builtin_nodes()`：
```rust
h3_nodes::register_h3_nodes(registry);
```

`h3_nodes.rs` 结构：
```rust
pub fn register_h3_nodes(registry: &mut NodeRegistry) {
    register_h3_context_ir(registry);
    register_h3_director(registry);
    register_h3_model_loader(registry);
}
```

### 3.3 前端：H3DirectorTimeline 组件

**新建文件**：`comfy-ui/src/components/timeline/H3DirectorTimeline.tsx`

参考现有 `LtxDirectorTimeline.tsx` 和 `AudioVideoTimeline.tsx`：
- 双轨时间线：Video Track + Audio Track + Music Track（三轨对应 H3 三字段）
- 显示生成的视频预览缩略图和音频波形（复用 `WaveformVisualizer`）
- 可拖拽调整 audio_offset / video_offset
- 显示时长（num_frames / fps）
- 进度条：生成时显示当前 step/total（通过 WebSocket 接收执行进度）

**修改** `comfy-ui/src/components/nodes/ComfyNode.tsx`：
```typescript
const isH3Director = classType === 'H3Director';
// 在 RESIZABLE_NODE_TYPES 中添加 'H3Director'
// 在渲染区添加：
{isH3Director && (
  <H3DirectorTimeline
    videoSource={...}
    audioSource={...}
    duration={num_frames / fps}
    onAudioOffsetChange={...}
    onVideoOffsetChange={...}
    generating={isExecuting}
    progress={...}
  />
)}
```

**修改** `comfy-ui/src/components/nodes/nodeColors.ts`：增加 H3 分类颜色。

节点面板（NodePanel）无需改动——节点从后端 `object_info` 自动发现。

### 3.4 工作流验证

典型工作流连线：
```
[H3ContextIR] --h3_context--> [H3Director] --video--> [SaveVideoWithAudio]
                                 |          --audio--> [SaveVideoWithAudio]
[LoadImage(s)] --images--> [H3ContextIR] (可选)
[LoadImage(s)] --reference_images--> [H3Director] (ref2va 模式)
```

### 3.5 验收标准

- [ ] 节点面板 H3 分类下出现 H3ContextIR、H3Director、H3ModelLoader
- [ ] H3Director 节点可调整大小，内嵌三轨时间线
- [ ] t2va 工作流：文本 → H3Director → SaveVideoWithAudio，生成带音频的 mp4
- [ ] ref2va 工作流：LoadImage → H3Director，参考图生视频
- [ ] Context-IR 输出直连 H3Director，三字段 prompt 正确传递
- [ ] 生成过程中前端显示 step 进度
- [ ] num_frames 自动对齐 17*n+5

---

## Phase 4: V100 Quantized GEMM Backend（性能加速）

> 优先级：P1（用户指定"然后做"）
> 预计工时：5-7 天
> 依赖：无（独立于 comfyui-rust，纯 Python/CUDA 优化）
> 位置：`flash_attn_llm/kernels/` 和 `flash_attn_llm/quantization/`

### 4.1 问题分析

当前 `QuantizedLinear.forward`（`kernels/linear.py` L439-L455）将 INT8 权重反量化至 BF16，
但 V100（sm_70）无原生 BF16 Tensor Core：
- BF16 GEMM 回退 FP32 CUDA Core → ~15 TFLOPS
- FP16 Tensor Core (HMMA) → 125 TFLOPS（8x 提升空间）

### 4.2 架构设计：统一量化 GEMM 后端

```
QuantizedLinear.forward(x)
        │
        ▼
┌─────────────────────────────────┐
│ V100QuantizedGEMM (dispatch)    │
│  ├─ 检测 dtype/head_dim/设备     │
│  └─ 选择后端:                    │
├──────────────┬──────────────────┤
│  FP16 路径    │  FP32 降级路径    │
│ (Tensor Core)│ (CUDA Core)      │
│              │                  │
│ 1.BF16→FP16  │ 小规模/异常时     │
│   分块缩放    │ 直接 torch.mm     │
│ 2.融合反量化  │                  │
│ 3.cublas GEMM│                  │
│   或 WMMA     │                  │
└──────────────┴──────────────────┘
```

### 4.3 任务分解

#### 4.3.1 BF16 Emulation（分块动态缩放）

**新建**：`flash_attn_llm/kernels/bf16_emulation.py`
- `bf16_to_fp16_blocks(tensor, block_size=32)`：按 block 计算 max_abs，动态缩放至 FP16 范围
- 记录每个 block 的 scale，GEMM 后 rescale
- CUDA kernel：`flash_attn_kernel.cu` 中新增 `bf16_emulate_gemm_kernel`
  - 输入 BF16 张量，内部转 FP16 用 HMMA
  - 分块 size 调优（32/64/128）

#### 4.3.2 融合反量化 + GEMM

**修改**：`flash_attn_llm/kernels/linear.py`
- `QuantizedLinear.forward`：反量化目标从 BF16 改为 FP16（V100 路径）
- 新增 `_fused_dequant_gemm_fp16()`：INT8 权重 → FP16（融合 scale）→ cublas GEMM
- 调用 `torch.cublas_sgemm` 或自定义 WMMA kernel

#### 4.3.3 WMMA GEMM Kernel

**修改**：`flash_attn_kernel.cu`
- 参考已有 `wmma_compute_qk`，新增 `wmma_quantized_linear`
- INT8 权重反量化 + FP16 HMMA 融合在一个 kernel
- 支持 block-wise scale

#### 4.3.4 后端自动分发

**新建**：`flash_attn_llm/kernels/gemm_backend.py`
```python
class V100GEMMBackend:
    @staticmethod
    def linear(x: torch.Tensor, weight_q, scale, bias=None):
        if x.device != torch.device('cuda') or x.dim() < 2:
            return F.linear(x, weight_q.dequantize(), bias)  # 降级
        if not _is_v100(x.device):
            return _bf16_native_path(...)  # A100+ 原生 BF16
        # V100 路径
        x_fp16 = bf16_to_fp16_blocks(x) if x.dtype == torch.bfloat16 else x.half()
        w_fp16 = dequant_to_fp16(weight_q, scale)
        out = torch.nn.functional.linear(x_fp16, w_fp16, None)
        out = rescale_blocks(out, ...)
        if bias is not None: out = out + bias
        return out
```

#### 4.3.5 集成与降级策略

- `QuantizedLinear` 自动检测 GPU 算力（sm_70 → V100 路径）
- NaN/Inf 检测：异常时自动回退 FP32 SDPA（符合 project_memory 约定）
- 可通过环境变量 `FA_GEMM_BACKEND=auto|v100_fp16|native_bf16|fp32` 强制选择

### 4.4 测试与基准

- 新建 `tests/test_gemm_backend.py`：正确性测试（对比 FP32 参考）
- 基准：`benchmark.py` 增加 GEMM 对比（BF16 FP32 vs V100 FP16 emulation）
- 目标：INT8 权重 → FP16 GEMM 路径相对当前 BF16→FP32 路径 **3-7x 加速**

### 4.5 验收标准

- [ ] V100 上 QuantizedLinear 走 FP16 Tensor Core 路径（nsight 可见 HMMA 指令）
- [ ] 输出与 FP32 参考数值误差 < 1e-3（RMSE）
- [ ] MiniMax-H3 推理端到端延迟降低 ≥ 2x
- [ ] NaN/Inf 时自动降级，不崩溃
- [ ] A100/H100 上自动走原生 BF16 路径（不受影响）

---

## Phase 5: 集成测试与打磨

> 优先级：P1
> 预计工时：2-3 天

### 5.1 端到端测试

- [ ] comfy-server 以 `backend: "flash-attn"` 启动，自动拉起 Python bridge
- [ ] 完整工作流：H3ContextIR → H3Director → SaveVideoWithAudio
- [ ] ref2va 工作流：LoadImage → H3Director
- [ ] 多节点串联：两个 H3Director 生成不同片段 → 视频拼接（若有节点）
- [ ] 进度回调：WebSocket 推送 step 进度到前端

### 5.2 错误处理

- Python bridge 崩溃 → Rust 后端自动重启 / 返回明确错误
- 模型加载失败 → 前端节点显示错误信息
- 显存不足 → 返回建议（降低分辨率/帧数）
- num_frames 不合法 → 自动对齐 + warning

### 5.3 文档

- [ ] 在 README 或 docs/ 中补充 FlashAttn 后端配置说明
- [ ] H3 节点使用示例工作流 JSON
- [ ] GEMM Backend 环境变量说明

---

## 文件变更清单

### 新建文件

| 文件 | 阶段 | 说明 |
|------|------|------|
| `flash_attn_bridge/__init__.py` | P1 | Python bridge 包 |
| `flash_attn_bridge/server.py` | P1 | FastAPI 服务 |
| `flash_attn_bridge/schemas.py` | P1 | Pydantic 模型 |
| `flash_attn_bridge/pipeline_runner.py` | P1 | pipeline 封装 |
| `flash_attn_bridge/context_ir.py` | P2 | Context-IR MVP |
| `flash_attn_bridge/progress.py` | P1 | SSE 进度 |
| `flash_attn_bridge/config.py` | P1 | 配置 |
| `crates/comfy-inference/src/flash_attn.rs` | P1 | Rust FlashAttnBackend |
| `crates/comfy-executor/src/h3_nodes.rs` | P2/P3 | H3 超级节点 |
| `comfy-ui/src/components/timeline/H3DirectorTimeline.tsx` | P3 | 前端时间线 |
| `flash_attn_llm/kernels/bf16_emulation.py` | P4 | BF16 模拟 |
| `flash_attn_llm/kernels/gemm_backend.py` | P4 | GEMM 分发 |
| `tests/test_gemm_backend.py` | P4 | GEMM 测试 |

### 修改文件

| 文件 | 阶段 | 变更 |
|------|------|------|
| `crates/comfy-inference/src/lib.rs` | P1 | 新增 flash_attn 模块 |
| `crates/comfy-inference/Cargo.toml` | P1 | 新增 flash-attn feature |
| `crates/comfy-inference/src/backend.rs` | P2 | trait 增加 context_ir_parse |
| `crates/comfy-inference/src/params.rs` | P1/P2 | 新增 H3Context/ContextIrParams |
| `crates/comfy-inference/src/image.rs` | P1 | SdVideo 增加 audio 字段 |
| `crates/comfy-core/src/graph/node.rs` | P2 | IoType 新增 H3Context |
| `crates/comfy-executor/src/builtin_nodes.rs` | P3 | 注册 h3_nodes |
| `crates/comfy-executor/src/lib.rs` | P3 | 导出 h3_nodes |
| `crates/comfy-api/src/state.rs` | P1 | with_flash_attn_backend |
| `crates/comfy-api/src/config.rs` | P1 | FlashAttnConfig |
| `crates/comfy-api/src/bin/comfy-server.rs` | P1 | 后端选择分支 |
| `comfy-ui/src/components/nodes/ComfyNode.tsx` | P3 | 内嵌 H3DirectorTimeline |
| `comfy-ui/src/components/nodes/nodeColors.ts` | P3 | H3 分类颜色 |
| `flash_attn_llm/kernels/linear.py` | P4 | 接入 GEMM backend |
| `flash_attn_kernel.cu` | P4 | 新增 WMMA/bf16 emulation kernel |

---

## 开发顺序与里程碑

```
Week 1:
  Day 1-2  ── Phase 1.1-1.4: Python bridge 骨架 + pipeline_runner
  Day 3-4  ── Phase 1.5-1.8: Rust FlashAttnBackend + 配置 + 联调
  Day 5    ── Phase 2.1-2.2: Context-IR Python 实现

Week 2:
  Day 1-2  ── Phase 2.3-2.6: Context-IR Rust 节点 + IoType + trait
  Day 3-5  ── Phase 3.1-3.3: H3Director 后端节点 + 前端时间线

Week 3:
  Day 1-2  ── Phase 3.4-3.5: 端到端联调 + 修复
  Day 3-7  ── Phase 4: GEMM Backend（BF16 emulation + 融合 GEMM + 测试）

Week 4:
  Day 1-3  ── Phase 5: 集成测试 + 错误处理 + 文档
```

**里程碑**：
- M1（Week 1 末）：bridge + FlashAttnBackend 可独立调用生成视频
- M2（Week 2 中）：Context-IR MVP 可用
- M3（Week 2 末）：H3 超级节点完整工作流在 UI 中跑通
- M4（Week 3 末）：GEMM Backend 加速生效，V100 推理 2x+ 提速
- M5（Week 4 中）：全部集成测试通过，可交付

---

## 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| Python bridge 进程崩溃 | 生成中断 | 实现健康检查 + 自动重启；Rust 侧返回明确错误 |
| V100 BF16 emulation 精度损失 | 生成质量下降 | block-wise 缩放 + RMSE 校验；异常自动降级 FP32 |
| 跨设备 tensor 传输数据损坏 | 黑块/NaN | 严格经 CPU 中转（project_memory 硬约束） |
| 显存不足 | OOM | 复用已有 AdaLN 预计算、VAE CPU offload 策略 |
| 前端时间线与节点状态同步 | UX 问题 | 通过 WebSocket 进度事件 + node data 驱动 |
| 模型加载耗时（首次量化） | 启动慢 | 复用量化磁盘缓存（已有 use_cache 机制） |

---

## 备注

- 所有 CUDA 扩展编译必须显式指定 CUDA 12.8（project_memory 硬约束）
- 测试前需征得用户同意（用户电脑正在跑任务，不可中断）
- GEMM Backend 为独立优化，不阻塞节点功能交付
- Context-IR MVP 优先使用规则+轻量 LLM，不追求完美，后续可迭代
