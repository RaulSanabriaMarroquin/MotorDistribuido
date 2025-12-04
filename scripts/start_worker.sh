#!/bin/bash
# Script para iniciar un Worker
# Uso: ./start_worker.sh <worker_number>

WORKER_NUM=${1:-1}
PORT=$((8081 + $WORKER_NUM - 1))

echo "🚀 Iniciando Worker $WORKER_NUM en puerto $PORT..."
export MASTER_URL="http://127.0.0.1:8080"
export WORKER_PORT=$PORT
export WORKER_HOST="127.0.0.1"
cargo run --bin worker

