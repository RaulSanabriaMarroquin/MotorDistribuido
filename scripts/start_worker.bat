@echo off
REM Script para iniciar un Worker en Windows
REM Uso: start_worker.bat <worker_number>

set WORKER_NUM=%1
if "%WORKER_NUM%"=="" set WORKER_NUM=1

set /a PORT=8080+%WORKER_NUM%

echo Iniciando Worker %WORKER_NUM% en puerto %PORT%...
set MASTER_URL=http://127.0.0.1:8080
set WORKER_PORT=%PORT%
set WORKER_HOST=127.0.0.1
cargo run --bin worker

