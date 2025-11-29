# Prueba simple del sistema
Write-Host "=== PRUEBA DEL SISTEMA DISTRIBUIDO ===" -ForegroundColor Cyan

# Limpiar procesos anteriores
Write-Host "`nLimpiando procesos anteriores..." -ForegroundColor Yellow
Get-Process | Where-Object {$_.ProcessName -eq "master" -or $_.ProcessName -eq "worker"} | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2

# Iniciar master
Write-Host "`n[1/4] Iniciando master..." -ForegroundColor Green
$masterJob = Start-Job -ScriptBlock {
    Set-Location $using:PWD
    cargo run --release --bin master 2>&1
}
Start-Sleep -Seconds 4

# Iniciar worker
Write-Host "[2/4] Iniciando worker..." -ForegroundColor Green
$workerJob = Start-Job -ScriptBlock {
    Set-Location $using:PWD
    $env:WORKER_PORT = "9001"
    $env:MASTER_URL = "http://127.0.0.1:8080"
    cargo run --release --bin worker 2>&1
}
Start-Sleep -Seconds 4

# Verificar que el master responde
Write-Host "[3/4] Verificando master..." -ForegroundColor Yellow
try {
    $response = Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/v1/workers" -Method Get -TimeoutSec 5
    Write-Host "✓ Master responde correctamente" -ForegroundColor Green
    Write-Host "  Workers registrados: $($response.workers.Count)" -ForegroundColor Cyan
} catch {
    Write-Host "✗ Error al conectar con master: $_" -ForegroundColor Red
    Stop-Job $masterJob, $workerJob
    Remove-Job $masterJob, $workerJob
    exit 1
}

# Enviar job de prueba
Write-Host "[4/4] Enviando job de prueba..." -ForegroundColor Yellow
$jobData = @{
    name = "test_map_add"
    operation = "map_add"
    param = 10
    input = @(1, 2, 3, 4, 5)
} | ConvertTo-Json

try {
    $jobResponse = Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/v1/jobs" `
        -Method Post `
        -Body $jobData `
        -ContentType "application/json" `
        -TimeoutSec 10
    
    Write-Host "✓ Job enviado exitosamente" -ForegroundColor Green
    Write-Host "  Job ID: $($jobResponse.job_id)" -ForegroundColor Cyan
    Write-Host "  Mensaje: $($jobResponse.message)" -ForegroundColor Cyan
    
    $jobId = $jobResponse.job_id
    
    # Esperar procesamiento
    Write-Host "`nEsperando procesamiento (5 segundos)..." -ForegroundColor Yellow
    Start-Sleep -Seconds 5
    
    # Verificar estado
    $status = Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/v1/jobs/$jobId" -Method Get -TimeoutSec 5
    Write-Host "`nEstado del job:" -ForegroundColor Cyan
    Write-Host "  Estado: $($status.status)" -ForegroundColor $(if ($status.status -eq "SUCCEEDED") { "Green" } else { "Yellow" })
    Write-Host "  Progreso: $($status.completed_tasks)/$($status.total_tasks) ($([math]::Round($status.progress_percent, 2))%)" -ForegroundColor Cyan
    
} catch {
    Write-Host "✗ Error al enviar job: $_" -ForegroundColor Red
}

# Limpiar
Write-Host "`nLimpiando procesos..." -ForegroundColor Yellow
Stop-Job $masterJob, $workerJob -ErrorAction SilentlyContinue
Remove-Job $masterJob, $workerJob -ErrorAction SilentlyContinue

Write-Host "`n=== PRUEBA COMPLETADA ===" -ForegroundColor Cyan

