# ComfyUI-Rust

**English** | [简体中文](README.zh-CN.md)

A Rust-native reimplementation of ComfyUI, providing a node-based visual workflow editor for AI image, video, audio, and 3D generation. Built with Axum (backend) + React + React Flow (frontend), integrating [stable-diffusion.cpp](https://github.com/leejet/stable-diffusion.cpp) for local inference via FFI, with a Python (diffusers/transformers) fallback for newer model families — plus a built-in **MCP server** so mainstream AI IDEs can build controllable generation workflows.

## Features

- **Node-based Workflow Editor** — Drag-and-drop visual graph editor with 100+ built-in nodes
- **Multi-Modal Generation** — Text-to-image, text-to-video, image-to-video, image+audio video, text-to-music, 3D Gaussian splatting
- **Local Inference** — Runs models locally via stable-diffusion.cpp FFI (no Python required for classic SD models)
- **Python Fallback Backend** — Newer architectures (Bernini-R, MiniMax-H3, MiniMax-Music3, TripoSplat) run through one-shot `py/flash_attn_v100/comfy_fallback` scripts inside `venv-cu128`
- **MCP Server for AI IDEs** — Built-in [Model Context Protocol](https://modelcontextprotocol.io) endpoint (Streamable HTTP at `/mcp`) and a standalone stdio binary; discover nodes/models, assemble/validate/submit workflows, and track execution from Cursor, Trae, Claude Code, etc.
- **Multi-Model Support** — SD1.5, SDXL, SD3, Flux, Wan2.1, LTX-2.3, TripoSplat, Bernini-R, MiniMax-H3, MiniMax-Music3
- **WebSocket Real-time** — Live execution progress, queue management, and streaming output
- **ControlNet** — Optional OpenCV-accelerated ControlNet preprocessing
- **LLM Integration** — Built-in LLM text generation and AI agent for workflow assistance
- **Prompt Relay Timeline** — Frame-by-frame prompt scheduling for video generation

## Architecture

```
comfyui-rust/
├── crates/
│   ├── comfy-core/          # DAG execution engine, workflow graph, type system
│   ├── comfy-inference/     # Inference backends (FFI/CLI/Python/Remote), params, media types
│   ├── comfy-executor/      # Node registry, 100+ builtin nodes, execution context
│   ├── comfy-api/           # REST/WebSocket API server (Axum), config, queue, /mcp mount
│   └── comfy-mcp/           # MCP server: 13 workflow tools, stdio binary + HTTP service
├── comfy-ui/                # React frontend (Vite + React Flow + Zustand)
├── cpp/                     # stable-diffusion.cpp (C++ inference library, optional)
├── py/flash_attn_v100/
│   ├── venv-cu128/          # Python env for fallback scripts (torch 2.8 cu128 / diffusers)
│   └── comfy_fallback/      # bernini_generate.py / h3_generate.py / h3_context_ir.py /
│                            # music3_generate.py / triposplat_generate.py / img_generate.py
├── models/                  # Model files (checkpoints, VAE, text encoders, etc.)
├── output/                  # Generated images, videos, audio, and saved workflows
├── config/                  # Configuration (config.json)
├── start.sh                 # Start backend + frontend (dev)
├── stop.sh                  # Stop all services (incl. comfy-mcp stdio)
└── build.sh                 # Release build of the Rust workspace only (never builds C++/frontend)
```

### Crate Overview

| Crate | Description |
|-------|-------------|
| `comfy-core` | Core DAG engine: graph building, topological sort, type checking, workflow validation |
| `comfy-inference` | Inference backends: `LocalBackend` (FFI), `CliBackend` (subprocess), `PythonBackend` (diffusers fallback), `RemoteBackend` (HTTP). FFI bindings to stable-diffusion.cpp |
| `comfy-executor` | Node registry and 100+ builtin node implementations (loaders, samplers, VAE, ControlNet, LTX, Wan, Bernini, MiniMax, etc.) |
| `comfy-api` | Axum HTTP/WebSocket server, prompt queue, model management, config, SQLite database, `/mcp` mount |
| `comfy-mcp` | MCP server over the existing REST API: node/model discovery, graph assembly, validation, submission and status tracking |

## Quick Start

### Prerequisites

- **Rust** (stable, with cargo)
- **Node.js** 18+ and npm
- **C++ compiler** (gcc/clang) and **CMake** — only if you build stable-diffusion.cpp from source
- **libclang-dev** — for OpenCV Rust bindings (optional, ControlNet acceleration)
- **FFmpeg** — for video encoding (MP4/WebM) and audio muxing
- **CUDA toolkit** (optional, for GPU acceleration)
- **Python 3.10+ venv** at `py/flash_attn_v100/venv-cu128` (only for Python-fallback models)

### Build stable-diffusion.cpp (optional, for FFI)

```bash
cd cpp/stable-diffusion.cpp
mkdir -p build && cd build
cmake .. -DSD_CUDA=ON -DCMAKE_BUILD_TYPE=Release
cmake --build . --config Release -j
```

This produces:
- `build/bin/sd-cli` — CLI executable
- `build/libstable-diffusion.a` — Static library for FFI

> Prefer `./build.sh` on a machine where this library already exists: it builds the
> **Rust workspace only**, links the prebuilt FFI library when present (otherwise
> falls back to CLI) and never invokes the C++/frontend sub-builds.

### Start the Application (dev)

```bash
./start.sh
```

This will:
1. Pre-build the `comfy-mcp` stdio binary
2. Start the Rust backend on port **8188** (with the `/mcp` HTTP endpoint)
3. Start the frontend dev server on port **3022**

Open http://localhost:3022 in your browser.

### Stop the Application

```bash
./stop.sh
```

### Release build (Rust workspace only)

```bash
./build.sh
# binaries: target/release/comfy-server, target/release/comfy-mcp
```

## MCP Server (AI IDE integration)

The MCP server lets AI IDEs operate on the running ComfyUI-Rust server to build
**controllable** generation workflows: `list_nodes` / `get_node_schema` →
`list_models` → `build_workflow` → `validate_workflow` → `submit_workflow` →
`get_prompt_status` / `get_queue` / `get_history` / `interrupt` (13 tools total,
plus workflow templates).

Two transports are supported:

### 1. Streamable HTTP (recommended)

The endpoint is mounted in-process by `comfy-server` — no extra process:

```
http://127.0.0.1:8188/mcp
```

Configure your IDE with this URL. Relevant environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `COMFY_MCP_ENABLED` | `1` | Set to `0`/`false` to disable the `/mcp` endpoint |
| `COMFY_MCP_ALLOWED_HOSTS` | `localhost,127.0.0.1,::1` | Comma-separated accepted `Host` headers (DNS-rebinding protection) |
| `COMFY_SERVER_URL` | `http://127.0.0.1:<port>` | REST upstream the MCP tools call |

### 2. stdio

For IDEs that launch MCP servers as subprocesses:

```bash
target/debug/comfy-mcp --server-url http://127.0.0.1:8188
# (or target/release/comfy-mcp after ./build.sh)
```

Example IDE configuration:

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
| **Local FFI** | `"local"` | Direct FFI calls to prebuilt stable-diffusion.cpp (recommended, fastest) |
| **CLI** | `"cli"` | Subprocess calls to `sd-cli` executable |
| **Python** | `"python"` | Python-only backend (HF transformers/diffusers fallback scripts) |

The `FallbackBackend` additionally routes automatically by model directory:
self-contained diffusers repos (Bernini-R) and MiniMax modular repos
(`modular_model_index.json` with `_class_name=MiniMaxH3*` / `*Music3*`) always go
to Python, which sd.cpp cannot load.

### Feature Flags

```bash
# FFI backend with pre-built library (recommended)
cargo run -p comfy-api --features "local-ffi,controlnet-opencv"

# FFI backend, auto-build stable-diffusion.cpp from source (compiles the C++ subproject)
cargo run -p comfy-api --features "local-build,controlnet"

# CLI backend only (no FFI, uses sd-cli subprocess)
cargo run -p comfy-api --features "local,controlnet"

# No local inference (Python/remote only)
cargo run -p comfy-api
```

| Feature | Description |
|---------|-------------|
| `local` | Enable local inference support |
| `local-ffi` | FFI bindings to stable-diffusion.cpp (requires pre-built library, no C++ compile) |
| `local-build` | Auto-build stable-diffusion.cpp from source via build.rs |
| `remote` | Remote HTTP backend support |
| `controlnet` | ControlNet preprocessing (uses `image` + `imageproc`) |
| `controlnet-opencv` | ControlNet with OpenCV acceleration (requires `libclang-dev`) |
| `flash-attn` | FlashAttention/H3 HTTP bridge support |

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
├── MiniMax-H3/               # MiniMax-H3 omni audio-video (modular repo)
├── MiniMax-Music3/           # MiniMax-Music3 text-to-music (modular repo)
└── background_removal/       # Background removal models
```

Set a custom model root with `COMFY_MODELS_DIR` (the model combo scanners read it
directly).

## Supported Models / Nodes

| Model | Type | Nodes |
|-------|------|-------|
| **Stable Diffusion 1.5 / SDXL / SD3** | Image | CheckpointLoader, KSampler, VAEDecode |
| **Flux / Qwen-Image / Boogu** | Image/Edit | FluxLoader, DualCLIPLoader, KSampler |
| **Wan 2.1** | Video | WanLoader, WanVideoSampler, VideoVAEDecode |
| **LTX-2.3** | Video/Audio | LTXLoader, LTXVideoSampler, VideoVAEDecode, SaveVideoWithAudio |
| **TripoSplat** | 3D | TripoSplatPipeline, Gaussian3DViewer |
| **Bernini-R** | Image/Video | BerniniRPipeline, BerniniRVideoPipeline (Python fallback) |
| **MiniMax-H3** | Video+Audio | MiniMaxH3ContextIR, MiniMaxH3Pipeline (t2va/i2va/ref2va; Python fallback) |
| **MiniMax-Music3** | Music | MiniMaxMusic3 (lyrics + structured caption → 32 kHz stereo WAV) |

MiniMax-H3 notes: 24 fps / 32 kHz stereo, 4–15 s clips, frame counts aligned to
`17n+5` (123–362), short edge 768, CFG-distilled (no negative prompt / guidance).
Set `MINIMAX_API_KEY` (or `MINIMAX_TOKEN`) to use the hosted H3-Context-IR API;
without a key the context parser falls back to a local offline template.

## API Endpoints

Routes are served at the root (no `/api` prefix):

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/object_info` | Get all node definitions and model lists |
| `GET` | `/object_info/{class}` | Get a specific node definition |
| `POST` | `/prompt` | Submit a workflow for execution |
| `GET` | `/history` / `/history/{id}` | Get execution history |
| `GET`/`POST` | `/queue` | Get queue status / cancel or pause items |
| `POST` | `/interrupt` | Interrupt current execution |
| `GET` | `/models` | List available models |
| `GET` | `/system_stats` | Get system statistics |
| `GET` | `/view` | Fetch output image/video/audio |
| `POST` | `/mcp` | MCP Streamable HTTP endpoint |
| `WS` | `/ws` | WebSocket for real-time updates |

## Example Workflow

A MiniMax-H3 text-to-audio-video flow (Context-IR → H3 → the pipeline node itself
saves the mp4 and sidecar wav):

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

Submit via API:

```bash
curl -X POST http://localhost:8188/prompt \
  -H "Content-Type: application/json" \
  -d '{"prompt": <workflow_json_above>}'
```

## Development

### Backend (Rust)

```bash
# Build (Rust workspace only, no C++/frontend sub-builds)
./build.sh

# Run with FFI backend
cargo run -p comfy-api --features "local-ffi,controlnet-opencv"

# Run the MCP stdio server standalone
cargo run -p comfy-mcp --bin comfy-mcp -- --server-url http://127.0.0.1:8188

# Run tests
cargo test --workspace
```

### Frontend (React)

```bash
cd comfy-ui
npm install
npm run dev      # Development server (port 3022 via start.sh)
npm run build   # Production build to dist/
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `COMFY_CONFIG_DIR` | `config` | Config directory path |
| `COMFY_MODELS_DIR` | `models` (or USB model library) | Model root directory |
| `COMFY_OUTPUT_DIR` | `output` | Output directory path |
| `COMFY_INPUT_DIR` | `input` | Input directory path |
| `COMFY_MCP_ENABLED` | `1` | Enable/disable the `/mcp` HTTP endpoint |
| `COMFY_MCP_ALLOWED_HOSTS` | `localhost,127.0.0.1,::1` | Allowed Host headers for MCP HTTP |
| `COMFY_SERVER_URL` | self URL | REST upstream URL used by MCP tools / stdio server |
| `MINIMAX_API_KEY` / `MINIMAX_TOKEN` | — | MiniMax hosted H3-Context-IR credentials (local template fallback when unset) |
| `FLASH_ATTN_BRIDGE_URL` | `http://127.0.0.1:8998` | FlashAttention/H3 Python bridge URL |
| `SD_CLI_PATH` | — | Path to `sd-cli` executable (CLI backend) |

## License

This project integrates [stable-diffusion.cpp](https://github.com/leejet/stable-diffusion.cpp), which is licensed under the MIT License.
