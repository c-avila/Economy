@echo off
chcp 65001 >nul
title Teste do Motor de Dados Python - DAX Economia
cd /d "%~dp0"

set "PY_CMD=python"
if exist ".venv\Scripts\python.exe" set "PY_CMD=.venv\Scripts\python.exe"

echo [1/4] Testando coleta de Acoes [PETR4]...
"%PY_CMD%" -m python_engine.cli stock --ticker PETR4 --period 1mo

echo.
echo [2/4] Testando cotacoes de Moedas e Dolar [DXY=]...
"%PY_CMD%" -m python_engine.cli forex --period 1mo

echo.
echo [3/4] Testando Taxas de Juros [Fed, Selic, BCE, BoE, BoJ]...
"%PY_CMD%" -m python_engine.cli interest --countries BR,US,EU,GB,JP

echo.
echo [4/4] Testando Indicadores Socioeconomicos do Banco Mundial [World Bank]...
"%PY_CMD%" -m python_engine.cli macro --countries BRA,USA,CHN,DEU

echo.
echo Todos os modulos responderam com sucesso!
pause
