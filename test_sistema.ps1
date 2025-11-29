# Script de prueba del sistema distribuido
Write-Host "=== Prueba del Sistema Distribuido ===" -ForegroundColor Cyan

# Limpiar procesos anteriores si existen
Write-Host "`nLimpiando procesos anteriores..." -ForegroundColor Yellow
Get-Process | Where-Object {$_.ProcessName -like "*master*" -or $_.ProcessName -like "*worker*"} | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2

# Iniciar master
Write-Host "`nIniciando master..." -ForegroundColor Green
$master = Start-Process -FilePath "target\release\master.exe" -PassThru -WindowStyle Minimized
Start-Sleep -Seconds 3

# Iniciar worker 1
Write-Host "Iniciando worker 1..." -ForegroundColor Green
$env:WORKER_PORT = "9001"
$env:MASTER_URL = "http://127.0.0.1:8080"
$worker1 = Start-Process -FilePath "target\release\worker.exe" -PassThru -WindowStyle Minimized -Environment @{"WORKER_PORT"="9001"; "MASTER_URL"="http://127.0.0.1:8080"}
Start-Sleep -Seconds 3

# Iniciar worker 2
Write-Host "Iniciando worker 2..." -ForegroundColor Green
$worker2 = Start-Process -FilePath "target\release\worker.exe" -PassThru -WindowStyle Minimized -Environment @{"WORKER_PORT"="9002"; "MASTER_URL"="http://127.0.0.1:8080"}
Start-Sleep -Seconds 3

# Verificar que los procesos están corriendo
Write-Host "`nVerificando procesos..." -ForegroundColor Yellow
if ($master -and !$master.HasExited) {
    Write-Host "✓ Master corriendo (PID: $($master.Id))" -ForegroundColor Green
} else {
    Write-Host "✗ Master no está corriendo" -ForegroundColor Red
    exit 1
}

if ($worker1 -and !$worker1.HasExited) {
    Write-Host "✓ Worker 1 corriendo (PID: $($worker1.Id))" -ForegroundColor Green
} else {
    Write-Host "✗ Worker 1 no está corriendo" -ForegroundColor Red
}

if ($worker2 -and !$worker2.HasExited) {
    Write-Host "✓ Worker 2 corriendo (PID: $($worker2.Id))" -ForegroundColor Green
} else {
    Write-Host "✗ Worker 2 no está corriendo" -ForegroundColor Red
}

# Esperar a que se registren los workers
Write-Host "`nEsperando registro de workers..." -ForegroundColor Yellow
Start-Sleep -Seconds 5

# Listar workers
Write-Host "`n=== Listando workers ===" -ForegroundColor Cyan
$listResponse = curl.exe -s "http://127.0.0.1:8080/api/v1/workers"
Write-Host $listResponse

# Enviar job de prueba
Write-Host "`n=== Enviando job de prueba ===" -ForegroundColor Cyan
$jobJson = @{
    name = "test_map_add"
    operation = "map_add"
    param = 10
    input = @(1, 2, 3, 4, 5)
} | ConvertTo-Json

Write-Host "Job JSON:" -ForegroundColor Yellow
Write-Host $jobJson

$submitResponse = curl.exe -s -X POST "http://127.0.0.1:8080/api/v1/jobs" `
    -H "Content-Type: application/json" `
    -d $jobJson

Write-Host "`nRespuesta de envío:" -ForegroundColor Yellow
Write-Host $submitResponse

# Extraer job_id
$submitObj = $submitResponse | ConvertFrom-Json
$jobId = $submitObj.job_id
Write-Host "`nJob ID: $jobId" -ForegroundColor Green

# Esperar un momento para que se procese
Write-Host "`nEsperando procesamiento..." -ForegroundColor Yellow
Start-Sleep -Seconds 5

# Verificar progreso
Write-Host "`n=== Verificando progreso del job ===" -ForegroundColor Cyan
$progressResponse = curl.exe -s "http://127.0.0.1:8080/api/v1/jobs/$jobId"
Write-Host $progressResponse

# Verificar estado
Write-Host "`n=== Verificando estado del job ===" -ForegroundColor Cyan
$statusResponse = curl.exe -s "http://127.0.0.1:8080/api/v1/jobs/$jobId"
Write-Host $statusResponse

# Limpiar
Write-Host "`n=== Limpiando procesos ===" -ForegroundColor Yellow
Stop-Process -Id $master.Id -Force -ErrorAction SilentlyContinue
Stop-Process -Id $worker1.Id -Force -ErrorAction SilentlyContinue
Stop-Process -Id $worker2.Id -Force -ErrorAction SilentlyContinue

Write-Host "`n=== Prueba completada ===" -ForegroundColor Cyan

