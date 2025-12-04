# Worker

Los nodos worker ejecutan las tareas asignadas por el master. Se registran automáticamente con el master, envían heartbeats periódicos y procesan tareas de forma aislada.

## Propósito

- Conectarse al master y registrarse
- Enviar heartbeats periódicos para indicar que está activo
- Recibir y ejecutar tareas asignadas por el master
- Reportar resultados y estado de las tareas
- Manejar fallos y reintentos de tareas

## Configuración

El worker se configura mediante variables de entorno:

- `MASTER_URL`: URL del master (por defecto: `http://127.0.0.1:8080`)
- `WORKER_HOST`: Dirección de bind del worker (por defecto: `127.0.0.1`, usar `0.0.0.0` para Docker)
- `WORKER_PORT`: Puerto de escucha del worker (por defecto: `9000`)
- `HEARTBEAT_INTERVAL_SECS`: Intervalo entre heartbeats en segundos (por defecto: `3`)
- `TASK_EXECUTION_DELAY_SECS`: Delay artificial en la ejecución de tareas para pruebas (por defecto: `0`)
- `RUST_LOG`: Nivel de logging (por defecto: `info`)
- `DOCKER`: Si está definido, el worker usa `0.0.0.0` como host por defecto

## Características

### Registro Automático

Al iniciar, el worker se registra automáticamente con el master enviando su ID, host y puerto.

### Heartbeats

El worker envía heartbeats periódicos al master cada 3 segundos (configurable). Esto permite al master detectar si el worker está activo o ha fallado.

### Ejecución de Tareas

El worker expone un endpoint HTTP para recibir tareas:

- `POST /api/v1/tasks/execute`: Recibe una tarea y la ejecuta

El worker ejecuta las siguientes operaciones:

- **map_add**: Suma un valor a cada elemento
- **map_mul**: Multiplica cada elemento por un valor
- **filter_gt**: Filtra elementos mayores que un valor
- **filter_lt**: Filtra elementos menores que un valor
- **reduce_by_key**: Agrupa valores por clave y los suma
- **join**: Une dos datasets por clave
- **flat_map**: Aplana una colección de colecciones

### Aislamiento

Cada tarea se ejecuta de forma aislada. El worker procesa una tarea a la vez, asegurando que los resultados de una tarea no afecten a otras.

### Reporte de Resultados

Después de ejecutar una tarea, el worker envía el resultado al master. Si la tarea falla, reporta el error para que el master pueda replanificarla.

## Endpoints

- `GET /health`: Endpoint de health check
- `POST /api/v1/tasks/execute`: Ejecuta una tarea asignada por el master

## Ejecución

### Local

```bash
cargo run --bin worker
```

### Con variables de entorno

```bash
MASTER_URL=http://127.0.0.1:8080 \
WORKER_HOST=127.0.0.1 \
WORKER_PORT=8081 \
HEARTBEAT_INTERVAL_SECS=3 \
cargo run --bin worker
```

### Con Docker Compose

Los workers se inician automáticamente con `docker-compose up`. Cada worker se configura con variables de entorno en el `docker-compose.yml`.

## Dependencias

- `axum`: Framework HTTP para el servidor
- `tokio`: Runtime asíncrono
- `reqwest`: Cliente HTTP para comunicación con el master
- `serde` / `serde_json`: Serialización JSON
- `tracing`: Logging estructurado
- `common`: Tipos y estructuras compartidas
- `worker`: Biblioteca con la lógica de ejecución de operaciones

## Pruebas

Las pruebas unitarias están en `src/lib.rs`:

```bash
cargo test --lib -p worker
```

Las pruebas verifican que cada operación se ejecuta correctamente con diferentes inputs y parámetros.
