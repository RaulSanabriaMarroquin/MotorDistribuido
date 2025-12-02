# Guía Rápida para el Video

Este documento es un resumen rápido de los archivos creados para el video demostrativo.

## Archivos Creados

1. **GUION_VIDEO.md** - Guion detallado de 15 minutos para dos personas
2. **INSTRUCCIONES_VIDEO.md** - Instrucciones completas de instalación, ejecución y pruebas
3. **Makefile** (actualizado) - Comandos para instalación y pruebas
4. **scripts/test_failure.ps1** - Script para simular fallo de worker
5. **scripts/run_worker.ps1** - Script helper para ejecutar workers

## Comandos Principales

### Instalación
```powershell
make install
# o manualmente:
cargo build --release
```

### Ejecución
```powershell
# Terminal 1: Master
make run-master

# Terminal 2: Worker
.\scripts\run_worker.ps1 -Port 9000

# Terminal 3: Cliente
cargo run --release --bin client -- list-workers
```

### Pruebas
```powershell
# Tests unitarios
make test

# Prueba básica
make test-basic

# Prueba completa
make test-complete

# Prueba de fallo
make test-failure
```

### Detener Todo
```powershell
make stop-all
```

## Estructura del Video

- **Parte 1 (Persona A, 7.5 min)**: Instalación y configuración
- **Parte 2 (Persona B, 7.5 min)**: Ejecución y casos de prueba (incl. fallo simulado)

## Checklist Pre-Grabación

- [ ] Rust instalado (`rustc --version`)
- [ ] Proyecto compilado (`make build-release`)
- [ ] Tests pasando (`make test`)
- [ ] Scripts funcionando (`make test-basic`)
- [ ] Archivos de prueba creados
- [ ] Múltiples terminales preparadas
- [ ] Guion revisado

## Notas Importantes

- Los scripts de prueba requieren que los binarios estén compilados en `target/release/`
- En Windows, usar `.\scripts\run_worker.ps1` es más confiable que `make run-worker`
- Para la simulación de fallo, asegurarse de tener al menos 2 workers corriendo
- Los tests pueden tardar varios segundos, tener paciencia

