# Arquitectura del Sistema Distribuido - Semana 1

## Descripción General

El sistema distribuido está compuesto por tres tipos de procesos principales:

- **Master**: Nodo coordinador central que gestiona el registro de workers, monitorea su estado mediante heartbeats y mantiene la visión global del sistema.

- **Workers**: Nodos ejecutores que se registran con el master y envían periódicamente señales de vida (heartbeats) para indicar que están activos y disponibles.

- **Client**: Aplicación cliente que interactúa con el master para consultar el estado del sistema y realizar operaciones de gestión básicas.

## Modelo de IPC (Inter-Process Communication)

### Protocolo: HTTP/JSON sobre TCP

- **Protocolo de transporte**: TCP/IP
- **Protocolo de aplicación**: HTTP/1.1
- **Formato de datos**: JSON
- **Framework**: Axum (async HTTP para Rust/Tokio)

### Versionado de Mensajes

Todos los mensajes intercambiados entre componentes incluyen un campo de versión para garantizar compatibilidad:

```json
{
  "version": "1.0",
  "message_type": "...",
  "payload": { ... }
}
```

Esto permite evolución del protocolo sin romper compatibilidad entre versiones diferentes de los componentes.

## Responsabilidades - Semana 1

### Master

1. **Registro de Workers**
   - Aceptar solicitudes de registro de nuevos workers
   - Asignar identificadores únicos a cada worker
   - Mantener un registro de todos los workers activos

2. **Gestión de Heartbeats**
   - Recibir y procesar heartbeats periódicos de los workers
   - Actualizar el timestamp de último contacto para cada worker
   - Validar que los heartbeats provengan de workers registrados

3. **Detección de Fallos**
   - Monitorear el tiempo transcurrido desde el último heartbeat de cada worker
   - Marcar workers como DOWN si no reciben heartbeats dentro del tiempo de espera configurado
   - Mantener estado histórico de workers (UP/DOWN)

4. **API de Consulta**
   - Exponer endpoints HTTP para que clientes consulten el estado del sistema
   - Proporcionar información sobre workers registrados y su estado actual
   - Reportar estadísticas básicas del sistema

### Worker

1. **Registro Inicial**
   - Conectarse al master al iniciar
   - Enviar solicitud de registro con información del worker (puerto, capacidades básicas)
   - Recibir y almacenar su identificador único asignado por el master

2. **Heartbeats Periódicos**
   - Enviar heartbeats periódicamente al master (ej: cada 5 segundos)
   - Incluir identificador del worker y timestamp en cada heartbeat
   - Manejar errores de comunicación y reintentos

3. **Gestión de Conexión**
   - Detectar pérdida de conexión con el master
   - Implementar lógica de reconexión automática si la conexión se pierde
   - Re-registrarse si es necesario tras una reconexión

### Client

1. **Consulta de Estado**
   - Conectarse al master para consultar el estado actual del sistema
   - Visualizar lista de workers registrados y su estado (UP/DOWN)
   - Obtener información básica de cada worker

2. **Interfaz de Usuario**
   - Proporcionar una interfaz (CLI o simple) para interactuar con el sistema
   - Mostrar información del sistema de forma legible

## Modelo de Concurrencia: Tokio Runtime

### Runtime Asíncrono

El sistema utiliza **Tokio**, el runtime asíncrono de Rust, para manejar concurrencia mediante el modelo de **tasks** (tareas) en lugar de threads tradicionales.

### Conceptos Clave

- **Tasks**: Unidades de trabajo asíncronas que se ejecutan en el runtime de Tokio
- **Runtime**: Gestiona un pool de threads del sistema operativo (worker threads)
- **Event Loop**: Maneja I/O asíncrono y despacha tareas a threads disponibles

### Ventajas

- **Escalabilidad**: Miles de tasks pueden ejecutarse concurrentemente con solo unos pocos threads del SO
- **Eficiencia**: Las tasks se bloquean solo en operaciones de I/O reales, no en esperas activas
- **Rendimiento**: Alto throughput de conexiones HTTP concurrentes

### En Nuestro Sistema

- Cada conexión HTTP es manejada por una task independiente
- El master puede gestionar múltiples workers y clientes concurrentemente
- Los workers ejecutan su loop de heartbeats como una task asíncrona
- Operaciones de red (HTTP requests/responses) no bloquean otras operaciones

## Flujos de Operación

### 1. Registro de Worker

