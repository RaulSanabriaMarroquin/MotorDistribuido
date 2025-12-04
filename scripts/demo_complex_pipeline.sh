#!/bin/bash
# Script para demostrar un pipeline complejo

echo "📊 Ejecutando pipeline complejo: Map -> Filter -> Reduce"

JOB_OUTPUT=$(cargo run --bin client submit-job \
  --name "demo-complex-pipeline" \
  --source-inline "1,10,2,20,1,5,3,15,2,10,4,25,3,20" \
  --stages "reduce_by_key" 2>&1)

JOB_ID=$(echo "$JOB_OUTPUT" | grep -oP 'job_id["\s:]+"\K[^"]+' | head -1)

if [ -z "$JOB_ID" ]; then
    echo "❌ Error al obtener job_id"
    exit 1
fi

echo "✅ Job enviado con ID: $JOB_ID"
echo "⏳ Esperando 3 segundos..."
sleep 3

echo "📈 Consultando progreso..."
cargo run --bin client get-progress --job-id "$JOB_ID"

