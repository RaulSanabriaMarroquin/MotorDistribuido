# Instrucciones de Ejecución

Este documento contiene las instrucciones completas para ejecutar el Motor Distribuido, incluyendo pruebas y despliegue con Docker Compose.

## Requisitos Previos

- Rust 1.90+ (edición 2021)
- Cargo
- Docker y Docker Compose (para despliegue con Docker)

## Compilación del Proyecto

### Compilación en modo desarrollo

```bash
cargo build
```

### Compilación en modo release

```bash
cargo build --release
```

## Ejecución de Pruebas

### Ejecutar todas las pruebas

Para ejecutar todas las pruebas (unitarias, integración y end-to-end):

```bash
cargo test
```

### Ejecutar solo pruebas unitarias

Las pruebas unitarias están en cada crate (`master`, `worker`, `common`):

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

### Ejecutar una prueba específica

```bash
cargo test -p tests --test integration_tests test_worker_failure_and_recovery
```

## Ejecución Local (sin Docker)

### Iniciar el Master

```bash
cargo run --bin master
```

El master se inicia en el puerto 8080 por defecto. Puedes configurar el host y puerto con variables de entorno:

```bash
MASTER_HOST=0.0.0.0 MASTER_PORT=8080 cargo run --bin master
```

### Iniciar un Worker

En una terminal separada:

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

### Iniciar múltiples Workers

Para iniciar múltiples workers, abre terminales adicionales y ejecuta:

```bash
# Worker 1
WORKER_PORT=8081 cargo run --bin worker

# Worker 2
WORKER_PORT=8082 cargo run --bin worker

# Worker 3
WORKER_PORT=8083 cargo run --bin worker
```

### Usar el Cliente

En otra terminal:

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

## Ejecución con Docker Compose

### Requisitos

- Docker Desktop (Windows/Mac) o Docker Engine + Docker Compose (Linux)
- Al menos 2GB de RAM disponible
- Puertos 8080-8084 disponibles

### Construir e iniciar el sistema

```bash
# Construir las imágenes y iniciar todos los servicios (master + 3 workers)
docker-compose up --build

# O en modo detached (en segundo plano)
docker-compose up -d --build
```

### Usar scripts de ayuda

```bash
# Linux/Mac
./scripts/docker-start.sh

# Windows
scripts\docker-start.bat
```

### Ver logs

```bash
# Todos los servicios
docker-compose logs -f

# Solo el master
docker-compose logs -f master

# Solo un worker específico
docker-compose logs -f worker1
docker-compose logs -f worker2
docker-compose logs -f worker3
```

### Verificar estado de los servicios

```bash
# Ver qué servicios están corriendo
docker-compose ps

# Verificar que los workers están registrados
cargo run --bin client list-workers
```

### Detener el sistema

```bash
# Detener todos los servicios
docker-compose down

# Detener y eliminar volúmenes
docker-compose down -v
```

### Reiniciar servicios

```bash
# Reiniciar todos los servicios
docker-compose restart

# Reiniciar un servicio específico
docker-compose restart worker1
```

### Simular fallo de worker

Para probar la tolerancia a fallos:

```bash
# Detener un worker
docker-compose stop worker1

# Verificar que el master lo detecta (esperar ~15 segundos)
cargo run --bin client list-workers

# Reiniciar el worker
docker-compose start worker1
```

### Reconstruir desde cero

Si necesitas reconstruir las imágenes:

```bash
# Eliminar contenedores, volúmenes e imágenes
docker-compose down -v --rmi all

# Reconstruir sin usar cache
docker-compose build --no-cache

# Iniciar
docker-compose up -d
```

## Ejecutar Pruebas con Docker Compose

### Preparación

Antes de ejecutar las pruebas, asegúrate de que Docker Compose no esté corriendo:

```bash
docker-compose down
```

### Ejecutar pruebas con el sistema en Docker

Si quieres ejecutar las pruebas mientras el sistema está corriendo en Docker:

1. Inicia el sistema con Docker Compose:
```bash
docker-compose up -d
```

2. Espera unos segundos para que los servicios estén listos

3. Ejecuta las pruebas (las pruebas se conectarán al master en localhost:8080):
```bash
cargo test -p tests --test integration_tests
```