```
┌─────────┐                    ┌─────────┐
│ Worker  │                    │ Master  │
└────┬────┘                    └────┬────┘
     │                              │
     │  1. POST /register           │
     │     {                        │
     │       "port": 8081,          │
     │       "host": "127.0.0.1"    │
     │     }                        │
     ├─────────────────────────────>│
     │                              │
     │                              │ 2. Validar request
     │                              │ 3. Generar worker_id único
     │                              │ 4. Registrar worker (estado: UP)
     │                              │
     │  5. 200 OK                   │
     │     {                        │
     │       "worker_id": "w001",   │
     │       "master_endpoint": ... │
     │     }                        │
     │<─────────────────────────────┤
     │                              │
     │ 6. Almacenar worker_id       │
     │ 7. Iniciar loop de heartbeats│
     │                              │
```

**Pasos detallados:**

1. Worker envía solicitud POST a `/register` con información básica (puerto, host)
2. Master valida la solicitud y genera un `worker_id` único (ej: "w001", "w002", ...)
3. Master registra el worker en su base de datos interna con estado `UP`
4. Master responde con el `worker_id` asignado y endpoints de comunicación
5. Worker almacena el `worker_id` recibido
6. Worker inicia su loop de envío de heartbeats periódicos

### 2. Heartbeats Periódicos

```
┌─────────┐                    ┌─────────┐
│ Worker  │                    │ Master  │
└────┬────┘                    └────┬────┘
     │                              │
     │  [Cada 5 segundos]           │
     │                              │
     │  1. POST /heartbeat          │
     │     {                        │
     │       "worker_id": "w001",   │
     │       "timestamp": 1234567890│
     │     }                        │
     ├─────────────────────────────>│
     │                              │
     │                              │ 2. Validar worker_id
     │                              │ 3. Actualizar last_seen
     │                              │ 4. Verificar estado (UP)
     │                              │
     │  5. 200 OK                   │
     │     { "status": "ok" }       │
     │<─────────────────────────────┤
     │                              │
     │  [Continúa loop...]          │
     │                              │
```

**Pasos detallados:**

1. Worker envía POST a `/heartbeat` cada N segundos (configurable, ej: 5s) con su `worker_id` y timestamp
2. Master valida que el `worker_id` existe y está registrado
3. Master actualiza el campo `last_seen` del worker con el timestamp actual
4. Master verifica que el worker esté en estado `UP` (si estaba DOWN, podría cambiarlo a UP)
5. Master responde con confirmación
6. Worker continúa el loop periódico

### 3. Marcado de Workers como DOWN

```
┌─────────┐                    ┌─────────┐
│ Master  │                    │         │
└────┬────┘                    │ Worker  │
     │                         │ (caído) │
     │  [Cada X segundos]      │         │
     │                         │         │
     │  1. Tarea de monitoreo  │         │
     │     ejecuta check       │         │
     │                         │         │
     │  2. Para cada worker:   │         │
     │     - Calcular tiempo   │         │
     │       desde last_seen   │         │
     │     - Si > timeout      │         │
     │       (ej: 15s):        │         │
     │       • Cambiar estado  │         │
     │         a DOWN          │         │
     │       • Registrar       │         │
     │         timestamp       │         │
     │                         │         │
     │  3. Worker "w001"       │         │
     │     marcado como DOWN   │         │
     │                         │         │
```

**Pasos detallados:**

1. El master ejecuta una tarea de monitoreo periódico (ej: cada 3 segundos)
2. Para cada worker registrado:
   - Calcula el tiempo transcurrido desde `last_seen`
   - Si el tiempo excede el timeout configurado (ej: 15 segundos = 3 heartbeats faltados)
   - Cambia el estado del worker de `UP` a `DOWN`
   - Registra el timestamp del cambio de estado
3. El worker queda marcado como DOWN hasta que se reconecte y envíe un heartbeat válido

**Nota**: Si un worker marcado como DOWN envía un heartbeat posterior, el master puede cambiarlo de vuelta a `UP`.

## Diagrama de Arquitectura

