# Motor Distribuido

Sistema distribuido de procesamiento batch implementado en Rust que simula un mini-Spark. Proyecto para el curso de Sistemas Operativos.

## Descripción

Sistema distribuido implementado en Rust con una arquitectura master-worker-client. El sistema procesa datos en batch utilizando un DAG de etapas con operadores como map, filter, reduce, join y aggregate. El master coordina múltiples workers que ejecutan tareas en paralelo, con soporte para tolerancia a fallos, reintentos automáticos y replanificación de tareas.

## Características Principales

- Arquitectura master-worker-client con comunicación HTTP/JSON
- Procesamiento batch con DAG de etapas (map, filter, flat_map, reduce_by_key, join)
- Soporte para múltiples formatos de entrada: CSV, JSONL, plain text
- Planificación round-robin con awareness de carga
- Heartbeats cada 3 segundos para detección de fallos
- Reintentos automáticos y replanificación de tareas fallidas
- Shuffle entre stages para operaciones de agregación
- Persistencia del estado del job
- Métricas y observabilidad del sistema
- Despliegue con Docker Compose

## Estructura del Proyecto

Este proyecto utiliza un workspace de Cargo con los siguientes componentes:

- **master/**: Nodo maestro que coordina el sistema distribuido
- **worker/**: Nodos trabajadores que ejecutan tareas
- **client/**: Cliente CLI para interactuar con el sistema
- **common/**: Biblioteca compartida con tipos y utilidades comunes
- **tests/**: Pruebas de unitarias, integración y end-to-end
- **docs/**: Documentación del proyecto
- **scripts/**: Scripts de utilidad y demostración

## Requisitos

- Rust 1.90+ (edición 2021)
- Cargo
- Docker y Docker Compose (para despliegue con Docker)

## Instalación

Para compilar el proyecto:

```bash
cargo build
```

Para compilar en modo release:

```bash
cargo build --release
```

## Uso

### Opción 1: Ejecución Local (sin Docker)

#### Ejecutar el Master

```bash
cargo run --bin master
```

El master se inicia en el puerto 8080 por defecto. Puedes configurar el host y puerto con variables de entorno:

```bash
MASTER_HOST=0.0.0.0 MASTER_PORT=8080 cargo run --bin master
```

#### Ejecutar un Worker

```bash
cargo run --bin worker
```

El worker se configura mediante variables de entorno:

```bash
MASTER_URL=http://127.0.0.1:8080 \
WORKER_HOST=127.0.0.1 \
WORKER_PORT=8081 \
HEARTBEAT_INTERVAL_SECS=3 \
cargo run --bin worker
```

#### Ejecutar el Cliente

```bash
# Listar workers registrados
cargo run --bin client list-workers

# Enviar un job
cargo run --bin client submit-job \
  --name "test" \
  --source-inline "1,2,3,4,5" \
  --stages "map_add:10,filter_gt:12"

# Ver progreso de un job
cargo run --bin client get-progress --job-id <JOB_ID>
```

### Opción 2: Ejecución con Docker Compose (Recomendado)

El proyecto incluye un `docker-compose.yml` simple para facilitar el despliegue multinodo, cumpliendo con el requisito del enunciado.

#### Requisitos
- Docker
- Docker Compose

#### Iniciar el sistema completo

```bash
# Construir e iniciar todos los servicios (master + 3 workers)
docker-compose up --build

# O en modo detached (en segundo plano)
docker-compose up -d --build
```

#### Usar scripts de ayuda

```bash
# Linux/Mac
./scripts/docker-start.sh

# Windows
scripts\docker-start.bat
```

#### Comandos útiles de Docker Compose

```bash
# Ver logs de todos los servicios
docker-compose logs -f

# Ver logs de un servicio específico
docker-compose logs -f master
docker-compose logs -f worker1

# Detener todos los servicios
docker-compose down

# Reiniciar un servicio específico
docker-compose restart worker1

# Detener un worker para simular fallo
docker-compose stop worker1

# Reiniciar un worker
docker-compose start worker1
```

#### Acceder al sistema desde el host

Una vez iniciado, puedes usar el cliente desde tu máquina local:

```bash
# Listar workers
cargo run --bin client list-workers

# Enviar un job
cargo run --bin client submit-job \
  --name "test" \
  --source-inline "1,2,3" \
  --stages "map_add:10"
```

#### Estructura del docker-compose

- **master**: Nodo coordinador en puerto 8080
- **worker1**: Worker en puerto 8081
- **worker2**: Worker en puerto 8082
- **worker3**: Worker en puerto 8083

Todos los servicios están en la misma red Docker y se comunican por nombre de servicio.

## Operadores Soportados

El sistema soporta los siguientes operadores:

- **map_add**: Suma un valor a cada elemento
- **map_mul**: Multiplica cada elemento por un valor
- **filter_gt**: Filtra elementos mayores que un valor
- **filter_lt**: Filtra elementos menores que un valor
- **reduce_by_key**: Agrupa valores por clave y los suma (requiere shuffle)
- **join**: Une dos datasets por clave (requiere shuffle)
- **flat_map**: Aplana una colección de colecciones

## Pruebas

El proyecto incluye tres tipos de pruebas: unitarias, de integración y end-to-end.

### Ejecutar todas las pruebas

Para ejecutar todas las pruebas (unitarias, integración y end-to-end):

```bash
cargo test
```

### Ejecutar solo pruebas unitarias

```bash
# Todas las pruebas unitarias de todos los crates
cargo test --lib

# Solo pruebas unitarias de un crate específico
cargo test --lib -p master
cargo test --lib -p worker
cargo test --lib -p common
```

### Ejecutar solo pruebas de integración (nodo único)

Las pruebas de integración verifican el funcionamiento del sistema con un solo nodo:

```bash
cargo test -p tests --test integration_tests -- test_worker_registration_single_node test_list_workers_single_node test_heartbeat_single_node test_submit_job_single_node test_job_progress_single_node
```

O usando un filtro de nombre:

```bash
cargo test -p tests --test integration_tests single_node
```

### Ejecutar solo pruebas end-to-end (multinodo local)

Las pruebas end-to-end verifican el funcionamiento del sistema con múltiples workers:

```bash
cargo test -p tests --test integration_tests -- test_multi_worker_registration test_job_execution_with_worker test_multi_stage_job test_parallel_execution_multiple_workers test_shuffle_between_stages_multiple_workers test_worker_failure_and_recovery
```

O usando un filtro de nombre:

```bash
cargo test -p tests --test integration_tests -- --skip single_node
```

## Arquitectura

El sistema sigue una arquitectura master-worker-client:

- **Master**: Coordina el sistema, registra workers, planifica tareas, monitorea heartbeats y gestiona jobs
- **Worker**: Se registra con el master, envía heartbeats periódicos, recibe y ejecuta tareas, reporta resultados
- **Client**: Interfaz CLI para interactuar con el master (listar workers, enviar jobs, consultar progreso)

### Comunicación

- Comunicación HTTP/JSON sobre TCP
- Heartbeats cada 3 segundos (configurable)
- Timeout de heartbeat de 15 segundos
- Planificación round-robin con awareness de carga

### Tolerancia a Fallos

- Detección de workers caídos mediante heartbeats
- Replanificación automática de tareas pendientes
- Reintentos automáticos (al menos 1 reintento por tarea fallida)
- Idempotencia básica para evitar duplicar resultados

## Desarrollo

Este proyecto está completamente desarrollado. Cada componente tiene su propio README con más detalles.

Para más información sobre la arquitectura, consulta `docs/architecture.md`.

## Documentación Adicional

- `DOCKER.md`: Guía completa de uso con Docker Compose
- `docs/architecture.md`: Documentación detallada de la arquitectura
- `scripts/GUION_VIDEO.md`: Guion para la demostración en video

## Autores

- Raúl Marroquin
- David Acuña