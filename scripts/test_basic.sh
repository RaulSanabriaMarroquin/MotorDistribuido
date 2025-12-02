#!/bin/bash
# Script de prueba básica del sistema distribuido (Bash/Linux/WSL)
# Prueba: Master -> Worker -> Cliente

set -e

echo "=== Prueba del Sistema Distribuido ==="
echo ""

# Colores
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Limpiar procesos anteriores si existen
echo -e "${YELLOW}Limpiando procesos anteriores...${NC}"
pkill -f "target/release/master" 2>/dev/null || true
pkill -f "target/release/worker" 2>/dev/null || true
sleep 2

# Verificar que los binarios existen
if [ ! -f "target/release/master" ] && [ ! -f "target/release/master.exe" ]; then
    echo -e "${YELLOW}Los binarios no están compilados. Compilando en modo release...${NC}"
    cargo build --release
    if [ $? -ne 0 ]; then
        echo -e "${RED}Error: Fallo al compilar. Revisa los errores arriba.${NC}"
        exit 1
    fi
fi

# Determinar qué binario usar (en WSL/Linux usar master, en Windows master.exe)
MASTER_BIN=""
if [ -f "target/release/master" ]; then
    MASTER_BIN="./target/release/master"
elif [ -f "target/release/master.exe" ]; then
    MASTER_BIN="./target/release/master.exe"
else
    echo -e "${RED}Error: No se encontró el binario del master${NC}"
    exit 1
fi

# Iniciar Master en background
echo -e "${GREEN}Iniciando Master en puerto 8080...${NC}"
echo -e "${CYAN}Usando binario: $MASTER_BIN${NC}"
$MASTER_BIN > /tmp/master.log 2>&1 &
MASTER_PID=$!

# Esperar y verificar que el master está corriendo
echo -e "${CYAN}Esperando a que el master inicie...${NC}"
for i in {1..10}; do
    sleep 1
    # Verificar si el proceso sigue corriendo
    if ! kill -0 $MASTER_PID 2>/dev/null; then
        echo -e "${RED}✗ Error: El proceso del master terminó inesperadamente${NC}"
        echo -e "${YELLOW}Últimas líneas del log del master:${NC}"
        cat /tmp/master.log 2>/dev/null || echo "No se pudo leer el log"
        exit 1
    fi
    # Verificar si el master responde
    if curl -s http://127.0.0.1:8080/api/v1/workers > /dev/null 2>&1; then
        echo -e "${GREEN}✓ Master está corriendo correctamente (intento $i/10)${NC}"
        break
    fi
    if [ $i -eq 10 ]; then
        echo -e "${RED}✗ Error: Master no responde después de 10 segundos${NC}"
        echo -e "${YELLOW}Últimas líneas del log del master:${NC}"
        cat /tmp/master.log 2>/dev/null || echo "No se pudo leer el log"
        echo ""
        echo -e "${YELLOW}Verificando si el proceso está corriendo...${NC}"
        ps aux | grep -E "target/release/master" | grep -v grep || echo "No se encontró el proceso"
        echo ""
        echo -e "${YELLOW}Verificando si el puerto 8080 está en uso...${NC}"
        netstat -tuln 2>/dev/null | grep :8080 || ss -tuln 2>/dev/null | grep :8080 || echo "No se pudo verificar el puerto"
        kill $MASTER_PID 2>/dev/null || true
        exit 1
    fi
done

# Iniciar Worker en background
echo -e "${GREEN}Iniciando Worker en puerto 9000...${NC}"
export WORKER_PORT=9000

# Determinar qué binario usar
WORKER_BIN=""
if [ -f "target/release/worker" ]; then
    WORKER_BIN="./target/release/worker"
elif [ -f "target/release/worker.exe" ]; then
    WORKER_BIN="./target/release/worker.exe"
else
    echo -e "${RED}Error: No se encontró el binario del worker${NC}"
    kill $MASTER_PID 2>/dev/null || true
    exit 1
fi

echo -e "${CYAN}Usando binario: $WORKER_BIN${NC}"
$WORKER_BIN > /tmp/worker.log 2>&1 &
WORKER_PID=$!
sleep 3

# Verificar que el worker se registró
sleep 2
WORKERS=$(curl -s http://127.0.0.1:8080/api/v1/workers | grep -o '"workers"' | wc -l || echo "0")
if [ "$WORKERS" -gt 0 ] || curl -s http://127.0.0.1:8080/api/v1/workers | grep -q "workers"; then
    echo -e "${GREEN}✓ Worker registrado correctamente${NC}"
else
    echo -e "${RED}✗ Error: Worker no se registró${NC}"
    kill $MASTER_PID $WORKER_PID 2>/dev/null || true
    exit 1
fi

# Enviar un job de prueba
echo ""
echo -e "${GREEN}Enviando job de prueba (map_add con input [1,2,3,4,5])...${NC}"

# Determinar qué binario usar
CLIENT_BIN=""
if [ -f "target/release/client" ]; then
    CLIENT_BIN="./target/release/client"
elif [ -f "target/release/client.exe" ]; then
    CLIENT_BIN="./target/release/client.exe"
else
    echo -e "${RED}Error: No se encontró el binario del client${NC}"
    kill $MASTER_PID $WORKER_PID 2>/dev/null || true
    exit 1
fi

JOB_OUTPUT=$($CLIENT_BIN submit-job --name "test-job" --operation "map_add" --param 10 --input "1,2,3,4,5" 2>&1)

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Job enviado correctamente${NC}"
    # Extraer job_id del output (formato: "Job ID: <uuid>")
    JOB_ID=$(echo "$JOB_OUTPUT" | grep -oP 'Job ID: \K[a-f0-9-]+' | head -1)
    if [ -n "$JOB_ID" ]; then
        echo -e "${CYAN}  Job ID: $JOB_ID${NC}"
        
        # Esperar un poco y verificar progreso
        sleep 2
        echo ""
        echo -e "${GREEN}Verificando progreso del job...${NC}"
        $CLIENT_BIN get-progress --job-id "$JOB_ID" || true
    fi
else
    echo -e "${RED}✗ Error al enviar job${NC}"
fi

# Limpiar
echo ""
echo -e "${YELLOW}Presiona Enter para detener los procesos...${NC}"
read -r
kill $MASTER_PID $WORKER_PID 2>/dev/null || true
echo -e "${GREEN}Procesos detenidos${NC}"

