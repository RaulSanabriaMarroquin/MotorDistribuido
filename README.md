# Motor Distribuido

Proyecto para el curso de Sistemas Operativos.

## Descripción

Sistema distribuido implementado en Rust con una arquitectura master-worker-client.

## Estructura del Proyecto

Este proyecto utiliza un workspace de Cargo con los siguientes componentes:

- **master/**: Nodo maestro que coordina el sistema distribuido
- **worker/**: Nodos trabajadores que ejecutan tareas
- **client/**: Cliente para interactuar con el sistema
- **common/**: Biblioteca compartida con tipos y utilidades comunes
- **docs/**: Documentación del proyecto
- **scripts/**: Scripts de utilidad

## Instalación

Requisitos:
- Rust 1.70+ (edición 2021)
- Cargo

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

#### Ejecutar un Worker
```bash
cargo run --bin worker
```

#### Ejecutar el Cliente
```bash
cargo run --bin client
```

### Opción 2: Ejecución con Docker Compose (Recomendado)

El proyecto incluye un `docker-compose.yml` simple para facilitar el despliegue multinodo.

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

# Escalar workers (agregar más workers)
docker-compose up -d --scale worker1=1 --scale worker2=1 --scale worker3=1
```

#### Acceder al sistema desde el host
Una vez iniciado, puedes usar el cliente desde tu máquina local:
```bash
# Listar workers
cargo run --bin client list-workers

# Enviar un job
cargo run --bin client submit-job --name "test" --source-inline "1,2,3" --stages "map_add:10"
```

#### Estructura del docker-compose
- **master**: Nodo coordinador en puerto 8080
- **worker1**: Worker en puerto 8081
- **worker2**: Worker en puerto 8082
- **worker3**: Worker en puerto 8083

Todos los servicios están en la misma red Docker y se comunican por nombre de servicio.

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
```

### Ejecutar solo pruebas de integración (nodo único)

Las pruebas de integración verifican el funcionamiento del sistema con un solo nodo:

```bash
cargo test -p tests --test integration_tests -- test_worker_registration_single_node test_list_workers_single_node test_heartbeat_single_node test_submit_job_single_node test_job_progress_single_node
```

### Ejecutar solo pruebas end-to-end (multinodo local)

Las pruebas end-to-end verifican el funcionamiento del sistema con múltiples workers:

```bash
cargo test -p tests --test integration_tests -- test_multi_worker_registration test_job_execution_with_worker test_multi_stage_job test_parallel_execution_multiple_workers test_shuffle_between_stages_multiple_workers test_worker_failure_and_recovery
```


## Desarrollo

Este proyecto está ccompletamente desarrollado. Cada componente tiene su propio README con más detalles.

## Autores

- Raúl Marroquin
- David Acuña

## Licencia

_Por agregar..._

