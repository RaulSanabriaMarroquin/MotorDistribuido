# Master

El nodo master es el coordinador central del sistema distribuido. Gestiona el registro de workers, la planificación de tareas, el monitoreo de heartbeats y la ejecución de jobs.

## Propósito

- Registrar y gestionar workers
- Recibir y planificar jobs
- Distribuir tareas entre workers usando round-robin con awareness de carga
- Monitorear heartbeats de workers y detectar fallos
- Gestionar el estado de los jobs y su progreso
- Replanificar tareas cuando un worker falla
- Proporcionar métricas del sistema

## API Endpoints

El master expone los siguientes endpoints HTTP/JSON:

### Workers

- `POST /api/v1/workers/register`: Registro de un nuevo worker
- `POST /api/v1/workers/:id/heartbeat`: Recepción de heartbeat de un worker
- `GET /api/v1/workers`: Lista todos los workers registrados

### Jobs

- `POST /api/v1/jobs/submit`: Envío de un nuevo job
- `GET /api/v1/jobs/:id/progress`: Consulta del progreso de un job
- `POST /api/v1/jobs/:id/task_result`: Recepción del resultado de una tarea
- `POST /api/v1/jobs/:id/next_stage`: Avance al siguiente stage de un job

### Métricas

- `GET /api/v1/metrics`: Obtiene métricas globales del sistema

## Configuración

El master se configura mediante variables de entorno:

- `MASTER_HOST`: Dirección de bind (por defecto: `127.0.0.1`, usar `0.0.0.0` para Docker)
- `MASTER_PORT`: Puerto de escucha (por defecto: `8080`)
- `RUST_LOG`: Nivel de logging (por defecto: `info`)

## Características

### Registro de Workers

Los workers se registran automáticamente al iniciar. El master mantiene un registro de todos los workers con su estado (UP/DOWN), última vez de heartbeat, host y puerto.

### Monitoreo de Heartbeats

El master ejecuta una tarea en background que monitorea los heartbeats de los workers cada 3 segundos. Si un worker no envía heartbeat en 15 segundos, se marca como DOWN y se replanifican sus tareas pendientes.

### Planificación de Tareas

El master planifica tareas usando una política round-robin con awareness de carga. Asigna tareas a workers disponibles balanceando la carga.

### Gestión de Jobs

Cada job se divide en tareas que se distribuyen entre workers. El master:
- Mantiene el estado del job (running, completed, failed)
- Rastrea tareas pendientes y completadas
- Almacena payloads de tareas para replanificación
- Soporta jobs multi-stage con DAG de etapas
- Maneja shuffle entre stages para operaciones como reduce_by_key y join

### Replanificación

Cuando un worker falla, el master:
- Marca el worker como DOWN
- Identifica tareas pendientes asignadas a ese worker
- Las agrega a la cola de replanificación
- Las reasigna a otros workers disponibles

### Persistencia

El master mantiene el estado de los jobs en memoria. El estado incluye:
- Información del job (ID, nombre, estado, progreso)
- Tareas pendientes y completadas
- Output de stages anteriores para usar como input del siguiente stage
- Payloads de tareas para replanificación

## Ejecución

### Local

```bash
cargo run --bin master
```

### Con Docker Compose

El master se inicia automáticamente con `docker-compose up`.

## Dependencias

- `axum`: Framework HTTP para el servidor
- `tokio`: Runtime asíncrono
- `reqwest`: Cliente HTTP para comunicación con workers
- `serde` / `serde_json`: Serialización JSON
- `tracing`: Logging estructurado
- `uuid`: Generación de IDs únicos
- `common`: Tipos y estructuras compartidas

## Pruebas

Las pruebas unitarias están en `src/lib.rs`:

```bash
cargo test --lib -p master
```
