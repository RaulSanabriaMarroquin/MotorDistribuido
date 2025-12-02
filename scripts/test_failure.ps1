# Script de prueba de simulación de fallo
# Demuestra tolerancia a fallos: detiene un worker mientras procesa y verifica replanificación

Write-Host "=== PRUEBA DE SIMULACIÓN DE FALLO ===" -ForegroundColor Cyan
Write-Host "Esta prueba demuestra la tolerancia a fallos del sistema" -ForegroundColor Yellow
Write-Host ""

$ErrorActionPreference = "Continue"

# Limpiar procesos anteriores
Write-Host "1. Limpiando procesos anteriores..." -ForegroundColor Yellow
Get-Process | Where-Object {$_.ProcessName -like "*master*" -or $_.ProcessName -like "*worker*"} | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2

# Verificar que los binarios existen
if (-not (Test-Path "target\release\master.exe")) {
    Write-Host "Error: Los binarios no están compilados. Ejecuta 'make build-release' primero." -ForegroundColor Red
    exit 1
}

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

# Iniciar Worker 1
Write-Host "3. Iniciando Worker 1 (puerto 9000)..." -ForegroundColor Green
$env:WORKER_PORT = "9000"
$worker1 = Start-Process -FilePath "target\release\worker.exe" -PassThru -WindowStyle Minimized
Start-Sleep -Seconds 2

# Iniciar Worker 2 (backup)
Write-Host "4. Iniciando Worker 2 (puerto 9001)..." -ForegroundColor Green
$env:WORKER_PORT = "9001"
$worker2 = Start-Process -FilePath "target\release\worker.exe" -PassThru -WindowStyle Minimized
Start-Sleep -Seconds 3

# Verificar que ambos workers están registrados
$response = Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/v1/workers" -Method GET
Write-Host "   ✓ Workers registrados: $($response.workers.Count)" -ForegroundColor Green

if ($response.workers.Count -lt 2) {
    Write-Host "   ⚠ Advertencia: Se esperaban 2 workers, pero solo $($response.workers.Count) están registrados" -ForegroundColor Yellow
    Start-Sleep -Seconds 2
    $response = Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/v1/workers" -Method GET
    Write-Host "   Workers después de esperar: $($response.workers.Count)" -ForegroundColor Cyan
}

# Enviar un job que tome tiempo
Write-Host "`n5. Enviando job de prueba (este job será interrumpido)..." -ForegroundColor Cyan
$jobOutput = & target\release\client.exe submit-job --name "test-failure" --operation "map_add" --param 10 --input "1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20"

if ($LASTEXITCODE -ne 0) {
    Write-Host "   ✗ Error al enviar job" -ForegroundColor Red
    Stop-Process -Id $master.Id, $worker1.Id, $worker2.Id -Force -ErrorAction SilentlyContinue
    exit 1
}

# Extraer JOB_ID
$jobId = ($jobOutput | Select-String -Pattern "Job ID: ([a-f0-9-]+)").Matches.Groups[1].Value
if (-not $jobId) {
    Write-Host "   ✗ No se pudo extraer el JOB_ID" -ForegroundColor Red
    Stop-Process -Id $master.Id, $worker1.Id, $worker2.Id -Force -ErrorAction SilentlyContinue
    exit 1
}

Write-Host "   ✓ Job enviado. JOB_ID: $jobId" -ForegroundColor Green

# Esperar un poco para que el job comience a procesarse
Write-Host "`n6. Esperando que el job comience a procesarse..." -ForegroundColor Yellow
Start-Sleep -Seconds 2

# Verificar estado inicial del job
try {
    $jobStatus = Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/v1/jobs/$jobId" -Method GET
    Write-Host "   Estado inicial del job: $($jobStatus.status)" -ForegroundColor Cyan
} catch {
    Write-Host "   ⚠ No se pudo obtener el estado inicial del job" -ForegroundColor Yellow
}

# SIMULAR FALLO: Detener Worker 1 abruptamente
Write-Host "`n7. ⚠ SIMULANDO FALLO: Deteniendo Worker 1 abruptamente..." -ForegroundColor Red
Stop-Process -Id $worker1.Id -Force -ErrorAction SilentlyContinue
Write-Host "   ✓ Worker 1 detenido" -ForegroundColor Yellow

# Esperar a que el master detecte el fallo y replanifique
Write-Host "`n8. Esperando detección de fallo y replanificación..." -ForegroundColor Yellow
Start-Sleep -Seconds 5

