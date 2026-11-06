# ComfyUI-Rust Windows 停止脚本
# 对应 stop.sh 的 Windows PowerShell 版本
# 用法: .\stop.ps1  或  stop.cmd（推荐，自动绕过执行策略）

$PROJECT_DIR = if ($PSScriptRoot) { $PSScriptRoot } else { Split-Path -Parent $MyInvocation.MyCommand.Path }
if (-not $PROJECT_DIR) { $PROJECT_DIR = Get-Location }

Write-Output "Stopping ComfyUI-Rust services..."

# ---------------------------------------------------------------------------
# 辅助：按 PID 文件停止进程
# ---------------------------------------------------------------------------
function Stop-ByPidFile {
    param([string]$PidFile, [string]$Name)
    if (Test-Path $PidFile) {
        $procId = Get-Content $PidFile -ErrorAction SilentlyContinue
        if ($procId -match '^\d+$') {
            $proc = Get-Process -Id ([int]$procId) -ErrorAction SilentlyContinue
            if ($proc) {
                Stop-Process -Id ([int]$procId) -Force -ErrorAction SilentlyContinue
                Write-Output "  ✓ $Name (PID $procId) stopped"
            } else {
                Write-Output "  - $Name (PID $procId) not running"
            }
        }
        Remove-Item $PidFile -Force -ErrorAction SilentlyContinue
    } else {
        Write-Output "  - $Name pidfile not found"
    }
}

# ---------------------------------------------------------------------------
# 辅助：按端口停止进程（最可靠的兜底方式，直接找到监听端口的 PID）
# ---------------------------------------------------------------------------
function Stop-ByPort {
    param([int]$Port, [string]$Name)
    $conns = netstat -ano | Select-String ":$Port\s"
    if (-not $conns) { return }
    $seen = @{}
    foreach ($line in $conns) {
        $procId = ($line -split '\s+')[-1]
        if ($procId -match '^\d+$' -and [int]$procId -gt 0 -and -not $seen.ContainsKey($procId)) {
            $seen[$procId] = $true
            $proc = Get-Process -Id ([int]$procId) -ErrorAction SilentlyContinue
            if ($proc) {
                Stop-Process -Id ([int]$procId) -Force -ErrorAction SilentlyContinue
                Write-Output "  ✓ $Name (PID $procId) stopped (port $Port)"
            }
        }
    }
}

# ---------------------------------------------------------------------------
# 辅助：按进程名停止（无需命令行匹配时使用，Get-Process 比 CIM 可靠）
# ---------------------------------------------------------------------------
function Stop-ByName {
    param([string[]]$Names, [string]$Label)
    foreach ($name in $Names) {
        $procs = Get-Process -Name $name -ErrorAction SilentlyContinue
        foreach ($p in $procs) {
            Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
            Write-Output "  ✓ $Label (PID $($p.Id)) stopped"
        }
    }
}

# ---------------------------------------------------------------------------
# 辅助：按进程名 + 命令行关键字停止（用于 cargo/rustup/python/node 等）
# Get-CimInstance 可能因 WMI 问题静默失败，失败时跳过但不中断脚本
# ---------------------------------------------------------------------------
function Stop-ByCmdPattern {
    param([string[]]$Names, [string]$Pattern, [string]$Label)
    foreach ($name in $Names) {
        $procs = Get-Process -Name $name -ErrorAction SilentlyContinue
        foreach ($p in $procs) {
            $cmdLine = $null
            try {
                $cmdLine = (Get-CimInstance Win32_Process -Filter "ProcessId=$($p.Id)" -ErrorAction SilentlyContinue).CommandLine
            } catch { }
            if ($cmdLine -and $cmdLine -match $Pattern) {
                Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
                Write-Output "  ✓ $Label (PID $($p.Id)) stopped"
            }
        }
    }
}

# ---------------------------------------------------------------------------
# 1. 停止 comfy-server 后端（端口 8188）
# ---------------------------------------------------------------------------
Write-Output "Stopping comfy-server..."
Stop-ByPidFile -PidFile (Join-Path $PROJECT_DIR ".comfy-server.pid") -Name "comfy-server"

# 直接按进程名停止 comfy-server.exe（cargo run 产生的子进程可能脱离父进程）
Stop-ByName -Names @("comfy-server") -Label "comfy-server"

# cargo / rustup 带 comfy-api 的进程
Stop-ByCmdPattern -Names @("cargo", "rustup") -Pattern "comfy-api" -Label "cargo run comfy-api"

# 端口兜底：8188
Stop-ByPort -Port 8188 -Name "comfy-server"

# ---------------------------------------------------------------------------
# 2. 停止 vite 前端开发服务器（端口 3022）
# ---------------------------------------------------------------------------
Write-Output "Stopping vite dev server..."
Stop-ByPidFile -PidFile (Join-Path $PROJECT_DIR ".vite.pid") -Name "vite dev server"

# vite 通过 npx.cmd 启动，监听 3022 的是 node 进程；按端口最可靠
Stop-ByPort -Port 3022 -Name "vite"

# 兜底：node 进程命令行包含 vite
Stop-ByCmdPattern -Names @("node") -Pattern "vite" -Label "vite node process"

# ---------------------------------------------------------------------------
# 3. 停止 MCP stdio 二进制
# ---------------------------------------------------------------------------
Write-Output "Stopping comfy-mcp..."
Stop-ByName -Names @("comfy-mcp") -Label "comfy-mcp"

# ---------------------------------------------------------------------------
# 4. 停止 WindowsBackend 自动拉起的 ComfyUI (python main.py)
# ---------------------------------------------------------------------------
Write-Output "Stopping ComfyUI (auto-started by WindowsBackend)..."
Stop-ByCmdPattern -Names @("python") -Pattern "main\.py" -Label "ComfyUI python"

Write-Output ""
Write-Output "All services stopped."
