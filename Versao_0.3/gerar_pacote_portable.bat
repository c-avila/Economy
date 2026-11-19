@echo off
chcp 65001 >nul
title DAX Economia - Gerador de Versao Portable [.ZIP]
echo ============================================================================
echo   GERADOR DE PACOTE PORTABLE PARA DISTRIBUICAO - DAX ECONOMIA
echo ============================================================================
echo.

cd /d "%~dp0"

if exist "%USERPROFILE%\.cargo\bin\cargo.exe" set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"

echo [1/4] Compilando executavel otimizado para Windows 11 [cargo build --release]...
cargo build --release --target-dir target_portable
if errorlevel 1 goto ERRO_BUILD

echo [2/4] Montando estrutura da pasta Portable em dist\DAX_Economia_Portable...
if exist "dist\DAX_Economia_Portable" rmdir /s /q "dist\DAX_Economia_Portable"
mkdir "dist\DAX_Economia_Portable"
mkdir "dist\DAX_Economia_Portable\python_engine"
mkdir "dist\DAX_Economia_Portable\config"
mkdir "dist\DAX_Economia_Portable\data"

copy /y "target_portable\release\dax_economia_desktop.exe" "dist\DAX_Economia_Portable\DAX_Economia.exe" >nul
copy /y "target_portable\release\dax_economia_desktop.exe" "DAX_Economia_v0.3.exe" >nul
copy /y "target_portable\release\dax_economia_desktop.exe" "DAX_Economia.exe" >nul
copy /y "python_engine\*.py" "dist\DAX_Economia_Portable\python_engine\" >nul
copy /y "config\user_settings.json" "dist\DAX_Economia_Portable\config\" >nul
copy /y "requirements.txt" "dist\DAX_Economia_Portable\" >nul
copy /y ".env.example" "dist\DAX_Economia_Portable\" >nul
copy /y "README.md" "dist\DAX_Economia_Portable\" >nul
copy /y "DOCUMENTACAO_v0.3.md" "dist\DAX_Economia_Portable\" >nul

echo [3/4] Criando inicializador rapido da versao Portable...
(
echo @echo off
echo chcp 65001 ^>nul
echo title DAX Economia Portable v0.3
echo cd /d "%%~dp0"
echo if exist ".venv\Scripts\python.exe" goto ABRIR_APP
echo echo Preparando motor de dados Python pela primeira vez...
echo python -m venv .venv
echo ".venv\Scripts\pip.exe" install -r requirements.txt
echo :ABRIR_APP
echo start "" "DAX_Economia.exe"
) > "dist\DAX_Economia_Portable\Iniciar_DAX_Economia.bat"

echo [4/4] Compactando arquivo final dist\DAX_Economia_v0.3_Portable_Win11.zip...
if exist "dist\DAX_Economia_v0.3_Portable_Win11.zip" del /f /q "dist\DAX_Economia_v0.3_Portable_Win11.zip"
python -c "import shutil; shutil.make_archive('dist/DAX_Economia_v0.3_Portable_Win11', 'zip', 'dist/DAX_Economia_Portable')"
copy /y "dist\DAX_Economia_v0.3_Portable_Win11.zip" "dist\DAX_Economia_Portable_Win11.zip" >nul
if exist "Versao_0.3\Portable" copy /y "dist\DAX_Economia_v0.3_Portable_Win11.zip" "Versao_0.3\Portable\DAX_Economia_v0.3_Portable_Win11.zip" >nul

echo.
echo ============================================================================
echo   PACOTE PORTABLE v0.3 GERADO COM SUCESSO!
echo   Pasta:   E:\AI\Economia\dist\DAX_Economia_Portable
echo   Arquivo: E:\AI\Economia\dist\DAX_Economia_v0.3_Portable_Win11.zip
echo   (Os pacotes anteriores v0.1 e v0.2 foram mantidos!)
echo ============================================================================
pause
exit /b 0

:ERRO_BUILD
echo [ERRO] Falha ao compilar o executavel release.
pause
exit /b 1

