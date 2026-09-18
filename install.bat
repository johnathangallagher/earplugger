@echo off
setlocal EnableDelayedExpansion
cd /d "%~dp0"

set "ARG=%~1"
if /i "!ARG!"=="--help" goto show_help
if /i "!ARG!"=="-h" goto show_help
if "!ARG!"=="/?" goto show_help

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
    if !ERRORLEVEL! NEQ 0 (
        echo [!] Build failed. Please ensure Rust and cargo are installed.
        pause
        exit /b !ERRORLEVEL!
    )
    set "EXE_PATH=%~dp0target\release\earplugger.exe"
)

echo [*] Installing Task Scheduler trigger...
rem Pass only the known-safe flags accepted by 'earplugger install'.
rem Quotes are stripped and re-wrapped so multi-word values (e.g. --device="RODE NT-USB")
rem and domain users (e.g. --user="DOMAIN\User") pass intact without double quotes.
set "EXTRA_ARGS="
:parse_args
if "%~1"=="" goto run_install

set "ARG=%~1"

rem Support --help, -h, /?
if /i "!ARG!"=="--help" goto show_help
if /i "!ARG!"=="-h" goto show_help
if "!ARG!"=="/?" goto show_help

rem Support --device=<NAME>
if /i "!ARG:~0,9!"=="--device=" (
    set "VAL=!ARG:~9!"
    set "VAL=!VAL:"=!"
    if "!VAL!"=="" (
        echo [-] Missing value for --device
        pause
        exit /b 1
    )
    set "EXTRA_ARGS=!EXTRA_ARGS! --device "!VAL!""
    shift & goto parse_args
)

rem Support --device <NAME>
if /i "%~1"=="--device" (
    if "%~2"=="" (
        echo [-] Missing value for --device
        pause
        exit /b 1
    )
    set "VAL=%~2"
    set "VAL=!VAL:"=!"
    set "EXTRA_ARGS=!EXTRA_ARGS! --device "!VAL!""
    shift & shift & goto parse_args
)

rem Support --delay-ms=<MS>
if /i "!ARG:~0,11!"=="--delay-ms=" (
    set "VAL=!ARG:~11!"
    set "VAL=!VAL:"=!"
    if "!VAL!"=="" (
        echo [-] Missing value for --delay-ms
        pause
        exit /b 1
    )
    for /f "delims=0123456789" %%A in ("!VAL!") do (
        echo [-] Invalid numeric value for --delay-ms: "!VAL!"
        pause
        exit /b 1
    )
    set "EXTRA_ARGS=!EXTRA_ARGS! --delay-ms !VAL!"
    shift & goto parse_args
)

rem Support --delay-ms <MS>
if /i "%~1"=="--delay-ms" (
    if "%~2"=="" (
        echo [-] Missing value for --delay-ms
        pause
        exit /b 1
    )
    set "VAL=%~2"
    set "VAL=!VAL:"=!"
    for /f "delims=0123456789" %%A in ("!VAL!") do (
        echo [-] Invalid numeric value for --delay-ms: "!VAL!"
        pause
        exit /b 1
    )
    set "EXTRA_ARGS=!EXTRA_ARGS! --delay-ms !VAL!"
    shift & shift & goto parse_args
)

rem Support --user=<USERNAME>
if /i "!ARG:~0,7!"=="--user=" (
    set "VAL=!ARG:~7!"
    set "VAL=!VAL:"=!"
    if "!VAL!"=="" (
        echo [-] Missing value for --user
        pause
        exit /b 1
    )
    set "EXTRA_ARGS=!EXTRA_ARGS! --user "!VAL!""
    shift & goto parse_args
)

rem Support --user <USERNAME>
if /i "%~1"=="--user" (
    if "%~2"=="" (
        echo [-] Missing value for --user
        pause
        exit /b 1
    )
    set "VAL=%~2"
    set "VAL=!VAL:"=!"
    set "EXTRA_ARGS=!EXTRA_ARGS! --user "!VAL!""
    shift & shift & goto parse_args
)

echo [-] Unknown argument: "!ARG!"
echo     Accepted: --device "NAME", --delay-ms MS, --user "USERNAME"
pause
exit /b 1

:show_help
echo Usage: install.bat [OPTIONS]
echo.
echo Options:
echo   --device "NAME"       Audio device name filter (e.g. "RODE NT-USB")
echo   --delay-ms MS         Millisecond delay for USB handshake (default: 150)
echo   --user "DOMAIN\User"  Target user for scheduled task
echo   --help, -h, /?        Show this help message
exit /b 0

:run_install
"%EXE_PATH%" install!EXTRA_ARGS!
if %ERRORLEVEL% NEQ 0 (
    echo [-] Installation failed with exit code %ERRORLEVEL%.
    pause
    exit /b %ERRORLEVEL%
)

echo [+] Installation complete.
pause
