# ComfyUI-Rust

A Rust-native reimplementation of ComfyUI, providing a node-based visual workflow editor for AI image, video, and 3D generation. Built with Axum (backend) + React + React Flow (frontend), integrating [stable-diffusion.cpp](https://github.com/leejet/stable-diffusion.cpp) for local inference via FFI.

## Features

- **Node-based Workflow Editor** — Drag-and-drop visual graph editor with 100+ built-in nodes
- **Multi-Modal Generation** — Text-to-image, text-to-video, image-to-video, 3D Gaussian splatting
- **Local Inference** — Runs models locally via stable-diffusion.cpp FFI (no Python required)
- **CLI Backend** — Alternative backend using `sd-cli` subprocess
- **Multi-Model Support** — SD1.5, SDXL, SD3, Flux, Wan2.1, LTX-2.3, TripoSplat
- **WebSocket Real-time** — Live execution progress, queue management, and streaming output
- **ControlNet** — Optional OpenCV-accelerated ControlNet preprocessing
- **LLM Integration** — Built-in LLM text generation and AI agent for workflow assistance
- **Prompt Relay Timeline** — Frame-by-frame prompt scheduling for video generation

## Architecture

```
comfyui-rust/
├── crates/
│   ├── comfy-core/          # DAG execution engine, workflow graph, type system
│   ├── comfy-inference/     # Inference backends (FFI/CLI/Remote), model loading, FFI bindings
│   ├── comfy-executor/      # Node registry, 100+ builtin nodes, execution context
│   └── comfy-api/           # REST/WebSocket API server (Axum), config, queue, database
├── comfy-ui/                # React frontend (Vite + React Flow + Zustand)
├── cpp/                     # stable-diffusion.cpp (C++ inference library)
├── models/                  # Model files (checkpoints, VAE, text encoders, etc.)
├── output/                  # Generated images, videos, and saved workflows
├── config/                 # Configuration (config.json)
├── start.sh                 # Start both backend + frontend
└── stop.sh                  # Stop all services
```

### Crate Overview

| Crate | Description |
|-------|-------------|
| `comfy-core` | Core DAG engine: graph building, topological sort, type checking, workflow validation |
| `comfy-inference` | Inference backends: `LocalBackend` (FFI), `CliBackend` (subprocess), `RemoteBackend` (HTTP). FFI bindings to stable-diffusion.cpp |
| `comfy-executor` | Node registry and 100+ builtin node implementations (loaders, samplers, VAE, ControlNet, LTX, Wan, etc.) |
| `comfy-api` | Axum HTTP/WebSocket server, prompt queue, model management, config, SQLite database |

## Quick Start

### Prerequisites

- **Rust** (stable, with cargo)
- **Node.js** 18+ and npm
- **C++ compiler** (gcc/clang) — for building stable-diffusion.cpp
- **CMake** — for building stable-diffusion.cpp
- **libclang-dev** — for OpenCV Rust bindings (optional, ControlNet acceleration)
- **FFmpeg** — for video encoding (MP4/WebM output)
- **CUDA toolkit** (optional, for GPU acceleration)

### Build stable-diffusion.cpp

```bash
cd cpp/stable-diffusion.cpp
mkdir -p build && cd build
cmake .. -DSD_CUDA=ON -DCMAKE_BUILD_TYPE=Release
cmake --build . --config Release -j
```

This produces:
- `build/bin/sd-cli` — CLI executable
- `build/libstable-diffusion.a` — Static library for FFI

### Start the Application

```bash
./start.sh
```

This will:
1. Start the Rust backend on port **8188**
2. Start the frontend dev server on port **3000**

Open http://localhost:3000 in your browser.

### Stop the Application

```bash
./stop.sh
```

## Configuration

Configuration is loaded from `config/config.json` (auto-created on first run with defaults). The config can also be modified at runtime via the web UI and is persisted to a SQLite database.

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

### Inference Backends

| Backend | Config Value | Description |
|---------|-------------|-------------|
| **Local FFI** | `"local"` | Direct FFI calls to stable-diffusion.cpp (recommended, fastest) |
| **CLI** | `"cli"` | Subprocess calls to `sd-cli` executable |

### Feature Flags

Build with different feature combinations:

```bash
# FFI backend with pre-built library (recommended)
cargo run -p comfy-api --features "local-ffi,controlnet-opencv"

# FFI backend, auto-build stable-diffusion.cpp from source
cargo run -p comfy-api --features "local-build,controlnet"

# CLI backend only (no FFI, uses sd-cli subprocess)
cargo run -p comfy-api --features "local,controlnet"

# No local inference (remote backend only)
cargo run -p comfy-api
```

| Feature | Description |
|---------|-------------|
| `local` | Enable local inference support |
| `local-ffi` | FFI bindings to stable-diffusion.cpp (requires pre-built library) |
| `local-build` | Auto-build stable-diffusion.cpp from source via build.rs |
| `remote` | Remote HTTP backend support |
| `controlnet` | ControlNet preprocessing (uses `image` + `imageproc`) |
| `controlnet-opencv` | ControlNet with OpenCV acceleration (requires `libclang-dev`) |

## Model Directory Structure

```
models/
├── checkpoints/              # Full model checkpoints (.safetensors, .gguf)
├── diffusion_models/         # Diffusion-only model weights (.gguf)
├── vae/                      # VAE models
├── text_encoders/            # Text encoders / LLMs (clip_l, t5xxl, gemma, etc.)
├── loras/                    # LoRA adapters
├── controlnet/               # ControlNet models
├── clip_vision/              # CLIP vision models
├── upscale_models/           # ESRGAN and other upscalers
├── llm/                      # LLM models (directory-based)
├── triposplat/               # TripoSplat 3D models
└── background_removal/       # Background removal models
```

## Supported Models

| Model | Type | Nodes |
|-------|------|-------|
| **Stable Diffusion 1.5** | Image | CheckpointLoader, KSampler, VAEDecode |
| **SDXL** | Image | CheckpointLoader, KSampler, VAEDecode |
| **Stable Diffusion 3** | Image | SD3Loader, DualCLIPLoader, KSampler |
| **Flux** | Image | FluxLoader, DualCLIPLoader, KSampler |
| **Wan 2.1** | Video | WanLoader, WanVideoSampler, VideoVAEDecode |
| **LTX-2.3** | Video/Audio | LTXLoader, LTXVideoSampler, VideoVAEDecode, SaveVideoWithAudio |
| **TripoSplat** | 3D | TripoSplatLoader, Gaussian3DViewer |

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/object_info` | Get all node definitions and model lists |
| `GET` | `/api/object_info/{class}` | Get a specific node definition |
| `POST` | `/api/prompt` | Submit a workflow for execution |
| `GET` | `/api/history` | Get execution history |
| `GET` | `/api/queue` | Get queue status |
| `POST` | `/api/queue` | Cancel/pause queue items |
| `GET` | `/api/models` | List available models |
| `POST` | `/api/models/download` | Download a model |
| `GET` | `/api/system_stats` | Get system statistics |
| `WS` | `/ws` | WebSocket for real-time updates |

## Example Workflow

A text-to-video workflow using LTX-2.3:

```json
{
  "1": {
    "class_type": "LTXLoader",
    "inputs": {
      "ckpt_name": "ltx-2.3-22b-dev-Q8_0.gguf",
      "vae_name": "ltx-2.3-22b-dev_video_vae.safetensors",
      "audio_vae_name": "ltx-2.3-22b-dev_audio_vae.safetensors",
      "llm_name": "gemma-3-12b-it-qat-UD-Q4_K_XL.gguf",
      "embeddings_connectors_name": "ltx-2.3-22b-dev_embeddings_connector.safetensors"
    }
  },
  "2": {
    "class_type": "CLIPTextEncode",
    "inputs": { "text": "a lovely cat", "clip": ["1", 1] }
  },
  "3": {
    "class_type": "CLIPTextEncode",
    "inputs": { "text": "worst quality, low quality, blurry", "clip": ["1", 1] }
  },
  "4": {
    "class_type": "LTXVideoSampler",
    "inputs": {
      "model": ["1", 0],
      "positive": ["2", 0],
      "negative": ["3", 0],
      "cfg": 6.0, "width": 1280, "height": 720,
      "video_frames": 33, "steps": 20,
      "sampler_name": "euler", "scheduler": "normal",
      "seed": 42
    }
  },
  "5": {
    "class_type": "VideoVAEDecode",
    "inputs": { "samples": ["4", 0], "vae": ["1", 2], "fps": 24 }
  },
  "6": {
    "class_type": "SaveVideo",
    "inputs": { "video": ["5", 0], "format": "mp4", "fps": 24 }
  }
}
```

Submit via API:

```bash
curl -X POST http://localhost:8188/api/prompt \
  -H "Content-Type: application/json" \
  -d '{"prompt": <workflow_json_above>}'
```

## Development

### Backend (Rust)

```bash
# Build
cargo build --release

# Run with FFI backend
cargo run -p comfy-api --features "local-ffi,controlnet-opencv"

# Run tests
cargo test --workspace
```

### Frontend (React)

```bash
cd comfy-ui
npm install
npm run dev      # Development server (port 3000)
npm run build    # Production build to dist/
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `COMFY_CONFIG_DIR` | `config` | Config directory path |
| `COMFY_OUTPUT_DIR` | `output` | Output directory path |
| `COMFY_INPUT_DIR` | `input` | Input directory path |
| `SD_CLI_PATH` | — | Path to `sd-cli` executable (CLI backend) |

## License

This project integrates [stable-diffusion.cpp](https://github.com/leejet/stable-diffusion.cpp), which is licensed under the MIT License.
