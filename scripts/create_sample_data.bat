@echo off
REM Script para crear archivos de datos de ejemplo en Windows
REM Usa PowerShell para evitar problemas con caracteres especiales

if not exist scripts\data mkdir scripts\data

REM Crear archivo CSV de ejemplo
echo value > scripts\data\sample.csv
for /L %%i in (1,1,20) do echo %%i >> scripts\data\sample.csv

REM Crear archivo JSONL de ejemplo usando PowerShell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\create_jsonl.ps1

REM Crear archivo plain text de ejemplo
type nul > scripts\data\sample.txt
for /L %%i in (1,1,20) do echo %%i >> scripts\data\sample.txt

echo Archivos de datos creados en scripts\data\
