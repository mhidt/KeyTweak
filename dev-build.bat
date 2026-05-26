@echo off
setlocal

cd /d "V:\Project\KeyTweak"
if errorlevel 1 exit /b %errorlevel%

call npm run build
if errorlevel 1 exit /b %errorlevel%

call npx tauri dev
exit /b %errorlevel%
