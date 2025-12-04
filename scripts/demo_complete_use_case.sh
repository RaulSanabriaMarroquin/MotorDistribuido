#!/bin/bash
# Script para demostrar un caso de uso completo

echo "🎯 Caso de uso completo: Análisis de datos distribuido"
echo ""

# Asegurar que los datos existen
if [ ! -f "scripts/data/sample.csv" ]; then
    echo "📝 Creando datos de ejemplo..."
    ./scripts/create_sample_data.sh
fi

echo "📊 Paso 1: Cargar datos desde CSV"
echo "📊 Paso 2: Aplicar transformación (multiplicar por 2)"
echo "📊 Paso 3: Filtrar valores mayores a 20"
echo ""

JOB_OUTPUT=$(cargo run --bin client submit-job \
  --name "demo-complete-analysis" \
  --source-csv scripts/data/sample.csv \
  --stages "map_mul:2,filter_gt:20" 2>&1)

JOB_ID=$(echo "$JOB_OUTPUT" | grep -oP 'job_id["\s:]+"\K[^"]+' | head -1)

if [ -z "$JOB_ID" ]; then
    echo "❌ Error al obtener job_id"
    exit 1
fi

echo "✅ Job enviado con ID: $JOB_ID"
echo "⏳ Esperando 5 segundos para procesamiento..."
sleep 5

echo ""
echo "📈 Resultado final:"
cargo run --bin client get-progress --job-id "$JOB_ID"

