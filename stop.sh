#!/bin/bash

echo "Stopping ComfyUI-Rust services..."

# Stop comfy-server (Rust backend)
echo "Stopping comfy-server..."
pkill -f "comfy-server" 2>/dev/null && echo "  ✓ comfy-server stopped" || echo "  - comfy-server not running"

# Stop vite dev server (frontend)
echo "Stopping vite dev server..."
pkill -f "vite.*--port 3000" 2>/dev/null && echo "  ✓ vite dev server stopped" || echo "  - vite dev server not running"

# Also kill any cargo run processes for comfy-api
pkill -f "cargo run.*comfy-api" 2>/dev/null && echo "  ✓ cargo run comfy-api stopped" || echo "  - cargo run comfy-api not running"

echo ""
echo "All services stopped."
