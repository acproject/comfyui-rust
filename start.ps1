# ComfyUI-Rust Windows 启动脚本
# 对应 start.sh 的 Windows PowerShell 版本
# 用法: .\start.ps1  或  start.cmd（推荐，自动绕过执行策略）

$ErrorActionPreference = "Stop"

# 脚本所在目录即项目根目录（$PSScriptRoot 比 $MyInvocation.MyCommand.Path 更可靠）
$PROJECT_DIR = if ($PSScriptRoot) { $PSScriptRoot } else { Split-Path -Parent $MyInvocation.MyCommand.Path }
if (-not $PROJECT_DIR) { $PROJECT_DIR = Get-Location }
Set-Location $PROJECT_DIR

# ---------------------------------------------------------------------------
# 1. 确保 node / npx 在 PATH 中
# ---------------------------------------------------------------------------
if (-not (Get-Command npx -ErrorAction SilentlyContinue)) {
    # 尝试常见安装位置
    $candidates = @(
        "$env:NVM_SYMLINK",
        "C:\Program Files\nodejs",
        "E:\nodejs",
        "D:\nodejs"
    ) | Where-Object { $_ -and (Test-Path $_) }
    foreach ($dir in $candidates) {
        if (Test-Path (Join-Path $dir "npx.cmd")) {
            $env:PATH = "$dir;$env:PATH"
            break
        }
    }
}
$nodeVer = (node --version 2>$null)
Write-Output "  Node: $nodeVer"
Write-Output "  项目目录: $PROJECT_DIR"

# ---------------------------------------------------------------------------
# 2. 模型库目录
# ---------------------------------------------------------------------------
$defaultComfyModels = "M:\Git-ComfyUI\models"
if (-not $env:COMFY_MODELS_DIR) {
    if (Test-Path $defaultComfyModels) {
        $env:COMFY_MODELS_DIR = $defaultComfyModels
    } else {
        $env:COMFY_MODELS_DIR = Join-Path $PROJECT_DIR "models"
    }
}
Write-Output "  模型库目录: $env:COMFY_MODELS_DIR"

Write-Output ""
Write-Output "========================================="
Write-Output "  ComfyUI-Rust 启动脚本 (Windows)"
Write-Output "========================================="
Write-Output ""

# ---------------------------------------------------------------------------
# 3. 检查端口占用，停止旧服务
# ---------------------------------------------------------------------------
function Stop-ProcessOnPort {
    param([int]$Port, [string]$Name)
    $conns = netstat -ano | Select-String ":$Port\s"
    if ($conns) {
        Write-Output "⚠️  端口 $Port 已被占用，正在停止旧 $Name ..."
        $pids = $conns | ForEach-Object {
            ($_ -split '\s+')[-1]
        } | Sort-Object -Unique
        foreach ($procId in $pids) {
            if ($procId -match '^\d+$') {
                Stop-Process -Id ([int]$procId) -Force -ErrorAction SilentlyContinue
                Write-Output "  已停止 PID $procId"
            }
        }
        Start-Sleep -Seconds 1
    }
}

Stop-ProcessOnPort -Port 8188 -Name "后端服务"
Stop-ProcessOnPort -Port 3022 -Name "前端服务"

# ---------------------------------------------------------------------------
# 4. Cargo features 检测（stable-diffusion.cpp）
# Windows 默认使用 WindowsBackend（ComfyUI HTTP 桥接），无需 sd-cpp。
# 若存在预编译的 stable-diffusion.cpp 库，可启用 local-ffi。
# ---------------------------------------------------------------------------
$SD_CPP_DIR = ""
foreach ($dir in @(
    (Join-Path $PROJECT_DIR "cpp\stable-diffusion.cpp"),
    (Join-Path $PROJECT_DIR "cpp\stable-diffusion-cpp")
)) {
    if (Test-Path $dir) { $SD_CPP_DIR = $dir; break }
}

