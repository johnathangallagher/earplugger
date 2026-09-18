@echo off
setlocal EnableDelayedExpansion
set "RAW_ARG=%~1"
if /i "!RAW_ARG!"=="--help" goto show_help
if /i "!RAW_ARG!"=="-h" goto show_help
if "!RAW_ARG!"=="/?" goto show_help

net session >nul 2>&1
if !ERRORLEVEL! NEQ 0 (
    echo [-] Administrator privileges are required.
    echo     Please right-click install.bat and select 'Run as administrator'.
    pause
    exit /b 1
)

cd /d "%~dp0"

set "EXE_PATH="
if exist "%~dp0earplugger.exe" (
    set "EXE_PATH=%~dp0earplugger.exe"
) else if exist "%~dp0target\release\earplugger.exe" (
    set "EXE_PATH=%~dp0target\release\earplugger.exe"
) else (
    echo [*] Release binary not found. Building via cargo...
    cargo build --release
    if errorlevel 1 (
        echo [!] Build failed. Please ensure Rust and cargo are installed.
        pause
        exit /b 1
    )
    set "EXE_PATH=%~dp0target\release\earplugger.exe"
)

echo [*] Installing Task Scheduler trigger...
rem Parse only the known-safe flags accepted by 'earplugger install'.
set "OPT_DEVICE="
set "OPT_DELAY="
set "OPT_USER="

:parse_args
if "%~1"=="" goto run_install

rem Support --help, -h, /?
set "ARG=%~1"
if /i "!ARG!"=="--help" goto show_help
if /i "!ARG!"=="-h" goto show_help
if "!ARG!"=="/?" goto show_help

rem Support --device <NAME>
if /i "!ARG!"=="--device" (
    if "%~2"=="" (
        echo [-] Missing value for --device
        pause
        exit /b 1
    )
    set "OPT_DEVICE=%~2"
    shift & shift & goto parse_args
)

rem Support --device=<NAME>
if /i "!ARG:~0,9!"=="--device=" (
    set "OPT_DEVICE=!ARG:~9!"
    if "!OPT_DEVICE:~0,1!"=="""" if "!OPT_DEVICE:~-1!"=="""" set "OPT_DEVICE=!OPT_DEVICE:~1,-1!"
    if not defined OPT_DEVICE (
        echo [-] Missing value for --device
        pause
        exit /b 1
    )
    shift & goto parse_args
)

rem Support --delay-ms <MS>
if /i "!ARG!"=="--delay-ms" (
    if "%~2"=="" (
        echo [-] Missing value for --delay-ms
        pause
        exit /b 1
    )
    set "TEST_VAL=%~2"
    for /f "eol=a delims=0123456789" %%A in ("!TEST_VAL!") do (
        echo [-] Invalid numeric value for --delay-ms: "%~2"
        pause
        exit /b 1
    )
    set "OPT_DELAY=%~2"
    shift & shift & goto parse_args
)

rem Support --delay-ms=<MS>
if /i "!ARG:~0,11!"=="--delay-ms=" (
    set "OPT_DELAY=!ARG:~11!"
    if "!OPT_DELAY:~0,1!"=="""" if "!OPT_DELAY:~-1!"=="""" set "OPT_DELAY=!OPT_DELAY:~1,-1!"
    if not defined OPT_DELAY (
        echo [-] Missing value for --delay-ms
        pause
        exit /b 1
    )
    for /f "eol=a delims=0123456789" %%A in ("!OPT_DELAY!") do (
        echo [-] Invalid numeric value for --delay-ms: "!OPT_DELAY!"
        pause
        exit /b 1
    )
    shift & goto parse_args
)

rem Support --user <USERNAME>
if /i "!ARG!"=="--user" (
    if "%~2"=="" (
        echo [-] Missing value for --user
        pause
        exit /b 1
    )
    set "OPT_USER=%~2"
    shift & shift & goto parse_args
)

rem Support --user=<USERNAME>
if /i "!ARG:~0,7!"=="--user=" (
    set "OPT_USER=!ARG:~7!"
    if "!OPT_USER:~0,1!"=="""" if "!OPT_USER:~-1!"=="""" set "OPT_USER=!OPT_USER:~1,-1!"
    if not defined OPT_USER (
        echo [-] Missing value for --user
        pause
        exit /b 1
    )
    shift & goto parse_args
)

echo [-] Unknown argument: "%~1"
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
if defined OPT_DEVICE (
    if defined OPT_DELAY (
        if defined OPT_USER (
            "%EXE_PATH%" install --device "%OPT_DEVICE%" --delay-ms %OPT_DELAY% --user "%OPT_USER%"
        ) else (
            "%EXE_PATH%" install --device "%OPT_DEVICE%" --delay-ms %OPT_DELAY%
        )
    ) else (
        if defined OPT_USER (
            "%EXE_PATH%" install --device "%OPT_DEVICE%" --user "%OPT_USER%"
        ) else (
            "%EXE_PATH%" install --device "%OPT_DEVICE%"
        )
    )
) else (
    if defined OPT_DELAY (
        if defined OPT_USER (
            "%EXE_PATH%" install --delay-ms %OPT_DELAY% --user "%OPT_USER%"
        ) else (
            "%EXE_PATH%" install --delay-ms %OPT_DELAY%
        )
    ) else (
        if defined OPT_USER (
            "%EXE_PATH%" install --user "%OPT_USER%"
        ) else (
            "%EXE_PATH%" install
        )
    )
)
if errorlevel 1 (
    echo [-] Installation failed.
    pause
    exit /b 1
)

echo [+] Installation complete.
pause
exit /b 0
