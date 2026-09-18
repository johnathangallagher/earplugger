@echo off
setlocal
cd /d "%~dp0"

echo Building release binary...
cargo build --release
if %ERRORLEVEL% NEQ 0 (
    echo [!] Build failed. Please ensure Rust and cargo are installed.
    pause
    exit /b %ERRORLEVEL%
)

echo Installing Task Scheduler trigger...
.\target\release\earplugger.exe install
pause
