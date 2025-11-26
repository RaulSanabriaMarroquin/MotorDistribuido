# Plan de Trabajo - Semanas 3 y 4

## Estado General
- ✅ Semana 1: Completada (Registro, Heartbeats, Detección de fallos)
- ✅ Semana 2: Parcialmente completada (Jobs básicos, map/filter simples)
- ⏳ Semana 3: En progreso
- ⏳ Semana 4: Pendiente

---

## SEMANA 3: Operadores Avanzados y Tolerancia a Fallos

### 3.1 Operadores Faltantes
- [x] **flat_map**: Implementar operador flat_map - ✅ Implementado en worker/src/operators.rs
- [x] **reduce_by_key**: Implementar reduce_by_key (requerido) - ✅ Implementado en worker/src/operators.rs
- [ ] **join**: Implementar join por clave (Ruta A - Batch)
- [ ] **Lectura/escritura**: Soporte para CSV y JSONL

### 3.2 Formato DAG según Enunciado
- [x] Actualizar `JobSpec` para soportar formato DAG (nodos, edges) - ✅ Estructuras agregadas en common/src/lib.rs
- [x] Implementar parser de DAG - ✅ Implementado en master/src/dag.rs con topological sort
- [x] Planificador que procese DAG y cree tareas por etapa - ✅ Implementado en master/src/main.rs submit_job
- [x] Manejo de particiones según especificación - ✅ Soporte básico implementado

### 3.3 Reintentos y Replanificación
- [x] Sistema de reintentos (al menos 1 reintento por tarea fallida) - ✅ Implementado con MAX_TASK_RETRIES=1
- [x] Task attempt ID para idempotencia - ✅ Implementado en TaskAssignment y TaskResult
- [x] Detección de tareas fallidas y reintento automático - ✅ Implementado en job_task_result
- [x] Replanificación cuando worker cae (marcar tareas pendientes) - ✅ Implementado en replanify_worker_tasks

### 3.4 Planificación Mejorada
- [ ] Round-robin con awareness de carga
- [ ] Tracking de carga por worker
- [ ] Asignación inteligente de tareas

---

## SEMANA 4: Observabilidad, Pulido y Demo

### 4.1 Métricas y Observabilidad
- [x] **Métricas por nodo**:
  - [x] Uso de CPU (aprox.) - ✅ Implementado en get_node_metrics
  - [x] Memoria - ✅ Implementado
  - [x] Número de tareas activas - ✅ Tracking en WorkerInfo
  - [x] Latencia promedio - ✅ Calculado de task_latencies
  - [x] #reintentos - ✅ Tracking en WorkerInfo
- [x] **Métricas por job/topología**:
  - [x] Tiempo total - ✅ Calculado de start_time/end_time
  - [x] Etapas - ✅ Calculado del DAG
  - [x] Throughput (Streaming) - ✅ Estructura preparada (None para batch)
  - [x] #fallos - ✅ Tracking en JobInfo
- [x] **Logging estructurado**: JSON o texto con niveles - ✅ Usando tracing con niveles

### 4.2 Persistencia
- [ ] Persistencia del estado del job/topología (archivos o sqlite)
- [ ] Recuperación de estado al reiniciar master

### 4.3 Almacenamiento y Memoria
- [ ] **Batch**: Cache en memoria por partición con spill a disco cuando supere umbral
- [ ] **Streaming**: Colas/buffers con backpressure simple

### 4.4 Infraestructura
- [x] Makefile completo (build, test, demo) - ✅ Creado con targets: build, test, clean, run-*, demo
- [x] Scripts de demo - ✅ Creados demo.sh y demo.ps1
- [ ] Docker-compose actualizado - Pendiente
- [ ] README actualizado con instrucciones - Pendiente

### 4.5 API según Enunciado
- [ ] `POST /api/v1/jobs`: Formato DAG completo
- [ ] `GET /api/v1/jobs/{id}`: Estado, progreso, métricas
- [ ] `GET /api/v1/jobs/{id}/results`: URL/paths de salida

---

## Cambios Realizados

### [Fecha] - Inicio
- Creado documento de planificación
- Analizado estado actual vs enunciado

