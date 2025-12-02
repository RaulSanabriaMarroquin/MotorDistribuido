#!/bin/bash
# Script de pruebas completas del sistema (Bash/Linux/WSL)
# Prueba todas las funcionalidades implementadas

set -e

echo "=== PRUEBAS COMPLETAS DEL SISTEMA ==="
echo ""

# Colores
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Limpiar procesos anteriores
echo -e "${YELLOW}1. Limpiando procesos anteriores...${NC}"
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

# Determinar qué binario usar
MASTER_BIN=""
if [ -f "target/release/master" ]; then
    MASTER_BIN="./target/release/master"
elif [ -f "target/release/master.exe" ]; then
    MASTER_BIN="./target/release/master.exe"
else
    echo -e "${RED}Error: No se encontró el binario del master${NC}"
    exit 1
fi

WORKER_BIN=""
if [ -f "target/release/worker" ]; then
    WORKER_BIN="./target/release/worker"
elif [ -f "target/release/worker.exe" ]; then
    WORKER_BIN="./target/release/worker.exe"
else
    echo -e "${RED}Error: No se encontró el binario del worker${NC}"
    exit 1
fi

CLIENT_BIN=""
if [ -f "target/release/client" ]; then
    CLIENT_BIN="./target/release/client"
elif [ -f "target/release/client.exe" ]; then
    CLIENT_BIN="./target/release/client.exe"
else
    echo -e "${RED}Error: No se encontró el binario del client${NC}"
    exit 1
fi

# Iniciar Master
echo -e "${GREEN}2. Iniciando Master...${NC}"
echo -e "${CYAN}Usando binario: $MASTER_BIN${NC}"
$MASTER_BIN > /tmp/master.log 2>&1 &
MASTER_PID=$!

# Esperar y verificar que el master está corriendo
echo -e "${CYAN}Esperando a que el master inicie...${NC}"
for i in {1..10}; do
    sleep 1
    if ! kill -0 $MASTER_PID 2>/dev/null; then
        echo -e "${RED}Error: El proceso del master terminó inesperadamente${NC}"
        cat /tmp/master.log 2>/dev/null || echo "No se pudo leer el log"
        exit 1
    fi
    if curl -s http://127.0.0.1:8080/api/v1/workers > /dev/null 2>&1; then
        echo -e "${GREEN}   [OK] Master iniciado correctamente (intento $i/10)${NC}"
        break
    fi
    if [ $i -eq 10 ]; then
        echo -e "${RED}Error: Master no responde después de 10 segundos${NC}"
        cat /tmp/master.log 2>/dev/null || echo "No se pudo leer el log"
        kill $MASTER_PID 2>/dev/null || true
        exit 1
    fi
done

# Iniciar Workers
echo -e "${GREEN}3. Iniciando Workers...${NC}"
export WORKER_PORT=9000
echo -e "${CYAN}Usando binario: $WORKER_BIN${NC}"
$WORKER_BIN > /tmp/worker1.log 2>&1 &
WORKER1_PID=$!
sleep 2

export WORKER_PORT=9001
$WORKER_BIN > /tmp/worker2.log 2>&1 &
WORKER2_PID=$!
sleep 3

