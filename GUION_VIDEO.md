# Guion de Video Demostrativo - Motor Distribuido
## Duración: 15 minutos | Dos Personas
## Entorno: WSL/Linux

---

## PARTE 1: PERSONA A (7.5 minutos)
### Instalación y Configuración Inicial

#### [0:00 - 0:30] Introducción
**Persona A:**
- "Hola, soy [Nombre]. En este video vamos a demostrar el Motor Distribuido, un sistema de procesamiento distribuido implementado en Rust."
- "Este video está dividido en dos partes: en la primera parte mostraré la instalación y configuración, y en la segunda parte [Persona B] demostrará la ejecución y casos de prueba."
- "Estamos ejecutando todo en WSL (Windows Subsystem for Linux) o Linux nativo."

#### [0:30 - 2:00] Verificación de Requisitos
**Persona A:**
- Mostrar la terminal y verificar que Rust está instalado:
  ```bash
  rustc --version
  cargo --version
  ```
- Explicar: "Necesitamos Rust 1.70 o superior y Cargo para compilar el proyecto."
- Verificar que estamos en el directorio del proyecto:
  ```bash
  pwd
  ls -la
  ```
- Verificar que tenemos `make` instalado:
  ```bash
  make --version
  ```
  - Si no está instalado: "En Ubuntu/Debian podemos instalarlo con `sudo apt-get install build-essential`"

#### [2:00 - 4:00] Instalación y Compilación
**Persona A:**
- "Ahora vamos a compilar el proyecto usando el Makefile."
- Mostrar ayuda del Makefile:
  ```bash
  make help
  ```
- Explicar: "El Makefile nos proporciona comandos convenientes para compilar, probar y ejecutar el sistema."
- Compilar el proyecto:
  ```bash
  make build-release
  ```
- Mientras compila, explicar: "Estamos compilando en modo release para obtener el mejor rendimiento. Esto puede tomar unos minutos, especialmente la primera vez, ya que necesita compilar todas las dependencias de Rust."
- Al finalizar, verificar que los binarios se crearon:
  ```bash
  ls -lh target/release/master target/release/worker target/release/client
  ```
- Explicar: "Tenemos tres binarios: master, worker y client. En Linux no tienen extensión .exe como en Windows."

#### [4:00 - 5:30] Verificación de Instalación
**Persona A:**
- "Vamos a verificar que la instalación fue exitosa ejecutando los tests unitarios:"
  ```bash
  make test
  ```
- Mientras se ejecutan, explicar: "Los tests verifican que todos los operadores (map, filter, reduce_by_key, join) y la lógica del DAG funcionan correctamente."
- Mostrar el resultado esperado:
  ```
  running 7 tests (master - DAG parsing)
  test result: ok. 7 passed; 0 failed
  
  running 20 tests (worker - operators)
  test result: ok. 20 passed; 0 failed
  
  Total: 27 tests pasando, 0 fallando
  ```
- Confirmar: "La instalación fue exitosa. Todos los componentes están funcionando correctamente."

#### [5:30 - 7:30] Preparación del Entorno
**Persona A:**
- "Antes de pasar a la ejecución, voy a preparar algunos archivos de prueba que usaremos más adelante."
- Crear un archivo de prueba CSV:
  ```bash
  echo -e "1\n2\n3\n4\n5" > test_data.csv
  cat test_data.csv
  ```
- Crear un archivo de prueba JSONL:
  ```bash
  echo '{"value": 10}' > test_data.jsonl
  echo '{"value": 20}' >> test_data.jsonl
  echo '{"value": 30}' >> test_data.jsonl
  cat test_data.jsonl
  ```
- Explicar: "Estos archivos los usaremos para probar las operaciones de lectura (read_csv y read_jsonl)."
- "Ahora voy a pasar la palabra a [Persona B] para que demuestre la ejecución y los casos de prueba."

---

## PARTE 2: PERSONA B (7.5 minutos)
### Ejecución y Casos de Prueba

#### [7:30 - 8:00] Transición
**Persona B:**
- "Hola, soy [Nombre]. Ahora voy a demostrar cómo ejecutar el sistema y realizar casos de prueba, incluyendo la simulación de un fallo."

