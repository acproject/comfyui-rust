#!/bin/bash

set -e

PROJECT_DIR="$(cd "$(dirname "$0")" && pwd)"

# 非交互 shell（如 setsid/nohup 拉起）通常没有 nvm 注入的 PATH，
# 导致 npx 找不到：若 npx 不在 PATH 且存在 nvm node，则补上最新版本的 bin
if ! command -v npx >/dev/null 2>&1 && [ -d "$HOME/.nvm/versions/node" ]; then
    NVM_NODE_BIN="$(ls -d "$HOME"/.nvm/versions/node/*/bin 2>/dev/null | sort -V | tail -1)"
    if [ -n "$NVM_NODE_BIN" ]; then
        export PATH="$NVM_NODE_BIN:$PATH"
    fi
fi

# 模型库目录：外部 USB 模型库优先（存在时），可用 COMFY_MODELS_DIR 覆盖
DEFAULT_USB_MODELS="/home/acproject/usb/comfyui/models"
if [ -z "${COMFY_MODELS_DIR:-}" ]; then
    if [ -d "$DEFAULT_USB_MODELS" ]; then
        export COMFY_MODELS_DIR="$DEFAULT_USB_MODELS"
    else
        export COMFY_MODELS_DIR="$PROJECT_DIR/models"
    fi
fi
echo "  模型库目录: $COMFY_MODELS_DIR"

echo "========================================="
echo "  ComfyUI-Rust 启动脚本"
echo "========================================="
echo ""

# 检查是否已有服务在运行
if lsof -i :8188 >/dev/null 2>&1; then
    echo "⚠️  端口 8188 已被占用，正在停止旧服务..."
    pkill -f "comfy-server" 2>/dev/null || true
    # 停止旧的 MCP stdio 进程（AI IDE 也可能自行拉起该二进制，这里只停本项目启动的）
    pkill -f "target/debug/comfy-mcp" 2>/dev/null || true
    sleep 1
fi

if lsof -i :3022 >/dev/null 2>&1; then
    echo "⚠️  端口 3022 已被占用，正在停止旧服务..."
    pkill -f "vite.*--port 3022" 2>/dev/null || true
    sleep 1
fi

echo "1/2 启动 Rust 后端服务器 (端口 8188)..."
cd "$PROJECT_DIR"

# 检查 OpenCV + CUDA 编译依赖
USE_OPENCV=true
if [ ! -f /usr/lib/llvm-18/lib/libclang.so ] && [ ! -f /usr/lib/llvm-15/lib/libclang.so ]; then
    echo "  检测到缺少 libclang-dev (OpenCV Rust 绑定编译所需)"
    echo "  正在尝试安装 libclang-dev..."
    sudo apt-get install -y libclang-dev 2>/dev/null || {
        echo "  ⚠️  安装 libclang-dev 失败，将回退到普通 ControlNet (无 OpenCV 加速)"
        USE_OPENCV=false
    }
fi

# 检测 OpenCV 与 CUDA 版本兼容性（opencv-rust 绑定要求精确 CUDA 版本匹配）
if [ "$USE_OPENCV" = "true" ]; then
    CUDA_VER=""
    if [ -f /usr/local/cuda/version.json ]; then
        CUDA_VER=$(python3 -c "import json; d=json.load(open('/usr/local/cuda/version.json')); print(d.get('cuda',{}).get('version','').split('.')[0]+'.'+d.get('cuda',{}).get('version','').split('.')[1] if '.' in d.get('cuda',{}).get('version','') else d.get('cuda',{}).get('version',''))" 2>/dev/null || echo "")
    elif [ -f /usr/local/cuda/version.txt ]; then
        CUDA_VER=$(grep -oP 'CUDA Version \K[0-9]+\.[0-9]+' /usr/local/cuda/version.txt 2>/dev/null || echo "")
    fi
    if [ -n "$CUDA_VER" ] && [ -d /usr/local/lib/cmake/opencv4 ]; then
        OPENCV_CUDA_VER=$(grep -roP 'CUDA_VERSION[^0-9]*\K[0-9]+\.[0-9]+' /usr/local/lib/cmake/opencv4/ 2>/dev/null | head -1 || echo "")
        if [ -n "$OPENCV_CUDA_VER" ] && [ "$OPENCV_CUDA_VER" != "$CUDA_VER" ]; then
            echo "  ⚠️  OpenCV 编译时使用 CUDA $OPENCV_CUDA_VER，但当前 CUDA 版本为 $CUDA_VER"
            echo "  ⚠️  opencv-rust 绑定要求精确版本匹配，将回退到无 OpenCV 模式"
            USE_OPENCV=false
        fi
    fi
fi