```
                    ┌─────────────┐
                    │   Client    │
                    │  (Consulta) │
                    └──────┬──────┘
                           │
                           │ HTTP/JSON
                           │
                    ┌──────▼──────┐
                    │             │
                    │   Master    │
                    │             │
                    │ ┌─────────┐ │
                    │ │Registry │ │  ┌─────────────┐
                    │ │(Workers)│ │  │   Worker 1  │
                    │ └─────────┘ │◄─┤  (w001)     │
                    │             │  └─────────────┘
                    │ ┌─────────┐ │  Heartbeat
                    │ │Monitor  │ │  ┌─────────────┐
                    │ │(UP/DOWN)│ │  │   Worker 2  │
                    │ └─────────┘ │◄─┤  (w002)     │
                    │             │  └─────────────┘
                    │ ┌─────────┐ │  Heartbeat
                    │ │  API    │ │  ┌─────────────┐
                    │ │(HTTP)   │ │  │   Worker N  │
                    │ └─────────┘ │◄─┤  (w00N)     │
                    └─────────────┘  └─────────────┘
                           │
                           │ HTTP/JSON
                           │
                    ┌──────▼──────┐
                    │   Client    │
                    │  (Consulta) │
                    └─────────────┘

Componentes del Master:
- Registry: Base de datos de workers registrados
- Monitor: Tarea que verifica heartbeats y marca workers DOWN
- API: Servidor HTTP que expone endpoints REST
```

## Evolución Futura (Semanas 2-4)

### Semana 2: Ejecución de Tareas
- El master recibirá solicitudes de ejecución de tareas desde clientes
- Los workers ejecutarán tareas asignadas por el master
- Implementación de cola de tareas y asignación inicial

### Semana 3: Programación y Balanceo de Carga
- Algoritmos de scheduling de tareas (FIFO, Round-Robin, etc.)
- Balanceo de carga entre workers disponibles
- Gestión de prioridades de tareas

### Semana 4: Tolerancia a Fallos y Recuperación
- Reasignación automática de tareas cuando un worker cae
- Persistencia de estado del master
- Checkpointing y recuperación de tareas en progreso
- Reintentos y manejo de errores robusto

### Consideraciones de Arquitectura
- La estructura base de registro y monitoreo de la Semana 1 será fundamental para las semanas posteriores
- Los mecanismos de comunicación HTTP/JSON establecidos se mantendrán y extenderán
- El sistema de detección de fallos será crucial para la tolerancia a fallos en Semana 4

---

## Gestión de Memoria y Almacenamiento

### Cache con Spill a Disco (Batch)

El sistema implementa un mecanismo de cache con spill automático a disco para optimizar el uso de memoria según la sección 4.3 del enunciado.

#### Arquitectura del Cache

- **Ubicación**: Módulo `worker/src/cache.rs`
- **Estrategia**: Cache híbrido en memoria y disco
- **Umbral configurable**: Variable de entorno `CACHE_THRESHOLD_MB` (default: 100MB)

#### Funcionamiento

1. **Almacenamiento en Memoria**:
   - Los datos se almacenan inicialmente en memoria (estructura `CacheEntry::InMemory`)
   - Se mantiene un contador de uso de memoria (`memory_usage_mb`)

2. **Spill a Disco**:
   - Cuando el uso de memoria supera el umbral configurable, se activa el spill
   - Los datos se serializan como JSON y se escriben a archivos en el directorio `cache/`
   - Las entradas se marcan como `CacheEntry::OnDisk` con la ruta del archivo

3. **Recuperación**:
   - Al recuperar datos, si están en disco, se deserializan desde el archivo
   - Los datos se mantienen en disco hasta que se limpien explícitamente

#### Configuración

```bash
# Configurar umbral de cache (en MB)
export CACHE_THRESHOLD_MB=200
```

#### Ventajas

- **Eficiencia**: Permite procesar datasets grandes sin agotar la memoria
- **Flexibilidad**: Umbral configurable según recursos disponibles
- **Persistencia**: Los datos en disco sobreviven a reinicios del worker

### Gestión de Memoria en el Master

El master mantiene el estado en memoria usando estructuras concurrentes:

- **Registry de Workers**: `HashMap<String, WorkerInfo>` con `RwLock` para acceso concurrente
- **Jobs**: `HashMap<String, JobInfo>` con tracking de tareas y resultados
- **Tasks**: Registro global de tareas para seguimiento y replanificación

**Nota**: Según la sección 4.1 del enunciado, se requiere persistencia en archivos o sqlite. Actualmente el estado se mantiene solo en memoria. Para producción, se recomendaría implementar persistencia periódica a sqlite.

### Particiones y Shuffle

- **Particiones**: Cada etapa del DAG puede especificar número de particiones
- **Shuffle**: Entre etapas, los datos se redistribuyen según las particiones configuradas
- **Balanceo**: Las particiones se distribuyen entre workers disponibles usando round-robin + awareness de carga

