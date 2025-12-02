# Script PowerShell equivalente al Makefile
# Uso: .\make.ps1 <target>

param(
    [Parameter(Position=0)]
    [string]$Target = "help"
)

$ErrorActionPreference = "Continue"

function Show-Help {
    Write-Host "Motor Distribuido - Makefile (PowerShell)" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Instalación:" -ForegroundColor Yellow
    Write-Host "  install        - Instalar dependencias y compilar (equivalente a: deps + build-release)"
    Write-Host ""
    Write-Host "Compilación:" -ForegroundColor Yellow
    Write-Host "  build          - Build in debug mode"
    Write-Host "  build-release  - Build in release mode (optimized)"
    Write-Host "  deps           - Descargar dependencias"
    Write-Host ""
    Write-Host "Pruebas:" -ForegroundColor Yellow
    Write-Host "  test           - Run unit tests"
    Write-Host "  test-basic     - Ejecutar prueba básica del sistema"
    Write-Host "  test-complete  - Ejecutar suite completa de pruebas"
    Write-Host "  test-failure   - Ejecutar prueba de simulación de fallo"
    Write-Host ""
    Write-Host "Ejecución:" -ForegroundColor Yellow
    Write-Host "  run-master     - Run master node"
    Write-Host "  run-worker     - Run worker node (requiere: -WorkerPort 9000)"
    Write-Host "  run-client     - Run client CLI"
    Write-Host "  stop-all       - Detener todos los procesos master/worker"
    Write-Host ""
    Write-Host "Utilidades:" -ForegroundColor Yellow
    Write-Host "  clean          - Clean build artifacts"
    Write-Host "  demo           - Run demo script"
    Write-Host "  fmt            - Formatear código"
    Write-Host "  lint           - Ejecutar clippy"
    Write-Host "  check          - Verificar código sin compilar"
    Write-Host ""
}

function Invoke-Build {
    Write-Host "Compilando en modo debug..." -ForegroundColor Green
    cargo build
}

function Invoke-BuildRelease {
    Write-Host "Compilando en modo release..." -ForegroundColor Green
    cargo build --release
}

function Invoke-Test {
    Write-Host "Ejecutando tests unitarios..." -ForegroundColor Green
    cargo test
}

function Invoke-TestBasic {
    Write-Host "Ejecutando prueba básica del sistema..." -ForegroundColor Green
    & powershell -ExecutionPolicy Bypass -File scripts/test_basic.ps1
}

function Invoke-TestComplete {
    Write-Host "Ejecutando suite completa de pruebas..." -ForegroundColor Green
    & powershell -ExecutionPolicy Bypass -File scripts/test_complete.ps1
}

function Invoke-TestFailure {
    Write-Host "Ejecutando prueba de simulación de fallo..." -ForegroundColor Green
    & powershell -ExecutionPolicy Bypass -File scripts/test_failure.ps1
}

function Invoke-Clean {
    Write-Host "Limpiando artefactos de compilación..." -ForegroundColor Yellow
    cargo clean
    Write-Host "Build artifacts cleaned" -ForegroundColor Green
}

function Invoke-RunMaster {
    Write-Host "Iniciando master node..." -ForegroundColor Green
    cargo run --release --bin master
}

function Invoke-RunWorker {
    param([int]$WorkerPort = 9000)
    
    if (-not $WorkerPort) {
        Write-Host "Error: Se requiere especificar WORKER_PORT" -ForegroundColor Red
        Write-Host "Uso: .\make.ps1 run-worker -WorkerPort 9000" -ForegroundColor Yellow
        exit 1
    }
    
    Write-Host "Iniciando worker node en puerto $WorkerPort..." -ForegroundColor Green
    $env:WORKER_PORT = $WorkerPort.ToString()
    cargo run --release --bin worker
}

function Invoke-RunClient {
    Write-Host "Iniciando cliente CLI..." -ForegroundColor Green
    cargo run --release --bin client
}

function Invoke-Demo {
    Write-Host "Ejecutando demo..." -ForegroundColor Green
    if (Test-Path "scripts/demo.ps1") {
        & powershell -ExecutionPolicy Bypass -File scripts/demo.ps1
    } elseif (Test-Path "scripts/demo.sh") {
        bash scripts/demo.sh
    } else {
        Write-Host "Error: No se encontró script de demo" -ForegroundColor Red
        exit 1
    }
}

function Invoke-Install {
    Write-Host "Instalando dependencias..." -ForegroundColor Green
    cargo fetch
    
    Write-Host "Compilando en modo release..." -ForegroundColor Green
    cargo build --release
    
    Write-Host "Instalación completada. Binarios disponibles en target/release/" -ForegroundColor Green
}

function Invoke-Deps {
    Write-Host "Descargando dependencias..." -ForegroundColor Green
    cargo fetch
}

function Invoke-Fmt {
    Write-Host "Formateando código..." -ForegroundColor Green
    cargo fmt
}

function Invoke-Lint {
    Write-Host "Ejecutando clippy..." -ForegroundColor Green
    cargo clippy -- -D warnings
}

function Invoke-Check {
    Write-Host "Verificando código..." -ForegroundColor Green
    cargo check
}

function Invoke-StopAll {
    Write-Host "Deteniendo todos los procesos master/worker..." -ForegroundColor Yellow
    Get-Process | Where-Object {$_.ProcessName -like '*master*' -or $_.ProcessName -like '*worker*'} | Stop-Process -Force -ErrorAction SilentlyContinue
    Write-Host "Procesos detenidos" -ForegroundColor Green
}

# Main dispatch
switch ($Target.ToLower()) {
    "help" { Show-Help }
    "build" { Invoke-Build }
    "build-release" { Invoke-BuildRelease }
    "test" { Invoke-Test }
    "test-basic" { Invoke-TestBasic }
    "test-complete" { Invoke-TestComplete }
    "test-failure" { Invoke-TestFailure }
    "clean" { Invoke-Clean }
    "run-master" { Invoke-RunMaster }
    "run-worker" { 
        $port = $env:WORKER_PORT
        if (-not $port) { $port = 9000 }
        Invoke-RunWorker -WorkerPort ([int]$port)
    }
    "run-client" { Invoke-RunClient }
    "demo" { Invoke-Demo }
    "install" { Invoke-Install }
    "deps" { Invoke-Deps }
    "fmt" { Invoke-Fmt }
    "lint" { Invoke-Lint }
    "check" { Invoke-Check }
    "stop-all" { Invoke-StopAll }
    default {
        Write-Host "Target desconocido: $Target" -ForegroundColor Red
        Write-Host ""
        Show-Help
        exit 1
    }
}

