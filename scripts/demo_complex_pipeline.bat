@echo off
REM Script para demostrar un pipeline complejo en Windows

echo Ejecutando pipeline complejo: Map -^> Filter -^> Reduce

cargo run --bin client submit-job --name "demo-complex-pipeline" --source-inline "1,10,2,20,1,5,3,15,2,10,4,25,3,20" --stages "reduce_by_key"

echo.
echo Espera unos segundos y luego ejecuta:
echo cargo run --bin client get-progress --job-id ^<JOB_ID^>

