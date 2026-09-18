@echo off
setlocal
cd /d "%~dp0"

net session >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    echo [-] Administrator privileges are required.
    echo     Please right-click install.bat and select 'Run as administrator'.
    pause
    exit /b 1
)

set "EXE_PATH="
if exist "%~dp0earplugger.exe" (
    set "EXE_PATH=%~dp0earplugger.exe"
) else if exist "%~dp0target\release\earplugger.exe" (
    set "EXE_PATH=%~dp0target\release\earplugger.exe"
) else (
    echo [*] Release binary not found. Building via cargo...
    cargo build --release
    if %ERRORLEVEL% NEQ 0 (
        echo [!] Build failed. Please ensure Rust and cargo are installed.
        pause
        exit /b %ERRORLEVEL%
    )
    set "EXE_PATH=%~dp0target\release\earplugger.exe"
)

echo [*] Installing Task Scheduler trigger...
"%EXE_PATH%" install %*
if %ERRORLEVEL% NEQ 0 (
    echo [-] Installation failed with exit code %ERRORLEVEL%.
    pause
    exit /b %ERRORLEVEL%
)

echo [+] Installation complete.
pause
