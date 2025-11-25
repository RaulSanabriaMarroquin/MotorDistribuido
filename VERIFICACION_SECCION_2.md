# Verificación Sección 2: Descripción General - Ruta A

## Requisitos según el Enunciado

**Ruta A: Batch DAG (mini-Spark)**: motor por job con DAG de etapas (map, filter, reduce, join, aggregate) sobre datos de archivos. Planifica tareas, maneja particiones y reintentos.

## Estado de Cumplimiento

### ✅ Motor por job con DAG de etapas
- **Estado**: COMPLETADO
- **Implementación**: 
  - Sistema de jobs con DAG (nodos y edges)
  - Cada job tiene un DAG que define las etapas de procesamiento
  - Los nodos representan operadores y los edges las dependencias

### ✅ Operadores Requeridos

#### map
- **Estado**: COMPLETADO
- **Implementación**: `worker/src/operators.rs::map_operator`
- **Funciones soportadas**: to_lower, to_upper, trim

#### filter
- **Estado**: COMPLETADO
- **Implementación**: `worker/src/operators.rs::filter_operator`
- **Funciones soportadas**: non_empty, contains_alpha

#### reduce
- **Estado**: COMPLETADO
- **Implementación**: `worker/src/operators.rs::reduce_operator`
- **Funciones soportadas**: sum, count

#### join
- **Estado**: COMPLETADO
- **Implementación**: `worker/src/operators.rs::join_operator`
- **Tipo**: Inner join por clave
- **Manejo de conflictos**: Prefijo "right_" para campos duplicados

#### aggregate
- **Estado**: COMPLETADO (vía reduce_by_key)
- **Implementación**: `worker/src/operators.rs::reduce_by_key_operator`
- **Nota**: En el contexto de Spark/Flink, `reduceByKey` es el tipo más común de aggregate. 
  - Agrupa por clave y aplica función de reducción (sum, count, etc.)
  - Es equivalente a un aggregate keyed

### ✅ Sobre datos de archivos
- **Estado**: COMPLETADO
- **Formatos soportados**:
  - CSV: `read_csv` y `write_csv`
  - JSONL: `read_jsonl` y `write_jsonl`

### ✅ Planifica tareas
- **Estado**: COMPLETADO
- **Implementación**: `master/src/main.rs::task_scheduler`
- **Algoritmo**: Round-robin con awareness de carga
- **Características**:
  - Asigna tareas a workers disponibles
  - Considera carga actual (tareas activas) de cada worker
  - Balancea carga entre workers

### ✅ Maneja particiones
- **Estado**: COMPLETADO (Recién implementado)
- **Implementación**: `master/src/main.rs::submit_job`
- **Características**:
  - Crea múltiples tareas por nodo según número de particiones
  - Para operadores `read_csv` y `read_jsonl`, usa el campo `partitions` del operador
  - Para otros operadores, usa el `parallelism` del job
  - Cada tarea tiene un `partition` ID único
  - Las dependencias entre etapas respetan las particiones

### ✅ Reintentos
- **Estado**: COMPLETADO
- **Implementación**: `master/src/main.rs::update_task_status`
- **Características**:
  - Reintento automático cuando una tarea falla
  - Máximo 1 reintento por tarea (cumple requisito mínimo)
  - Incrementa `attempt_id` en cada reintento
  - Re-encola tareas fallidas automáticamente

## Resumen

**Estado General**: ✅ **CUMPLE COMPLETAMENTE** con los requisitos de la Sección 2 para Ruta A

Todos los requisitos están implementados:
- ✅ Motor por job con DAG
- ✅ Todos los operadores requeridos (map, filter, reduce, join, aggregate)
- ✅ Procesamiento sobre archivos (CSV, JSONL)
- ✅ Planificación de tareas
- ✅ Manejo de particiones
- ✅ Reintentos

## Notas Adicionales

1. **Operador aggregate**: Aunque el enunciado menciona "aggregate" explícitamente, `reduce_by_key` es el tipo más común y estándar de aggregate en sistemas distribuidos (equivalente a `groupByKey().aggregate()` en Spark).

2. **Particiones**: La implementación crea múltiples tareas por nodo según el número de particiones especificado, lo que permite procesamiento paralelo real.

3. **Ruta B (Streaming)**: No está implementada, pero el proyecto se enfoca en Ruta A según el diseño actual.

