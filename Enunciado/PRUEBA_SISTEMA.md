# Guía de Prueba del Sistema Distribuido

## Prerequisitos
- Rust instalado
- Proyecto compilado: `cargo build --release`

## Pasos para Probar el Sistema

### 1. Compilar el Proyecto
```powershell
cargo build --release
```

### 2. Iniciar el Master (Terminal 1)
```powershell
cargo run --release --bin master
```
Deberías ver: `Nodo master iniciando...` y `Nodo master escuchando en 127.0.0.1:8080`

### 3. Iniciar Worker 1 (Terminal 2)
```powershell
$env:WORKER_PORT="9001"
$env:MASTER_URL="http://127.0.0.1:8080"
cargo run --release --bin worker
```
Deberías ver: `Nodo worker iniciando...` y `Registro exitoso`

### 4. Iniciar Worker 2 (Terminal 3 - Opcional)
```powershell
$env:WORKER_PORT="9002"
$env:MASTER_URL="http://127.0.0.1:8080"
cargo run --release --bin worker
```

### 5. Listar Workers (Terminal 4)
```powershell
cargo run --release --bin client -- list-workers
```
Deberías ver la lista de workers registrados.

### 6. Enviar un Job de Prueba
```powershell
cargo run --release --bin client -- submit-job --name "test_map" --operation "map_add" --param 10 --input "1,2,3,4,5"
```
Deberías ver: `¡Job enviado exitosamente!` y un `Job ID`.

### 7. Verificar Progreso
```powershell
cargo run --release --bin client -- get-progress --job-id "<JOB_ID_OBTENIDO>"
```
Deberías ver el progreso del job.

### 8. Verificar Estado del Job (usando curl)
```powershell
curl http://127.0.0.1:8080/api/v1/jobs/<JOB_ID>
```

### 9. Verificar Resultados
```powershell
curl http://127.0.0.1:8080/api/v1/jobs/<JOB_ID>/results
```

## Pruebas Adicionales

### Prueba con DAG
Crear un archivo `test_dag.json`:
```json
{
  "name": "test_dag",
  "dag": {
    "nodes": [
      {
        "id": "read1",
        "op": "read_csv",
        "path": "data/test.csv",
        "partitions": 2
      },
      {
        "id": "map1",
        "op": "map",
        "fn_name": "add",
        "partitions": 2
      }
    ],
    "edges": [
      ["read1", "map1"]
    ]
  },
  "parallelism": 2
}
```

Enviar el job:
```powershell
curl -X POST http://127.0.0.1:8080/api/v1/jobs -H "Content-Type: application/json" -d @test_dag.json
```

## Verificación de Funcionalidades

- [x] Master inicia correctamente
- [x] Workers se registran en el master
- [x] Heartbeats funcionan
- [x] Jobs se pueden enviar
- [x] Tareas se ejecutan en workers
- [x] Progreso se actualiza correctamente
- [x] Resultados se pueden obtener
- [x] Mensajes en español

## Limpieza

Para detener todos los procesos:
```powershell
Get-Process | Where-Object {$_.ProcessName -eq "master" -or $_.ProcessName -eq "worker"} | Stop-Process -Force
```

