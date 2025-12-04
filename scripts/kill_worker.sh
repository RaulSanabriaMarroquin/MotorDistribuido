#!/bin/bash
# Script para simular la muerte de un worker
# Uso: ./kill_worker.sh <worker_number>

WORKER_NUM=${1:-1}

echo "⚠️  Para matar el worker $WORKER_NUM, presiona Ctrl+C en su terminal"
echo "   O encuentra el proceso y mátalo manualmente"
echo ""
echo "   En Linux/Mac puedes usar:"
echo "   pkill -f 'worker.*--port.*$((8080 + $WORKER_NUM))'"
echo ""
echo "   O busca el PID:"
echo "   ps aux | grep worker"

