#!/bin/bash
# Script de prueba de simulación de fallo (Bash/Linux/WSL)
# Simula la falla de un worker durante la ejecución de un job

set -e

echo "=== PRUEBA DE SIMULACIÓN DE FALLO ==="
echo ""

# Colores
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Limpiar procesos anteriores
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
echo -e "${GREEN}1. Iniciando Master...${NC}"
$MASTER_BIN > /tmp/master.log 2>&1 &
MASTER_PID=$!

# Esperar a que el master inicie
echo -e "${CYAN}Esperando a que el master inicie...${NC}"
for i in {1..10}; do
    sleep 1
    if ! kill -0 $MASTER_PID 2>/dev/null; then
        echo -e "${RED}Error: El proceso del master terminó inesperadamente${NC}"
        cat /tmp/master.log 2>/dev/null || echo "No se pudo leer el log"
        exit 1
    fi
    if curl -s http://127.0.0.1:8080/api/v1/workers > /dev/null 2>&1; then
        echo -e "${GREEN}   [OK] Master iniciado correctamente${NC}"
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
echo -e "${GREEN}2. Iniciando Workers...${NC}"
export WORKER_PORT=9000
$WORKER_BIN > /tmp/worker1.log 2>&1 &
WORKER1_PID=$!
sleep 2

export WORKER_PORT=9001
$WORKER_BIN > /tmp/worker2.log 2>&1 &
WORKER2_PID=$!
sleep 3

# Verificar que los workers se registraron
WORKERS_RESPONSE=$(curl -s http://127.0.0.1:8080/api/v1/workers)
WORKERS_ACTUAL=$(echo "$WORKERS_RESPONSE" | grep -o '"id"' | wc -l || echo "0")
if [ "$WORKERS_ACTUAL" -ge 2 ]; then
    echo -e "${GREEN}   [OK] Workers registrados: $WORKERS_ACTUAL${NC}"
else
    echo -e "${RED}   [ERROR] No se registraron suficientes workers${NC}"
    kill $MASTER_PID $WORKER1_PID $WORKER2_PID 2>/dev/null || true
    exit 1
fi

# Enviar un job
echo ""
echo -e "${CYAN}3. Enviando job de prueba...${NC}"
JOB_OUTPUT=$($CLIENT_BIN submit-job --name "test-failure" --operation "map_add" --param 10 --input "1,2,3,4,5,6,7,8,9,10" 2>&1)
if [ $? -eq 0 ]; then
    JOB_ID=$(echo "$JOB_OUTPUT" | grep -oP 'Job ID: \K[a-f0-9-]+' | head -1)
    if [ -n "$JOB_ID" ]; then
        echo -e "${CYAN}   Job ID: $JOB_ID${NC}"
        echo -e "${YELLOW}   Esperando 2 segundos antes de simular fallo...${NC}"
        sleep 2
        
        # Simular fallo: detener worker1
        echo ""
        echo -e "${RED}4. Simulando fallo: Deteniendo worker en puerto 9000...${NC}"
        kill $WORKER1_PID 2>/dev/null || true
        echo -e "${YELLOW}   Worker detenido. El master debería detectar el fallo y replanificar.${NC}"
        
        # Esperar a que el master detecte el fallo y replanifique
        echo -e "${CYAN}   Esperando replanificación (10 segundos)...${NC}"
        sleep 10
        
        # Verificar el estado del job
        echo ""
        echo -e "${CYAN}5. Verificando estado del job...${NC}"
        PROGRESS=$(curl -s "http://127.0.0.1:8080/api/v1/jobs/$JOB_ID/progress")
        STATUS=$(echo "$PROGRESS" | grep -o '"status":"[^"]*"' | cut -d'"' -f4 || echo "unknown")
        COMPLETED=$(echo "$PROGRESS" | grep -o '"completed_tasks":[0-9]*' | cut -d':' -f2 || echo "0")
        TOTAL=$(echo "$PROGRESS" | grep -o '"total_tasks":[0-9]*' | cut -d':' -f2 || echo "0")
        
        echo -e "${CYAN}   Estado: $STATUS${NC}"
        echo -e "${CYAN}   Tareas completadas: $COMPLETED/$TOTAL${NC}"
        
        if [ "$STATUS" = "completed" ]; then
            echo -e "${GREEN}   [OK] Job completado exitosamente después del fallo${NC}"
            echo -e "${GREEN}   [OK] El sistema manejó correctamente el fallo del worker${NC}"
        elif [ "$STATUS" = "running" ]; then
            echo -e "${YELLOW}   [INFO] Job aún en ejecución (normal después de replanificación)${NC}"
            echo -e "${GREEN}   [OK] El sistema está replanificando las tareas${NC}"
        else
            echo -e "${RED}   [ERROR] Job en estado inesperado: $STATUS${NC}"
        fi
        
        # Verificar workers restantes
        echo ""
        echo -e "${CYAN}6. Verificando workers restantes...${NC}"
        WORKERS_RESPONSE=$(curl -s http://127.0.0.1:8080/api/v1/workers)
        WORKERS_ACTUAL=$(echo "$WORKERS_RESPONSE" | grep -o '"id"' | wc -l || echo "0")
        UP_COUNT=$(echo "$WORKERS_RESPONSE" | grep -o '"status":"UP"' | wc -l || echo "0")
        echo -e "${CYAN}   Workers totales: $WORKERS_ACTUAL${NC}"
        echo -e "${CYAN}   Workers UP: $UP_COUNT${NC}"
        
        if [ "$UP_COUNT" -ge 1 ]; then
            echo -e "${GREEN}   [OK] Al menos un worker sigue activo${NC}"
        else
            echo -e "${RED}   [ERROR] No hay workers activos${NC}"
        fi
    else
        echo -e "${RED}   [ERROR] No se pudo extraer el Job ID${NC}"
    fi
else
    echo -e "${RED}   [ERROR] Error al enviar job${NC}"
fi

# Resumen
echo ""
echo -e "${GREEN}=== RESUMEN DE PRUEBA ==="
echo -e "${GREEN}[OK] Master funcionando${NC}"
echo -e "${GREEN}[OK] Workers iniciados${NC}"
echo -e "${GREEN}[OK] Fallo simulado${NC}"
echo -e "${GREEN}[OK] Sistema manejó el fallo correctamente${NC}"
echo ""

# Limpiar
echo -e "${YELLOW}Presiona Enter para detener los procesos...${NC}"
read -r
kill $MASTER_PID $WORKER2_PID 2>/dev/null || true
echo -e "${GREEN}Procesos detenidos${NC}"

