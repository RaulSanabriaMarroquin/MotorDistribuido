# Guion para Video de Demostración - Motor Distribuido
## Duración: 15 minutos (7.5 min por persona)

---

## PARTE 1: Raúl Marroquin (0:00 - 7:30)

### [0:00 - 0:45] Introducción y Arquitectura

**Narración:**
"Hola, soy Raúl Marroquin. En este video vamos a demostrar el Motor Distribuido, un sistema de procesamiento batch implementado en Rust que simula un mini-Spark. El sistema tiene una arquitectura master-worker-client donde el master coordina múltiples workers que procesan datos en paralelo. Para facilitar el despliegue, utilizamos Docker Compose como se requiere en el enunciado del proyecto."

**Comandos:**
```bash
# Mostrar estructura del proyecto
tree -L 2 -I target
# O en Windows:
dir /B

# Mostrar docker-compose.yml
cat docker-compose.yml
# O en Windows:
type docker-compose.yml
```

**Puntos clave a mencionar:**
- Arquitectura distribuida master-worker-client
- Implementado en Rust con Tokio para concurrencia asíncrona
- Comunicación HTTP/JSON entre componentes
- Despliegue con Docker Compose (requerido por el enunciado)
- Soporte para operaciones batch: map, filter, flat_map, reduce, join, aggregate
- Planificación round-robin con awareness de carga

---

### [0:45 - 2:00] Iniciando el Sistema con Docker Compose

**Narración:**
"Vamos a iniciar el sistema completo usando Docker Compose. Esto nos permite desplegar el master y múltiples workers de forma sencilla y reproducible, cumpliendo con el requisito del enunciado."

**Comandos:**
```bash
# Construir las imágenes y iniciar todos los servicios
docker-compose up --build -d

# Ver los logs para verificar que todo está funcionando
docker-compose logs -f
# (Presionar Ctrl+C después de ver que los servicios están listos)

# Verificar que los contenedores están corriendo
docker-compose ps
```

**Puntos clave:**
- Docker Compose inicia 1 master y 3 workers automáticamente
- El master se inicia en el puerto 8080
- Los workers se registran automáticamente con el master
- Los workers envían heartbeats cada 3 segundos (como se requiere en el enunciado)
- Todos los servicios están en la misma red Docker

---

### [2:00 - 3:00] Verificando el Estado del Sistema

**Narración:**
"Ahora vamos a verificar que los workers se hayan registrado correctamente usando el cliente. El cliente se ejecuta desde nuestra máquina local y se conecta al master que está en Docker."

**Comandos:**
```bash
# Listar workers registrados
cargo run --bin client list-workers

# También podemos ver los logs del master para ver los registros
docker-compose logs master | tail -20
```

**Salida esperada:**
- Debe mostrar 3 workers con estado UP
- Mostrar IDs, hosts, puertos y estado

**Puntos clave:**
- El master mantiene un registro de todos los workers
- El estado UP indica que el worker está activo y enviando heartbeats cada 3 segundos
- El sistema detecta workers caídos automáticamente mediante heartbeats

---

### [3:00 - 4:30] Operaciones Básicas: Map, Filter y Flat Map

**Narración:**
"Vamos a ejecutar nuestro primer job. Empezaremos con operaciones básicas: map, filter y flat_map. Usaremos datos inline para simplicidad."

**Comandos:**
```bash
# Job 1: Map Add - Sumar 10 a cada elemento
cargo run --bin client submit-job \
  --name "demo-map-add" \
  --source-inline "1,2,3,4,5,6,7,8,9,10" \
  --stages "map_add:10"

# Esperar un momento y verificar progreso
cargo run --bin client get-progress --job-id <JOB_ID>
```

**Narración:**
"Ahora vamos a hacer un pipeline con múltiples operaciones: primero sumamos 10, luego filtramos los valores mayores a 15."

**Comandos:**
```bash
# Job 2: Pipeline Map + Filter
cargo run --bin client submit-job \
  --name "demo-map-filter" \
  --source-inline "1,2,3,4,5,6,7,8,9,10" \
  --stages "map_add:10,filter_gt:15"

# Verificar resultado
cargo run --bin client get-progress --job-id <JOB_ID>
```

**Puntos clave:**
- Las operaciones se ejecutan en secuencia formando un DAG
- El sistema particiona los datos automáticamente
- Las tareas se distribuyen entre los workers usando round-robin con awareness de carga
- El planificador asigna tareas de forma balanceada

---

### [4:30 - 6:00] Procesamiento de Archivos: CSV y JSONL

**Narración:**
"El sistema también puede procesar datos desde archivos, como se requiere en el enunciado. Vamos a demostrar el procesamiento de archivos CSV y JSONL. Los archivos se leen y particionan automáticamente."

