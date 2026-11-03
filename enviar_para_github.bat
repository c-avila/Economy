@echo off
chcp 65001 >nul
title DAX Economia - Enviar Projeto para o GitHub
echo ============================================================================
echo   DAX ECONOMIA - SINCRONIZACAO E ENVIO PARA O GITHUB
echo ============================================================================
echo.

cd /d "%~dp0"

where git >nul 2>nul
if errorlevel 1 goto ERRO_GIT

:: Verifica se ja existe um repositorio remoto "origin" configurado
git remote get-url origin >nul 2>nul
if not errorlevel 1 goto FAZER_COMMIT_E_PUSH

echo Digite ou cole abaixo o link do seu repositorio no GitHub
echo Exemplo: https://github.com/seu-usuario/dax-economia.git
echo.
set /p REPO_URL="URL do Repositorio GitHub: "
if "%REPO_URL%"=="" goto CANCELADO

git remote add origin "%REPO_URL%"

:FAZER_COMMIT_E_PUSH
echo.
echo [1/3] Adicionando alteracoes ao controle de versao Git...
git add .

echo [2/3] Gravando commit...
git commit -m "Atualiza plataforma DAX Economia (Rust egui + Python Data Engine)" >nul 2>nul

echo [3/3] Enviando para o GitHub [branch main]...
git branch -M main
git push -u origin main
echo.
echo Concluido! Verifique seu repositorio no GitHub.
pause
exit /b 0

:ERRO_GIT
echo [ERRO] O Git nao foi encontrado no sistema.
echo Instale em: https://git-scm.com/download/win
pause
exit /b 1

:CANCELADO
echo Operacao cancelada: nenhuma URL informada.
pause
exit /b 0

