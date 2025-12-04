#!/bin/bash
# Script para limpiar procesos y archivos temporales

echo "🧹 Limpiando procesos..."

# Matar procesos del master y workers
pkill -f "cargo run --bin master" 2>/dev/null
pkill -f "cargo run --bin worker" 2>/dev/null

echo "✅ Limpieza completada"
echo ""
echo "Nota: Los archivos en scripts/data/ se mantienen para futuras ejecuciones"
echo "      Si quieres eliminarlos, ejecuta: rm -rf scripts/data/"

