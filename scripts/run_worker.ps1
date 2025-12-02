# Script para ejecutar un worker
# Uso: .\scripts\run_worker.ps1 -Port 9000

param(
    [Parameter(Mandatory=$true)]
    [int]$Port
)

$env:WORKER_PORT = $Port.ToString()

Write-Host "Iniciando worker en puerto $Port..." -ForegroundColor Green
cargo run --release --bin worker

