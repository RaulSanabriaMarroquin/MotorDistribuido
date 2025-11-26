# Script de prueba básica del sistema distribuido
# Prueba: Master -> Worker -> Cliente

Write-Host "=== Prueba del Sistema Distribuido ===" -ForegroundColor Cyan
Write-Host ""

# Limpiar procesos anteriores si existen
Write-Host "Limpiando procesos anteriores..." -ForegroundColor Yellow
Get-Process | Where-Object {$_.ProcessName -like "*master*" -or $_.ProcessName -like "*worker*"} | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2

# Iniciar Master en background
Write-Host "Iniciando Master en puerto 8080..." -ForegroundColor Green
$master = Start-Process -FilePath "target\release\master.exe" -PassThru -WindowStyle Minimized
Start-Sleep -Seconds 3

# Verificar que el master está corriendo
try {
    $response = Invoke-WebRequest -Uri "http://127.0.0.1:8080/api/v1/workers" -Method GET -TimeoutSec 2
    Write-Host "✓ Master está corriendo correctamente" -ForegroundColor Green
} catch {
    Write-Host "✗ Error: Master no responde" -ForegroundColor Red
    Stop-Process -Id $master.Id -Force -ErrorAction SilentlyContinue
    exit 1
}

# Iniciar Worker en background
Write-Host "Iniciando Worker en puerto 9000..." -ForegroundColor Green
$env:WORKER_PORT = "9000"
$worker = Start-Process -FilePath "target\release\worker.exe" -PassThru -WindowStyle Minimized
Start-Sleep -Seconds 3

# Verificar que el worker se registró
Start-Sleep -Seconds 2
try {
    $response = Invoke-WebRequest -Uri "http://127.0.0.1:8080/api/v1/workers" -Method GET
    $workers = ($response.Content | ConvertFrom-Json).workers
    if ($workers.Count -gt 0) {
        Write-Host "✓ Worker registrado correctamente: $($workers[0].id)" -ForegroundColor Green
    } else {
        Write-Host "✗ Error: Worker no se registró" -ForegroundColor Red
        Stop-Process -Id $master.Id, $worker.Id -Force -ErrorAction SilentlyContinue
        exit 1
    }
} catch {
    Write-Host "✗ Error al verificar workers" -ForegroundColor Red
    Stop-Process -Id $master.Id, $worker.Id -Force -ErrorAction SilentlyContinue
    exit 1
}

# Enviar un job de prueba
Write-Host ""
Write-Host "Enviando job de prueba (map_add con input [1,2,3,4,5])..." -ForegroundColor Green
$jobResult = & target\release\client.exe submit-job --name "test-job" --operation "map_add" --param 10 --input "1,2,3,4,5"

if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ Job enviado correctamente" -ForegroundColor Green
    # Extraer job_id del output
    $jobId = ($jobResult | Select-String -Pattern "Job ID: ([a-f0-9-]+)").Matches.Groups[1].Value
    Write-Host "  Job ID: $jobId" -ForegroundColor Cyan
    
    # Esperar un poco y verificar progreso
    Start-Sleep -Seconds 2
    Write-Host ""
    Write-Host "Verificando progreso del job..." -ForegroundColor Green
    & target\release\client.exe get-progress --job-id $jobId
} else {
    Write-Host "✗ Error al enviar job" -ForegroundColor Red
}

# Limpiar
Write-Host ""
Write-Host "Presiona Enter para detener los procesos..." -ForegroundColor Yellow
Read-Host
Stop-Process -Id $master.Id, $worker.Id -Force -ErrorAction SilentlyContinue
Write-Host "Procesos detenidos" -ForegroundColor Green