#### [8:00 - 9:30] Inicio del Sistema
**Persona B:**
- "Primero, vamos a iniciar el master. Necesitamos múltiples terminales para esto."
- **Paso 1: Iniciar Master (Terminal 1)**
  ```bash
  make run-master
  ```
  - Explicar: "El master está escuchando en http://127.0.0.1:8080. Verás el mensaje 'Nodo master escuchando en 127.0.0.1:8080'."
  - Dejar esta terminal corriendo.
- **Paso 2: Iniciar Worker (Terminal 2 - Nueva ventana)**
  ```bash
  export WORKER_PORT=9000
  make run-worker
  # O directamente:
  WORKER_PORT=9000 cargo run --release --bin worker
  ```
  - Explicar: "El worker se registra automáticamente con el master y comienza a enviar heartbeats cada 3 segundos."
  - Mostrar el mensaje: "Registro exitoso worker_id=w001" y "Heartbeat enviado".
  - Dejar esta terminal corriendo.
- **Paso 3: Verificar Registro (Terminal 3 - Nueva ventana)**
  ```bash
  cargo run --release --bin client -- list-workers
  ```
  - Mostrar la salida esperada:
    ```
    Workers registrados:
    - w001 @ 127.0.0.1:9000 [UP] tareas activas: 0
    ```
  - Explicar: "Podemos ver que el worker está registrado, activo (UP), y listo para recibir tareas."

#### [9:30 - 11:30] Casos de Prueba Básicos
**Persona B:**
- "Ahora vamos a ejecutar pruebas automatizadas. Primero, necesitamos detener los procesos que están corriendo."
- Detener procesos actuales:
  ```bash
  make stop-all
  # O manualmente: Ctrl+C en cada terminal
  ```
- **Paso 1: Prueba Básica Automatizada**
  - "Ahora ejecutaremos el script de prueba básica que automatiza todo el proceso:"
    ```bash
    make test-basic
    ```
  - Mientras se ejecuta, explicar: "Este script automáticamente:"
    - "1. Limpia procesos anteriores"
    - "2. Compila si es necesario"
    - "3. Inicia master y worker"
    - "4. Verifica el registro"
    - "5. Envía un job de prueba (map_add)"
    - "6. Verifica que el job se completó"
  - Mostrar los resultados esperados:
    ```
    ✓ Master está corriendo correctamente
    ✓ Worker registrado correctamente
    ✓ Job enviado correctamente
    Job ID: <uuid>
    Estado: completed
    ```
- **Paso 2: Prueba Completa**
  - "Ahora probaremos todas las operaciones disponibles:"
    ```bash
    make test-complete
    ```
  - Explicar: "Este script prueba:"
    - "✓ Operación legacy (map_add)"
    - "✓ Operación flat_map (split)"
    - "✓ Operación reduce_by_key (sum)"
    - "✓ Verificación de métricas"
    - "✓ Estado de workers"
  - Mostrar los resultados de cada prueba.
  - Explicar: "Todas las operaciones funcionaron correctamente."

#### [11:30 - 13:30] Prueba con DAG Complejo
**Persona B:**
- "Ahora vamos a probar un DAG más complejo con múltiples etapas. Primero, necesitamos tener master y worker corriendo."
- **Paso 1: Iniciar Sistema (si no está corriendo)**
  - Terminal 1: `make run-master`
  - Terminal 2: `WORKER_PORT=9000 make run-worker`
- **Paso 2: Crear Archivo DAG**
  - Crear archivo `test_dag.json` con el siguiente contenido:
    ```bash
    cat > test_dag.json << 'EOF'
    {
      "name": "test-dag-complejo",
      "dag": {
        "nodes": [
          {
            "id": "read",
            "op": "read_csv",
            "path": "test_data.csv"
          },
          {
            "id": "map1",
            "op": "map",
            "fn_name": "add",
            "param": 10
          },
          {
            "id": "filter1",
            "op": "filter",
            "fn_name": "gt",
            "param": 12
          },
          {
            "id": "reduce1",
            "op": "reduce_by_key",
            "fn_name": "sum"
          }
        ],
        "edges": [
          ["read", "map1"],
          ["map1", "filter1"],
          ["filter1", "reduce1"]
        ]
      }
    }
    EOF
    ```
  - Verificar el contenido:
    ```bash
    cat test_dag.json
    ```
  - Explicar: "Este DAG tiene 4 etapas: leer CSV, aplicar map (sumar 10), filtrar valores mayores a 12, y reducir por clave."
