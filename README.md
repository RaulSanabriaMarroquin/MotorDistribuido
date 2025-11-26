# Motor Distribuido

Sistema distribuido de procesamiento implementado en Rust para el curso de Sistemas Operativos. Implementa un motor de procesamiento distribuido tipo Spark/Flink desde cero.

## Descripción

Este proyecto implementa un sistema distribuido master-worker que permite ejecutar trabajos de procesamiento de datos en paralelo. El sistema soporta:

- **Ruta A (Batch DAG)**: Procesamiento por lotes con DAG de etapas (map, filter, reduce, join)
- Operadores: map, flat_map, filter, reduce_by_key
- Planificación de tareas con DAG
- Tolerancia a fallos con reintentos automáticos
- Métricas y observabilidad

## Arquitectura

El sistema está compuesto por tres componentes principales:

- **Master**: Nodo coordinador que gestiona workers, planifica tareas y mantiene el estado del sistema
- **Workers**: Nodos ejecutores que procesan tareas asignadas por el master
- **Client**: Cliente CLI para interactuar con el sistema

## Estructura del Proyecto

```
MotorDistribuido/
├── master/          # Nodo maestro
├── worker/          # Nodos trabajadores
├── client/          # Cliente CLI
├── common/          # Biblioteca compartida
├── docs/            # Documentación
├── scripts/         # Scripts de utilidad
└── Enunciado/       # Especificación del proyecto
```

## Requisitos

- Rust 1.70+ (edición 2021)
- Cargo
- Docker y Docker Compose (opcional, para despliegue con contenedores)

## Instalación

### Compilación Local

```bash
# Compilar en modo debug
cargo build

# Compilar en modo release (recomendado)
cargo build --release
```

### Usando Makefile

```bash
# Ver todas las opciones
make help

# Compilar
make build-release

# Ejecutar tests
make test

# Limpiar artefactos
make clean
```

## Uso

### Ejecución Local

#### 1. Iniciar el Master

```bash
# Opción 1: Usando cargo
cargo run --release --bin master

# Opción 2: Usando make
make run-master

# Opción 3: Binario compilado
target/release/master.exe  # Windows
target/release/master       # Linux/Mac
```

El master estará disponible en `http://127.0.0.1:8080`

#### 2. Iniciar Workers

```bash
# Opción 1: Usando cargo
WORKER_PORT=9000 cargo run --release --bin worker

# Opción 2: Usando make
make run-worker WORKER_PORT=9000

# Opción 3: Binario compilado
WORKER_PORT=9000 target/release/worker.exe  # Windows
WORKER_PORT=9000 target/release/worker      # Linux/Mac
```

Puedes iniciar múltiples workers en diferentes puertos (9000, 9001, 9002, etc.)

#### 3. Usar el Cliente

```bash
# Listar workers
cargo run --release --bin client list-workers

# Enviar un job (formato legacy)
cargo run --release --bin client submit-job \
  --name "test-job" \
  --operation "map_add" \
  --param 10 \
  --input "1,2,3,4,5"

# Ver progreso de un job
cargo run --release --bin client get-progress --job-id <JOB_ID>
```

### Ejecución con Docker Compose

```bash
# Construir y levantar todos los servicios
docker-compose up --build

# En modo detached
docker-compose up -d --build

# Ver logs
docker-compose logs -f

# Detener servicios
docker-compose down
```

Esto iniciará:
- 1 master en puerto 8080
- 3 workers en puertos 9000, 9001, 9002

## API Endpoints

### Workers

- `POST /api/v1/workers/register` - Registrar un worker
- `POST /api/v1/workers/:id/heartbeat` - Enviar heartbeat
- `GET /api/v1/workers` - Listar workers

### Jobs

- `POST /api/v1/jobs/submit` - Enviar un job
- `GET /api/v1/jobs/:id/progress` - Obtener progreso de un job
- `POST /api/v1/jobs/:id/task_result` - Enviar resultado de tarea

### Métricas

- `GET /api/v1/metrics` - Obtener todas las métricas
- `GET /api/v1/metrics/nodes` - Métricas por nodo
- `GET /api/v1/metrics/jobs` - Métricas por job

## Formato de Jobs

### Formato Legacy (Simple)

