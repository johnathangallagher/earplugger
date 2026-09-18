@echo off
setlocal
cd /d "%~dp0"

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
    echo [-] Unknown argument: %~1
    echo     Accepted: --disable-channel
    pause
    exit /b 1
)

if exist "%~dp0earplugger.exe" (
    "%~dp0earplugger.exe" uninstall%EXTRA_ARGS%
) else if exist "%~dp0target\release\earplugger.exe" (
    "%~dp0target\release\earplugger.exe" uninstall%EXTRA_ARGS%
) else (
    echo [*] Binary not found. Removing scheduled task directly via schtasks...
    "%SystemRoot%\System32\schtasks.exe" /delete /tn "Earplugger_AutoRestart" /f
    if /i "%~1"=="--disable-channel" (
        "%SystemRoot%\System32\wevtutil.exe" sl "Microsoft-Windows-Audio/Operational" /e:false >nul 2>&1
    )
)

if %ERRORLEVEL% NEQ 0 (
    echo [-] Uninstallation failed or task was not found.
    pause
    exit /b %ERRORLEVEL%
)

echo [+] Uninstallation complete.
pause
