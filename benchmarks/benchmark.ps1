# Script de Benchmarks para Motor Distribuido
# Genera reporte de rendimiento con 1M registros según especificación del enunciado

param(
    [int]$RecordCount = 1000000,
    [string]$MasterUrl = "http://127.0.0.1:8080",
    [string]$OutputDir = "benchmarks/results"
)

Write-Host "=== BENCHMARKS DEL SISTEMA DISTRIBUIDO ===" -ForegroundColor Cyan
Write-Host "Registros a procesar: $RecordCount" -ForegroundColor Yellow
Write-Host ""

# Crear directorio de resultados
if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
}

$timestamp = Get-Date -Format "yyyyMMdd_HHmmss"
$reportFile = Join-Path $OutputDir "benchmark_report_$timestamp.md"

# Inicializar reporte
$report = @"
# Reporte de Benchmarks - Motor Distribuido

**Fecha:** $(Get-Date -Format "yyyy-MM-dd HH:mm:ss")
**Registros procesados:** $RecordCount
**URL Master:** $MasterUrl

---

## Configuración del Sistema

- **Sistema Operativo:** $($env:OS)
- **Procesador:** $($env:PROCESSOR_ARCHITECTURE)
- **Número de Workers:** (se detectará automáticamente)

---

## Resultados

"@

# Función para generar datos de prueba
function Generate-TestData {
    param([int]$Count, [string]$OutputFile)
    
    Write-Host "Generando $Count registros en $OutputFile..." -ForegroundColor Yellow
    
    $data = 1..$Count | ForEach-Object { 
        "$_,$([math]::Floor((Get-Random -Minimum 1 -Maximum 1000))),$([math]::Floor((Get-Random -Minimum 1 -Maximum 100)))"
    }
    
    $data | Out-File -FilePath $OutputFile -Encoding UTF8
    Write-Host "✓ Datos generados: $OutputFile" -ForegroundColor Green
}

# Función para medir tiempo de ejecución
function Measure-JobExecution {
    param(
        [string]$JobName,
        [object]$JobSpec,
        [string]$MasterUrl
    )
    
    Write-Host "`nEjecutando: $JobName..." -ForegroundColor Cyan
    
    $startTime = Get-Date
    
    try {
        $response = Invoke-RestMethod -Uri "$MasterUrl/api/v1/jobs" -Method POST -Body ($JobSpec | ConvertTo-Json -Depth 10) -ContentType "application/json"
        $jobId = $response.job_id
        
        Write-Host "  Job ID: $jobId" -ForegroundColor Gray
        
        # Esperar a que el job se complete
        $maxWait = 300 # 5 minutos máximo
        $waitTime = 0
        $status = "running"
        
        while ($status -eq "running" -and $waitTime -lt $maxWait) {
            Start-Sleep -Seconds 2
            $waitTime += 2
            
            try {
                $jobStatus = Invoke-RestMethod -Uri "$MasterUrl/api/v1/jobs/$jobId" -Method GET
                $status = $jobStatus.status
                $progress = $jobStatus.progress_percent
                
                if ($waitTime % 10 -eq 0) {
                    Write-Host "  Progreso: $progress%" -ForegroundColor Gray
                }
            } catch {
                # Job puede no estar listo aún
            }
        }
        
        $endTime = Get-Date
        $duration = ($endTime - $startTime).TotalSeconds
        
        if ($status -eq "completed") {
            Write-Host "  ✓ Completado en $([math]::Round($duration, 2)) segundos" -ForegroundColor Green
            
            # Obtener métricas
            try {
                $metrics = Invoke-RestMethod -Uri "$MasterUrl/api/v1/jobs/$jobId" -Method GET
                $throughput = if ($metrics.metrics.throughput) { $metrics.metrics.throughput } else { "N/A" }
                
                return @{
                    Success = $true
                    Duration = $duration
                    JobId = $jobId
                    Throughput = $throughput
                    Status = $status
                }
            } catch {
                return @{
                    Success = $true
                    Duration = $duration
                    JobId = $jobId
                    Throughput = "N/A"
                    Status = $status
                }
            }
        } else {
            Write-Host "  ✗ Job no completado. Estado: $status" -ForegroundColor Red
            return @{
                Success = $false
                Duration = $duration
                JobId = $jobId
                Status = $status
            }
        }
    } catch {
        Write-Host "  ✗ Error: $_" -ForegroundColor Red
        return @{
            Success = $false
            Error = $_.Exception.Message
        }
    }
}

# Verificar que el master esté corriendo
Write-Host "Verificando conexión con master..." -ForegroundColor Yellow
try {
    $workers = Invoke-RestMethod -Uri "$MasterUrl/api/v1/workers" -Method GET
    $workerCount = $workers.workers.Count
    Write-Host "✓ Master conectado. Workers disponibles: $workerCount" -ForegroundColor Green
    $report += "`n**Workers disponibles:** $workerCount`n`n"
} catch {
    Write-Host "✗ Error: Master no está disponible en $MasterUrl" -ForegroundColor Red
    Write-Host "Por favor, inicia el master y los workers antes de ejecutar los benchmarks." -ForegroundColor Yellow
    exit 1
}

