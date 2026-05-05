#!/bin/bash

set -e

PROJECT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "========================================="
echo "  ComfyUI-Rust 启动脚本"
echo "========================================="
echo ""

# 检查是否已有服务在运行
if lsof -i :8188 >/dev/null 2>&1; then
    echo "⚠️  端口 8188 已被占用，正在停止旧服务..."
    pkill -f "comfy-server" 2>/dev/null || true
    sleep 1
fi

if lsof -i :3000 >/dev/null 2>&1; then
    echo "⚠️  端口 3000 已被占用，正在停止旧服务..."
    pkill -f "vite.*--port 3000" 2>/dev/null || true
    sleep 1
fi

echo "1/2 启动 Rust 后端服务器 (端口 8188)..."
cd "$PROJECT_DIR"

# 检查 OpenCV + CUDA 编译依赖
if [ ! -f /usr/lib/llvm-18/lib/libclang.so ] && [ ! -f /usr/lib/llvm-15/lib/libclang.so ]; then
    echo "  检测到缺少 libclang-dev (OpenCV Rust 绑定编译所需)"
    echo "  正在尝试安装 libclang-dev..."
    sudo apt-get install -y libclang-dev 2>/dev/null || {
        echo "  ⚠️  安装 libclang-dev 失败，将回退到普通 ControlNet (无 OpenCV 加速)"
        USE_OPENCV=false
    }
fi

SD_CPP_DIR=""
for dir in "$PROJECT_DIR/cpp/stable-diffusion-cpp" "$PROJECT_DIR/cpp/stable-diffusion.cpp"; do
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
        CARGO_FEATURES="local-ffi,$CONTROLNET_FEATURE"
        echo "  使用 FFI + CLI 后端 (预编译库已就绪) + ControlNet ($CONTROLNET_FEATURE)"
    else
        CARGO_FEATURES="local-build,$CONTROLNET_FEATURE"
        echo "  预编译库未找到，将自动编译 stable-diffusion-cpp (首次编译较慢) + ControlNet ($CONTROLNET_FEATURE)..."
    fi
else
    CONTROLNET_FEATURE="controlnet-opencv"
    if [ "${USE_OPENCV:-true}" = "false" ]; then
        CONTROLNET_FEATURE="controlnet"
    fi
    CARGO_FEATURES="local,$CONTROLNET_FEATURE"
    echo "  stable-diffusion-cpp 未找到，使用 CLI 后端 (需要 sd-cli 可执行文件) + ControlNet ($CONTROLNET_FEATURE)"
fi

cargo run -p comfy-api --features "$CARGO_FEATURES" &
SERVER_PID=$!
echo "  ✓ 后端 PID: $SERVER_PID"

echo ""
echo "等待后端启动..."
sleep 3

echo ""
echo "2/2 启动前端开发服务器 (端口 3000)..."
cd "$PROJECT_DIR/comfy-ui"
npx vite --port 3000 &
VITE_PID=$!
echo "  ✓ 前端 PID: $VITE_PID"

echo ""
echo "========================================="
echo "  服务已启动"
echo "========================================="
echo "  前端: http://localhost:3000"
echo "  后端: http://127.0.0.1:8188"
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
