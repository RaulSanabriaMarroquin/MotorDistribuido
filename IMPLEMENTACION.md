# Estado de Implementación

## ✅ Implementado

### Semana 1 - Base (Completado)
- ✅ Registro de workers
- ✅ Heartbeats periódicos
- ✅ Detección de fallos (marcar workers como DOWN)
- ✅ API básica HTTP/JSON
- ✅ Cliente CLI básico

### Semana 2 - Jobs y Tareas (Completado)
- ✅ API de Jobs:
  - ✅ POST /api/v1/jobs - Enviar job
  - ✅ GET /api/v1/jobs/{id} - Estado del job
  - ✅ GET /api/v1/jobs/{id}/results - Resultados
- ✅ Tipos de datos para DAG (nodos, edges, operadores)
- ✅ Planificador de tareas (cola + round-robin con awareness de carga)
- ✅ Ejecución de tareas en workers (pool de hilos con tokio::spawn_blocking)
- ✅ Operadores básicos:
  - ✅ map
  - ✅ flat_map
  - ✅ filter
  - ✅ reduce
  - ✅ reduce_by_key
- ✅ Lectura/escritura CSV y JSONL
- ✅ Cliente CLI extendido (submit-job, job-status, job-results)

### Infraestructura
- ✅ Makefile para build/test/demo
- ✅ Estructura del proyecto organizada

## ✅ Nuevas Funcionalidades Implementadas (Actualización)

### Tolerancia a Fallos
- ✅ Reintentos automáticos de tareas fallidas (máximo 1 reintento por tarea)
- ✅ Replanificación de tareas cuando un worker cae (tareas se reasignan automáticamente)
- ✅ Manejo de señales para apagado ordenado (SIGTERM/Ctrl+C en master y worker)

### Operadores Avanzados
- ✅ join por clave (inner join implementado)
- ❌ shuffle entre etapas (pendiente)
- ✅ Manejo de particiones (implementado: crea múltiples tareas por nodo según partitions/parallelism)

### Infraestructura
- ✅ docker-compose.yml para despliegue multinodo
- ✅ Dockerfiles para master y worker

## ⚠️ Parcialmente Implementado

- ✅ Idempotencia básica: Task attempt ID en nombres de archivo y verificación de outputs existentes
- ⚠️ Métricas: Estructuras definidas pero métricas reales (CPU, memoria) no implementadas

## ❌ Pendiente

### Persistencia
- ✅ Persistencia de estado (SQLite implementado)
  - Base de datos SQLite local (`master_state.db`)
  - Guarda jobs y tasks al crearlos/actualizarlos
  - Carga estado al iniciar el master
  - Re-encola tareas pendientes/assigned al reiniciar

### Almacenamiento
- ✅ Cache con spill a disco para Batch (módulo `cache.rs` implementado)
  - Cache por partición con umbral configurable (default: 100 MB)
  - Spill automático a disco cuando se excede el umbral
  - Recuperación automática desde disco
- ❌ Backpressure para Streaming (no aplica - Ruta A)

### Observabilidad
- ❌ Métricas reales de CPU y memoria
- ❌ Logging estructurado completo
- ❌ Métricas por nodo y por job

### Streaming (Ruta B)
- ❌ Topologías de streaming
- ❌ Ventanas por tiempo
- ❌ Checkpointing
- ❌ Endpoint /api/v1/ingest

### Testing
- ❌ Tests unitarios de operadores
- ❌ Tests de integración
- ❌ Tests end-to-end

## Uso Básico

### 1. Compilar
```bash
make build
```

### 2. Ejecutar Master
```bash
make run-master
# o
cargo run --bin master
```

### 3. Ejecutar Workers (en terminales separadas)
```bash
make run-worker
# o con puerto diferente
WORKER_PORT=9001 make run-worker
```

### 4. Usar el Cliente
```bash
# Listar workers
cargo run --bin client -- list-workers

# Enviar un job
cargo run --bin client -- submit-job \
  --name "wordcount" \
  --input "data/input.csv" \
  --output "data/output.jsonl" \
  --parallelism 2

# Ver estado del job
cargo run --bin client -- job-status <job_id>

# Ver resultados
cargo run --bin client -- job-results <job_id>
```

## Notas de Implementación

- El sistema usa Tokio para concurrencia asíncrona
- Los operadores se ejecutan en `spawn_blocking` para no bloquear el runtime
- El planificador usa round-robin con awareness de carga (tareas activas por worker)
- Los operadores básicos están implementados pero son simplificados (no manejan particiones reales)
- El sistema está diseñado para Batch (Ruta A), Streaming (Ruta B) no está implementado

## Próximos Pasos Recomendados

1. ✅ ~~Implementar reintentos y replanificación~~ (COMPLETADO)
2. Agregar persistencia de estado (sqlite)
3. Implementar métricas reales (CPU, memoria)
4. Agregar tests (unitarios, integración, E2E)
5. ✅ ~~Implementar operador join~~ (COMPLETADO)
6. Implementar operador shuffle
7. Implementar cache con spill a disco
8. Mejorar manejo de particiones