$CARGO_FEATURES = "flash-attn"
if ($SD_CPP_DIR) {
    $SD_LIB = Join-Path $SD_CPP_DIR "build\libstable-diffusion.a"
    # Windows 上静态库扩展名不同，同时检查
    $SD_LIB_WIN = Join-Path $SD_CPP_DIR "build\stable-diffusion.lib"
    if ((Test-Path $SD_LIB) -or (Test-Path $SD_LIB_WIN)) {
        $CARGO_FEATURES = "local-ffi,controlnet,flash-attn"
        Write-Output "  使用 FFI 后端 (stable-diffusion.cpp 预编译库已就绪) + FlashAttn Bridge"
    } else {
        $CARGO_FEATURES = "local-build,controlnet,flash-attn"
        Write-Output "  stable-diffusion.cpp 未编译，将自动编译 + ControlNet + FlashAttn Bridge..."
    }
} else {
    Write-Output "  使用 WindowsBackend (ComfyUI HTTP 桥接) + FlashAttn Bridge"
}

# ---------------------------------------------------------------------------
# 5. FlashAttn Bridge URL 与 MCP 配置
# ---------------------------------------------------------------------------
if (-not $env:FLASH_ATTN_BRIDGE_URL) {
    $env:FLASH_ATTN_BRIDGE_URL = "http://127.0.0.1:8998"
}
Write-Output "  FlashAttn Bridge URL: $env:FLASH_ATTN_BRIDGE_URL"

if (-not $env:COMFY_MCP_ENABLED) { $env:COMFY_MCP_ENABLED = "1" }
if (-not $env:COMFY_MCP_ALLOWED_HOSTS) {
    $env:COMFY_MCP_ALLOWED_HOSTS = "localhost,127.0.0.1,::1"
}
Write-Output "  MCP HTTP 端点: http://127.0.0.1:8188/mcp (COMFY_MCP_ENABLED=$env:COMFY_MCP_ENABLED)"

# ---------------------------------------------------------------------------
# 6. 预构建 MCP stdio 二进制 和 后端二进制
# ---------------------------------------------------------------------------
# cargo 彩色输出走 stderr，需临时放宽错误处理以免被误判为失败
$prevEAP = $ErrorActionPreference
$ErrorActionPreference = "Continue"

Write-Output "  预构建 MCP stdio 二进制 (comfy-mcp)..."
cargo build -p comfy-mcp 2>&1 | Out-Host

Write-Output "  预构建后端二进制 (comfy-api)..."
cargo build -p comfy-api --features $CARGO_FEATURES 2>&1 | Out-Host

$ErrorActionPreference = $prevEAP

# ---------------------------------------------------------------------------
# 7. 启动 Rust 后端 (端口 8188)
# ---------------------------------------------------------------------------
# 直接运行编译好的二进制，避免 cargo run 产生多级子进程导致 PID 文件
# 记录的是 rustup/cargo 的 PID 而非真实的 comfy-server.exe PID。
Write-Output ""
Write-Output "1/2 启动 Rust 后端服务器 (端口 8188)..."

$serverExe = Join-Path $PROJECT_DIR "target\debug\comfy-server.exe"
$backendLog = Join-Path $PROJECT_DIR "server-runtime.log"
$backendProc = Start-Process -FilePath $serverExe `
    -WorkingDirectory $PROJECT_DIR `
    -RedirectStandardOutput $backendLog `
    -RedirectStandardError (Join-Path $PROJECT_DIR "server-runtime.err.log") `
    -PassThru -WindowStyle Hidden

$SERVER_PID = $backendProc.Id
Set-Content -Path (Join-Path $PROJECT_DIR ".comfy-server.pid") -Value $SERVER_PID
Write-Output "  ✓ 后端 PID: $SERVER_PID"
Write-Output "  ✓ 日志: $backendLog"
Write-Output "  ✓ MCP stdio: $PROJECT_DIR\target\debug\comfy-mcp.exe"

Write-Output ""
Write-Output "等待后端启动..."
Start-Sleep -Seconds 5

