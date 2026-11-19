@echo off
chcp 65001 >nul
title Abrindo DAX Economia...
cd /d "%~dp0"

:: 1. Se o ambiente Python [.venv] ja existir, avanca direto
if exist ".venv\Scripts\python.exe" goto VERIFICAR_EXE
echo Configurando motor Python pela primeira vez...
python -m venv .venv
".venv\Scripts\pip.exe" install -r requirements.txt

:VERIFICAR_EXE
:: 2. Sincroniza o executavel mais recente v0.3 e abre instantaneamente!
if exist "DAX_Economia_v0.3.exe" copy /y "DAX_Economia_v0.3.exe" "DAX_Economia.exe" >nul 2>nul
if exist "DAX_Economia.exe" goto ABRIR_DIRETO

if not exist "target_portable\release\dax_economia_desktop.exe" goto TENTAR_TARGET_PADRAO
copy /y "target_portable\release\dax_economia_desktop.exe" "DAX_Economia.exe" >nul
goto ABRIR_DIRETO

:TENTAR_TARGET_PADRAO
if not exist "target\release\dax_economia_desktop.exe" goto COMPILAR_PRIMEIRA_VEZ
copy /y "target\release\dax_economia_desktop.exe" "DAX_Economia.exe" >nul
goto ABRIR_DIRETO

:COMPILAR_PRIMEIRA_VEZ
if not exist "%USERPROFILE%\.cargo\bin\cargo.exe" goto RODAR_CARGO_BUILD
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"

:RODAR_CARGO_BUILD
echo Compilando DAX_Economia.exe pela primeira vez...
cargo build --release --target-dir target_portable
copy /y "target_portable\release\dax_economia_desktop.exe" "DAX_Economia.exe" >nul

:ABRIR_DIRETO
start "" "%~dp0DAX_Economia.exe"
exit /b 0
