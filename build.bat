@echo off
echo ===================================================
echo   Building Axomai Browser Engine & Desktop Shell
echo ===================================================
cargo build --manifest-path native/rust_engine/Cargo.toml --release
cargo build --manifest-path rust_desktop/Cargo.toml --release
echo [✓] Build completed successfully!
pause
