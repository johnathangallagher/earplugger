@echo off
setlocal DisableDelayedExpansion

if "%~1"=="--help" goto show_help
if "%~1"=="-h" goto show_help
if "%~1"=="/?" goto show_help

net session >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    echo [-] Administrator privileges are required.
    echo     Please right-click %~nx0 and select 'Run as administrator'.
    pause
    exit /b 1
)

set "SCRIPT_DIR=%~dp0"

set "EXE_PATH="
if exist "%SCRIPT_DIR%earplugger.exe" (set "EXE_PATH=%SCRIPT_DIR%earplugger.exe" & goto run_exe)
if exist "%SCRIPT_DIR%..\earplugger.exe" (set "EXE_PATH=%SCRIPT_DIR%..\earplugger.exe" & goto run_exe)
if exist "%SCRIPT_DIR%..\target\release\earplugger.exe" (set "EXE_PATH=%SCRIPT_DIR%..\target\release\earplugger.exe" & goto run_exe)

echo [*] Release binary not found. Building via cargo...
cd /d "%SCRIPT_DIR%.."
cargo build --release
if errorlevel 1 (
    echo [!] Build failed. Please ensure Rust and cargo are installed.
    pause
    exit /b 1
)
set "EXE_PATH=%SCRIPT_DIR%..\target\release\earplugger.exe"

:run_exe
echo [*] Installing Task Scheduler trigger...
"%EXE_PATH%" install %*
set "EXIT_CODE=%ERRORLEVEL%"
if %EXIT_CODE% NEQ 0 (
    echo [-] Installation failed.
    pause
    exit /b %EXIT_CODE%
)

echo [+] Installation complete.
pause
exit /b 0

:show_help
echo Usage: %~nx0 [OPTIONS]
echo.
echo Options:
echo   --device "NAME"       Audio device name filter (e.g. "RODE NT-USB")
echo   --delay-ms MS         Millisecond delay for USB handshake (default: 150)
echo   --user "DOMAIN\User"  Target user for scheduled task
echo   --no-wake             Disable triggers on system wake/resume from sleep
echo   --help, -h, /?        Show this help message
exit /b 0
