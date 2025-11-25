#!/bin/bash
# Demo script for the distributed system
# This script helps start the master and workers for demonstration

set -e

echo "=========================================="
echo "Distributed System Demo"
echo "=========================================="
echo ""

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo "Error: cargo is not installed. Please install Rust first."
    exit 1
fi

# Build the project
echo -e "${YELLOW}Building project...${NC}"
cargo build --release
echo -e "${GREEN}Build complete!${NC}"
echo ""

# Check if master is already running
if lsof -Pi :8080 -sTCP:LISTEN -t >/dev/null 2>&1 ; then
    echo -e "${YELLOW}Warning: Port 8080 is already in use. Master might already be running.${NC}"
    read -p "Continue anyway? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

echo "=========================================="
echo "Starting components..."
echo "=========================================="
echo ""
echo "To run the demo:"
echo "  1. Start master:    make run-master"
echo "  2. Start worker 1:  make run-worker"
echo "  3. Start worker 2:  WORKER_PORT=9001 make run-worker"
echo "  4. Use client:      make run-client -- list-workers"
echo ""
echo "Or use docker-compose:"
echo "  docker-compose up"
echo ""
echo "Press Ctrl+C to exit this help message"
echo ""

# Option to start with docker-compose
read -p "Start with docker-compose? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    if command -v docker-compose &> /dev/null; then
        echo -e "${GREEN}Starting with docker-compose...${NC}"
        docker-compose up
    else
        echo "Error: docker-compose is not installed"
        exit 1
    fi
fi

