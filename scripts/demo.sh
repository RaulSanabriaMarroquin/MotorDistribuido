#!/bin/bash
# Demo script for Motor Distribuido

set -e

echo "=== Motor Distribuido Demo ==="
echo ""

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Check if binaries exist
if [ ! -f "target/release/master" ] && [ ! -f "target/release/master.exe" ]; then
    echo -e "${YELLOW}Building release binaries...${NC}"
    cargo build --release
fi

# Clean up any existing processes
echo -e "${YELLOW}Cleaning up existing processes...${NC}"
pkill -f "target/release/master" || true
pkill -f "target/release/worker" || true
sleep 2

# Start master
echo -e "${GREEN}Starting master on port 8080...${NC}"
if [ -f "target/release/master.exe" ]; then
    ./target/release/master.exe &
else
    ./target/release/master &
fi
MASTER_PID=$!
sleep 3

# Verify master is running
if ! curl -s http://127.0.0.1:8080/api/v1/workers > /dev/null; then
    echo -e "${RED}Error: Master failed to start${NC}"
    kill $MASTER_PID 2>/dev/null || true
    exit 1
fi
echo -e "${GREEN}✓ Master is running${NC}"

# Start worker
echo -e "${GREEN}Starting worker on port 9000...${NC}"
WORKER_PORT=9000
if [ -f "target/release/worker.exe" ]; then
    WORKER_PORT=$WORKER_PORT ./target/release/worker.exe &
else
    WORKER_PORT=$WORKER_PORT ./target/release/worker &
fi
WORKER_PID=$!
sleep 3

# Verify worker registered
WORKERS=$(curl -s http://127.0.0.1:8080/api/v1/workers | jq '.workers | length')
if [ "$WORKERS" -eq "0" ]; then
    echo -e "${RED}Error: Worker failed to register${NC}"
    kill $MASTER_PID $WORKER_PID 2>/dev/null || true
    exit 1
fi
echo -e "${GREEN}✓ Worker registered (total: $WORKERS)${NC}"

# Submit a test job
echo ""
echo -e "${GREEN}Submitting test job (map_add)...${NC}"
if [ -f "target/release/client.exe" ]; then
    JOB_OUTPUT=$(./target/release/client.exe submit-job --name "demo-job" --operation "map_add" --param 10 --input "1,2,3,4,5")
else
    JOB_OUTPUT=$(./target/release/client submit-job --name "demo-job" --operation "map_add" --param 10 --input "1,2,3,4,5")
fi

JOB_ID=$(echo "$JOB_OUTPUT" | grep "Job ID:" | awk '{print $3}')
echo "Job ID: $JOB_ID"

# Wait and check progress
sleep 2
echo -e "${GREEN}Checking job progress...${NC}"
if [ -f "target/release/client.exe" ]; then
    ./target/release/client.exe get-progress --job-id "$JOB_ID"
else
    ./target/release/client get-progress --job-id "$JOB_ID"
fi

# Get metrics
echo ""
echo -e "${GREEN}Getting metrics...${NC}"
curl -s http://127.0.0.1:8080/api/v1/metrics | jq '.'

# Cleanup
echo ""
echo -e "${YELLOW}Stopping processes...${NC}"
kill $MASTER_PID $WORKER_PID 2>/dev/null || true
echo -e "${GREEN}Demo completed!${NC}"

