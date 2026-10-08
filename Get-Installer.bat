@echo off
setlocal EnableExtensions
title Craft Launcher - download installer
echo.
echo  Downloads the latest Craft Launcher installer from your GitHub repo and runs it.
echo  Nothing else gets installed on your PC.
echo.

set "REPO="
if exist "%~dp0repo.txt" set /p REPO=<"%~dp0repo.txt"
if defined REPO goto :have_repo
set /p REPO= Enter your GitHub repo as name/repo (example: jane/craft-launcher): 
if not defined REPO goto :fail
> "%~dp0repo.txt" echo %REPO%
:have_repo

set "URL=https://github.com/%REPO%/releases/latest/download/CraftLauncher-Setup.exe"
set "OUT=%TEMP%\CraftLauncher-Setup.exe"
echo Downloading from %URL%
powershell -NoProfile -Command "try { [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; Invoke-WebRequest -Uri $env:URL -OutFile $env:OUT -UseBasicParsing } catch { exit 1 }"
if errorlevel 1 goto :fail_dl
if not exist "%OUT%" goto :fail_dl

echo Starting the installer...
start "" "%OUT%"
exit /b 0

:fail_dl
echo.
echo Couldn't download the installer. Check that:
echo   - the repo name is right (delete repo.txt to re-enter it),
echo   - the repo is PUBLIC, and
echo   - the "Build Windows installer" action has finished and published a Release.
pause
exit /b 1
:fail
echo No repo entered.
pause
exit /b 1
