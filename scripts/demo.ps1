# Demo script for Motor Distribuido (PowerShell)

Write-Host "=== Motor Distribuido Demo ===" -ForegroundColor Cyan
Write-Host ""

# Check if binaries exist
if (-not (Test-Path "target\release\master.exe")) {
    Write-Host "Building release binaries..." -ForegroundColor Yellow
    cargo build --release
}

# Clean up any existing processes
Write-Host "Cleaning up existing processes..." -ForegroundColor Yellow
Get-Process | Where-Object {$_.ProcessName -like "*master*" -or $_.ProcessName -like "*worker*"} | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2

# Start master
Write-Host "Starting master on port 8080..." -ForegroundColor Green
$master = Start-Process -FilePath "target\release\master.exe" -PassThru -WindowStyle Minimized
Start-Sleep -Seconds 3

# Verify master is running
try {
    $null = Invoke-WebRequest -Uri "http://127.0.0.1:8080/api/v1/workers" -Method GET -TimeoutSec 2
    Write-Host "✓ Master is running" -ForegroundColor Green
} catch {
    Write-Host "Error: Master failed to start" -ForegroundColor Red
    Stop-Process -Id $master.Id -Force -ErrorAction SilentlyContinue
    exit 1
}

# Start worker
Write-Host "Starting worker on port 9000..." -ForegroundColor Green
$env:WORKER_PORT = "9000"
$worker = Start-Process -FilePath "target\release\worker.exe" -PassThru -WindowStyle Minimized
Start-Sleep -Seconds 3

# Verify worker registered
$response = Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/v1/workers" -Method GET
if ($response.workers.Count -eq 0) {
    Write-Host "Error: Worker failed to register" -ForegroundColor Red
    Stop-Process -Id $master.Id, $worker.Id -Force -ErrorAction SilentlyContinue
    exit 1
}
Write-Host "✓ Worker registered (total: $($response.workers.Count))" -ForegroundColor Green

# Submit a test job
Write-Host ""
Write-Host "Submitting test job (map_add)..." -ForegroundColor Green
$jobOutput = & target\release\client.exe submit-job --name "demo-job" --operation "map_add" --param 10 --input "1,2,3,4,5"
$jobId = ($jobOutput | Select-String -Pattern "Job ID: ([a-f0-9-]+)").Matches.Groups[1].Value
Write-Host "Job ID: $jobId"

# Wait and check progress
Start-Sleep -Seconds 2
Write-Host "Checking job progress..." -ForegroundColor Green
& target\release\client.exe get-progress --job-id $jobId

# Get metrics
Write-Host ""
Write-Host "Getting metrics..." -ForegroundColor Green
$metrics = Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/v1/metrics" -Method GET
$metrics | ConvertTo-Json -Depth 10

# Cleanup
Write-Host ""
Write-Host "Stopping processes..." -ForegroundColor Yellow
Stop-Process -Id $master.Id, $worker.Id -Force -ErrorAction SilentlyContinue
Write-Host "Demo completed!" -ForegroundColor Green

