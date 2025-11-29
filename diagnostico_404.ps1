# Script de diagnóstico para el error 404
Write-Host "=== DIAGNÓSTICO DEL ERROR 404 ===" -ForegroundColor Cyan

# 1. Verificar que el master está corriendo
Write-Host "`n[1] Verificando que el master está corriendo..." -ForegroundColor Yellow
$masterProcess = Get-Process | Where-Object {$_.ProcessName -eq "master"}
if ($masterProcess) {
    Write-Host "  ✓ Master encontrado (PID: $($masterProcess.Id))" -ForegroundColor Green
} else {
    Write-Host "  ✗ Master no está corriendo" -ForegroundColor Red
    Write-Host "    Inicia el master con: cargo run --release --bin master" -ForegroundColor Yellow
    exit 1
}

# 2. Verificar que el master responde en el puerto 8080
Write-Host "`n[2] Verificando respuesta del master en puerto 8080..." -ForegroundColor Yellow
try {
    $response = Invoke-WebRequest -Uri "http://127.0.0.1:8080/api/v1/workers" -Method Get -TimeoutSec 3
    Write-Host "  ✓ Master responde (Status: $($response.StatusCode))" -ForegroundColor Green
    Write-Host "    Contenido: $($response.Content.Substring(0, [Math]::Min(100, $response.Content.Length)))..." -ForegroundColor Gray
} catch {
    Write-Host "  ✗ Master no responde: $_" -ForegroundColor Red
    Write-Host "    Verifica que el master esté escuchando en 127.0.0.1:8080" -ForegroundColor Yellow
    exit 1
}

# 3. Verificar que hay workers registrados
Write-Host "`n[3] Verificando workers registrados..." -ForegroundColor Yellow
try {
    $workersResponse = Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/v1/workers" -Method Get -TimeoutSec 3
    if ($workersResponse.workers.Count -gt 0) {
        Write-Host "  ✓ Hay $($workersResponse.workers.Count) worker(s) registrado(s)" -ForegroundColor Green
        foreach ($w in $workersResponse.workers) {
            Write-Host "    - $($w.id) @ $($w.host):$($w.port) [$($w.status)]" -ForegroundColor Cyan
        }
    } else {
        Write-Host "  ⚠ No hay workers registrados" -ForegroundColor Yellow
        Write-Host "    El job necesita al menos un worker para ejecutarse" -ForegroundColor Yellow
    }
} catch {
    Write-Host "  ✗ Error al obtener workers: $_" -ForegroundColor Red
}

# 4. Probar la ruta /api/v1/jobs con diferentes métodos
Write-Host "`n[4] Probando ruta /api/v1/jobs..." -ForegroundColor Yellow

# Probar GET (debería dar 405 Method Not Allowed o 404)
try {
    $getResponse = Invoke-WebRequest -Uri "http://127.0.0.1:8080/api/v1/jobs" -Method Get -TimeoutSec 3
    Write-Host "  GET: Status $($getResponse.StatusCode)" -ForegroundColor $(if ($getResponse.StatusCode -eq 405) { "Green" } else { "Yellow" })
} catch {
    $statusCode = $_.Exception.Response.StatusCode.value__
    Write-Host "  GET: Status $statusCode" -ForegroundColor $(if ($statusCode -eq 405 -or $statusCode -eq 404) { "Yellow" } else { "Red" })
}

# 5. Probar POST con JSON válido
Write-Host "`n[5] Probando POST a /api/v1/jobs con JSON..." -ForegroundColor Yellow

$jobData = @{
    name = "test_map_add"
    operation = "map_add"
    param = 10
    input = @(1, 2, 3, 4, 5)
} | ConvertTo-Json -Depth 10 -Compress

Write-Host "  JSON a enviar:" -ForegroundColor Gray
Write-Host "  $jobData" -ForegroundColor Gray

try {
    $jobResponse = Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/v1/jobs" `
        -Method Post `
        -Body $jobData `
        -ContentType "application/json; charset=utf-8" `
        -TimeoutSec 10
    
    Write-Host "  ✓ Job enviado exitosamente!" -ForegroundColor Green
    Write-Host "    Job ID: $($jobResponse.job_id)" -ForegroundColor Cyan
    Write-Host "    Mensaje: $($jobResponse.message)" -ForegroundColor Cyan
    
} catch {
    $statusCode = $_.Exception.Response.StatusCode.value__
    Write-Host "  ✗ Error al enviar job" -ForegroundColor Red
    Write-Host "    Status Code: $statusCode" -ForegroundColor Red
    Write-Host "    Mensaje: $($_.Exception.Message)" -ForegroundColor Red
    
    if ($statusCode -eq 404) {
        Write-Host "`n  DIAGNÓSTICO DEL 404:" -ForegroundColor Yellow
        Write-Host "    - La ruta /api/v1/jobs no existe o no está registrada" -ForegroundColor Yellow
        Write-Host "    - Verifica que el master tenga la ruta registrada" -ForegroundColor Yellow
        Write-Host "    - Verifica los logs del master para ver si hay errores" -ForegroundColor Yellow
    } elseif ($statusCode -eq 503) {
        Write-Host "`n  DIAGNÓSTICO DEL 503:" -ForegroundColor Yellow
        Write-Host "    - No hay workers disponibles" -ForegroundColor Yellow
        Write-Host "    - Inicia al menos un worker antes de enviar jobs" -ForegroundColor Yellow
    } elseif ($statusCode -eq 400) {
        Write-Host "`n  DIAGNÓSTICO DEL 400:" -ForegroundColor Yellow
        Write-Host "    - El formato del JSON es incorrecto" -ForegroundColor Yellow
        Write-Host "    - Verifica que el JSON tenga los campos requeridos" -ForegroundColor Yellow
    }
    
    # Intentar obtener más detalles del error
    try {
        $errorResponse = $_.Exception.Response
        $reader = New-Object System.IO.StreamReader($errorResponse.GetResponseStream())
        $responseBody = $reader.ReadToEnd()
        Write-Host "    Respuesta del servidor: $responseBody" -ForegroundColor Gray
    } catch {
        # Ignorar si no se puede leer la respuesta
    }
}

# 6. Probar también la ruta alternativa /api/v1/jobs/submit
Write-Host "`n[6] Probando ruta alternativa /api/v1/jobs/submit..." -ForegroundColor Yellow
try {
    $jobResponse2 = Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/v1/jobs/submit" `
        -Method Post `
        -Body $jobData `
        -ContentType "application/json; charset=utf-8" `
        -TimeoutSec 10
    
    Write-Host "  ✓ Job enviado exitosamente usando /api/v1/jobs/submit!" -ForegroundColor Green
    Write-Host "    Job ID: $($jobResponse2.job_id)" -ForegroundColor Cyan
} catch {
    $statusCode = $_.Exception.Response.StatusCode.value__
    Write-Host "  ✗ Error con /api/v1/jobs/submit (Status: $statusCode)" -ForegroundColor Red
}

Write-Host "`n=== DIAGNÓSTICO COMPLETADO ===" -ForegroundColor Cyan

