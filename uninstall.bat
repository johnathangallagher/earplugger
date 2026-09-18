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
    echo     Please right-click uninstall.bat and select 'Run as administrator'.
    pause
    exit /b 1
)

set "EXTRA_ARGS="
if /i "%~1"=="--disable-channel" (
    set "EXTRA_ARGS= --disable-channel"
) else if not "%~1"=="" (
    echo [-] Unknown argument: "!ARG!"
    echo     Accepted: --disable-channel
    pause
    exit /b 1
)

if exist "%~dp0earplugger.exe" (
    "%~dp0earplugger.exe" uninstall!EXTRA_ARGS!
    if !ERRORLEVEL! NEQ 0 (
        echo [-] Uninstallation failed or task was not found.
        pause
        exit /b !ERRORLEVEL!
    )
) else if exist "%~dp0target\release\earplugger.exe" (
    "%~dp0target\release\earplugger.exe" uninstall!EXTRA_ARGS!
    if !ERRORLEVEL! NEQ 0 (
        echo [-] Uninstallation failed or task was not found.
        pause
        exit /b !ERRORLEVEL!
    )
) else (
    echo [*] Binary not found. Removing scheduled task directly via schtasks...
    "%SystemRoot%\System32\schtasks.exe" /delete /tn "Earplugger_AutoRestart" /f
    set "SCHTASKS_ERR=!ERRORLEVEL!"
    if /i "%~1"=="--disable-channel" (
        "%SystemRoot%\System32\wevtutil.exe" sl "Microsoft-Windows-Audio/Operational" /e:false >nul 2>&1
    )
    if !SCHTASKS_ERR! NEQ 0 (
        echo [-] Uninstallation failed or task was not found.
        pause
        exit /b !SCHTASKS_ERR!
    )
)

echo [+] Uninstallation complete.
pause
exit /b 0

:show_help
echo Usage: uninstall.bat [OPTIONS]
echo.
echo Options:
echo   --disable-channel   Also disable Microsoft-Windows-Audio/Operational event channel
echo   --help, -h, /?      Show this help message
exit /b 0
