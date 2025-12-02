# Makefile for Motor Distribuido
# Build, test, and demo scripts

.PHONY: help install build build-release test test-basic test-complete test-failure clean run-master run-worker run-client demo stop-all

# Default target
help:
	@echo "Motor Distribuido - Makefile"
	@echo ""
	@echo "Instalación:"
	@echo "  install        - Instalar dependencias y compilar (equivalente a: deps + build-release)"
	@echo ""
	@echo "Compilación:"
	@echo "  build          - Build in debug mode"
	@echo "  build-release  - Build in release mode (optimized)"
	@echo "  deps           - Descargar dependencias"
	@echo ""
	@echo "Pruebas:"
	@echo "  test           - Run unit tests"
	@echo "  test-basic     - Ejecutar prueba básica del sistema"
	@echo "  test-complete  - Ejecutar suite completa de pruebas"
	@echo "  test-failure   - Ejecutar prueba de simulación de fallo"
	@echo ""
	@echo "Ejecución:"
	@echo "  run-master     - Run master node"
	@echo "  run-worker     - Run worker node (set WORKER_PORT=9000)"
	@echo "  run-client     - Run client CLI"
	@echo "  stop-all       - Detener todos los procesos master/worker"
	@echo ""
	@echo "Utilidades:"
	@echo "  clean          - Clean build artifacts"
	@echo "  demo           - Run demo script"
	@echo "  fmt            - Formatear código"
	@echo "  lint           - Ejecutar clippy"
	@echo "  check          - Verificar código sin compilar"
	@echo ""

# Build targets
build:
	cargo build

build-release:
	cargo build --release

# Test targets
test:
	cargo test

# Test básico del sistema
test-basic:
	@echo "Ejecutando prueba básica del sistema..."
	@if command -v powershell >/dev/null 2>&1; then \
		powershell -ExecutionPolicy Bypass -File scripts/test_basic.ps1; \
	elif [ -f "scripts/test_basic.sh" ]; then \
		bash scripts/test_basic.sh; \
	else \
		echo "Error: No se encontró script de prueba"; \
		echo "En WSL/Linux, ejecuta: bash scripts/test_basic.sh"; \
		echo "O ejecuta tests unitarios: cargo test"; \
		exit 1; \
	fi

# Test completo del sistema
test-complete:
	@echo "Ejecutando suite completa de pruebas..."
	@if [ "$(OS)" = "Windows_NT" ]; then \
		if command -v powershell >/dev/null 2>&1; then \
			powershell -ExecutionPolicy Bypass -File scripts/test_complete.ps1; \
		else \
			echo "Error: PowerShell no encontrado"; \
			exit 1; \
		fi \
	elif [ -f "scripts/test_complete.sh" ]; then \
		bash scripts/test_complete.sh; \
	else \
		echo "Error: No se encontró script de prueba completa"; \
		echo "En WSL/Linux, ejecuta: bash scripts/test_complete.sh"; \
		exit 1; \
	fi

# Test de simulación de fallo
test-failure:
	@echo "Ejecutando prueba de simulación de fallo..."
	@if [ "$(OS)" = "Windows_NT" ]; then \
		if command -v powershell >/dev/null 2>&1; then \
			powershell -ExecutionPolicy Bypass -File scripts/test_failure.ps1; \
		else \
			echo "Error: PowerShell no encontrado"; \
			exit 1; \
		fi \
	elif [ -f "scripts/test_failure.sh" ]; then \
		bash scripts/test_failure.sh; \
	else \
		echo "Error: No se encontró script de prueba de fallo"; \
		echo "En WSL/Linux, ejecuta: bash scripts/test_failure.sh"; \
		exit 1; \
	fi

# Clean target
clean:
	cargo clean
	@echo "Build artifacts cleaned"

# Run targets
run-master:
	cargo run --release --bin master

run-worker:
	@if [ -z "$(WORKER_PORT)" ]; then \
		echo "Usage: make run-worker WORKER_PORT=9000"; \
		exit 1; \
	fi
	WORKER_PORT=$(WORKER_PORT) cargo run --release --bin worker

run-client:
	cargo run --release --bin client

# Demo script
demo:
	@echo "Running demo..."
	@bash scripts/demo.sh || powershell -ExecutionPolicy Bypass -File scripts/demo.ps1

# Install dependencies and build
install: deps build-release
	@echo "Instalación completada. Binarios disponibles en target/release/"

# Install dependencies (if needed)
deps:
	cargo fetch

# Format code
fmt:
	cargo fmt

# Lint code
lint:
	cargo clippy -- -D warnings

# Check code
check:
	cargo check

# Stop all master/worker processes
stop-all:
	@echo "Deteniendo todos los procesos master/worker..."
	@if command -v powershell >/dev/null 2>&1; then \
		powershell -Command "Get-Process | Where-Object {$$_.ProcessName -like '*master*' -or $$_.ProcessName -like '*worker*'} | Stop-Process -Force -ErrorAction SilentlyContinue"; \
	else \
		pkill -f "target/release/master" || true; \
		pkill -f "target/release/worker" || true; \
		pkill -f "master" || true; \
		pkill -f "worker" || true; \
	fi
	@echo "Procesos detenidos"

