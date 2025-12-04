# Scripts de Utilidad

Esta carpeta contiene scripts para facilitar el uso, demostración y pruebas del Motor Distribuido.

## Scripts Disponibles

### Inicio del Sistema con Docker Compose

- **docker-start.sh / docker-start.bat**: Inicia el sistema completo usando Docker Compose (master + 3 workers)

### Inicio del Sistema Local (sin Docker)

- **start_master.sh / start_master.bat**: Inicia el master en el puerto 8080
- **start_worker.sh / start_worker.bat**: Inicia un worker (requiere número de worker como argumento)

### Preparación de Datos

- **create_sample_data.sh / create_sample_data.bat**: Crea archivos de ejemplo (CSV, JSONL, plain text) en `scripts/data/`
- **create_join_data.sh / create_join_data.bat**: Crea datos de ejemplo para operaciones de join

### Demostraciones

- **demo_complex_pipeline.sh / demo_complex_pipeline.bat**: Demuestra un pipeline complejo con reduce_by_key
- **demo_complete_use_case.sh / demo_complete_use_case.bat**: Caso de uso completo con procesamiento de archivos

### Utilidades

- **kill_worker.sh / kill_worker.bat**: Simula la muerte de un worker específico (para pruebas de tolerancia a fallos)
- **cleanup.sh / cleanup.bat**: Limpia procesos en ejecución y datos generados

### Documentación

- **GUION_VIDEO.md**: Guion completo para la demostración en video (15 minutos)

## Uso

### Con Docker Compose (Recomendado)

```bash
# Linux/Mac
./scripts/docker-start.sh

# Windows
scripts\docker-start.bat
```

Esto construye las imágenes Docker e inicia el sistema completo (master + 3 workers).

### Sin Docker (Local)

#### En Linux/Mac:

```bash
chmod +x scripts/*.sh

# Terminal 1: Iniciar Master
./scripts/start_master.sh

# Terminal 2: Iniciar Worker 1
./scripts/start_worker.sh 1

# Terminal 3: Iniciar Worker 2
./scripts/start_worker.sh 2
```

#### En Windows:

```cmd
REM Terminal 1: Iniciar Master
scripts\start_master.bat

REM Terminal 2: Iniciar Worker 1
scripts\start_worker.bat 1

REM Terminal 3: Iniciar Worker 2
scripts\start_worker.bat 2
```

### Crear Datos de Ejemplo

```bash
# Linux/Mac
./scripts/create_sample_data.sh

# Windows
scripts\create_sample_data.bat
```

Esto crea archivos de ejemplo en `scripts/data/`:
- `sample.csv`: Archivo CSV con números del 1 al 20
- `sample.jsonl`: Archivo JSONL con objetos `{"value": n}`
- `sample.txt`: Archivo de texto plano con números del 1 al 20

### Ejecutar Demostraciones

```bash
# Pipeline complejo
./scripts/demo_complex_pipeline.sh
# O en Windows:
scripts\demo_complex_pipeline.bat

# Caso de uso completo
./scripts/demo_complete_use_case.sh
# O en Windows:
scripts\demo_complete_use_case.bat
```

### Simular Fallo de Worker

Para probar la tolerancia a fallos:

```bash
# Con Docker Compose
docker-compose stop worker1

# Local (Linux/Mac)
./scripts/kill_worker.sh 1

# Local (Windows)
scripts\kill_worker.bat 1
```

### Limpiar

```bash
# Linux/Mac
./scripts/cleanup.sh

# Windows
scripts\cleanup.bat
```

## Notas

- Los scripts `.sh` son para Linux/Mac (Bash/Zsh)
- Los scripts `.bat` son para Windows (Command Prompt)
- Algunos scripts requieren argumentos (ver el guion para detalles)
- Los scripts de Docker Compose son los recomendados para uso general
- Los scripts locales son útiles para desarrollo y debugging

## Estructura de Datos

Los scripts crean datos en `scripts/data/`:

- `sample.csv`: CSV con columna `value`
- `sample.jsonl`: JSONL con objetos `{"value": n}`
- `sample.txt`: Texto plano, un número por línea
- `test.csv`: CSV adicional para pruebas
