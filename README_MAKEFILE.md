# Uso del Makefile / Script PowerShell

## Problema en Windows

El comando `make` no está disponible por defecto en Windows. Para solucionarlo, se ha creado un script PowerShell equivalente (`make.ps1`) que replica toda la funcionalidad del Makefile.

## Uso

### En Windows (PowerShell)

```powershell
# Ver ayuda
.\make.ps1 help
# o simplemente
.\make.ps1

# Verificar código
.\make.ps1 check

# Compilar
.\make.ps1 build-release

# Ejecutar tests
.\make.ps1 test

# Instalar dependencias
.\make.ps1 install

# Ejecutar pruebas del sistema
.\make.ps1 test-basic
.\make.ps1 test-complete
.\make.ps1 test-failure

# Ejecutar componentes
.\make.ps1 run-master
.\make.ps1 run-worker  # Requiere: $env:WORKER_PORT=9000
.\make.ps1 run-client

# Detener procesos
.\make.ps1 stop-all
```

### En Linux/Mac (con make instalado)

```bash
# Ver ayuda
make help

# Verificar código
make check

# Compilar
make build-release

# Ejecutar tests
make test

# Instalar
make install

# Ejecutar pruebas
make test-basic
make test-complete
make test-failure

# Ejecutar componentes
make run-master
make run-worker WORKER_PORT=9000
make run-client

# Detener procesos
make stop-all
```

## Comandos Disponibles

### Instalación
- `install` - Instalar dependencias y compilar (equivalente a: deps + build-release)
- `deps` - Descargar dependencias

### Compilación
- `build` - Build in debug mode
- `build-release` - Build in release mode (optimized)

### Pruebas
- `test` - Run unit tests
- `test-basic` - Ejecutar prueba básica del sistema
- `test-complete` - Ejecutar suite completa de pruebas
- `test-failure` - Ejecutar prueba de simulación de fallo

### Ejecución
- `run-master` - Run master node
- `run-worker` - Run worker node (requiere WORKER_PORT en Windows)
- `run-client` - Run client CLI
- `stop-all` - Detener todos los procesos master/worker

### Utilidades
- `clean` - Clean build artifacts
- `demo` - Run demo script
- `fmt` - Formatear código
- `lint` - Ejecutar clippy
- `check` - Verificar código sin compilar

## Notas

- En Windows, el script `make.ps1` requiere PowerShell
- Para ejecutar `run-worker` en Windows, primero configura la variable de entorno:
  ```powershell
  $env:WORKER_PORT=9000
  .\make.ps1 run-worker
  ```
- Los scripts de prueba (`test-basic`, `test-complete`, `test-failure`) requieren que los binarios estén compilados en `target/release/`

