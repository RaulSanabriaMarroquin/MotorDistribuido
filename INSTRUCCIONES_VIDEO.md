# Instrucciones para el Video Demostrativo
## Motor Distribuido - Sistema de Procesamiento Distribuido

---

## 📋 Índice

1. [Instalación](#instalación)
2. [Ejecución](#ejecución)
3. [Casos de Prueba](#casos-de-prueba)
4. [Simulación de Fallo](#simulación-de-fallo)
5. [Troubleshooting](#troubleshooting)

---

## 🔧 Instalación

### Requisitos Previos

1. **Rust y Cargo** (versión 1.70 o superior)
   ```powershell
   # Verificar instalación
   rustc --version
   cargo --version
   
   # Si no está instalado, descargar desde: https://www.rust-lang.org/tools/install
   ```

2. **Sistema Operativo**: Windows 10/11, Linux, o macOS

3. **Espacio en disco**: Al menos 500 MB para compilación

### Pasos de Instalación

#### Opción 1: Usando Makefile (Recomendado)

```powershell
# 1. Navegar al directorio del proyecto
cd MotorDistribuido

# 2. Ver ayuda del Makefile
make help

# 3. Instalar dependencias y compilar
make install

# Esto ejecutará:
# - cargo fetch (descargar dependencias)
# - cargo build --release (compilar en modo optimizado)
```

#### Opción 2: Instalación Manual

```powershell
# 1. Descargar dependencias
cargo fetch

# 2. Compilar en modo release (optimizado)
cargo build --release

# 3. Verificar que los binarios se crearon
ls target\release\*.exe
# Deberías ver: master.exe, worker.exe, client.exe
```

### Verificación de Instalación

```powershell
# Ejecutar tests unitarios
make test
# o
cargo test

# Deberías ver: "27 tests passed, 0 failed"
```

---

## 🚀 Ejecución

### Inicio del Sistema

El sistema requiere **múltiples terminales** (una para master, una o más para workers).

#### Terminal 1: Master

```powershell
# Opción 1: Usando Makefile
make run-master

# Opción 2: Usando Cargo directamente
cargo run --release --bin master

# Opción 3: Usando binario compilado
target\release\master.exe
```

**Salida esperada:**
```
Nodo master iniciando...
Nodo master escuchando en 127.0.0.1:8080
```

#### Terminal 2: Worker 1

```powershell
# Opción 1: Usando script PowerShell (Recomendado para Windows)
.\scripts\run_worker.ps1 -Port 9000

# Opción 2: Usando Makefile (requiere que WORKER_PORT esté configurado)
$env:WORKER_PORT="9000"
make run-worker

# Opción 3: Usando Cargo directamente
$env:WORKER_PORT="9000"
cargo run --release --bin worker

# Opción 4: Usando binario compilado
$env:WORKER_PORT="9000"
target\release\worker.exe
```

**Salida esperada:**
```
Nodo worker iniciando...
Registro exitoso con master
Enviando heartbeats...
```

#### Terminal 3: Worker 2 (Opcional, para pruebas de balanceo de carga)

```powershell
# Opción 1: Usando script PowerShell
.\scripts\run_worker.ps1 -Port 9001

# Opción 2: Usando Makefile
$env:WORKER_PORT="9001"
make run-worker
```

#### Terminal 4: Cliente (para enviar comandos)

```powershell
# Listar workers registrados
cargo run --release --bin client -- list-workers

# Enviar un job
cargo run --release --bin client -- submit-job --name "test-job" --operation "map_add" --param 10 --input "1,2,3,4,5"

# Ver progreso de un job
cargo run --release --bin client -- get-progress --job-id <JOB_ID>
```

### Detener el Sistema

```powershell
# En cada terminal, presionar Ctrl+C
# O usar el comando:
make stop-all
```

---

## 🧪 Casos de Prueba

### Prueba Básica (Automática)

```powershell
# Ejecutar script de prueba básica
make test-basic

# Este script:
# 1. Limpia procesos anteriores
# 2. Inicia master y worker
# 3. Verifica registro
# 4. Envía un job de prueba
# 5. Verifica resultados
```

### Prueba Completa (Automática)

```powershell
# Ejecutar suite completa de pruebas
make test-complete

# Este script prueba:
# - Operaciones legacy (map_add)
# - Operaciones nuevas (flat_map, reduce_by_key)
# - Métricas del sistema
# - Estado de workers
```

### Pruebas Manuales

#### 1. Prueba de Operación Map

```powershell
# Enviar job con operación map
cargo run --release --bin client -- submit-job `
  --name "test-map" `
  --operation "map_add" `
  --param 10 `
  --input "1,2,3,4,5"

# Resultado esperado: [11, 12, 13, 14, 15]
```

#### 2. Prueba de Operación Filter

```powershell
# Crear job JSON para filter
$body = @{
    name = "test-filter"
    operation = "filter"
    fn_name = "gt"
    param = 3
    input = @(1,2,3,4,5,6,7,8,9,10)
} | ConvertTo-Json

# Enviar usando curl
curl -X POST http://127.0.0.1:8080/api/v1/jobs `
  -H "Content-Type: application/json" `
  -d $body

# Resultado esperado: [4, 5, 6, 7, 8, 9, 10]
```

#### 3. Prueba de Operación Reduce by Key

```powershell
$body = @{
    name = "test-reduce"
    operation = "reduce_by_key"
    fn_name = "sum"
    input = @(1,2,2,3,3,3,4,4,4,4)
} | ConvertTo-Json

curl -X POST http://127.0.0.1:8080/api/v1/jobs `
  -H "Content-Type: application/json" `
  -d $body

# Resultado esperado: conteo de cada valor
```

#### 4. Prueba con DAG Complejo

Crear archivo `test_dag.json`:

```json
{
  "name": "test-dag-complejo",
  "dag": {
    "nodes": [
      {
        "id": "read",
        "op": "read_csv",
        "path": "test_data.csv"
      },
      {
        "id": "map1",
        "op": "map",
        "fn_name": "add",
        "param": 10
      },
      {
        "id": "filter1",
        "op": "filter",
        "fn_name": "gt",
        "param": 12
      },
      {
        "id": "reduce1",
        "op": "reduce_by_key",
        "fn_name": "sum"
      }
    ],
    "edges": [
      ["read", "map1"],
      ["map1", "filter1"],
      ["filter1", "reduce1"]
    ]
  }
}
```

Enviar el DAG:

```powershell
curl -X POST http://127.0.0.1:8080/api/v1/jobs `
  -H "Content-Type: application/json" `
  -d (Get-Content test_dag.json -Raw)
```

Verificar progreso:

```powershell
# Obtener JOB_ID de la respuesta anterior
curl http://127.0.0.1:8080/api/v1/jobs/<JOB_ID>
```

---

## 💥 Simulación de Fallo

### Objetivo
Demostrar que el sistema detecta fallos de workers y replanifica tareas automáticamente.

### Pasos

1. **Iniciar Master y Múltiples Workers**
   ```powershell
   # Terminal 1: Master
   make run-master
   
   # Terminal 2: Worker 1
   $env:WORKER_PORT="9000"
   make run-worker
   
   # Terminal 3: Worker 2
   $env:WORKER_PORT="9001"
   make run-worker
   ```

2. **Enviar un Job que Tome Tiempo**
   ```powershell
   # Terminal 4: Cliente
   cargo run --release --bin client -- submit-job `
     --name "test-failure" `
     --operation "map_add" `
     --param 10 `
     --input "1,2,3,4,5,6,7,8,9,10,11,12,13,14,15"
   
   # Anotar el JOB_ID
   ```

3. **Detener Worker 1 Mientras Procesa**
   ```powershell
   # En Terminal 2, presionar Ctrl+C
   # O cerrar la ventana de terminal
   ```

4. **Observar Replanificación**
   ```powershell
   # En Terminal 4, verificar progreso
   cargo run --release --bin client -- get-progress --job-id <JOB_ID>
   
   # Deberías ver que:
   # - El master detecta que el worker 1 no responde
   # - Las tareas pendientes se replanifican al worker 2
   # - El job se completa exitosamente
   ```

5. **Verificar Resultados**
   ```powershell
   # Obtener resultados del job
   curl http://127.0.0.1:8080/api/v1/jobs/<JOB_ID>/results
   ```

### Resultado Esperado

- El master detecta el fallo del worker (por falta de heartbeats)
- Las tareas pendientes se reasignan al worker 2
- El job se completa correctamente
- Las métricas muestran el fallo y la replanificación

---

## 🔍 Troubleshooting

### Problema: "Master no responde"

**Solución:**
```powershell
# Verificar que el master está corriendo
curl http://127.0.0.1:8080/api/v1/workers

# Si no responde, verificar que no hay otro proceso usando el puerto 8080
netstat -ano | findstr :8080

# Matar proceso si es necesario
taskkill /PID <PID> /F
```

### Problema: "Worker no se registra"

**Solución:**
```powershell
# Verificar que el master está corriendo
curl http://127.0.0.1:8080/api/v1/workers

# Verificar variable de entorno
echo $env:WORKER_PORT

# Verificar que el puerto no está en uso
netstat -ano | findstr :9000
```

### Problema: "Job no se completa"

**Solución:**
```powershell
# Verificar estado del job
curl http://127.0.0.1:8080/api/v1/jobs/<JOB_ID>

# Verificar que hay workers activos
cargo run --release --bin client -- list-workers

# Ver logs del master (si están habilitados)
```

### Problema: "Tests fallan"

**Solución:**
```powershell
# Limpiar y recompilar
make clean
make build-release

# Ejecutar tests individuales
cargo test --package worker --bin worker
cargo test --package master --bin master
```

### Problema: "Puerto ya en uso"

**Solución:**
```powershell
# Encontrar proceso usando el puerto
netstat -ano | findstr :8080
netstat -ano | findstr :9000

# Matar proceso
taskkill /PID <PID> /F

# O usar puertos diferentes
$env:MASTER_PORT="8081"
$env:WORKER_PORT="9002"
```

---

## 📝 Notas Adicionales

### Variables de Entorno

```powershell
# Master
$env:MASTER_PORT="8080"  # Puerto del master (por defecto: 8080)
$env:MASTER_HOST="127.0.0.1"  # Host del master

# Worker
$env:WORKER_PORT="9000"  # Puerto del worker (requerido)
$env:MASTER_URL="http://127.0.0.1:8080"  # URL del master

# Cache
$env:CACHE_THRESHOLD_MB="100"  # Umbral de memoria para cache (MB)
```

### Logs y Debugging

```powershell
# Habilitar logs detallados
$env:RUST_LOG="debug"
make run-master

# Ver métricas en tiempo real
while ($true) {
    curl http://127.0.0.1:8080/api/v1/metrics
    Start-Sleep -Seconds 5
}
```

### Performance

```powershell
# Compilar en modo release para mejor rendimiento
make build-release

# Ejecutar benchmarks
cd benchmarks
.\benchmark.ps1
```

---

## 📚 Recursos Adicionales

- **README.md**: Documentación general del proyecto
- **GUION_VIDEO.md**: Guion detallado para el video
- **docs/architecture.md**: Documentación de arquitectura
- **Makefile**: Comandos disponibles (`make help`)

---

## ✅ Checklist Pre-Video

- [ ] Rust y Cargo instalados y funcionando
- [ ] Proyecto compilado (`make build-release`)
- [ ] Todos los tests pasando (`make test`)
- [ ] Scripts de prueba funcionando (`make test-basic`, `make test-complete`)
- [ ] Archivos de prueba creados (CSV, JSONL)
- [ ] Múltiples terminales preparadas
- [ ] Guion revisado
- [ ] Navegador abierto para mostrar documentación si es necesario