SD_CPP_DIR=""
for dir in "$PROJECT_DIR/cpp/stable-diffusion.cpp" "$PROJECT_DIR/cpp/stable-diffusion-cpp"; do
    if [ -d "$dir" ]; then
        SD_CPP_DIR="$dir"
        break
    fi
done

if [ -n "$SD_CPP_DIR" ]; then
    SD_CLI="$SD_CPP_DIR/build/bin/sd-cli"
    if [ -f "$SD_CLI" ]; then
        chmod +x "$SD_CLI" 2>/dev/null || true
        xattr -cr "$SD_CLI" 2>/dev/null || true
    fi

    SD_LIB="$SD_CPP_DIR/build/libstable-diffusion.a"
    CONTROLNET_FEATURE="controlnet-opencv"
    if [ "${USE_OPENCV:-true}" = "false" ]; then
        CONTROLNET_FEATURE="controlnet"
    fi
    if [ -f "$SD_LIB" ]; then
        CARGO_FEATURES="local-ffi,$CONTROLNET_FEATURE,flash-attn"
        echo "  使用 FFI + CLI 后端 (预编译库已就绪) + ControlNet ($CONTROLNET_FEATURE) + FlashAttn Bridge"
    else
        CARGO_FEATURES="local-build,$CONTROLNET_FEATURE,flash-attn"
        echo "  预编译库未找到，将自动编译 stable-diffusion-cpp (首次编译较慢) + ControlNet ($CONTROLNET_FEATURE) + FlashAttn Bridge..."
    fi
else
    CONTROLNET_FEATURE="controlnet-opencv"
    if [ "${USE_OPENCV:-true}" = "false" ]; then
        CONTROLNET_FEATURE="controlnet"
    fi
    CARGO_FEATURES="local,$CONTROLNET_FEATURE,flash-attn"
    echo "  stable-diffusion-cpp 未找到，使用 CLI 后端 (需要 sd-cli 可执行文件) + ControlNet ($CONTROLNET_FEATURE) + FlashAttn Bridge"
fi

# FlashAttn Bridge URL 配置
export FLASH_ATTN_BRIDGE_URL="${FLASH_ATTN_BRIDGE_URL:-http://127.0.0.1:8998}"
echo "  FlashAttn Bridge URL: $FLASH_ATTN_BRIDGE_URL"

# MCP 配置（供 AI IDE 接入：Cursor / Trae / Claude Code 等）
# - HTTP 端点 /mcp 随 comfy-server 自动挂载，无需额外进程
# - COMFY_MCP_ENABLED=0 可关闭；COMFY_MCP_ALLOWED_HOSTS 控制允许的 Host（默认仅本机）
export COMFY_MCP_ENABLED="${COMFY_MCP_ENABLED:-1}"
if [ -z "${COMFY_MCP_ALLOWED_HOSTS:-}" ]; then
    export COMFY_MCP_ALLOWED_HOSTS="localhost,127.0.0.1,::1"
fi
echo "  MCP HTTP 端点: http://127.0.0.1:8188/mcp (COMFY_MCP_ENABLED=$COMFY_MCP_ENABLED)"

# 预先构建 MCP stdio 独立二进制（cargo run -p comfy-api 不会构建其它 workspace 成员的 bin）
# AI IDE 通过子进程方式接入 MCP 时使用 target/debug/comfy-mcp
echo "  预构建 MCP stdio 二进制 (comfy-mcp)..."
cargo build -p comfy-mcp

cargo run -p comfy-api --features "$CARGO_FEATURES" &
SERVER_PID=$!
echo "  ✓ 后端 PID: $SERVER_PID"
echo "  ✓ MCP stdio: $PROJECT_DIR/target/debug/comfy-mcp"

echo ""
echo "等待后端启动..."
sleep 3

echo ""
echo "2/2 启动前端开发服务器 (端口 3022)..."
cd "$PROJECT_DIR/comfy-ui"
npx vite --port 3022 &
VITE_PID=$!
echo "  ✓ 前端 PID: $VITE_PID"

echo ""
echo "========================================="
echo "  服务已启动"
echo "========================================="
echo "  前端    : http://localhost:3022"
echo "  后端    : http://127.0.0.1:8188"
echo "  MCP HTTP: http://127.0.0.1:8188/mcp  (AI IDE Streamable HTTP 接入)"
echo "  MCP stdio: target/debug/comfy-mcp    (AI IDE 子进程方式接入)"
echo ""
echo "  按 Ctrl+C 停止所有服务"
echo "========================================="
echo ""

# 捕获退出信号
cleanup() {
    echo ""
    echo "正在停止服务..."
    kill $SERVER_PID 2>/dev/null || true
    kill $VITE_PID 2>/dev/null || true
    echo "所有服务已停止"
    exit 0
}

trap cleanup INT TERM

# 等待后台进程
wait
