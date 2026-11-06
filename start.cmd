@echo off
REM ComfyUI-Rust Windows start script wrapper
REM Bypasses PowerShell execution policy and calls start.ps1
REM Usage: double-click or run start.cmd

setlocal
cd /d "%~dp0"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0start.ps1"
endlocal
