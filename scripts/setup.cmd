@echo off
setlocal
set "SCRIPT_DIR=%~dp0"
set "PS_ARGS="
:parse_args
if "%~1"=="" goto run_setup
if /I "%~1"=="--android" (
  set "PS_ARGS=%PS_ARGS% -Android"
) else if /I "%~1"=="--help" (
  set "PS_ARGS=%PS_ARGS% -Help"
) else (
  set "PS_ARGS=%PS_ARGS% %~1"
)
shift
goto parse_args

:run_setup
powershell -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT_DIR%setup\setup-windows.ps1" %PS_ARGS%
endlocal
