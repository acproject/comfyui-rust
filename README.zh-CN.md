# ComfyUI-Rust

[English](README.md) | **简体中文**

ComfyUI 的 Rust 原生重实现：一个基于节点的可视化工作流编辑器，支持 AI 图像、视频、音频与 3D 生成。后端使用 Axum，前端使用 React + React Flow；通过 FFI 集成 [stable-diffusion.cpp](https://github.com/leejet/stable-diffusion.cpp) 进行本地推理，并为更新的模型族提供 Python（diffusers/transformers）兜底；同时内置 **MCP 服务**，让主流 AI IDE 可以辅助生成可控的生成流程。

## 特性

- **节点式工作流编辑器** —— 拖拽式可视化图编辑，内置 100+ 节点
- **多模态生成** —— 文生图、文生视频、图生视频、音视频联合生成、文生音乐、3D 高斯溅射
- **本地推理** —— 经典 SD 模型通过 stable-diffusion.cpp FFI 本地运行，无需 Python
- **Python 兜底后端** —— 新架构（Bernini-R、MiniMax-H3、MiniMax-Music3、TripoSplat）通过
  `py/flash_attn_v100/comfy_fallback` 下的一次性脚本在 `venv-cu128` 中运行
- **面向 AI IDE 的 MCP 服务** —— 内置 [Model Context Protocol](https://modelcontextprotocol.io)
  端点（`/mcp` Streamable HTTP）及独立 stdio 二进制；可在 Cursor、Trae、Claude Code 等 IDE 中
  发现节点/模型、组装/校验/提交工作流并跟踪执行
- **多模型支持** —— SD1.5、SDXL、SD3、Flux、Wan2.1、LTX-2.3、TripoSplat、Bernini-R、MiniMax-H3、MiniMax-Music3
- **WebSocket 实时通信** —— 实时执行进度、队列管理与流式输出
- **ControlNet** —— 可选 OpenCV 加速的 ControlNet 预处理
- **LLM 集成** —— 内置 LLM 文本生成与工作流辅助 Agent
- **提示词中继时间轴** —— 视频生成的逐帧提示词调度

## 架构

```
comfyui-rust/
├── crates/
│   ├── comfy-core/          # DAG 执行引擎、工作流图、类型系统
│   ├── comfy-inference/     # 推理后端（FFI/CLI/Python/Remote）、参数、媒体类型
│   ├── comfy-executor/      # 节点注册表、100+ 内置节点、执行上下文
│   ├── comfy-api/           # REST/WebSocket API 服务（Axum）、配置、队列、/mcp 挂载
│   └── comfy-mcp/           # MCP 服务：13 个工作流工具、stdio 二进制 + HTTP service
├── comfy-ui/                # React 前端（Vite + React Flow + Zustand）
├── cpp/                     # stable-diffusion.cpp（C++ 推理库，可选）
├── py/flash_attn_v100/
│   ├── venv-cu128/          # 兜底脚本使用的 Python 环境（torch 2.8 cu128 / diffusers）
│   └── comfy_fallback/      # bernini_generate.py / h3_generate.py / h3_context_ir.py /
│                            # music3_generate.py / triposplat_generate.py / img_generate.py
├── models/                  # 模型文件（checkpoints、VAE、text encoder 等）
├── output/                  # 生成的图片、视频、音频及保存的工作流
├── config/                  # 配置（config.json）
├── start.sh                 # 启动后端 + 前端（开发模式）
├── stop.sh                  # 停止所有服务（含 comfy-mcp stdio）
└── build.sh                 # 仅 release 编译 Rust 工作区（绝不编译 C++/前端）
```

### Crate 概览

| Crate | 说明 |
|-------|------|
| `comfy-core` | 核心 DAG 引擎：建图、拓扑排序、类型检查、工作流校验 |
| `comfy-inference` | 推理后端：`LocalBackend`（FFI）、`CliBackend`（子进程）、`PythonBackend`（diffusers 兜底）、`RemoteBackend`（HTTP）；stable-diffusion.cpp FFI 绑定 |
| `comfy-executor` | 节点注册表与 100+ 内置节点实现（加载器、采样器、VAE、ControlNet、LTX、Wan、Bernini、MiniMax 等） |
| `comfy-api` | Axum HTTP/WebSocket 服务、提示词队列、模型管理、配置、SQLite 数据库、`/mcp` 挂载 |
| `comfy-mcp` | 基于现有 REST API 的 MCP 服务：节点/模型发现、建图、校验、提交与状态跟踪 |

## 快速开始

### 前置依赖

- **Rust**（stable，含 cargo）
- **Node.js** 18+ 与 npm
- **C++ 编译器**（gcc/clang）与 **CMake** —— 仅当需要从源码编译 stable-diffusion.cpp
- **libclang-dev** —— OpenCV Rust 绑定所需（可选，用于 ControlNet 加速）
- **FFmpeg** —— 视频编码（MP4/WebM）与音频合流
- **CUDA toolkit**（可选，GPU 加速）
- **Python 3.10+ venv**：`py/flash_attn_v100/venv-cu128`（仅 Python 兜底模型需要）

### 编译 stable-diffusion.cpp（可选，FFI 用）

```bash
cd cpp/stable-diffusion.cpp
mkdir -p build && cd build
cmake .. -DSD_CUDA=ON -DCMAKE_BUILD_TYPE=Release
cmake --build . --config Release -j
```

产物：
- `build/bin/sd-cli` —— CLI 可执行文件
- `build/libstable-diffusion.a` —— 供 FFI 使用的静态库

> 如果该预编译库已存在，推荐直接用 `./build.sh`：它**只编译 Rust 工作区**，
> 有预编译库时链接 FFI（否则回退 CLI），绝不触发 C++ 或前端的子项目编译。

### 启动应用（开发模式）

```bash
./start.sh
```

脚本会：
1. 预先构建 `comfy-mcp` stdio 二进制；
2. 在 **8188** 端口启动 Rust 后端（含 `/mcp` HTTP 端点）；
3. 在 **3022** 端口启动前端开发服务器。

浏览器打开 http://localhost:3022 。

### 停止应用

```bash
./stop.sh
```

### Release 编译（仅 Rust 工作区）

```bash
./build.sh
# 产物：target/release/comfy-server、target/release/comfy-mcp
```

## MCP 服务（AI IDE 接入）

MCP 服务让 AI IDE 操作正在运行的 ComfyUI-Rust，构建**可控**的生成流程：
`list_nodes` / `get_node_schema` → `list_models` → `build_workflow` →
`validate_workflow` → `submit_workflow` → `get_prompt_status` / `get_queue` /
`get_history` / `interrupt`（共 13 个工具，另含工作流模板工具）。

支持两种传输方式：

### 1. Streamable HTTP（推荐）

端点由 `comfy-server` 进程内挂载，无需额外进程：

```
http://127.0.0.1:8188/mcp
```

在 IDE 中填入该 URL 即可。相关环境变量：

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `COMFY_MCP_ENABLED` | `1` | 设为 `0`/`false` 可关闭 `/mcp` 端点 |
| `COMFY_MCP_ALLOWED_HOSTS` | `localhost,127.0.0.1,::1` | 逗号分隔的允许 `Host` 头（防 DNS rebinding） |
| `COMFY_SERVER_URL` | `http://127.0.0.1:<端口>` | MCP 工具调用的 REST 上游地址 |

### 2. stdio

适用于以子进程方式启动 MCP 服务的 IDE：

```bash
target/debug/comfy-mcp --server-url http://127.0.0.1:8188
# （执行 ./build.sh 后可用 target/release/comfy-mcp）
```

IDE 配置示例：

```json
{
  "mcpServers": {
    "comfyui-rust": {
      "url": "http://127.0.0.1:8188/mcp"
    }
  }
}
```

```json
{
  "mcpServers": {
    "comfyui-rust": {
      "command": "/abs/path/to/comfyui-rust/target/release/comfy-mcp",
      "args": ["--server-url", "http://127.0.0.1:8188"]
    }
  }
}
```

## 配置

配置从 `config/config.json` 加载（首次运行自动生成默认配置），也可通过 Web UI 运行时修改并持久化到 SQLite 数据库。

```json
{
  "server": {
    "host": "127.0.0.1",
    "port": 8188
  },
  "models": {
    "base_dir": "models",
    "checkpoints": "checkpoints",
    "vae": "vae",
    "text_encoders": "text_encoders",
    "diffusion_models": "diffusion_models",
    "loras": "loras",
    "controlnet": "controlnet"
  },
  "inference": {
    "backend": "local",
    "n_threads": 0,
    "flash_attn": false,
    "diffusion_flash_attn": false,
    "offload_params_to_cpu": false,
    "enable_mmap": true,
    "sd_cli_path": null
  },
  "output": {
    "dir": "output",
    "format": "png"
  }
}
```

### 推理后端

| 后端 | 配置值 | 说明 |
|------|--------|------|
| **本地 FFI** | `"local"` | 直接 FFI 调用预编译的 stable-diffusion.cpp（推荐，最快） |
| **CLI** | `"cli"` | 以子进程调用 `sd-cli` 可执行文件 |
| **Python** | `"python"` | 纯 Python 后端（HF transformers/diffusers 兜底脚本） |

`FallbackBackend` 还会按模型目录自动路由：自包含 diffusers 仓库（Bernini-R）
与 MiniMax 模块化仓库（`modular_model_index.json` 中 `_class_name=MiniMaxH3*` /
`*Music3*`）始终走 Python —— sd.cpp 目前无法加载它们。

### Feature Flags

```bash
# 预编译库 + FFI 后端（推荐）
cargo run -p comfy-api --features "local-ffi,controlnet-opencv"

# FFI 后端，从源码自动编译 stable-diffusion.cpp（会编译 C++ 子项目）
cargo run -p comfy-api --features "local-build,controlnet"

# 仅 CLI 后端（无 FFI，使用 sd-cli 子进程）
cargo run -p comfy-api --features "local,controlnet"

# 不编译本地推理（仅 Python/远程）
cargo run -p comfy-api
```

| Feature | 说明 |
|---------|------|
| `local` | 启用本地推理支持 |
| `local-ffi` | 链接预编译 stable-diffusion.cpp 静态库（不编译 C++） |
| `local-build` | 通过 build.rs 从源码自动编译 stable-diffusion.cpp |
| `remote` | 远程 HTTP 后端支持 |
| `controlnet` | ControlNet 预处理（使用 `image` + `imageproc`） |
| `controlnet-opencv` | OpenCV 加速的 ControlNet（需要 `libclang-dev`） |
| `flash-attn` | FlashAttention/H3 HTTP bridge 支持 |

## 模型目录结构

```
models/
├── checkpoints/              # 完整模型 checkpoint（.safetensors、.gguf）
├── diffusion_models/         # 纯扩散模型权重（.gguf）
├── vae/                      # VAE 模型
├── text_encoders/            # 文本编码器 / LLM（clip_l、t5xxl、gemma 等）
├── loras/                    # LoRA 适配器
├── controlnet/               # ControlNet 模型
├── clip_vision/              # CLIP vision 模型
├── upscale_models/           # ESRGAN 等放大器
├── llm/                      # LLM 模型（目录形式）
├── triposplat/               # TripoSplat 3D 模型
├── MiniMax-H3/               # MiniMax-H3 全模态音视频（模块化仓库）
├── MiniMax-Music3/           # MiniMax-Music3 文生音乐（模块化仓库）
└── background_removal/       # 背景移除模型
```

可通过 `COMFY_MODELS_DIR` 指定模型根目录（节点的模型下拉扫描直接读取该变量）。

## 支持的模型 / 节点

| 模型 | 类型 | 节点 |
|------|------|------|
| **Stable Diffusion 1.5 / SDXL / SD3** | 图像 | CheckpointLoader、KSampler、VAEDecode |
| **Flux / Qwen-Image / Boogu** | 图像/编辑 | FluxLoader、DualCLIPLoader、KSampler |
| **Wan 2.1** | 视频 | WanLoader、WanVideoSampler、VideoVAEDecode |
| **LTX-2.3** | 视频/音频 | LTXLoader、LTXVideoSampler、VideoVAEDecode、SaveVideoWithAudio |
| **TripoSplat** | 3D | TripoSplatPipeline、Gaussian3DViewer |
| **Bernini-R** | 图像/视频 | BerniniRPipeline、BerniniRVideoPipeline（Python 兜底） |
| **MiniMax-H3** | 视频+音频 | MiniMaxH3ContextIR、MiniMaxH3Pipeline（t2va/i2va/ref2va；Python 兜底） |
| **MiniMax-Music3** | 音乐 | MiniMaxMusic3（lyrics + 结构化说明 → 32kHz 立体声 WAV） |

MiniMax-H3 参数要点：24fps / 32kHz 立体声，时长 4–15 秒，帧数按 `17n+5` 对齐
（123–362），短边 768；CFG-distilled 模型（negative prompt / guidance 无效）。
设置 `MINIMAX_API_KEY`（或 `MINIMAX_TOKEN`）可使用托管 H3-Context-IR API；
未设置时上下文解析降级为本地离线模板。

## API 端点

所有路由均在根路径下（**没有** `/api` 前缀）：

| 方法 | 路径 | 说明 |
|------|------|------|
| `GET` | `/object_info` | 获取全部节点定义与模型列表 |
| `GET` | `/object_info/{class}` | 获取指定节点定义 |
| `POST` | `/prompt` | 提交工作流执行 |
| `GET` | `/history` / `/history/{id}` | 获取执行历史 |
| `GET`/`POST` | `/queue` | 查询队列 / 取消或暂停队列项 |
| `POST` | `/interrupt` | 中断当前执行 |
| `GET` | `/models` | 列出可用模型 |
| `GET` | `/system_stats` | 获取系统统计 |
| `GET` | `/view` | 获取输出图片/视频/音频 |
| `POST` | `/mcp` | MCP Streamable HTTP 端点 |
| `WS` | `/ws` | 实时更新 WebSocket |

## 示例工作流

MiniMax-H3 文生音视频流程（Context-IR → H3；Pipeline 节点会自行保存 mp4 与 sidecar wav）：

```json
{
  "1": {
    "class_type": "MiniMaxH3ContextIR",
    "inputs": { "text_prompt": "a red panda in a misty bamboo forest, cinematic" }
  },
  "2": {
    "class_type": "MiniMaxH3Pipeline",
    "inputs": {
      "prompt": ["1", 1],
      "minimax_h3_model": "MiniMax-H3",
      "mode": "t2va",
      "num_frames": 123,
      "steps": 30,
      "seed": 42
    }
  }
}
```

通过 API 提交：

```bash
curl -X POST http://localhost:8188/prompt \
  -H "Content-Type: application/json" \
  -d '{"prompt": <上面的工作流 JSON>}'
```

## 开发

### 后端（Rust）

```bash
# 仅编译 Rust 工作区（不编译 C++/前端子项目）
./build.sh

# 以 FFI 后端运行
cargo run -p comfy-api --features "local-ffi,controlnet-opencv"

# 单独运行 MCP stdio 服务
cargo run -p comfy-mcp --bin comfy-mcp -- --server-url http://127.0.0.1:8188

# 运行测试
cargo test --workspace
```

### 前端（React）

```bash
cd comfy-ui
npm install
npm run dev      # 开发服务器（start.sh 中使用 3022 端口）
npm run build    # 生产构建到 dist/
```

### 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `COMFY_CONFIG_DIR` | `config` | 配置目录 |
| `COMFY_MODELS_DIR` | `models`（或 USB 模型库） | 模型根目录 |
| `COMFY_OUTPUT_DIR` | `output` | 输出目录 |
| `COMFY_INPUT_DIR` | `input` | 输入素材目录 |
| `COMFY_MCP_ENABLED` | `1` | 启用/关闭 `/mcp` HTTP 端点 |
| `COMFY_MCP_ALLOWED_HOSTS` | `localhost,127.0.0.1,::1` | MCP HTTP 允许的 Host 头 |
| `COMFY_SERVER_URL` | 本机自调地址 | MCP 工具 / stdio 服务调用的 REST 上游 URL |
| `MINIMAX_API_KEY` / `MINIMAX_TOKEN` | — | MiniMax 托管 H3-Context-IR 凭证（未设置时用本地模板降级） |
| `FLASH_ATTN_BRIDGE_URL` | `http://127.0.0.1:8998` | FlashAttention/H3 Python bridge 地址 |
| `SD_CLI_PATH` | — | `sd-cli` 可执行文件路径（CLI 后端） |

## 许可证

本项目集成了 [stable-diffusion.cpp](https://github.com/leejet/stable-diffusion.cpp)（MIT 许可证）。
