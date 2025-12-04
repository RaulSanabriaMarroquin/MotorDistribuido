# Scripts de Demostración

Esta carpeta contiene scripts para facilitar la demostración del Motor Distribuido.

## Scripts Disponibles

### Inicio del Sistema

- **start_master.sh / start_master.bat**: Inicia el master en el puerto 8080
- **start_worker.sh / start_worker.bat**: Inicia un worker (requiere número de worker como argumento)

### Preparación de Datos

- **create_sample_data.sh / create_sample_data.bat**: Crea archivos de ejemplo (CSV, JSONL, plain text)
- **create_join_data.sh / create_join_data.bat**: Crea datos de ejemplo para operaciones de join

### Demostraciones

- **demo_complex_pipeline.sh / demo_complex_pipeline.bat**: Demuestra un pipeline complejo con reduce_by_key
- **demo_complete_use_case.sh / demo_complete_use_case.bat**: Caso de uso completo con procesamiento de archivos

### Utilidades

- **kill_worker.sh / kill_worker.bat**: Instrucciones para simular la muerte de un worker (para pruebas de tolerancia a fallos)
- **cleanup.sh / cleanup.bat**: Limpia procesos en ejecución

## Uso

### En Linux/Mac:
```bash
chmod +x scripts/*.sh
./scripts/start_master.sh
```

### En Windows:
```cmd
scripts\start_master.bat
```

## Notas

- Los scripts `.sh` son para Linux/Mac
- Los scripts `.bat` son para Windows
- Algunos scripts requieren argumentos (ver el guion para detalles)