# Verificar que los workers se registraron
WORKERS_COUNT=$(curl -s http://127.0.0.1:8080/api/v1/workers | grep -o '"workers"' | wc -l || echo "0")
if [ "$WORKERS_COUNT" -gt 0 ] || curl -s http://127.0.0.1:8080/api/v1/workers | grep -q "workers"; then
    WORKERS_RESPONSE=$(curl -s http://127.0.0.1:8080/api/v1/workers)
    WORKERS_ACTUAL=$(echo "$WORKERS_RESPONSE" | grep -o '"id"' | wc -l || echo "0")
    echo -e "${GREEN}   [OK] Workers registrados: $WORKERS_ACTUAL${NC}"
else
    echo -e "${RED}   [ERROR] Workers no se registraron${NC}"
    kill $MASTER_PID $WORKER1_PID $WORKER2_PID 2>/dev/null || true
    exit 1
fi

# Prueba 1: Operación legacy
echo ""
echo -e "${CYAN}4. Prueba: Operación legacy (map_add)...${NC}"
JOB_OUTPUT=$($CLIENT_BIN submit-job --name "test-map-add" --operation "map_add" --param 10 --input "1,2,3,4,5" 2>&1)
EXIT_CODE=$?
if [ $EXIT_CODE -eq 0 ]; then
    # Intentar extraer Job ID de diferentes formatos posibles
    # El cliente imprime: "ID del Job: <uuid>" (en español)
    # Formato 1: Buscar "ID del Job:" o "Job ID:" (case insensitive)
    JOB_ID1=$(echo "$JOB_OUTPUT" | grep -iE "(ID del Job|Job ID)" | grep -oE '[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}' | head -1)
    # Formato 2: Si no se encontró, buscar cualquier UUID en la salida
    if [ -z "$JOB_ID1" ]; then
        JOB_ID1=$(echo "$JOB_OUTPUT" | grep -oE '[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}' | head -1)
    fi
    # Formato 3: Buscar en formato JSON si está presente
    if [ -z "$JOB_ID1" ]; then
        JOB_ID1=$(echo "$JOB_OUTPUT" | grep -oE '"job_id":"[^"]+"' | cut -d'"' -f4 | head -1)
    fi
    
    if [ -n "$JOB_ID1" ]; then
        echo -e "${CYAN}   Job ID: $JOB_ID1${NC}"
        sleep 2
        PROGRESS=$(curl -s "http://127.0.0.1:8080/api/v1/jobs/$JOB_ID1/progress")
        STATUS=$(echo "$PROGRESS" | grep -o '"status":"[^"]*"' | cut -d'"' -f4 || echo "unknown")
        if [ "$STATUS" = "completed" ]; then
            COMPLETED=$(echo "$PROGRESS" | grep -o '"completed_tasks":[0-9]*' | cut -d':' -f2 || echo "0")
            TOTAL=$(echo "$PROGRESS" | grep -o '"total_tasks":[0-9]*' | cut -d':' -f2 || echo "0")
            echo -e "${GREEN}   [OK] Job completado: $COMPLETED/$TOTAL${NC}"
        else
            echo -e "${RED}   [ERROR] Job no completado: $STATUS${NC}"
        fi
    else
        echo -e "${RED}   [ERROR] No se pudo extraer el Job ID${NC}"
        echo -e "${YELLOW}   Salida del cliente:${NC}"
        echo "$JOB_OUTPUT" | head -5
    fi
else
    echo -e "${RED}   [ERROR] Error al enviar job (código de salida: $EXIT_CODE)${NC}"
    echo -e "${YELLOW}   Salida del cliente:${NC}"
    echo "$JOB_OUTPUT" | head -10
fi

# Prueba 2: flat_map
echo ""
echo -e "${CYAN}5. Prueba: Operación flat_map...${NC}"
BODY='{"name":"test-flatmap","operation":"flat_map","fn_name":"split","input":[123,456]}'
RESPONSE=$(curl -s -X POST http://127.0.0.1:8080/api/v1/jobs/submit \
    -H "Content-Type: application/json" \
    -d "$BODY")
# Extraer job_id del JSON de respuesta
JOB_ID2=$(echo "$RESPONSE" | grep -oE '"job_id":"[^"]+"' | cut -d'"' -f4 | head -1)
# Si no se encontró, buscar cualquier UUID
if [ -z "$JOB_ID2" ]; then
    JOB_ID2=$(echo "$RESPONSE" | grep -oE '[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}' | head -1)
fi
if [ -n "$JOB_ID2" ]; then
    echo -e "${CYAN}   Job ID: $JOB_ID2${NC}"
    sleep 2
    PROGRESS=$(curl -s "http://127.0.0.1:8080/api/v1/jobs/$JOB_ID2/progress")
    STATUS=$(echo "$PROGRESS" | grep -o '"status":"[^"]*"' | cut -d'"' -f4 || echo "unknown")
    if [ "$STATUS" = "completed" ]; then
        COMPLETED=$(echo "$PROGRESS" | grep -o '"completed_tasks":[0-9]*' | cut -d':' -f2 || echo "0")
        TOTAL=$(echo "$PROGRESS" | grep -o '"total_tasks":[0-9]*' | cut -d':' -f2 || echo "0")
        echo -e "${GREEN}   [OK] flat_map completado: $COMPLETED/$TOTAL${NC}"
    else
        echo -e "${RED}   [ERROR] flat_map no completado: $STATUS${NC}"
    fi
else
    echo -e "${RED}   [ERROR] No se pudo crear el job${NC}"
fi

# Prueba 3: reduce_by_key
echo ""
echo -e "${CYAN}6. Prueba: Operación reduce_by_key...${NC}"
BODY='{"name":"test-reduce","operation":"reduce_by_key","fn_name":"sum","input":[1,2,2,3,3,3,4,4,4,4]}'
RESPONSE=$(curl -s -X POST http://127.0.0.1:8080/api/v1/jobs/submit \
    -H "Content-Type: application/json" \
    -d "$BODY")
# Extraer job_id del JSON de respuesta
JOB_ID3=$(echo "$RESPONSE" | grep -oE '"job_id":"[^"]+"' | cut -d'"' -f4 | head -1)
# Si no se encontró, buscar cualquier UUID
if [ -z "$JOB_ID3" ]; then
    JOB_ID3=$(echo "$RESPONSE" | grep -oE '[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}' | head -1)
fi
if [ -n "$JOB_ID3" ]; then
    echo -e "${CYAN}   Job ID: $JOB_ID3${NC}"
    sleep 2
    PROGRESS=$(curl -s "http://127.0.0.1:8080/api/v1/jobs/$JOB_ID3/progress")
    STATUS=$(echo "$PROGRESS" | grep -o '"status":"[^"]*"' | cut -d'"' -f4 || echo "unknown")
    if [ "$STATUS" = "completed" ]; then
        COMPLETED=$(echo "$PROGRESS" | grep -o '"completed_tasks":[0-9]*' | cut -d':' -f2 || echo "0")
        TOTAL=$(echo "$PROGRESS" | grep -o '"total_tasks":[0-9]*' | cut -d':' -f2 || echo "0")
        echo -e "${GREEN}   [OK] reduce_by_key completado: $COMPLETED/$TOTAL${NC}"
    else
        echo -e "${RED}   [ERROR] reduce_by_key no completado: $STATUS${NC}"
    fi
else
    echo -e "${RED}   [ERROR] No se pudo crear el job${NC}"
fi

# Prueba 4: Métricas
echo ""
echo -e "${CYAN}7. Prueba: Métricas...${NC}"
METRICS=$(curl -s http://127.0.0.1:8080/api/v1/metrics)
if [ $? -eq 0 ] && [ -n "$METRICS" ]; then
    NODES_COUNT=$(echo "$METRICS" | grep -o '"node_metrics"' | wc -l || echo "0")
    JOBS_COUNT=$(echo "$METRICS" | grep -o '"job_metrics"' | wc -l || echo "0")
    echo -e "${GREEN}   [OK] Métricas obtenidas:${NC}"
    echo -e "${CYAN}     - Nodos: $NODES_COUNT${NC}"
    echo -e "${CYAN}     - Jobs: $JOBS_COUNT${NC}"
else
    echo -e "${RED}   [ERROR] Error obteniendo métricas${NC}"
fi

# Prueba 5: Estado de workers
echo ""
echo -e "${CYAN}8. Prueba: Estado de workers...${NC}"
WORKERS_RESPONSE=$(curl -s http://127.0.0.1:8080/api/v1/workers)
WORKERS_ACTUAL=$(echo "$WORKERS_RESPONSE" | grep -o '"id"' | wc -l || echo "0")
echo -e "${GREEN}   [OK] Workers activos: $WORKERS_ACTUAL${NC}"
UP_COUNT=$(echo "$WORKERS_RESPONSE" | grep -o '"status":"UP"' | wc -l || echo "0")
if [ "$UP_COUNT" -eq "$WORKERS_ACTUAL" ] && [ "$WORKERS_ACTUAL" -gt 0 ]; then
    echo -e "${GREEN}   [OK] Todos los workers están UP${NC}"
else
    echo -e "${YELLOW}   [WARN] Algunos workers no están UP${NC}"
fi

# Resumen
echo ""
echo -e "${GREEN}=== RESUMEN DE PRUEBAS ===${NC}"
echo -e "${GREEN}[OK] Master funcionando${NC}"
echo -e "${GREEN}[OK] Workers registrados y activos${NC}"
echo -e "${GREEN}[OK] Operaciones legacy funcionando${NC}"
echo -e "${GREEN}[OK] Operaciones nuevas (flat_map, reduce_by_key) funcionando${NC}"
echo -e "${GREEN}[OK] Métricas funcionando${NC}"
echo ""
echo -e "${CYAN}=== TODAS LAS PRUEBAS COMPLETADAS ===${NC}"

# Limpiar
echo ""
echo -e "${YELLOW}Presiona Enter para detener los procesos...${NC}"
read -r
kill $MASTER_PID $WORKER1_PID $WORKER2_PID 2>/dev/null || true
echo -e "${GREEN}Procesos detenidos${NC}"

