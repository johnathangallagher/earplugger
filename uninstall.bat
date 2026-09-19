@echo off
setlocal DisableDelayedExpansion

if "%~1"=="--help" goto show_help
if "%~1"=="-h" goto show_help
if "%~1"=="/?" goto show_help

net session >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    echo [-] Administrator privileges are required.
    echo     Please right-click uninstall.bat and select 'Run as administrator'.
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
    echo [*] Binary not found. Removing scheduled task directly via schtasks...
    "%SystemRoot%\System32\schtasks.exe" /delete /tn "Earplugger_AutoRestart" /f
    set "SCHTASKS_ERR=%ERRORLEVEL%"
    if "%~1"=="--disable-channel" (
        "%SystemRoot%\System32\wevtutil.exe" sl "Microsoft-Windows-Audio/Operational" /e:false
        if errorlevel 1 (
            echo [-] Failed to disable Microsoft-Windows-Audio/Operational channel.
            pause
            exit /b 1
        )
    )
    if %SCHTASKS_ERR% NEQ 0 (
        echo [-] Uninstallation failed or task was not found.
        pause
        exit /b %SCHTASKS_ERR%
    )
    goto finish
)

echo [*] Removing Task Scheduler trigger...
"%EXE_PATH%" uninstall %*
set "EXIT_CODE=%ERRORLEVEL%"
if %EXIT_CODE% NEQ 0 (
    echo [-] Uninstallation failed.
    pause
    exit /b %EXIT_CODE%
)

:finish
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