4. Al terminar, detén Docker Compose:
```bash
docker-compose down
```

## Verificar Métricas

### Métricas globales

```bash
# Usando curl
curl http://localhost:8080/api/v1/metrics

# O desde el código, puedes usar el cliente para ver métricas
```

### Métricas de un worker específico

```bash
curl http://localhost:8080/api/v1/workers/w001/metrics
```

### Progreso de un job con métricas

```bash
cargo run --bin client get-progress --job-id <JOB_ID>
```

Esto mostrará información incluyendo:
- Tiempo total de ejecución
- Número de etapas
- Etapa actual
- Total de reintentos

## Solución de Problemas

### Puerto ya en uso

Si obtienes un error de puerto en uso:

```bash
# En Windows, encontrar qué proceso usa el puerto
netstat -ano | findstr :8080

# En Linux/Mac
lsof -i :8080

# Luego matar el proceso o cambiar el puerto en docker-compose.yml
```

### Workers no se registran

1. Verifica que el master esté corriendo:
```bash
docker-compose logs master
```

2. Verifica la conectividad de red:
```bash
docker-compose exec worker1 ping master
```

3. Verifica las variables de entorno en docker-compose.yml

### Las pruebas fallan

1. Asegúrate de que no hay procesos corriendo en los puertos:
```bash
docker-compose down
```

2. Ejecuta las pruebas con más verbosidad:
```bash
RUST_LOG=debug cargo test -p tests --test integration_tests -- --nocapture
```

3. Verifica que todas las dependencias estén instaladas:
```bash
cargo check
```

### Problemas con Docker Compose

1. Verifica que Docker esté corriendo:
```bash
docker ps
```

2. Verifica la versión de Docker Compose:
```bash
docker-compose --version
```

3. Si hay problemas con la red Docker:
```bash
docker network ls
docker network inspect motor_motor-network
```

## Ejemplos de Uso Completo

### Ejemplo 1: Pipeline simple con Docker Compose

```bash
# 1. Iniciar el sistema
docker-compose up -d

# 2. Esperar unos segundos
sleep 5

# 3. Verificar workers
cargo run --bin client list-workers

# 4. Enviar un job
cargo run --bin client submit-job \
  --name "demo" \
  --source-inline "1,2,3,4,5,6,7,8,9,10" \
  --stages "map_add:10,filter_gt:15"

# 5. Ver progreso (usar el job_id de la respuesta anterior)
cargo run --bin client get-progress --job-id <JOB_ID>

# 6. Ver métricas
curl http://localhost:8080/api/v1/metrics

# 7. Detener
docker-compose down
```

### Ejemplo 2: Procesar archivo CSV

```bash
# 1. Crear datos de ejemplo
scripts\create_sample_data.bat
# O en Linux/Mac:
./scripts/create_sample_data.sh

# 2. Iniciar sistema
docker-compose up -d

# 3. Enviar job procesando CSV
cargo run --bin client submit-job \
  --name "process-csv" \
  --source-csv scripts/data/sample.csv \
  --stages "map_mul:2,filter_gt:10"

# 4. Ver progreso
cargo run --bin client get-progress --job-id <JOB_ID>
```

### Ejemplo 3: Probar tolerancia a fallos

```bash
# 1. Iniciar sistema
docker-compose up -d

# 2. Enviar un job largo
cargo run --bin client submit-job \
  --name "fault-test" \
  --source-inline "1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20" \
  --stages "map_add:100,filter_gt:105,map_mul:2"

# 3. Obtener job_id de la respuesta

# 4. En otra terminal, detener un worker
docker-compose stop worker1

# 5. Verificar que el master detecta el fallo
cargo run --bin client list-workers

# 6. Verificar que el job continúa
cargo run --bin client get-progress --job-id <JOB_ID>

# 7. Reiniciar el worker
docker-compose start worker1
```

## Notas Adicionales

- Los logs del sistema se pueden ver con `docker-compose logs -f`
- El nivel de logging se controla con la variable de entorno `RUST_LOG` (info, debug, trace)
- Los workers envían heartbeats cada 3 segundos por defecto
- El timeout de heartbeat es de 15 segundos
- Las métricas se actualizan en tiempo real durante la ejecución

