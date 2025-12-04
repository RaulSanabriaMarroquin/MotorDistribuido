# Dockerfile para el Motor Distribuido
FROM rust:1.90 as builder

WORKDIR /app

# Copiar archivos de configuración de Cargo
COPY Cargo.toml Cargo.lock ./
COPY master/Cargo.toml ./master/
COPY worker/Cargo.toml ./worker/
COPY client/Cargo.toml ./client/
COPY common/Cargo.toml ./common/
COPY tests/Cargo.toml ./tests/

# Crear estructura de directorios y copiar código fuente
COPY master/src ./master/src
COPY worker/src ./worker/src
COPY client/src ./client/src
COPY common/src ./common/src

# Compilar en modo release solo los binarios necesarios (master, worker, client)
RUN cargo build --release -p master -p worker -p client

# Imagen final más pequeña
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copiar binarios compilados
COPY --from=builder /app/target/release/master /app/master
COPY --from=builder /app/target/release/worker /app/worker
COPY --from=builder /app/target/release/client /app/client

# Exponer puertos
EXPOSE 8080 8081 8082 8083 8084

# El comando por defecto puede ser sobrescrito en docker-compose
CMD ["./master"]