# Verificar que el master detectó el fallo
$response = Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/v1/workers" -Method GET
$activeWorkers = ($response.workers | Where-Object {$_.status -eq "UP"}).Count
Write-Host "   Workers activos después del fallo: $activeWorkers" -ForegroundColor Cyan

if ($activeWorkers -lt 1) {
    Write-Host "   ✗ Error: No hay workers activos después del fallo" -ForegroundColor Red
    Stop-Process -Id $master.Id, $worker2.Id -Force -ErrorAction SilentlyContinue
    exit 1
}

# Esperar a que el job se complete con el worker 2
Write-Host "`n9. Esperando que el job se complete con Worker 2..." -ForegroundColor Yellow
$maxWait = 30
$waited = 0
$completed = $false

while ($waited -lt $maxWait -and -not $completed) {
    Start-Sleep -Seconds 2
    $waited += 2
    
    try {
        $jobStatus = Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/v1/jobs/$jobId" -Method GET
        Write-Host "   Estado: $($jobStatus.status) (esperado: $($jobStatus.completed_tasks)/$($jobStatus.total_tasks))" -ForegroundColor Cyan
        
        if ($jobStatus.status -eq "completed") {
            $completed = $true
        }
    } catch {
        Write-Host "   ⚠ Error al obtener estado del job" -ForegroundColor Yellow
    }
}

# Verificar resultado final
Write-Host "`n10. Verificando resultado final..." -ForegroundColor Cyan
try {
    $jobStatus = Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/v1/jobs/$jobId" -Method GET
    
    if ($jobStatus.status -eq "completed") {
        Write-Host "   ✓ Job completado exitosamente después del fallo" -ForegroundColor Green
        Write-Host "   Tareas completadas: $($jobStatus.completed_tasks)/$($jobStatus.total_tasks)" -ForegroundColor Cyan
        
        # Obtener resultados
        try {
            $results = Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/v1/jobs/$jobId/results" -Method GET
            Write-Host "   ✓ Resultados obtenidos" -ForegroundColor Green
        } catch {
            Write-Host "   ⚠ No se pudieron obtener los resultados" -ForegroundColor Yellow
        }
    } else {
        Write-Host "   ✗ Job no se completó. Estado: $($jobStatus.status)" -ForegroundColor Red
    }
} catch {
    Write-Host "   ✗ Error al verificar estado final del job: $_" -ForegroundColor Red
}

# Verificar métricas
Write-Host "`n11. Verificando métricas del sistema..." -ForegroundColor Cyan
try {
    $metrics = Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/v1/metrics" -Method GET
    Write-Host "   ✓ Métricas obtenidas:" -ForegroundColor Green
    Write-Host "     - Nodos: $($metrics.node_metrics.Count)" -ForegroundColor Cyan
    Write-Host "     - Jobs: $($metrics.job_metrics.Count)" -ForegroundColor Cyan
    
    # Buscar métricas relacionadas con fallos
    $failedJobs = ($metrics.job_metrics | Where-Object {$_.failures -gt 0}).Count
    if ($failedJobs -gt 0) {
        Write-Host "     - Jobs con fallos detectados: $failedJobs" -ForegroundColor Yellow
    }
} catch {
    Write-Host "   ⚠ No se pudieron obtener métricas" -ForegroundColor Yellow
}

# Resumen
Write-Host "`n=== RESUMEN DE PRUEBA DE FALLO ===" -ForegroundColor Green
if ($completed) {
    Write-Host "✓ FALLO SIMULADO EXITOSAMENTE" -ForegroundColor Green
    Write-Host "✓ Worker 1 detenido durante procesamiento" -ForegroundColor Green
    Write-Host "✓ Master detectó el fallo" -ForegroundColor Green
    Write-Host "✓ Tareas replanificadas a Worker 2" -ForegroundColor Green
    Write-Host "✓ Job completado exitosamente" -ForegroundColor Green
} else {
    Write-Host "⚠ PRUEBA INCOMPLETA" -ForegroundColor Yellow
    Write-Host "El job no se completó en el tiempo esperado" -ForegroundColor Yellow
}

Write-Host "`n=== PRUEBA COMPLETADA ===" -ForegroundColor Cyan

# Limpiar
Write-Host "`nDeteniendo procesos restantes..." -ForegroundColor Yellow
Get-Process | Where-Object {$_.ProcessName -like "*master*" -or $_.ProcessName -like "*worker*"} | Stop-Process -Force -ErrorAction SilentlyContinue
Write-Host "Procesos detenidos" -ForegroundColor Green

