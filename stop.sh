#!/bin/bash

echo "Stopping ComfyUI-Rust services..."

# Stop comfy-server (Rust backend)
echo "Stopping comfy-server..."
pkill -f "comfy-server" 2>/dev/null && echo "  ✓ comfy-server stopped" || echo "  - comfy-server not running"

# Stop vite dev server (frontend)
echo "Stopping vite dev server..."
pkill -f "vite.*--port 3022" 2>/dev/null && echo "  ✓ vite dev server stopped" || echo "  - vite dev server not running"

# Also kill any cargo run processes for comfy-api
pkill -f "cargo run.*comfy-api" 2>/dev/null && echo "  ✓ cargo run comfy-api stopped" || echo "  - cargo run comfy-api not running"

# Stop MCP stdio helper binaries spawned from this project (debug + release).
# Note: /mcp HTTP endpoint is in-process inside comfy-server, so it stops with it.
pkill -f "target/debug/comfy-mcp" 2>/dev/null && echo "  ✓ comfy-mcp (debug stdio) stopped" || echo "  - comfy-mcp (debug stdio) not running"
pkill -f "target/release/comfy-mcp" 2>/dev/null && echo "  ✓ comfy-mcp (release stdio) stopped" || echo "  - comfy-mcp (release stdio) not running"

echo ""
echo "All services stopped."