- **Paso 3: Enviar el DAG (Terminal 3)**
  ```bash
  curl -X POST http://127.0.0.1:8080/api/v1/jobs \
    -H "Content-Type: application/json" \
    -d @test_dag.json
  ```
  - Guardar el `job_id` de la respuesta (mostrar cómo extraerlo):
    ```bash
    JOB_ID=$(curl -s -X POST http://127.0.0.1:8080/api/v1/jobs \
      -H "Content-Type: application/json" \
      -d @test_dag.json | grep -oP '"job_id":"\K[a-f0-9-]+')
    echo "Job ID: $JOB_ID"
    ```
- **Paso 4: Monitorear Progreso**
  ```bash
  # Ver estado del job
  cargo run --release --bin client -- get-progress --job-id $JOB_ID
  
  # O usando curl
  curl -s http://127.0.0.1:8080/api/v1/jobs/$JOB_ID | jq .
  ```
  - Si `jq` no está instalado, usar:
    ```bash
    curl -s http://127.0.0.1:8080/api/v1/jobs/$JOB_ID
    ```
  - Explicar: "Podemos ver el progreso: cuántas tareas están completadas, cuántas fallaron, y el estado general."
- **Paso 5: Ver Resultados**
  ```bash
  curl -s http://127.0.0.1:8080/api/v1/jobs/$JOB_ID/results | jq .
  # O sin jq:
  curl -s http://127.0.0.1:8080/api/v1/jobs/$JOB_ID/results
  ```
  - Mostrar los resultados finales y explicar: "El DAG se ejecutó correctamente, procesando los datos a través de todas las etapas."

#### [13:30 - 14:30] Simulación de Fallo
**Persona B:**
- "Ahora vamos a simular un fallo de worker para demostrar la tolerancia a fallos del sistema."
- **Paso 1: Configurar Sistema con Múltiples Workers**
  - Asegurarse de que master está corriendo (Terminal 1)
  - Iniciar Worker 1 (Terminal 2):
    ```bash
    WORKER_PORT=9000 make run-worker
    ```
  - Iniciar Worker 2 (Terminal 4 - Nueva ventana):
    ```bash
    WORKER_PORT=9001 make run-worker
    ```
  - Verificar que ambos están registrados:
    ```bash
    cargo run --release --bin client -- list-workers
    ```
    - Mostrar: "Debemos ver dos workers: w001 y w002, ambos UP."
- **Paso 2: Enviar Job que Tome Tiempo (Terminal 3)**
  ```bash
  cargo run --release --bin client -- submit-job \
    --name "test-failure" \
    --operation "map_add" \
    --param 10 \
    --input "1,2,3,4,5,6,7,8,9,10,11,12,13,14,15"
  ```
  - Guardar el `job_id` de la respuesta:
    ```bash
    JOB_OUTPUT=$(cargo run --release --bin client -- submit-job \
      --name "test-failure" \
      --operation "map_add" \
      --param 10 \
      --input "1,2,3,4,5,6,7,8,9,10,11,12,13,14,15")
    JOB_ID=$(echo "$JOB_OUTPUT" | grep -oP 'Job ID: \K[a-f0-9-]+')
    echo "Job ID: $JOB_ID"
    ```
  - Explicar: "Este job tiene suficientes datos para que tarde unos segundos en procesarse."
- **Paso 3: Simular Fallo del Worker 1**
  - Explicar: "Ahora vamos a simular que el Worker 1 falla mientras está procesando."
  - En Terminal 2 (donde está Worker 1), presionar `Ctrl+C` para detenerlo abruptamente.
  - Explicar: "El master detectará que el Worker 1 dejó de enviar heartbeats (timeout de 15 segundos) y lo marcará como DOWN."
