.PHONY: build test run-master run-worker run-client clean help

# Default target
help:
	@echo "Available targets:"
	@echo "  build          - Build all components"
	@echo "  build-release  - Build all components in release mode"
	@echo "  test           - Run tests"
	@echo "  run-master     - Run the master node"
	@echo "  run-worker     - Run a worker node (set WORKER_PORT=9001 for multiple workers)"
	@echo "  run-client     - Run the client CLI"
	@echo "  clean          - Clean build artifacts"
	@echo ""
	@echo "Examples:"
	@echo "  make build"
	@echo "  make run-master"
	@echo "  WORKER_PORT=9001 make run-worker"

build:
	cargo build

build-release:
	cargo build --release

test:
	cargo test

run-master:
	cargo run --bin master

run-worker:
	cargo run --bin worker

run-client:
	cargo run --bin client

clean:
	cargo clean

# Demo script helpers
demo-start:
	@echo "Starting master and workers..."
	@echo "Run in separate terminals:"
	@echo "  Terminal 1: make run-master"
	@echo "  Terminal 2: make run-worker"
	@echo "  Terminal 3: WORKER_PORT=9001 make run-worker"
	@echo "  Terminal 4: make run-client -- list-workers"

