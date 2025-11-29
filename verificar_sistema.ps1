# Script de verificación del sistema
# Ejecutar: powershell -ExecutionPolicy Bypass -File verificar_sistema.ps1

$ErrorActionPreference = "Continue"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  VERIFICACIÓN DEL SISTEMA DISTRIBUIDO  " -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

# 1. Verificar compilación
Write-Host "`n[1/5] Verificando compilación..." -ForegroundColor Yellow
if (Test-Path "target\release\master.exe") {
    Write-Host "  ✓ Master compilado" -ForegroundColor Green
} else {
    Write-Host "  ✗ Master no encontrado. Ejecuta: cargo build --release" -ForegroundColor Red
    exit 1
}

if (Test-Path "target\release\worker.exe") {
    Write-Host "  ✓ Worker compilado" -ForegroundColor Green
} else {
    Write-Host "  ✗ Worker no encontrado. Ejecuta: cargo build --release" -ForegroundColor Red
    exit 1
}

if (Test-Path "target\release\client.exe") {
    Write-Host "  ✓ Client compilado" -ForegroundColor Green
} else {
    Write-Host "  ✗ Client no encontrado. Ejecuta: cargo build --release" -ForegroundColor Red
    exit 1
}

# 2. Limpiar procesos anteriores
Write-Host "`n[2/5] Limpiando procesos anteriores..." -ForegroundColor Yellow
Get-Process | Where-Object {$_.ProcessName -eq "master" -or $_.ProcessName -eq "worker"} | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2
Write-Host "  ✓ Procesos limpiados" -ForegroundColor Green

# 3. Iniciar master
Write-Host "`n[3/5] Iniciando master..." -ForegroundColor Yellow
$masterProcess = Start-Process -FilePath "target\release\master.exe" -PassThru -WindowStyle Minimized
Start-Sleep -Seconds 4

if ($masterProcess -and !$masterProcess.HasExited) {
    Write-Host "  ✓ Master iniciado (PID: $($masterProcess.Id))" -ForegroundColor Green
} else {
    Write-Host "  ✗ Error al iniciar master" -ForegroundColor Red
    exit 1
}

# 4. Verificar que el master responde
Write-Host "`n[4/5] Verificando respuesta del master..." -ForegroundColor Yellow
Start-Sleep -Seconds 2

try {
    $response = Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/v1/workers" -Method Get -TimeoutSec 5
    Write-Host "  ✓ Master responde correctamente" -ForegroundColor Green
    Write-Host "    Versión: $($response.version)" -ForegroundColor Cyan
    Write-Host "    Workers registrados: $($response.workers.Count)" -ForegroundColor Cyan
} catch {
    Write-Host "  ✗ Error al conectar con master: $_" -ForegroundColor Red
    Stop-Process -Id $masterProcess.Id -Force -ErrorAction SilentlyContinue
    exit 1
}

# 5. Iniciar worker
Write-Host "`n[5/5] Iniciando worker..." -ForegroundColor Yellow
$env:WORKER_PORT = "9001"
$env:MASTER_URL = "http://127.0.0.1:8080"
$workerProcess = Start-Process -FilePath "target\release\worker.exe" -PassThru -WindowStyle Minimized -Environment @{"WORKER_PORT"="9001"; "MASTER_URL"="http://127.0.0.1:8080"}
Start-Sleep -Seconds 4

if ($workerProcess -and !$workerProcess.HasExited) {
    Write-Host "  ✓ Worker iniciado (PID: $($workerProcess.Id))" -ForegroundColor Green
} else {
    Write-Host "  ✗ Error al iniciar worker" -ForegroundColor Red
    Stop-Process -Id $masterProcess.Id -Force -ErrorAction SilentlyContinue
    exit 1
}

# Verificar registro del worker
Write-Host "`nVerificando registro del worker..." -ForegroundColor Yellow
Start-Sleep -Seconds 3