**Comandos:**
```bash
# Crear archivo CSV de ejemplo (si no existe)
./scripts/create_sample_data.sh
# O en Windows:
scripts\create_sample_data.bat

# Job 3: Procesar CSV
cargo run --bin client submit-job \
  --name "demo-csv-processing" \
  --source-csv scripts/data/sample.csv \
  --stages "map_mul:2,filter_gt:10"

# Verificar progreso
cargo run --bin client get-progress --job-id <JOB_ID>

# Job 4: Procesar JSONL
cargo run --bin client submit-job \
  --name "demo-jsonl-processing" \
  --source-jsonl scripts/data/sample.jsonl \
  --stages "map_add:5,filter_lt:20"
```

**Puntos clave:**
- Soporte para múltiples formatos de entrada (CSV, JSONL, plain text)
- El master lee y particiona los archivos automáticamente
- Los workers procesan las particiones en paralelo
- El sistema maneja particiones y distribuye el trabajo eficientemente

---

### [6:00 - 7:30] Métricas y Observabilidad

**Narración:**
"Antes de continuar, vamos a ver las métricas del sistema. El sistema proporciona métricas por nodo y por job, como se requiere en el enunciado."

**Comandos:**
```bash
# Ver métricas del sistema
# (Si hay endpoint de métricas implementado)
# O mostrar logs estructurados
docker-compose logs master | grep -i metric
docker-compose logs worker1 | tail -10
```

**Puntos clave:**
- El sistema registra métricas por nodo (CPU, memoria, tareas activas)
- Métricas por job (tiempo total, progreso, número de tareas)
- Logging estructurado con niveles (info, debug, error)
- Observabilidad para monitoreo del sistema distribuido

---

### [7:00 - 7:30] Transición

**Narración:**
"Hasta aquí hemos visto las operaciones básicas, el procesamiento de archivos y las métricas. Ahora David va a continuar mostrando operaciones más avanzadas como reduce, join, y la tolerancia a fallos del sistema."

---

## PARTE 2: David Acuña (7:30 - 15:00)

### [7:30 - 8:00] Introducción a Operaciones Avanzadas

**Narración:**
"Hola, soy David Acuña. Voy a continuar mostrando las capacidades avanzadas del sistema, incluyendo operaciones de agregación y joins."

---

### [7:30 - 9:00] Reduce y Agregaciones

**Narración:**
"Vamos a demostrar la operación reduce_by_key, que agrupa valores por clave y los suma. Esta operación requiere un shuffle entre stages, redistribuyendo los datos entre workers."

**Comandos:**
```bash
# Job 5: Reduce by Key
# Los datos deben estar en formato clave-valor (pares intercalados)
cargo run --bin client submit-job \
  --name "demo-reduce-by-key" \
  --source-inline "1,10,2,20,1,5,3,15,2,10" \
  --stages "reduce_by_key"

# Verificar resultado - debe agrupar por clave y sumar valores
cargo run --bin client get-progress --job-id <JOB_ID>
```

**Narración:**
"Ahora vamos a hacer un pipeline más complejo con múltiples stages que incluye shuffle."

**Comandos:**
```bash
# Job 6: Pipeline complejo con múltiples stages
./scripts/demo_complex_pipeline.sh
# O en Windows:
scripts\demo_complex_pipeline.bat
```

**Puntos clave:**
- reduce_by_key requiere shuffle entre workers
- El sistema maneja automáticamente la redistribución de datos (shuffle)
- Las claves se agrupan correctamente incluso si están en diferentes particiones
- El shuffle redistribuye datos por clave para agrupación correcta

---

### [9:00 - 10:30] Join entre Datasets

**Narración:**
"El sistema también soporta operaciones de join entre dos datasets, como se requiere en el enunciado. El join es una operación avanzada que requiere que ambos datasets estén en formato clave-valor."

**Comandos:**
```bash
# Job 7: Join entre dos datasets
# Dataset izquierdo: 1,10,2,20,3,30
# Dataset derecho: 2,200,3,300,4,400
# El join encontrará las claves 2 y 3 en ambos datasets
cargo run --bin client submit-job \
  --name "demo-join" \
  --source-inline "1,10,2,20,3,30" \
  --stages "join"

# Verificar resultado
cargo run --bin client get-progress --job-id <JOB_ID>
```

**Puntos clave:**
- El join requiere que ambos datasets estén en formato clave-valor (pares intercalados)
- El sistema distribuye el join entre workers
- Solo se incluyen las claves presentes en ambos datasets (inner join)
- El resultado muestra: clave, valor_izquierdo, valor_derecho
- El join también requiere shuffle para redistribuir datos por clave

---

### [10:30 - 12:30] Tolerancia a Fallos y Reintentos

**Narración:**
"Una característica importante del sistema es su tolerancia a fallos, como se requiere en el enunciado. Si un worker cae durante la ejecución, el master detecta el fallo mediante heartbeats y reasigna las tareas a otros workers. El sistema también implementa reintentos automáticos."

**Comandos:**
```bash
# Job 8: Job largo para demostrar tolerancia a fallos
cargo run --bin client submit-job \
  --name "demo-fault-tolerance" \
  --source-inline "1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20" \
  --stages "map_add:100,filter_gt:105,map_mul:2"

# En otra terminal, detener un worker mientras el job está ejecutando
docker-compose stop worker1

# Verificar que el master detecta el worker caído (esperar ~15 segundos para timeout de heartbeat)
cargo run --bin client list-workers

# Ver los logs del master para ver la detección del fallo
docker-compose logs master | tail -20

# Verificar que el job continúa y se completa (las tareas se reasignan)
cargo run --bin client get-progress --job-id <JOB_ID>

# Reiniciar el worker
docker-compose start worker1
```

