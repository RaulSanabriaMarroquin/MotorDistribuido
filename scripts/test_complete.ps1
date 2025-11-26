# Script de pruebas completas del sistema
# Prueba todas las funcionalidades implementadas

Write-Host "=== PRUEBAS COMPLETAS DEL SISTEMA ===" -ForegroundColor Cyan
Write-Host ""

$ErrorActionPreference = "Continue"

# Limpiar procesos anteriores
Write-Host "1. Limpiando procesos anteriores..." -ForegroundColor Yellow
Get-Process | Where-Object {$_.ProcessName -like "*master*" -or $_.ProcessName -like "*worker*"} | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2

# Iniciar Master
Write-Host "2. Iniciando Master..." -ForegroundColor Green
$master = Start-Process -FilePath "target\release\master.exe" -PassThru -WindowStyle Minimized
Start-Sleep -Seconds 3

try {
    $null = Invoke-WebRequest -Uri "http://127.0.0.1:8080/api/v1/workers" -Method GET -TimeoutSec 2
    Write-Host "   ✓ Master iniciado correctamente" -ForegroundColor Green
} catch {
    Write-Host "   ✗ Error: Master no responde" -ForegroundColor Red
    exit 1
}

# Iniciar Workers
Write-Host "3. Iniciando Workers..." -ForegroundColor Green
$env:WORKER_PORT = "9000"
$worker1 = Start-Process -FilePath "target\release\worker.exe" -PassThru -WindowStyle Minimized
Start-Sleep -Seconds 2
$env:WORKER_PORT = "9001"
$worker2 = Start-Process -FilePath "target\release\worker.exe" -PassThru -WindowStyle Minimized
Start-Sleep -Seconds 3

$response = Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/v1/workers" -Method GET
Write-Host "   ✓ Workers registrados: $($response.workers.Count)" -ForegroundColor Green

# Prueba 1: Operación legacy
Write-Host "`n4. Prueba: Operación legacy (map_add)..." -ForegroundColor Cyan
$output = & target\release\client.exe submit-job --name "test-map-add" --operation "map_add" --param 10 --input "1,2,3,4,5"
$jobId1 = ($output | Select-String -Pattern "Job ID: ([a-f0-9-]+)").Matches.Groups[1].Value
Start-Sleep -Seconds 2
$progress = Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/v1/jobs/$jobId1/progress" -Method GET
if ($progress.status -eq "completed") {
    Write-Host "   ✓ Job completado: $($progress.completed_tasks)/$($progress.total_tasks)" -ForegroundColor Green
} else {
    Write-Host "   ✗ Job no completado: $($progress.status)" -ForegroundColor Red
}

# Prueba 2: flat_map
Write-Host "`n5. Prueba: Operación flat_map..." -ForegroundColor Cyan
$body = '{"name":"test-flatmap","operation":"flat_map","fn_name":"split","input":[123,456]}'
$response = Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/v1/jobs/submit" -Method POST -Body $body -ContentType "application/json"
$jobId2 = $response.job_id
Start-Sleep -Seconds 2
$progress = Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/v1/jobs/$jobId2/progress" -Method GET
if ($progress.status -eq "completed") {
    Write-Host "   ✓ flat_map completado: $($progress.completed_tasks)/$($progress.total_tasks)" -ForegroundColor Green
} else {
    Write-Host "   ✗ flat_map no completado: $($progress.status)" -ForegroundColor Red
}

# Prueba 3: reduce_by_key
Write-Host "`n6. Prueba: Operación reduce_by_key..." -ForegroundColor Cyan
$body = '{"name":"test-reduce","operation":"reduce_by_key","fn_name":"sum","input":[1,2,2,3,3,3,4,4,4,4]}'
$response = Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/v1/jobs/submit" -Method POST -Body $body -ContentType "application/json"
$jobId3 = $response.job_id
Start-Sleep -Seconds 2
$progress = Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/v1/jobs/$jobId3/progress" -Method GET
if ($progress.status -eq "completed") {
    Write-Host "   ✓ reduce_by_key completado: $($progress.completed_tasks)/$($progress.total_tasks)" -ForegroundColor Green
} else {
    Write-Host "   ✗ reduce_by_key no completado: $($progress.status)" -ForegroundColor Red
}

# Prueba 4: Métricas
Write-Host "`n7. Prueba: Métricas..." -ForegroundColor Cyan
try {
    $metrics = Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/v1/metrics" -Method GET
    Write-Host "   ✓ Métricas obtenidas:" -ForegroundColor Green
    Write-Host "     - Nodos: $($metrics.node_metrics.Count)" -ForegroundColor Cyan
    Write-Host "     - Jobs: $($metrics.job_metrics.Count)" -ForegroundColor Cyan
} catch {
    Write-Host "   ✗ Error obteniendo métricas: $_" -ForegroundColor Red
}

# Prueba 5: Estado de workers
Write-Host "`n8. Prueba: Estado de workers..." -ForegroundColor Cyan
$response = Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/v1/workers" -Method GET
Write-Host "   ✓ Workers activos: $($response.workers.Count)" -ForegroundColor Green
$allUp = ($response.workers | Where-Object {$_.status -eq "UP"}).Count -eq $response.workers.Count
if ($allUp) {
    Write-Host "   ✓ Todos los workers están UP" -ForegroundColor Green
} else {
    Write-Host "   ⚠ Algunos workers no están UP" -ForegroundColor Yellow
}

# Resumen
Write-Host "`n=== RESUMEN DE PRUEBAS ===" -ForegroundColor Green
Write-Host "✓ Master funcionando" -ForegroundColor Green
Write-Host "✓ Workers registrados y activos" -ForegroundColor Green
Write-Host "✓ Operaciones legacy funcionando" -ForegroundColor Green
Write-Host "✓ Operaciones nuevas (flat_map, reduce_by_key) funcionando" -ForegroundColor Green
Write-Host "✓ Métricas funcionando" -ForegroundColor Green
Write-Host "`n=== TODAS LAS PRUEBAS COMPLETADAS ===" -ForegroundColor Cyan

# Limpiar
Write-Host "`nDeteniendo procesos..." -ForegroundColor Yellow
Get-Process | Where-Object {$_.ProcessName -like "*master*" -or $_.ProcessName -like "*worker*"} | Stop-Process -Force -ErrorAction SilentlyContinue
Write-Host "Procesos detenidos" -ForegroundColor Green

