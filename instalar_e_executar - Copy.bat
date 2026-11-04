@echo off
chcp 65001 >nul
title DAX Economia - Instalador e Inicializador Automatico
echo ============================================================================
echo   DAX ECONOMIA - PLATAFORMA DESKTOP [RUST EGUI + PYTHON DATA ENGINE]
echo ============================================================================
echo.

cd /d "%~dp0"

:: 1. Verifica se o Python esta instalado
where python >nul 2>nul
if errorlevel 1 goto ERRO_PYTHON

:: 2. Cria o ambiente virtual isolado .venv caso ainda nao exista
if exist ".venv\Scripts\python.exe" goto INSTALAR_PACOTES
echo [1/3] Criando ambiente virtual isolado do Python [.venv]...
python -m venv .venv
if errorlevel 1 goto ERRO_VENV

:INSTALAR_PACOTES
echo [2/3] Verificando e instalando bibliotecas de dados [pandas, yfinance, fredapi, plotly]...
".venv\Scripts\python.exe" -m pip install --upgrade pip >nul 2>nul
".venv\Scripts\pip.exe" install -r requirements.txt

:: 3. Verifica se o compilador Rust [cargo] esta disponivel no PATH ou na pasta padrao do rustup
where cargo >nul 2>nul
if not errorlevel 1 goto RODAR_RUST

if not exist "%USERPROFILE%\.cargo\bin\cargo.exe" goto SEM_RUST
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
goto RODAR_RUST

:SEM_RUST
echo.
echo [AVISO] Compilador Rust [cargo] ainda nao encontrado no sistema.
echo Para abrir a janela Desktop em Rust, instale o Rust em: https://rustup.rs/
echo.
echo Enquanto isso, geraremos e abriremos agora o Relatorio Interativo no seu navegador!
".venv\Scripts\python.exe" -m python_engine.cli export_chart --ticker PETR4 --period 1y --format plotly
for /f "delims=" %%F in ('dir /b /s "data\exports\relatorio_PETR4_1y.html" 2^>nul') do start "" "%%F"
pause
exit /b 0

:RODAR_RUST
echo [3/3] Compilando e iniciando a interface nativa Windows 11 em Rust [egui]...
taskkill /f /im dax_economia_desktop.exe >nul 2>nul
cargo run --release
pause
exit /b 0

:ERRO_PYTHON
echo [ERRO] Python nao encontrado no sistema!
echo Por favor, instale o Python 3.11+ em: https://www.python.org/downloads/
echo IMPORTANTE: Marque a opcao "Add python.exe to PATH" durante a instalacao.
pause
exit /b 1

:ERRO_VENV
echo [ERRO] Nao foi possivel criar o ambiente virtual .venv.
pause
exit /b 1
