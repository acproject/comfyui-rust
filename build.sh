#!/bin/bash
#
# build.sh —— 仅编译 ComfyUI-Rust Rust 工作区（release）
#
# 本脚本“不编译任何子项目”：
#   - 不触发 stable-diffusion.cpp 的 C++ 源码编译（绝不使用 local-build feature，
#     该 feature 会通过 build.rs 自动编译 cpp/stable-diffusion.cpp）；
#   - 不构建前端（comfy-ui，需要时请手动 cd comfy-ui && npm run build）。
#
# 后端选择策略：
#   - 检测到预编译的 build/libstable-diffusion.a -> local-ffi（直接链接预编译库）
#   - 否则                                        -> local（仅 CLI 后端，调用 sd-cli）
#
# 产物：
#   target/release/comfy-server   REST/WebSocket 服务（内含 /mcp HTTP 端点）
#   target/release/comfy-mcp      MCP stdio 独立二进制（供 AI IDE 以子进程方式启动）

set -e

PROJECT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$PROJECT_DIR"

echo "========================================="
echo "  ComfyUI-Rust 编译脚本（不编译子项目）"
echo "========================================="

# 模型库目录仅用于运行期，编译期同样导出以保持与 start.sh 一致
DEFAULT_USB_MODELS="/home/acproject/usb/comfyui/models"
if [ -z "${COMFY_MODELS_DIR:-}" ]; then
    if [ -d "$DEFAULT_USB_MODELS" ]; then
        export COMFY_MODELS_DIR="$DEFAULT_USB_MODELS"
    else
        export COMFY_MODELS_DIR="$PROJECT_DIR/models"
    fi
fi

# ---------------------------------------------------------------------------
# ControlNet feature：复用 start.sh 的 OpenCV/libclang 检测逻辑
# ---------------------------------------------------------------------------
USE_OPENCV=true
if [ ! -f /usr/lib/llvm-18/lib/libclang.so ] && [ ! -f /usr/lib/llvm-15/lib/libclang.so ]; then
    echo "  检测到缺少 libclang (OpenCV Rust 绑定编译所需)"
    if command -v apt-get >/dev/null 2>&1; then
        echo "  正在尝试安装 libclang-dev..."
        sudo apt-get install -y libclang-dev 2>/dev/null || {
            echo "  ⚠️  安装失败，回退到普通 ControlNet (无 OpenCV 加速)"
            USE_OPENCV=false
        }
    else
        echo "  ⚠️  无 apt-get，回退到普通 ControlNet (无 OpenCV 加速)"
        USE_OPENCV=false
    fi
fi

CONTROLNET_FEATURE="controlnet-opencv"
if [ "$USE_OPENCV" = "false" ]; then
    CONTROLNET_FEATURE="controlnet"
fi

# OpenCV 与 CUDA 版本精确匹配检测
if [ "$USE_OPENCV" = "true" ] && [ -f /usr/local/cuda/version.json ] && [ -d /usr/local/lib/cmake/opencv4 ]; then
    CUDA_VER=$(python3 -c "import json; d=json.load(open('/usr/local/cuda/version.json')); v=d.get('cuda',{}).get('version',''); print('.'.join(v.split('.')[:2]) if '.' in v else v)" 2>/dev/null || echo "")
    OPENCV_CUDA_VER=$(grep -roP 'CUDA_VERSION[^0-9]*\K[0-9]+\.[0-9]+' /usr/local/lib/cmake/opencv4/ 2>/dev/null | head -1 || echo "")
    if [ -n "$CUDA_VER" ] && [ -n "$OPENCV_CUDA_VER" ] && [ "$OPENCV_CUDA_VER" != "$CUDA_VER" ]; then
        echo "  ⚠️  OpenCV 编译时 CUDA $OPENCV_CUDA_VER 与当前 CUDA $CUDA_VER 不匹配，回退无 OpenCV 模式"
        CONTROLNET_FEATURE="controlnet"
    fi
fi

# ---------------------------------------------------------------------------
# 只在“预编译库已存在”时启用 FFI；绝不启用 local-build（那会编译 C++ 子项目）
# ---------------------------------------------------------------------------
SD_CPP_DIR=""
for dir in "$PROJECT_DIR/cpp/stable-diffusion.cpp" "$PROJECT_DIR/cpp/stable-diffusion-cpp"; do
    if [ -d "$dir" ]; then
        SD_CPP_DIR="$dir"
        break
    fi
done

if [ -n "$SD_CPP_DIR" ] && [ -f "$SD_CPP_DIR/build/libstable-diffusion.a" ]; then
    BACKEND_FEATURE="local-ffi"
    echo "  后端: local-ffi（使用预编译库 $SD_CPP_DIR/build/libstable-diffusion.a，不编译 C++）"
else
    BACKEND_FEATURE="local"
    echo "  后端: local（未发现预编译 FFI 库，仅编译 CLI 后端支持，不编译 C++）"
    echo "        如需 FFI，请先按 README 手动编译 stable-diffusion.cpp 后重新运行本脚本"
fi

# flash-attn 为纯 Rust bridge feature，不涉及 C++ 子项目编译，默认启用
CARGO_FEATURES="$BACKEND_FEATURE,$CONTROLNET_FEATURE,flash-attn"
echo "  Cargo features: $CARGO_FEATURES"
echo ""

# ---------------------------------------------------------------------------
# 编译整个 Rust 工作区（含 comfy-server 与 comfy-mcp 两个二进制）
# 不使用 --workspace 以外的 C++ 构建步骤；前端不在此编译
# ---------------------------------------------------------------------------
cargo build --release --workspace --features "$CARGO_FEATURES"

echo ""
echo "========================================="
echo "  编译完成（未编译任何子项目）"
echo "========================================="
echo "  服务端 : $PROJECT_DIR/target/release/comfy-server"
echo "  MCP    : $PROJECT_DIR/target/release/comfy-mcp"
echo ""
echo "  运行服务端 : ./target/release/comfy-server"
echo "  MCP HTTP  : http://127.0.0.1:8188/mcp（随服务端自动挂载）"
echo "  MCP stdio : ./target/release/comfy-mcp --server-url http://127.0.0.1:8188"
echo "========================================="