# Generar archivo de datos de prueba
$testDataFile = Join-Path $OutputDir "test_data_$RecordCount.csv"
if (-not (Test-Path $testDataFile)) {
    Generate-TestData -Count $RecordCount -OutputFile $testDataFile
} else {
    Write-Host "✓ Archivo de datos ya existe: $testDataFile" -ForegroundColor Green
}

# Benchmark 1: Operación map simple
Write-Host "`n=== BENCHMARK 1: Operación Map ===" -ForegroundColor Cyan
$mapJob = @{
    name = "benchmark_map_$RecordCount"
    operation = "map_add"
    param = 10
    input = (1..1000)  # Para jobs pequeños, usamos array directo
    parallelism = 4
}

$mapResult = Measure-JobExecution -JobName "Map (1000 elementos)" -JobSpec $mapJob -MasterUrl $MasterUrl

# Benchmark 2: Operación filter
Write-Host "`n=== BENCHMARK 2: Operación Filter ===" -ForegroundColor Cyan
$filterJob = @{
    name = "benchmark_filter_$RecordCount"
    operation = "filter_gt"
    param = 500
    input = (1..1000)
    parallelism = 4
}

$filterResult = Measure-JobExecution -JobName "Filter (1000 elementos)" -JobSpec $filterJob -MasterUrl $MasterUrl

# Benchmark 3: Operación reduce_by_key
Write-Host "`n=== BENCHMARK 3: Operación Reduce by Key ===" -ForegroundColor Cyan
$reduceJob = @{
    name = "benchmark_reduce_$RecordCount"
    operation = "reduce_by_key_sum"
    input = (1..1000 | ForEach-Object { Get-Random -Minimum 1 -Maximum 100 })
    parallelism = 4
}

$reduceResult = Measure-JobExecution -JobName "Reduce by Key (1000 elementos)" -JobSpec $reduceJob -MasterUrl $MasterUrl

# Benchmark 4: DAG complejo (read -> map -> filter -> reduce)
Write-Host "`n=== BENCHMARK 4: DAG Completo ===" -ForegroundColor Cyan
$dagJob = @{
    name = "benchmark_dag_$RecordCount"
    dag = @{
        nodes = @(
            @{
                id = "read"
                op = "read_csv"
                path = $testDataFile
                partitions = 4
            },
            @{
                id = "map"
                op = "map"
                fn_name = "add"
                partitions = 4
            },
            @{
                id = "filter"
                op = "filter"
                fn_name = "gt"
                partitions = 4
            },
            @{
                id = "reduce"
                op = "reduce_by_key"
                fn_name = "sum"
            }
        )
        edges = @(
            @("read", "map"),
            @("map", "filter"),
            @("filter", "reduce")
        )
    }
    parallelism = 4
}

$dagResult = Measure-JobExecution -JobName "DAG Completo (read->map->filter->reduce)" -JobSpec $dagJob -MasterUrl $MasterUrl

# Generar reporte final
$report += @"

### Benchmark 1: Operación Map
- **Operación:** map_add (sumar 10 a cada elemento)
- **Elementos:** 1000
- **Duración:** $([math]::Round($mapResult.Duration, 2)) segundos
- **Estado:** $($mapResult.Status)
- **Throughput:** $($mapResult.Throughput)

### Benchmark 2: Operación Filter
- **Operación:** filter_gt (elementos > 500)
- **Elementos:** 1000
- **Duración:** $([math]::Round($filterResult.Duration, 2)) segundos
- **Estado:** $($filterResult.Status)

### Benchmark 3: Operación Reduce by Key
- **Operación:** reduce_by_key_sum
- **Elementos:** 1000
- **Duración:** $([math]::Round($reduceResult.Duration, 2)) segundos
- **Estado:** $($reduceResult.Status)

### Benchmark 4: DAG Completo
- **Pipeline:** read_csv -> map -> filter -> reduce_by_key
- **Archivo:** $testDataFile
- **Registros:** $RecordCount
- **Duración:** $([math]::Round($dagResult.Duration, 2)) segundos
- **Estado:** $($dagResult.Status)

---

## Resumen

| Operación | Duración (s) | Estado |
|:----------|:------------:|:------:|
| Map | $([math]::Round($mapResult.Duration, 2)) | $($mapResult.Status) |
| Filter | $([math]::Round($filterResult.Duration, 2)) | $($filterResult.Status) |
| Reduce by Key | $([math]::Round($reduceResult.Duration, 2)) | $($reduceResult.Status) |
| DAG Completo | $([math]::Round($dagResult.Duration, 2)) | $($dagResult.Status) |

---

## Métricas del Sistema

"@

# Obtener métricas finales del sistema
try {
    $systemMetrics = Invoke-RestMethod -Uri "$MasterUrl/api/v1/metrics" -Method GET
    $report += "`n```json`n$($systemMetrics | ConvertTo-Json -Depth 5)`n```\n"
} catch {
    $report += "`n*No se pudieron obtener métricas del sistema*\n"
}

$report += @"

---

**Generado por:** benchmark.ps1
**Versión:** 1.0

"@

# Guardar reporte
$report | Out-File -FilePath $reportFile -Encoding UTF8
Write-Host "`n✓ Reporte guardado en: $reportFile" -ForegroundColor Green

Write-Host "`n=== BENCHMARKS COMPLETADOS ===" -ForegroundColor Cyan