- **Paso 4: Verificar Replanificación**
  - Esperar 5-10 segundos para que el master detecte el fallo.
  - Verificar estado del job:
    ```bash
    cargo run --release --bin client -- get-progress --job-id $JOB_ID
    ```
  - Explicar: "El master detectó el fallo y replanificó las tareas pendientes al Worker 2."
  - Verificar workers:
    ```bash
    cargo run --release --bin client -- list-workers
    ```
    - Mostrar: "Worker 1 está DOWN, Worker 2 está UP y procesando."
- **Paso 5: Confirmar Completación**
  - Esperar a que el job termine (puede tomar unos segundos más).
  - Verificar resultado final:
    ```bash
    cargo run --release --bin client -- get-progress --job-id $JOB_ID
    ```
  - Mostrar: "Estado: completed. A pesar del fallo del Worker 1, el job se completó exitosamente gracias a la replanificación automática."

#### [14:30 - 15:00] Verificación de Métricas y Cierre
**Persona B:**
- "Finalmente, vamos a verificar las métricas del sistema para ver el rendimiento y estadísticas."
- **Paso 1: Ver Métricas Generales**
  ```bash
  curl -s http://127.0.0.1:8080/api/v1/metrics | jq .
  # O sin jq:
  curl -s http://127.0.0.1:8080/api/v1/metrics
  ```
  - Explicar: "Esto muestra todas las métricas: nodos y jobs."
- **Paso 2: Ver Métricas de Nodos**
  ```bash
  curl -s http://127.0.0.1:8080/api/v1/metrics/nodes | jq .
  ```
  - Explicar: "Aquí vemos métricas por worker: CPU, memoria, tareas activas, latencia promedio, y número de reintentos."
- **Paso 3: Ver Métricas de Jobs**
  ```bash
  curl -s http://127.0.0.1:8080/api/v1/metrics/jobs | jq .
  ```
  - Explicar: "Aquí vemos métricas por job: tiempo total, número de etapas, y fallos."
  - Mostrar un ejemplo de salida y explicar qué significa cada campo.
- **Paso 4: Prueba de Fallo Automatizada (Opcional)**
  - "También tenemos un script automatizado para probar la tolerancia a fallos:"
    ```bash
    make test-failure
    ```
  - Explicar: "Este script automatiza todo el proceso: inicia master y workers, envía un job, simula un fallo, y verifica la replanificación."
- **Paso 5: Cerrar el Sistema**
  - "Para cerrar, detendremos todos los procesos:"
    ```bash
    make stop-all
    ```
  - O manualmente: presionar `Ctrl+C` en cada terminal donde están corriendo master y workers.
  - Verificar que se detuvieron:
    ```bash
    ps aux | grep -E "master|worker" | grep -v grep
    ```
    - Mostrar: "No debería haber procesos corriendo."
- **Cierre:**
  - **Persona B:** "Hemos demostrado exitosamente: la instalación, la ejecución del sistema, casos de prueba básicos y complejos con DAGs, y la tolerancia a fallos con replanificación automática. El sistema está funcionando correctamente y cumple con todos los requisitos."
  - **Persona A (si aparece):** "Gracias por ver el video. Para más información, consulten el README, la documentación de arquitectura, y los scripts de prueba en el repositorio."

---

## Notas para los Presentadores

### Persona A:
- Habla claro y pausado
- Muestra los comandos en pantalla antes de ejecutarlos
- Explica qué hace cada comando
- Si algo falla, muestra el error y explica cómo solucionarlo
- En WSL, asegúrate de que los binarios estén compilados para Linux (no .exe)

### Persona B:
- Mantén el ritmo, pero no te apresures
- Asegúrate de que los procesos estén corriendo antes de continuar
- Si un test falla, explica qué significa y continúa con el siguiente
- Para la simulación de fallo, asegúrate de que el job esté en progreso antes de detener el worker
- Usa `jq` para formatear JSON si está disponible, o muestra el JSON sin formatear

