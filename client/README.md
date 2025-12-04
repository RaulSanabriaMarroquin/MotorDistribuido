# Client

Aplicación cliente CLI para interactuar con el sistema distribuido. Permite listar workers, enviar jobs y consultar el progreso de los jobs.

## Propósito

- Enviar jobs al master
- Consultar el estado de workers registrados
- Consultar el progreso y resultados de jobs
- Interfaz de línea de comandos para interactuar con el sistema

## Comandos

### list-workers

Lista todos los workers registrados en el master.

```bash
cargo run --bin client list-workers
```

Muestra:
- ID del worker
- Host y puerto
- Estado (UP/DOWN)
- Última vez de heartbeat

### submit-job

Envía un nuevo job al master para ejecución.

```bash
cargo run --bin client submit-job \
  --name "nombre-del-job" \
  --source-inline "1,2,3,4,5" \
  --stages "map_add:10,filter_gt:12"
```

Opciones:
- `--name`: Nombre del job
- `--source-inline`: Datos inline separados por comas
- `--source-csv`: Ruta a un archivo CSV
- `--source-jsonl`: Ruta a un archivo JSONL
- `--source-txt`: Ruta a un archivo de texto plano
- `--stages`: Lista de operaciones separadas por comas (formato: `operacion:parametro`)

Ejemplos:

```bash
# Job con datos inline y una operación
cargo run --bin client submit-job \
  --name "test-map" \
  --source-inline "1,2,3,4,5" \
  --stages "map_add:10"

# Job con múltiples stages
cargo run --bin client submit-job \
  --name "test-pipeline" \
  --source-inline "1,2,3,4,5,6,7,8,9,10" \
  --stages "map_add:10,filter_gt:15"

# Job procesando un archivo CSV
cargo run --bin client submit-job \
  --name "process-csv" \
  --source-csv scripts/data/sample.csv \
  --stages "map_mul:2,filter_gt:10"

# Job con reduce_by_key
cargo run --bin client submit-job \
  --name "reduce-demo" \
  --source-inline "1,10,2,20,1,5,3,15,2,10" \
  --stages "reduce_by_key"
```

### get-progress

Consulta el progreso de un job.

```bash
cargo run --bin client get-progress --job-id <JOB_ID>
```

Muestra:
- Estado del job (running, completed, failed)
- Progreso (tareas completadas / total)
- Resultado (si el job está completado)

## Configuración

El cliente se puede configurar con:

- `--master-url`: URL del master (por defecto: `http://127.0.0.1:8080`)
- `RUST_LOG`: Nivel de logging (por defecto: `info`)

Ejemplo:

```bash
cargo run --bin client --master-url http://localhost:8080 list-workers
```

## Formato de Jobs

Los jobs se especifican con:

- **Nombre**: Identificador del job
- **Fuente de datos**: Datos inline, archivo CSV, JSONL o texto plano
- **Stages**: Lista de operaciones a ejecutar en secuencia

Cada stage tiene el formato: `operacion:parametro`

Operaciones disponibles:
- `map_add:valor` - Suma un valor a cada elemento
- `map_mul:valor` - Multiplica cada elemento por un valor
- `filter_gt:valor` - Filtra elementos mayores que el valor
- `filter_lt:valor` - Filtra elementos menores que el valor
- `reduce_by_key` - Agrupa por clave y suma valores (no requiere parámetro)
- `join` - Une dos datasets por clave (no requiere parámetro)
- `flat_map` - Aplana una colección (no requiere parámetro)

## Ejecución

```bash
cargo run --bin client <comando> [opciones]
```

## Dependencias

- `reqwest`: Cliente HTTP para comunicación con el master
- `serde` / `serde_json`: Serialización JSON
- `tracing`: Logging estructurado
- `common`: Tipos y estructuras compartidas
