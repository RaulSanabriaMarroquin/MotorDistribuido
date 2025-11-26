# Makefile for Motor Distribuido
# Build, test, and demo scripts

.PHONY: help build build-release test clean run-master run-worker run-client demo

# Default target
help:
	@echo "Motor Distribuido - Makefile"
	@echo ""
	@echo "Targets:"
	@echo "  build          - Build in debug mode"
	@echo "  build-release  - Build in release mode (optimized)"
	@echo "  test           - Run tests"
	@echo "  clean          - Clean build artifacts"
	@echo "  run-master     - Run master node"
	@echo "  run-worker     - Run worker node (set WORKER_PORT=9000)"
	@echo "  run-client     - Run client CLI"
	@echo "  demo           - Run demo script"
	@echo ""

# Build targets
build:
	cargo build

build-release:
	cargo build --release

# Test target
test:
	cargo test

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