### [Hoy] - Actualización de Estructuras
- ✅ Actualizado `common/src/lib.rs`:
  - Agregadas estructuras `DagNode`, `DagEdge`, `Dag` según especificación del enunciado
  - Actualizado `JobSpec` para soportar formato DAG (con compatibilidad hacia atrás)
  - Actualizado `TaskAssignment` con nuevos campos: `node_id`, `fn_name`, `key`, `input_path`, `attempt_id`
  - Actualizado `TaskResult` con campos: `node_id`, `attempt_id`, `success`, `error`, `output_data`
- ✅ Actualizado `client/src/main.rs` para usar nuevo formato de `JobSpec`
- ✅ Actualizado `worker/src/main.rs` para manejar nuevo formato de `TaskAssignment` y `TaskResult`
- ✅ Actualizado `master/src/main.rs` para usar nuevas estructuras (soporte legacy mantenido)
- ✅ Compilación exitosa - todo funciona con formato legacy

### [Hoy] - Implementación de Operadores y DAG
- ✅ Creado módulo `worker/src/operators.rs` con operadores:
  - `map`: Operaciones map con funciones add, mul, to_lower
  - `flat_map`: Operaciones flat_map con funciones tokenize, split
  - `filter`: Operaciones filter con funciones gt, lt, eq
  - `reduce_by_key`: Agregación por clave con funciones sum, count, max, min
- ✅ Actualizado `worker/src/main.rs` para usar nuevos operadores
- ✅ Creado módulo `master/src/dag.rs`:
  - Parser de DAG con topological sort
  - Detección de ciclos
  - Funciones helper para root/leaf nodes
- ✅ Actualizado `master/src/main.rs`:
  - Soporte para formato DAG completo
  - Procesamiento de stages con particiones
  - Mantiene compatibilidad con formato legacy
- ✅ Compilación exitosa

### [Hoy] - Pruebas del Sistema
- ✅ **Prueba 1 - Master**: Master inicia correctamente y responde en puerto 8080
- ✅ **Prueba 2 - Worker**: Worker se registra correctamente con el master (w001)
- ✅ **Prueba 3 - Job map_add**: Job enviado y completado exitosamente
  - Input: [1,2,3,4,5], operación: map_add con param=10
  - Resultado: Job completado, 1/1 tareas, 0 fallos
- ✅ **Prueba 4 - Job filter_gt**: Job de filtrado completado exitosamente
  - Input: [1,2,3,4,5,6,7,8,9,10], operación: filter_gt con param=5
  - Resultado: Job completado, 1/1 tareas, 0 fallos
- ✅ **Conclusión**: El sistema funciona correctamente con el formato legacy actualizado

### [Hoy] - Sistema de Reintentos
- ✅ Agregadas estructuras `TaskInfo` y `TaskStatus` para tracking de tareas
- ✅ Actualizado `JobInfo` para incluir HashMap de tareas
- ✅ Implementada lógica de reintentos en `job_task_result`:
  - Detecta tareas fallidas
  - Reintenta automáticamente hasta MAX_TASK_RETRIES (1)
  - Marca tareas como Failed después de max reintentos
  - Marca job como failed si >50% de tareas fallan
- ✅ Registro de tareas en submit_job (tanto DAG como legacy)
- ✅ Compilación exitosa

### [Hoy] - Pruebas del Sistema Actualizado
- ✅ **Prueba 1 - Master**: Master inicia y responde correctamente
- ✅ **Prueba 2 - Worker**: Worker se registra y mantiene estado UP
- ✅ **Prueba 3 - Operación legacy (map_add)**: 
  - Job completado exitosamente
  - Status: completed, Progress: 1/1, Failed: 0
- ✅ **Prueba 4 - Operación flat_map**: 
  - Job completado exitosamente con operación flat_map
  - Status: completed, Progress: 1/1
- ✅ **Prueba 5 - Operación reduce_by_key**: 
  - Job completado exitosamente con operación reduce_by_key
  - Status: completed, Progress: 1/1
