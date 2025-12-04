#!/bin/bash
# Script para crear archivos de datos de ejemplo

mkdir -p scripts/data

# Crear archivo CSV de ejemplo
echo "value" > scripts/data/sample.csv
for i in {1..20}; do
    echo "$i" >> scripts/data/sample.csv
done

# Crear archivo JSONL de ejemplo
> scripts/data/sample.jsonl
for i in {1..20}; do
    echo "{\"value\": $i}" >> scripts/data/sample.jsonl
done

# Crear archivo plain text de ejemplo
> scripts/data/sample.txt
for i in {1..20}; do
    echo "$i" >> scripts/data/sample.txt
done

echo "✅ Archivos de datos creados en scripts/data/"

