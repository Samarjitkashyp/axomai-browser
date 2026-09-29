@echo off
echo =======================================================
echo  Building Axomai Browser (Rust Core + C++ Native GUI)
echo =======================================================

echo.
echo [1/3] Building Rust Engine DLL (Release)...
cd native\rust_engine
cargo build --release
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] Rust Engine build failed!
    exit /b %ERRORLEVEL%
)
cd ..\..

echo.
echo [2/3] Copying axomai_engine.dll...
copy /Y "native\rust_engine\target\release\axomai_engine.dll" "native\cpp_gui\axomai_engine.dll"

echo.
echo [3/3] Compiling C++ Win32 GUI App...
cd native\cpp_gui
call "C:\Program Files\Microsoft Visual Studio\18\Community\VC\Auxiliary\Build\vcvars64.bat"
cl /std:c++17 /EHsc /O2 /Fe:axomai_browser.exe main.cpp gdiplus.lib user32.lib gdi32.lib shell32.lib
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] C++ GUI build failed!
    exit /b %ERRORLEVEL%
)
cd ..\..

echo.
echo =======================================================
echo  SUCCESS! Axomai Browser compiled successfully.
echo  Run: .\native\cpp_gui\axomai_browser.exe
echo =======================================================
