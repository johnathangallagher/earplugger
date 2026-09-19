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
if exist "%SCRIPT_DIR%earplugger.exe" set "EXE_PATH=%SCRIPT_DIR%earplugger.exe" & goto run_exe
if exist "%SCRIPT_DIR%..\earplugger.exe" set "EXE_PATH=%SCRIPT_DIR%..\earplugger.exe" & goto run_exe
if exist "%SCRIPT_DIR%..\target\release\earplugger.exe" set "EXE_PATH=%SCRIPT_DIR%..\target\release\earplugger.exe" & goto run_exe

:fallback_uninstall
echo [*] Binary not found. Removing scheduled task directly via schtasks...
"%SystemRoot%\System32\schtasks.exe" /delete /tn "Earplugger_AutoRestart" /f >nul 2>&1
set "SCHTASKS_ERR=%ERRORLEVEL%"

set "DISABLE_CHANNEL=0"
if not "%~1"=="" (
    for %%A in (%*) do (
        if /i "%%~A"=="--disable-channel" set "DISABLE_CHANNEL=1"
    )
)
if "%DISABLE_CHANNEL%"=="1" (
    "%SystemRoot%\System32\wevtutil.exe" sl "Microsoft-Windows-Audio/Operational" /e:false
    if errorlevel 1 (
        echo [-] Failed to disable Microsoft-Windows-Audio/Operational channel.
        pause
        exit /b 1
    )
)
if %SCHTASKS_ERR% NEQ 0 (
    if exist "%SystemRoot%\System32\Tasks\Earplugger_AutoRestart" (
        echo [-] Uninstallation failed. Could not remove scheduled task.
        pause
        exit /b %SCHTASKS_ERR%
    )
)
goto finish

:run_exe
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
echo Usage: %~nx0 [OPTIONS]
echo.
echo Options:
echo   --disable-channel   Also disable Microsoft-Windows-Audio/Operational event channel
echo   --help, -h, /?      Show this help message
exit /b 0