### Tips Técnicos:
- **Terminales:** Usa terminales con fondo oscuro y fuente grande (14-16pt) para mejor visibilidad
- **Ventanas:** Maximiza las ventanas de terminal para que se vean claramente los comandos
- **Editor de texto:** Considera usar un editor de texto (nano, vim, VS Code) para mostrar archivos JSON/DAG mientras los explicas
- **Post-producción:** Si algo tarda mucho (compilación), puedes acelerar esas partes en post-producción
- **Múltiples terminales:** Prepara 4-5 terminales abiertas antes de grabar para no perder tiempo
- **Comandos preparados:** Ten los comandos copiados en el portapapeles para pegarlos rápidamente
- **jq (opcional):** Instala `jq` para formatear JSON: `sudo apt-get install jq` (opcional pero útil)

---

## Checklist Pre-Grabación

### Requisitos del Sistema
- [ ] Rust y Cargo instalados y funcionando (`rustc --version`, `cargo --version`)
- [ ] `make` instalado (`make --version`)
- [ ] Proyecto compilado en modo release (`make build-release`)
- [ ] Todos los tests pasando (`make test` - debe mostrar 27 tests pasando)
- [ ] Binarios verificados en `target/release/` (master, worker, client - sin .exe)
- [ ] WSL/Linux funcionando correctamente

### Scripts y Herramientas
- [ ] Makefile funcionando correctamente (`make help`)
- [ ] Scripts de prueba probados (`make test-basic` funciona)
- [ ] Scripts bash funcionando (`bash scripts/test_basic.sh`, `bash scripts/test_complete.sh`, `bash scripts/test_failure.sh`)
- [ ] `curl` instalado y funcionando
- [ ] `jq` instalado (opcional pero recomendado: `sudo apt-get install jq`)

### Archivos de Prueba
- [ ] Archivo `test_data.csv` creado con datos de prueba
- [ ] Archivo `test_data.jsonl` creado con datos de prueba
- [ ] Archivo `test_dag.json` preparado (se puede crear durante el video)

### Configuración de Terminales
- [ ] 4-5 terminales abiertas y configuradas
- [ ] Terminales con fuente grande (14-16pt) y fondo oscuro
- [ ] Terminales maximizadas para mejor visibilidad
- [ ] Editor de texto abierto para mostrar archivos JSON (opcional)

### Preparación del Entorno
- [ ] Navegador abierto con documentación (opcional)
- [ ] Guion impreso o visible en segunda pantalla
- [ ] Comandos importantes copiados en portapapeles
- [ ] Procesos anteriores detenidos (`make stop-all`)

### Verificación Final
- [ ] Probar que `make run-master` inicia correctamente
- [ ] Probar que worker se registra correctamente (`WORKER_PORT=9000 make run-worker`)
- [ ] Probar que `make test-basic` funciona completamente
- [ ] Probar que `make test-complete` funciona completamente
- [ ] Probar que `make test-failure` funciona completamente
- [ ] Verificar que curl funciona para pruebas de API
- [ ] Verificar que los binarios son de Linux (no .exe)

---

## Comandos Rápidos de Referencia

### Compilación y Tests
```bash
make build-release    # Compilar en modo release
make test             # Ejecutar tests unitarios
make test-basic       # Prueba básica automatizada
make test-complete    # Pruebas completas automatizadas
make test-failure     # Prueba de simulación de fallo
```

### Ejecución Manual
```bash
make run-master                    # Iniciar master
WORKER_PORT=9000 make run-worker   # Iniciar worker en puerto 9000
make stop-all                      # Detener todos los procesos
```

### Cliente CLI
```bash
cargo run --release --bin client -- list-workers
cargo run --release --bin client -- submit-job --name "test" --operation "map_add" --param 10 --input "1,2,3,4,5"
cargo run --release --bin client -- get-progress --job-id <JOB_ID>
```

### API con curl
```bash
curl http://127.0.0.1:8080/api/v1/workers
curl http://127.0.0.1:8080/api/v1/metrics
curl -X POST http://127.0.0.1:8080/api/v1/jobs -H "Content-Type: application/json" -d @test_dag.json
```