# ---------------------------------------------------------------------------
# 8. 启动前端 Vite 开发服务器 (端口 3022)
# ---------------------------------------------------------------------------
Write-Output ""
Write-Output "2/2 启动前端开发服务器 (端口 3022)..."

$uiDir = Join-Path $PROJECT_DIR "comfy-ui"
if (Test-Path (Join-Path $uiDir "node_modules")) {
    # 直接用 node 运行 vite.js，避免 npx 产生子进程导致 PID 文件记录的是
    # npx/cmd 的 PID 而非真实的 node 进程 PID。
    $viteBin = Join-Path $uiDir "node_modules\vite\bin\vite.js"
    if (Test-Path $viteBin) {
        $viteProc = Start-Process -FilePath "node" `
            -ArgumentList @($viteBin, "--port", "3022") `
            -WorkingDirectory $uiDir `
            -PassThru -WindowStyle Hidden
    } else {
        # 兜底：vite 二进制不存在时用 npx
        $viteProc = Start-Process -FilePath "npx" `
            -ArgumentList @("vite", "--port", "3022") `
            -WorkingDirectory $uiDir `
            -PassThru -WindowStyle Hidden
    }
    $VITE_PID = $viteProc.Id
    Set-Content -Path (Join-Path $PROJECT_DIR ".vite.pid") -Value $VITE_PID
    Write-Output "  ✓ 前端 PID: $VITE_PID"
} else {
    Write-Output "  ⚠️  comfy-ui\node_modules 不存在，正在执行 npm install ..."
    Push-Location $uiDir
    npm.cmd install 2>&1 | Out-Host
    Pop-Location
    if (Test-Path (Join-Path $uiDir "node_modules")) {
        Write-Output "  ✓ npm install 完成，重新启动前端..."
        $viteBin = Join-Path $uiDir "node_modules\vite\bin\vite.js"
        if (Test-Path $viteBin) {
            $viteProc = Start-Process -FilePath "node" `
                -ArgumentList @($viteBin, "--port", "3022") `
                -WorkingDirectory $uiDir `
                -PassThru -WindowStyle Hidden
        } else {
            $viteProc = Start-Process -FilePath "npx" `
                -ArgumentList @("vite", "--port", "3022") `
                -WorkingDirectory $uiDir `
                -PassThru -WindowStyle Hidden
        }
        $VITE_PID = $viteProc.Id
        Set-Content -Path (Join-Path $PROJECT_DIR ".vite.pid") -Value $VITE_PID
        Write-Output "  ✓ 前端 PID: $VITE_PID"
    } else {
        Write-Output "  ⚠️  npm install 失败，跳过前端启动"
        $VITE_PID = $null
    }
}

Write-Output ""
Write-Output "========================================="
Write-Output "  服务已启动"
Write-Output "========================================="
Write-Output "  前端    : http://localhost:3022"
Write-Output "  后端    : http://127.0.0.1:8188"
Write-Output "  MCP HTTP: http://127.0.0.1:8188/mcp"
Write-Output "  MCP stdio: target\debug\comfy-mcp.exe"
Write-Output ""
Write-Output "  运行 stop.ps1 (或 stop.cmd) 停止所有服务"
Write-Output "========================================="
Write-Output ""

# ---------------------------------------------------------------------------
# 9. 等待后端进程，退出时清理
# ---------------------------------------------------------------------------
try {
    # 等待后端进程（保持脚本运行）
    $backendProc.WaitForExit()
} finally {
    Write-Output ""
    Write-Output "正在停止服务..."
    if ($SERVER_PID) { Stop-Process -Id $SERVER_PID -Force -ErrorAction SilentlyContinue }
    if ($VITE_PID) { Stop-Process -Id $VITE_PID -Force -ErrorAction SilentlyContinue }
    # 同时停止 WindowsBackend 自动拉起的 ComfyUI
    Get-Process -Name python -ErrorAction SilentlyContinue | Where-Object { $_.Path -match "main\.py" } | Stop-Process -Force -ErrorAction SilentlyContinue
    Write-Output "所有服务已停止"
}
