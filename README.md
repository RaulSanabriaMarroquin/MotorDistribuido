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

## Desarrollo

Este proyecto está en desarrollo. Cada componente tiene su propio README con más detalles.

## Contribución

_Por agregar..._

## Licencia

_Por agregar..._

