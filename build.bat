@echo off
setlocal
set "RUST_LOG=debug"
set "FEATURES=%~1"
if "%FEATURES%"=="" (
   set "CMD=cargo run"
) else (
   set "CMD=cargo run --features %FEATURES%"
)
call %CMD%
endlocal
