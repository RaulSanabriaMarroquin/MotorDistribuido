@echo off
REM Script para demostrar un caso de uso completo en Windows

echo Caso de uso completo: Analisis de datos distribuido
echo.

REM Asegurar que los datos existen
if not exist scripts\data\sample.csv (
    echo Creando datos de ejemplo...
    call scripts\create_sample_data.bat
)

echo Paso 1: Cargar datos desde CSV
echo Paso 2: Aplicar transformacion (multiplicar por 2)
echo Paso 3: Filtrar valores mayores a 20
echo.

cargo run --bin client submit-job --name "demo-complete-analysis" --source-csv scripts\data\sample.csv --stages "map_mul:2,filter_gt:20"

echo.
echo Esperando 5 segundos para que el job se procese...
timeout /t 5 /nobreak >nul

echo.
echo Consultando progreso del job...
echo (Nota: Si el status es 'stage_complete', el monitor avanzara al siguiente stage automaticamente)
echo (Si el status es 'completed', el job termino exitosamente)
echo.
echo Para consultar el progreso manualmente, usa:
echo cargo run --bin client get-progress --job-id ^<JOB_ID^>