try {
    $response = Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/v1/workers" -Method Get -TimeoutSec 5
    if ($response.workers.Count -gt 0) {
        Write-Host "  ✓ Worker registrado correctamente" -ForegroundColor Green
        foreach ($worker in $response.workers) {
            Write-Host "    - $($worker.id) @ $($worker.host):$($worker.port) [$($worker.status)]" -ForegroundColor Cyan
        }
    } else {
        Write-Host "  ⚠ No hay workers registrados aún" -ForegroundColor Yellow
    }
} catch {
    Write-Host "  ✗ Error al verificar workers: $_" -ForegroundColor Red
}

# Prueba de envío de job
Write-Host "`n=== PRUEBA DE ENVÍO DE JOB ===" -ForegroundColor Cyan
$jobData = @{
    name = "test_map_add"
    operation = "map_add"
    param = 10
    input = @(1, 2, 3, 4, 5)
} | ConvertTo-Json -Depth 10 -Compress

Write-Host "JSON a enviar: $jobData" -ForegroundColor Gray

try {
    Write-Host "Enviando job de prueba..." -ForegroundColor Yellow
    
    # Verificar primero que el master responde
    try {
        $testResponse = Invoke-WebRequest -Uri "http://127.0.0.1:8080/api/v1/workers" -Method Get -TimeoutSec 2
        Write-Host "  ✓ Master responde (status: $($testResponse.StatusCode))" -ForegroundColor Green
    } catch {
        Write-Host "  ✗ Master no responde. Verifica que esté corriendo." -ForegroundColor Red
        Write-Host "    Error: $_" -ForegroundColor Red
        exit 1
    }
    
    $jobResponse = Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/v1/jobs" `
        -Method Post `
        -Body $jobData `
        -ContentType "application/json" `
        -TimeoutSec 10
    
    Write-Host "  ✓ Job enviado exitosamente" -ForegroundColor Green
    Write-Host "    Job ID: $($jobResponse.job_id)" -ForegroundColor Cyan
    Write-Host "    Mensaje: $($jobResponse.message)" -ForegroundColor Cyan
    
    $jobId = $jobResponse.job_id
    
    # Esperar procesamiento
    Write-Host "`nEsperando procesamiento (5 segundos)..." -ForegroundColor Yellow
    Start-Sleep -Seconds 5
    
    # Verificar estado
    Write-Host "Verificando estado del job..." -ForegroundColor Yellow
    $status = Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/v1/jobs/$jobId" -Method Get -TimeoutSec 5
    Write-Host "  Estado: $($status.status)" -ForegroundColor $(if ($status.status -eq "SUCCEEDED") { "Green" } elseif ($status.status -eq "RUNNING") { "Yellow" } else { "Red" })
    Write-Host "  Progreso: $($status.completed_tasks)/$($status.total_tasks) ($([math]::Round($status.progress_percent, 2))%)" -ForegroundColor Cyan
    
    if ($status.status -eq "SUCCEEDED" -or $status.status -eq "RUNNING") {
        Write-Host "  ✓ Job procesado correctamente" -ForegroundColor Green
    }
    
} catch {
    Write-Host "  ✗ Error al enviar/verificar job: $_" -ForegroundColor Red
}

# Resumen
Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "  RESUMEN DE VERIFICACIÓN" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Master: " -NoNewline
if ($masterProcess -and !$masterProcess.HasExited) {
    Write-Host "✓ Corriendo (PID: $($masterProcess.Id))" -ForegroundColor Green
} else {
    Write-Host "✗ No está corriendo" -ForegroundColor Red
}

Write-Host "Worker: " -NoNewline
if ($workerProcess -and !$workerProcess.HasExited) {
    Write-Host "✓ Corriendo (PID: $($workerProcess.Id))" -ForegroundColor Green
} else {
    Write-Host "✗ No está corriendo" -ForegroundColor Red
}

Write-Host "`nPara detener los procesos:" -ForegroundColor Yellow
Write-Host "  Stop-Process -Id $($masterProcess.Id), $($workerProcess.Id) -Force" -ForegroundColor Cyan

Write-Host "`n=== VERIFICACIÓN COMPLETADA ===" -ForegroundColor Cyan

