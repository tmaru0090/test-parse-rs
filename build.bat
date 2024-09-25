@echo off
setlocal
set "RUST_LOG=%~1"
cargo run --features "%~2" "%~3"
endlocal
