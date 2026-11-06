@echo off
REM ComfyUI-Rust Windows stop script wrapper
REM Bypasses PowerShell execution policy and calls stop.ps1
REM Usage: double-click or run stop.cmd

setlocal
cd /d "%~dp0"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0stop.ps1"
endlocal
