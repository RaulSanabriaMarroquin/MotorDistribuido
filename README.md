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

### Ejecutar el Master
```bash
cargo run --bin master
```

### Ejecutar un Worker
```bash
cargo run --bin worker
```

### Ejecutar el Cliente
```bash
cargo run --bin client
```

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

## Desarrollo

Este proyecto está en desarrollo. Cada componente tiene su propio README con más detalles.

## Autores

- Raúl Marroquin
- David Acuña

## Licencia

_Por agregar..._

