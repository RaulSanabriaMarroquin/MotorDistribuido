# Guía de Uso con Docker Compose

Este documento explica cómo usar Docker Compose para ejecutar el Motor Distribuido.

## Requisitos

- Docker Desktop (Windows/Mac) o Docker Engine + Docker Compose (Linux)
- Al menos 2GB de RAM disponible
- Puertos 8080-8084 disponibles

## Inicio Rápido

### 1. Construir e iniciar el sistema

```bash
# Construir las imágenes y iniciar todos los servicios
docker-compose up --build
```

Esto iniciará:
- 1 Master en puerto 8080
- 3 Workers en puertos 8081, 8082, 8083

### 2. Verificar que todo está funcionando

En otra terminal:

```bash
# Listar workers registrados
cargo run --bin client list-workers
```

Deberías ver 3 workers con estado UP.

### 3. Detener el sistema

```bash
# Detener y eliminar contenedores
docker-compose down
```

## Comandos Útiles

### Ver logs

```bash
# Todos los servicios
docker-compose logs -f

# Solo el master
docker-compose logs -f master

# Solo un worker
docker-compose logs -f worker1
```

### Reiniciar servicios

```bash
# Reiniciar todos
docker-compose restart

# Reiniciar solo un worker
docker-compose restart worker1
```

### Escalar workers

Para agregar más workers, edita `docker-compose.yml` o usa:

```bash
# Iniciar con más instancias de worker2
docker-compose up -d --scale worker2=2
```

### Ejecutar comandos dentro de contenedores

```bash
# Ejecutar cliente dentro del contenedor master
docker-compose exec master ./client list-workers

# Acceder a shell del contenedor
docker-compose exec master /bin/bash
```

## Estructura de Red

Todos los servicios están en la red `motor-network` y se comunican usando nombres de servicio:

- Master: `http://master:8080`
- Workers: `http://worker1:8081`, `http://worker2:8082`, etc.

Desde el host, usa `localhost`:
- Master: `http://localhost:8080`
- Workers: `http://localhost:8081`, etc.

## Variables de Entorno

Puedes personalizar el comportamiento editando las variables de entorno en `docker-compose.yml`:

- `MASTER_URL`: URL del master (para workers)
- `WORKER_PORT`: Puerto del worker
- `HEARTBEAT_INTERVAL_SECS`: Intervalo de heartbeats (default: 3)
- `RUST_LOG`: Nivel de logging (info, debug, trace)

## Troubleshooting

### Los workers no se registran

1. Verifica que el master esté corriendo:
   ```bash
   docker-compose logs master
   ```

2. Verifica la conectividad de red:
   ```bash
   docker-compose exec worker1 ping master
   ```

### Los puertos están ocupados

Edita `docker-compose.yml` y cambia los puertos mapeados:
```yaml
ports:
  - "9080:8080"  # Cambiar 8080 a 9080
```

### Reconstruir desde cero

```bash
# Eliminar contenedores, volúmenes e imágenes
docker-compose down -v --rmi all

# Reconstruir
docker-compose build --no-cache
```

## Desarrollo con Docker

Para desarrollo activo, puedes montar el código fuente:

```yaml
# En docker-compose.yml, agregar volumes:
volumes:
  - .:/app
  - cargo-cache:/root/.cargo
```

Esto permite cambios en tiempo real, pero es más lento que usar binarios pre-compilados.

