setlocal
set "FEATURES=%~1"
if "%FEATURES%"=="" (
   set "CMD=cargo run"
) else (
   set "CMD=cargo run --features %FEATURES%"
)
endlocal
call %CMD%