- ✅ **Prueba 6 - Formato DAG completo**: 
  - DAG procesado correctamente
  - Parser de DAG funciona (6 tareas creadas para 3 stages x 2 particiones)
  - Topological sort funcionando
- ✅ **Conclusión**: 
  - Todas las operaciones implementadas funcionan correctamente
  - El parser de DAG procesa correctamente los jobs
  - El sistema mantiene compatibilidad con formato legacy
  - Workers se mantienen activos y procesan tareas

### [Hoy] - Semana 4: Métricas y Observabilidad
- ✅ Creado módulo `common/src/metrics.rs` con estructuras:
  - `NodeMetrics`: CPU, memoria, tareas activas, latencia, reintentos
  - `JobMetrics`: tiempo total, etapas, throughput, fallos
  - `MetricsResponse`: respuesta unificada
- ✅ Actualizado `master/src/main.rs`:
  - Agregados campos de métricas a `WorkerInfo` y `JobInfo`
  - Endpoints de métricas: `/api/v1/metrics`, `/api/v1/metrics/nodes`, `/api/v1/metrics/jobs`
  - Funciones para calcular métricas de nodos y jobs
  - Tracking de start_time/end_time en jobs
  - Tracking de stages en jobs
- ✅ Logging estructurado: Ya implementado con `tracing` (info, warn, error)
- ✅ Makefile creado con targets:
  - `build`, `build-release`, `test`, `clean`
  - `run-master`, `run-worker`, `run-client`
  - `demo`, `fmt`, `lint`, `check`
- ✅ Scripts de demo:
  - `scripts/demo.sh` (bash)
  - `scripts/demo.ps1` (PowerShell)
- ✅ Compilación exitosa

### [Hoy] - Completando Pendientes Finales
- ✅ **Docker-compose actualizado**:
  - Creado `docker-compose.yml` con master y 3 workers
  - Configuración de red, healthchecks, y variables de entorno
  - Soporte para múltiples workers en diferentes puertos
- ✅ **Dockerfiles creados**:
  - `Dockerfile.master`: Imagen para el nodo master
  - `Dockerfile.worker`: Imagen para los nodos worker
  - Multi-stage builds para optimizar tamaño
- ✅ **README actualizado**:
  - Documentación completa del proyecto
  - Instrucciones de instalación y uso
  - Ejemplos de API y formatos de jobs
  - Guía de troubleshooting
  - Documentación de características implementadas
- ✅ **Replanificación cuando worker cae**:
  - Función `replanify_worker_tasks` implementada
  - Detecta tareas Running asignadas a worker que cayó
  - Reasigna tareas a workers disponibles
  - Mantiene attempt_id para idempotencia
  - Actualiza tracking de active_tasks
- ✅ **Tracking mejorado de métricas**:
  - Actualización de active_tasks al asignar/completar tareas
  - Tracking de retry_count en workers
  - Decremento de active_tasks cuando tareas fallan o completan
- ✅ Compilación exitosa

### [Hoy] - Pruebas Completas del Sistema
- ✅ **Prueba 1 - Master**: Inicia y responde correctamente
- ✅ **Prueba 2 - Workers**: 2 workers se registran y mantienen estado UP
- ✅ **Prueba 3 - Operación legacy (map_add)**: Job completado exitosamente (1/1)
- ✅ **Prueba 4 - Operación flat_map**: Job completado exitosamente (1/1)
- ✅ **Prueba 5 - Operación reduce_by_key**: Job completado exitosamente (1/1)
- ✅ **Prueba 6 - Métricas**: Endpoints funcionando (2 nodos, 3 jobs)
- ✅ **Prueba 7 - Estado de workers**: Workers mantienen estado UP correctamente
- ✅ **Prueba 8 - Replanificación**: Job completado después de simular fallo de worker
- ✅ **Script de pruebas**: Creado `scripts/test_complete.ps1` para pruebas automatizadas
- ✅ **Conclusión**: Sistema funcionando correctamente, todas las funcionalidades operativas
- ✅ **Publicación**: Todos los cambios publicados en la rama `David2` del repositorio

---

## Notas
- Trabajar paso a paso
- Probar cada funcionalidad antes de continuar
- Mantener compatibilidad con lo ya implementado