**Puntos clave:**
- El master detecta workers caídos mediante heartbeats (cada 3 segundos, timeout de 15 segundos)
- Si un worker deja de latir, el master lo marca como DOWN
- Las tareas pendientes se reasignan automáticamente a otros workers
- El sistema implementa reintentos (al menos 1 reintento por tarea fallida)
- El job se completa exitosamente a pesar del fallo
- Idempotencia básica para evitar duplicar resultados

---

### [12:30 - 13:45] Caso de Uso Completo y Persistencia

**Narración:**
"Vamos a demostrar un caso de uso completo: procesar un dataset con múltiples operaciones en pipeline. El sistema también mantiene persistencia mínima del estado del job."

**Comandos:**
```bash
# Job 9: Caso de uso completo
./scripts/demo_complete_use_case.sh
# O en Windows:
scripts\demo_complete_use_case.bat

# Verificar el estado persistido del job
cargo run --bin client get-progress --job-id <JOB_ID>
```

**Este script ejecuta:**
1. Carga datos desde CSV
2. Aplica transformaciones (map)
3. Filtra datos
4. Agrupa por clave (reduce_by_key)
5. Muestra el resultado final

**Puntos clave:**
- El sistema maneja pipelines complejos con múltiples stages
- La distribución de tareas es automática usando round-robin
- El procesamiento es eficiente gracias al paralelismo
- El master mantiene persistencia del estado del job
- Cache en memoria con spill a disco cuando es necesario

---

### [13:45 - 14:30] Pruebas y Reproducibilidad

**Narración:**
"El proyecto incluye una suite completa de pruebas: unitarias, de integración y end-to-end. También tenemos scripts para build, test y demo, cumpliendo con el requisito de reproducibilidad del enunciado."

**Comandos:**
```bash
# Mencionar que las pruebas están disponibles
# (No ejecutar en el video para ahorrar tiempo, solo mencionar)
echo "Pruebas disponibles:"
echo "- Unitarias: cargo test --lib"
echo "- Integración: cargo test -p tests --test integration_tests -- --skip single_node"
echo "- End-to-end: cargo test -p tests --test integration_tests single_node"

# Mostrar estructura de scripts
ls scripts/
# O en Windows:
dir scripts
```

**Puntos clave:**
- Suite de pruebas: unitarias, integración (nodo único), end-to-end (multinodo)
- Scripts para build, test y demo (reproducibilidad)
- Docker Compose para despliegue reproducible
- README con instrucciones completas

---

### [14:30 - 15:00] Resumen y Cierre

**Narración:**
"Para resumir, hemos demostrado todos los requisitos del enunciado:
- Arquitectura master-worker-client con Docker Compose
- Operadores mínimos: map, filter, flat_map, reduce_by_key, join
- Procesamiento de archivos: CSV y JSONL
- Planificación round-robin con awareness de carga
- Heartbeats cada 3 segundos y detección de fallos
- Reintentos automáticos y replanificación
- Shuffle entre stages para reduce y join
- Métricas y observabilidad
- Persistencia del estado del job
- Tolerancia a fallos simulada con recuperación automática
- Pipelines complejos con múltiples stages

El sistema está completamente funcional, cumple con todas las restricciones del enunciado (sin frameworks distribuidos, solo librerías estándar), y está listo para procesar datos distribuidos. Gracias por ver el video."

**Comandos finales:**
```bash
# Mostrar estadísticas finales
cargo run --bin client list-workers

# Detener el sistema Docker
docker-compose down
```

---

## Notas para la Grabación

1. **Preparación:**
   - Tener Docker y Docker Compose instalados y funcionando
   - Compilar el proyecto antes: `cargo build --release`
   - Construir las imágenes Docker: `docker-compose build`
   - Tener las terminales abiertas y organizadas
   - Tener los scripts probados previamente
   - Crear datos de ejemplo: `scripts/create_sample_data.bat` o `.sh`

2. **Durante la grabación:**
   - Pausar entre comandos para que se vea la salida
   - Explicar qué está pasando mientras se ejecuta
   - Mostrar las salidas importantes en pantalla
   - Mencionar explícitamente los requisitos del enunciado cuando se demuestren
   - Mantener el ritmo para no exceder 15 minutos

3. **Puntos críticos a mencionar:**
   - Docker Compose como requisito del enunciado
   - Heartbeats cada 3 segundos (requisito: 1-3s)
   - Reintentos automáticos (al menos 1)
   - Shuffle entre stages
   - Round-robin con awareness de carga
   - Sin frameworks distribuidos (solo librerías estándar)

4. **Edición:**
   - Añadir zoom a las terminales cuando sea necesario
   - Resaltar comandos importantes
   - Añadir transiciones entre secciones
   - Mostrar el tiempo transcurrido si es posible

