# Common

Biblioteca compartida que contiene tipos, estructuras de datos y utilidades comunes utilizadas por todos los componentes del sistema distribuido (master, worker y client).

## Propósito

- Definir estructuras de datos compartidas
- Proporcionar tipos de mensajes comunes para la comunicación
- Compartir funciones de utilidad
- Mantener definiciones de protocolo
- Asegurar consistencia en la serialización/deserialización

## Estructuras Principales

### Mensajes de Workers

- `RegisterRequest`: Solicitud de registro de un worker
- `RegisterResponse`: Respuesta al registro
- `HeartbeatRequest`: Solicitud de heartbeat
- `HeartbeatResponse`: Respuesta al heartbeat
- `WorkerStatus`: Estado del worker (UP/DOWN)
- `WorkerListItem`: Información de un worker para listado
- `WorkersListResponse`: Respuesta con lista de workers

### Mensajes de Jobs

- `JobSpec`: Especificación de un job
- `Stage`: Etapa de un job con operación y parámetro
- `DatasetSource`: Fuente de datos (inline, CSV, JSONL, texto)
- `SubmitJobResponse`: Respuesta al envío de un job
- `JobProgress`: Progreso de un job
- `TaskAssignment`: Asignación de una tarea a un worker
- `TaskResult`: Resultado de una tarea

### Utilidades

- `MESSAGE_VERSION`: Versión del protocolo de mensajes para compatibilidad

## Serialización

Todas las estructuras implementan `Serialize` y `Deserialize` de Serde para comunicación JSON entre componentes.

## Uso

Esta biblioteca es una dependencia de todos los otros componentes:

```toml
[dependencies]
common = { path = "../common" }
```

## Pruebas

Las pruebas unitarias verifican la serialización/deserialización de las estructuras:

```bash
cargo test --lib -p common
```

## Dependencias

- `serde`: Serialización/deserialización
- `serde_json`: Soporte JSON
- `uuid`: IDs únicos (con features de serde)
