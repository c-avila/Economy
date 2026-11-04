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
echo [1/4] Garantindo a tag da Versao 0.1 (v0.1) no commit inicial...
git rev-parse v0.1 >nul 2>nul
if errorlevel 1 git tag -a v0.1 -m "Versao 0.1 - Release Inicial Congelada" HEAD >nul 2>nul

echo [2/4] Adicionando alteracoes da Versao 0.2 ao controle de versao Git...
git add .

echo [3/4] Gravando commit e tag da Versao 0.2 (v0.2)...
git commit -m "Release v0.2: melhorias abas (b)/(c)/(d), sincronizacao de paises, legendas abaixo dos graficos e layout corrigido" >nul 2>nul
git tag -f -a v0.2 -m "Versao 0.2 - Melhorias Macro, Moedas, Juros, Legendas e Layout"

echo [4/4] Enviando codigo e tags (v0.1 e v0.2) para o GitHub [branch main]...
git branch -M main
git push -u origin main --tags
echo.
echo Concluido! A Versao 0.1 (tag v0.1) e a Versao 0.2 (tag v0.2) foram enviadas ao GitHub.
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

