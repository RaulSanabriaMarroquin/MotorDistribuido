# Motor de Procesamiento Distribuido desde Cero

---

## 1. Objetivo de Aprendizaje

Dise├▒ar e implementar un sistema distribuido desde cero que ejercite conceptos de procesos/hilos, IPC/redes, planificaci├│n, memoria, sistema de archivos y coordinaci├│n distribuida. El sistema debe exponer una API cliente para enviar trabajos y consultar estado, y ejecutar dichos trabajos en un cluster sencillo de nodos workers coordinados por un master.

---

## 2. Descripci├│n General

Cada equipo implementar├í una de las dos rutas:

- **Ruta A: Batch DAG (mini-Spark)**: motor por job con DAG de etapas (map, filter, reduce, join, aggregate) sobre datos de archivos. Planifica tareas, maneja particiones y reintentos.

- **Ruta B: Streaming (mini-Flink)**: motor por topolog├¡a con operadores (map, filter, keyed window aggregate) sobre flujos. Debe soportar ventanas por tiempo y checkpointing simplificado.

> **Nota**: El proyecto es acad├®mico (no producci├│n). Se prioriza claridad de dise├▒o, cobertura de pruebas y evidencia de conceptos de SO.

---

## 3. Lenguaje, Entorno y Restricciones

### Lenguaje
- **Rust** o **Go**

### Restricciones
- ÔØî **Prohibido** usar frameworks de computaci├│n distribuida/streaming (Spark/Flink/Ray/Temporal/etc.)
- Ô£à **Permitidas** librer├¡as est├índar de:
  - Red (TCP/UDP/HTTP)
  - Serializaci├│n (JSON/MsgPack)
  - Logging
  - Utilidades b├ísicas

### Ejecuci├│n
- Multi-proceso o multi-hilo por nodo
- Comunicaci├│n inter-nodo por sockets TCP o HTTP/1.1
- Despliegue local multinodo: m├║ltiples procesos en la misma m├íquina o en varias m├íquinas (opcional)
- Se debe usar un **docker-compose simple**

### Reproducibilidad
- Makefile o scripts bash para build, test y demo

---

## 4. Arquitectura M├¡nima Requerida

### 4.1 Componentes

#### 1. Master/Coordinator

- Registro de workers y heartbeats
- Recepci├│n de jobs (Batch) o topolog├¡as (Streaming)
- **Planificador**: asigna tareas/operadores a workers; pol├¡tica b├ísica (round-robin + awareness de carga)
- Persistencia m├¡nima del estado del job/topolog├¡a (en archivos o sqlite local)

#### 2. Workers

- Ejecuci├│n de tareas/operadores aisladas en hilos o procesos
- Manejo de particiones de datos (Batch) o buffers de eventos (Streaming)
- Reintentos y reportes de estado (├®xito/falla, m├®tricas)

#### 3. Cliente (CLI)

- Env├¡o de job/topolog├¡a v├¡a API
- Consulta de estado, progreso, m├®tricas y descarga de resultados

### 4.2 Coordinaci├│n y Tolerancia a Fallos (Simulada)

- **Heartbeats** (cada 1-3 s) desde workers; si un worker deja de latir, el master lo marca DOWN y replanifica tareas pendientes
- **Reintentos**: al menos 1 reintento por tarea fallida
- **Checkpoints** (s├│lo Ruta B): snapshot peri├│dico del estado de ventanas/keys a disco local (best-effort)
- **Idempotencia b├ísica**: evitar duplicar resultados en reejecuciones (e.g., task attempt id)

### 4.3 Almacenamiento y Memoria

- **Batch**: cache en memoria por partici├│n con spill a disco cuando supere umbral configurable
- **Streaming**: colas/buffers con backpressure simple (bloqueo o ca├¡da controlada de tasa)

---

## 5. API (Especificaci├│n)

### 5.1 Contrato HTTP/JSON m├¡nimo

#### Endpoints Batch:
- `POST /api/v1/jobs`: cuerpo JSON con dag (nodos, edges), inputs, paralelismo, partitions
- `GET /api/v1/jobs/{id}`: estado (ACCEPTED/RUNNING/FAILED/SUCCEEDED), progreso (%), m├®tricas
- `GET /api/v1/jobs/{id}/results`: URL/paths de salida

#### Endpoints Streaming:
- `POST /api/v1/topologies`: operadores y wiring; ventanas (tama├▒o, slide); claves
- `GET /api/v1/topologies/{id}`: estado, progreso, m├®tricas
- `POST /api/v1/ingest`: endpoint opcional para inyectar eventos (JSON por l├¡nea)

### 5.2 Formato de Job/Topolog├¡a

#### Ejemplo de Job (Batch):

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

---

## 6. Operadores M├¡nimos por Ruta

### Operadores Comunes

- `map`
- `flat_map`
- `filter`
- `reduce` / `reduce_by_key`

### Ruta A (Batch DAG)

- `join` por clave entre dos colecciones
- `shuffle` entre etapas (reparto de claves ÔåÆ particiones)
- Lectura/escritura: CSV y JSONL

### Ruta B (Streaming)

- Ventanas por tiempo (tumbling y/o sliding)
- Keyed aggregate con estado por clave
- Ingesta por stdin, archivo tail o `/api/v1/ingest`

---

## 7. Planificador y Ejecuci├│n

