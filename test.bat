@echo off
echo ===================================================
echo   Running All Axomai Browser Engine Unit Tests
echo ===================================================
cargo test --manifest-path native/rust_engine/Cargo.toml --all-targets
echo [✓] All test suites completed!
pause
