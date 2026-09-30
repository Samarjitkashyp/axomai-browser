@echo off
echo ===================================================
echo  Compiling & Running Axomai Browser via Rust (Wry)
echo ===================================================
cd rust_desktop
cargo run
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo [INFO] Rust/Cargo not found or build failed.
    echo You can preview the exact UI design in your browser right now by running:
    echo .\preview_ui.bat
)
cd ..
pause