- **Planificaci├│n**: cola de tareas en el master + asignaci├│n round-robin con carga
- **Ejecuci├│n**: cada worker mantiene un pool de hilos/procesos configurables
- **Aislamiento**: cada tarea se ejecuta con l├¡mites de memoria/tiempo configurables (terminaci├│n si excede)

---

## 8. M├®tricas y Observabilidad

### Por nodo:
- Uso de CPU (aprox.)
- Memoria
- N├║mero de tareas activas
- Latencia promedio
- #reintentos

### Por job/topolog├¡a:
- Tiempo total
- Etapas
- Throughput (Streaming)
- #fallos

### Logging:
- Logging estructurado (JSON o texto) con niveles

---

## 9. Casos de Prueba y Datasets

### Batch recomendado:
- **WordCount** en CSV/JSONL (text, ts)
- **Joins**: ventas & cat├ílogo (100-500 MB totales con archivos repetidos para particiones)

### Streaming recomendado:
- Flujo de logs (JSONL con ts, level, service, msg) y agregaci├│n por ventana

---

## 10. Hitos por Semana (4 Semanas)

### Semana 1 - Dise├▒o y scaffolding
- Documento de arquitectura (proceso master/worker, IPC, API)
- Prototipo de registro de workers y healthcheck

### Semana 2 - Planificaci├│n y ejecuci├│n b├ísica
- Env├¡o de job/topolog├¡a
- Ejecuci├│n de map/filter
- M├®trica de progreso

### Semana 3 - Operadores avanzados y tolerancia a fallos
- reduce_by_key + (join o ventanas)
- Reintentos y replanificaci├│n

### Semana 4 - Observabilidad, pulido y demo
- M├®tricas, logging
- Makefile, README
- Video y defensa

---

## 11. Instrucciones de Desarrollo

### Estructura del Repositorio

```
/master
/worker
/client
/docs
/scripts
```

### Pruebas
- **Unitarias**: operadores
- **Integraci├│n**: nodo ├║nico
- **End-to-end**: multinodo local

### Fallos simulados
- Matar un worker durante la ejecuci├│n y demostrar recuperaci├│n

### Benchmarks m├¡nimos
- **Batch**: lote de 1M registros
- **Streaming**: 5k eventos/s durante 60s
- Con reporte de resultados

---

## 12. Restricciones y Buenas Pr├ícticas

- ÔØî Sin frameworks distribuidos; escribir su propio planificador y ejecuci├│n
- Ô£à Serializaci├│n consistente (JSON o MsgPack) y versionado del mensaje
- Ô£à No bloquear el master con trabajo pesado (s├│lo coordinaci├│n)
- Ô£à Manejo de se├▒ales para apagado ordenado

---

## 13. Entregables

1. **C├│digo fuente** en repo con:
   - README (build/ejecuci├│n)
   - Makefile
   - Scripts

2. **Documento de arquitectura**: 
   - Modelo de procesos/hilos
   - API
   - Protocolos
   - Planificaci├│n
   - Memoria
   - Fallos

3. **Suite de pruebas** (con c├│mo ejecutarlas)

4. **Video demostrativo**: 
   - Instalaci├│n
   - Ejecuci├│n
   - Casos de prueba (incl. fallo simulado)

5. **Reporte de benchmarks**: 
   - Entorno
   - Par├ímetros
   - Resultados

---

## 14. Pol├¡tica de Entrega

- **Entrega**: antes de las **10:00 pm** del d├¡a de la entrega
- **Formato**: archivo `.zip` mediante el TecDigital
- **Grupos**: en los grupos de trabajo previamente establecidos
- **Penalizaci├│n**: 5 puntos porcentuales por cada 24 horas de retraso acumuladas

---

## 15. R├║brica de Evaluaci├│n

| Criterio | Pond. | Descripci├│n |
|:---------|:----:|:------------|
| Dise├▒o y documento de arquitectura | 15% | Claridad, decisiones de SO, diagramas |
| API y cliente (contrato y usabilidad) | 10% | Endpoints, validaci├│n, CLI usable |
| Coordinaci├│n y planificaci├│n | 15% | Registro, heartbeats, asignaci├│n, reintentos |
| Ejecuci├│n distribuida y operadores m├¡nimos | 20% | map/filter/flat_map + reduce_by_key; (join o ventanas) |
| Tolerancia a fallos (simulada) | 10% | Detecci├│n, replanificaci├│n, idempotencia b├ísica |
| Memoria/Almacenamiento & backpressure | 10% | Cache/Spill (Batch) o buffers/ventanas (Streaming) |
| Pruebas (unitarias/integraci├│n/E2E) | 10% | Cobertura y automatizaci├│n |
| Observabilidad y m├®tricas | 5% | Logs, m├®tricas por nodo y por job/topolog├¡a |
| Demo y benchmarks | 5% | Video claro, escenarios y reporte de rendimiento |
| Calidad del c├│digo y repositorio | 5% | Organizaci├│n, lectura, scripts de build |
| **Total** | **100%** | |

---

## 16. Notas y Alcances

- Se evaluar├í el **entendimiento de SO** evidenciado en el dise├▒o y en la implementaci├│n, no s├│lo la funcionalidad
- Se permite usar IA como apoyo (b├║squeda, documentaci├│n); cualquier uso debe declararse en el README

---
