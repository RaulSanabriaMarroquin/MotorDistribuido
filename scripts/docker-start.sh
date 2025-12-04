#!/bin/bash
# Script para iniciar el sistema usando docker-compose

echo "🐳 Iniciando Motor Distribuido con Docker Compose..."
echo ""

# Construir las imágenes
echo "📦 Construyendo imágenes..."
docker-compose build

# Iniciar los servicios
echo "🚀 Iniciando servicios (master + 3 workers)..."
docker-compose up -d

echo ""
echo "✅ Sistema iniciado!"
echo ""
echo "Servicios disponibles:"
echo "  - Master: http://localhost:8080"
echo "  - Worker 1: http://localhost:8081"
echo "  - Worker 2: http://localhost:8082"
echo "  - Worker 3: http://localhost:8083"
echo ""
echo "Para ver los logs:"
echo "  docker-compose logs -f"
echo ""
echo "Para detener el sistema:"
echo "  docker-compose down"
echo ""
echo "Para listar workers:"
echo "  docker-compose exec master ./client list-workers"
echo "  (o desde el host: cargo run --bin client list-workers)"