```json
{
  "name": "test-job",
  "operation": "map_add",
  "param": 10,
  "input": [1, 2, 3, 4, 5]
}
```

### Formato DAG (Según Enunciado)

```json
{
  "name": "wordcount-batch",
  "dag": {
    "nodes": [
      {"id": "read", "op": "read_csv", "path": "data/*.csv", "partitions": 4},
      {"id": "flat", "op": "flat_map", "fn": "tokenize"},
      {"id": "map1", "op": "map", "fn": "to_lower"},
      {"id": "agg", "op": "reduce_by_key", "key": "token", "fn": "sum"}
    ],
    "edges": [
      ["read", "flat"],
      ["flat", "map1"],
      ["map1", "agg"]
    ]
  },
  "parallelism": 4
}
```

## Operadores Disponibles

- **map**: Transformación elemento a elemento
  - Funciones: `add`, `mul`, `to_lower`
- **flat_map**: Transformación y aplanamiento
  - Funciones: `tokenize`, `split`
- **filter**: Filtrado de elementos
  - Funciones: `gt`, `lt`, `eq`
- **reduce_by_key**: Agregación por clave
  - Funciones: `sum`, `count`, `max`, `min`

## Demo

Ejecutar el script de demo:

```bash
# Linux/Mac
bash scripts/demo.sh

# Windows PowerShell
powershell -ExecutionPolicy Bypass -File scripts/demo.ps1

# O usando make
make demo
```

## Características Implementadas

### Semana 1 ✅
- Registro de workers
- Heartbeats periódicos
- Detección de fallos (marcar workers como DOWN)
- API básica de consulta

### Semana 2 ✅
- Envío de jobs
- Ejecución de operaciones básicas (map, filter)
- Métricas de progreso

### Semana 3 ✅
- Operadores avanzados (flat_map, reduce_by_key)
- Parser y planificador de DAG
- Sistema de reintentos (máximo 1 reintento por tarea)
- Task attempt ID para idempotencia

### Semana 4 ✅
- Métricas por nodo (CPU, memoria, tareas activas, latencia, reintentos)
- Métricas por job (tiempo total, etapas, fallos)
- Logging estructurado
- Makefile completo
- Scripts de demo

## Desarrollo

### Estructura de Código

- `master/src/main.rs`: Lógica del master, planificación, métricas
- `master/src/dag.rs`: Parser y procesamiento de DAG
- `worker/src/main.rs`: Lógica del worker, ejecución de tareas
- `worker/src/operators.rs`: Implementación de operadores
- `common/src/lib.rs`: Tipos y estructuras compartidas
- `common/src/metrics.rs`: Estructuras de métricas

### Tests

```bash
# Ejecutar todos los tests
cargo test

# Tests de un componente específico
cargo test --package worker
```

### Formateo y Linting

```bash
# Formatear código
cargo fmt

# Linting
cargo clippy

# O usando make
make fmt
make lint
```

## Configuración

### Variables de Entorno

**Master:**
- `RUST_LOG`: Nivel de logging (default: info)

**Worker:**
- `MASTER_URL`: URL del master (default: http://127.0.0.1:8080)
- `WORKER_HOST`: Host del worker (default: 127.0.0.1)
- `WORKER_PORT`: Puerto del worker (default: 9000)
- `HEARTBEAT_INTERVAL_SECS`: Intervalo de heartbeats en segundos (default: 3)
- `RUST_LOG`: Nivel de logging (default: info)

## Troubleshooting

### El master no inicia
- Verificar que el puerto 8080 no esté en uso
- Revisar logs con `RUST_LOG=debug`

### Workers no se registran
- Verificar que el master esté corriendo
- Verificar `MASTER_URL` en el worker
- Revisar conectividad de red

### Jobs no se completan
- Verificar que haya workers activos: `GET /api/v1/workers`
- Revisar logs del worker
- Verificar formato del job

## Contribución

Este es un proyecto académico. Para contribuir:

1. Fork el repositorio
2. Crea una rama para tu feature
3. Realiza tus cambios
4. Asegúrate de que los tests pasen
5. Envía un pull request

## Licencia

Proyecto académico - Sin licencia específica

## Referencias

- [Enunciado del Proyecto](Enunciado/_PSO_PY02b__Simplementación_de_SparkFlink.ipynb)
- [Documentación de Arquitectura](docs/architecture.md)
