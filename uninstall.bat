@echo off
setlocal
cd /d "%~dp0"

if exist ".\target\release\earplugger.exe" (
    .\target\release\earplugger.exe uninstall
) else (
    schtasks /delete /tn "Earplugger_AutoRestart" /f
)
pause
