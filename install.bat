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
rem Pass only the known-safe flags accepted by 'earplugger install'.
rem Quotes are preserved so multi-word values (e.g. --device "RODE NT-USB")
rem and domain users (e.g. --user "DOMAIN\User") pass intact to earplugger.
set "EXTRA_ARGS="
:parse_args
if "%~1"=="" goto run_install
if /i "%~1"=="--device" (
    if "%~2"=="" (
        echo [-] Missing value for --device
        pause
        exit /b 1
    )
    set "EXTRA_ARGS=%EXTRA_ARGS% --device "%~2""
    shift & shift & goto parse_args
)
if /i "%~1"=="--delay-ms" (
    if "%~2"=="" (
        echo [-] Missing value for --delay-ms
        pause
        exit /b 1
    )
    set "EXTRA_ARGS=%EXTRA_ARGS% --delay-ms %~2"
    shift & shift & goto parse_args
)
if /i "%~1"=="--user" (
    if "%~2"=="" (
        echo [-] Missing value for --user
        pause
        exit /b 1
    )
    set "EXTRA_ARGS=%EXTRA_ARGS% --user "%~2""
    shift & shift & goto parse_args
)
echo [-] Unknown argument: %~1
echo     Accepted: --device "NAME", --delay-ms MS, --user "USERNAME"
pause
exit /b 1

:run_install
"%EXE_PATH%" install%EXTRA_ARGS%
if %ERRORLEVEL% NEQ 0 (
    echo [-] Installation failed with exit code %ERRORLEVEL%.
    pause
    exit /b %ERRORLEVEL%
)

echo [+] Installation complete.
pause
