# Solución de Problemas en WSL

## Error: `make: *** [Makefile:71: run-master] Error 101`

Este error indica que la compilación falló en WSL. El error más común es:

### Error: `Could not find directory of OpenSSL installation`

**Causa:** Falta OpenSSL y sus librerías de desarrollo en WSL.

**Solución Rápida (Recomendada):**

```bash
# En WSL, instalar OpenSSL y sus librerías de desarrollo
sudo apt update
sudo apt install -y pkg-config libssl-dev

# Luego compilar
cargo build --release
```

**Alternativa (si la anterior no funciona):**

```bash
# Para Ubuntu/Debian
sudo apt install -y build-essential pkg-config libssl-dev

# Para Fedora/RHEL
sudo dnf install -y openssl-devel pkg-config

# Para Arch Linux
sudo pacman -S openssl pkg-config
```

## Otros Errores de Compilación

## Diagnóstico

### 1. Verificar OpenSSL (Error más común)

```bash
# Verificar si OpenSSL está instalado
openssl version

# Verificar librerías de desarrollo
pkg-config --modversion openssl

# Si no está instalado, instalar:
sudo apt install -y pkg-config libssl-dev
```

### 2. Verificar que el proyecto compila en WSL

```bash
# En WSL
cd /mnt/c/Users/USUARIO/Documents/Github/MotorDistribuido

# Verificar estructura del proyecto
ls -la common/src/

# Deberías ver:
# - lib.rs
# - metrics.rs

# Verificar que Cargo.toml existe
ls -la common/Cargo.toml
```

### 2. Verificar compilación manual

```bash
# Intentar compilar manualmente
cargo build --release --bin master

# Si falla, ver el error completo:
cargo build --release --bin master 2>&1 | tee build.log
```

## Soluciones Comunes

### Solución 1: Problemas de Permisos

Los archivos creados en Windows pueden tener permisos incorrectos en WSL:

```bash
# Dar permisos de ejecución y lectura
chmod -R u+w common/
chmod -R u+r common/src/*
```

### Solución 2: Recompilar desde cero

```bash
# Limpiar y recompilar
cargo clean
cargo build --release
```

### Solución 3: Verificar que common existe

Si el directorio `common` no existe o está vacío en WSL:

```bash
# Verificar estructura
ls -la common/
ls -la common/src/

# Si falta, recrear desde Windows o copiar
```

### Solución 4: Problemas de Rutas (Windows/WSL)

Si el proyecto está en `/mnt/c/...`, puede haber problemas de rendimiento o permisos:

```bash
# Opción A: Copiar proyecto a WSL nativo
cp -r /mnt/c/Users/USUARIO/Documents/Github/MotorDistribuido ~/MotorDistribuido
cd ~/MotorDistribuido
cargo build --release

# Opción B: Trabajar directamente desde /mnt/c (más lento pero funciona)
cd /mnt/c/Users/USUARIO/Documents/Github/MotorDistribuido
cargo build --release
```

### Solución 5: Verificar Dependencias

```bash
# Actualizar dependencias
cargo update

# Verificar que todas las dependencias están disponibles
cargo fetch
```

## Verificación Rápida

Ejecuta este script en WSL para diagnosticar:

```bash
#!/bin/bash
echo "=== Diagnóstico WSL ==="
echo "1. Verificando estructura..."
ls -la common/src/ 2>/dev/null && echo "✓ common/src existe" || echo "✗ common/src NO existe"
ls -la common/Cargo.toml 2>/dev/null && echo "✓ common/Cargo.toml existe" || echo "✗ common/Cargo.toml NO existe"

echo ""
echo "2. Verificando compilación..."
cargo check --workspace 2>&1 | tail -5

echo ""
echo "3. Verificando permisos..."
ls -la common/ | head -5
```

## Solución Recomendada

Si el problema persiste, la mejor solución es:

1. **Trabajar desde Windows** para desarrollo (más rápido)
2. **O copiar el proyecto a WSL nativo** (`~/MotorDistribuido`) para mejor rendimiento

```bash
# Copiar a WSL
cp -r /mnt/c/Users/USUARIO/Documents/Github/MotorDistribuido ~/MotorDistribuido
cd ~/MotorDistribuido

# Compilar
cargo build --release

# Ejecutar
make run-master
# o
cargo run --release --bin master
```

## Nota sobre Makefile en WSL

El Makefile original funciona en WSL/Linux. El script `make.ps1` es solo para Windows PowerShell.

En WSL, usa directamente:
```bash
make run-master
# o
cargo run --release --bin master
```

