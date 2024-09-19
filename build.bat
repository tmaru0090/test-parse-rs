@echo off
setlocal
set "RUST_LOG=%~1"
cargo run "%~2"
endlocal
