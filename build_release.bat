@echo off
set HTTP_PROXY=http://127.0.0.1:33333
set HTTPS_PROXY=http://127.0.0.1:33333
echo Proxy set to http://127.0.0.1:33333

echo ==========================================
echo      Go Game - Build ^& Release Script
echo ==========================================

echo.
echo [1/3] Installing dependencies...
call npm install
if %errorlevel% neq 0 (
    echo Error: npm install failed.
    pause
    exit /b %errorlevel%
)

echo.
echo [2/3] Building Tauri application (Release mode)...
call npm run tauri -- build
if %errorlevel% neq 0 (
    echo Error: Build failed.
    pause
    exit /b %errorlevel%
)

echo.
echo [3/3] Build successful!
echo.
echo You can find the release files in:
echo   src-tauri\target\release\bundle\
echo.
echo The executable is located at:
echo   src-tauri\target\release\go-game.exe
echo.
pause
