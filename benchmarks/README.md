# Benchmarks del Sistema Distribuido

Este directorio contiene scripts y reportes de benchmarks para evaluar el rendimiento del sistema distribuido según los requisitos del enunciado.

## Requisitos

- Master y workers corriendo
- PowerShell (Windows) o PowerShell Core (cross-platform)
- Acceso a la red local (master en `http://127.0.0.1:8080`)

## Uso

### Ejecutar Benchmarks

```powershell
# Benchmarks con 1M registros (por defecto)
.\benchmarks\benchmark.ps1

# Benchmarks con número personalizado de registros
.\benchmarks\benchmark.ps1 -RecordCount 500000

# Especificar URL del master
.\benchmarks\benchmark.ps1 -MasterUrl "http://localhost:8080"
```

### Benchmarks Incluidos

1. **Operación Map**: Suma constante a cada elemento
2. **Operación Filter**: Filtrado por condición
3. **Operación Reduce by Key**: Agregación por clave
4. **DAG Completo**: Pipeline completo (read -> map -> filter -> reduce)

## Resultados

Los reportes se guardan en `benchmarks/results/` con el formato:
- `benchmark_report_YYYYMMDD_HHMMSS.md`

Cada reporte incluye:
- Configuración del sistema
- Resultados de cada benchmark
- Métricas de rendimiento
- Resumen comparativo

## Notas

- Los benchmarks pueden tardar varios minutos dependiendo del tamaño de los datos
- Asegúrate de tener suficiente espacio en disco para los archivos de datos generados
- Los archivos de datos se reutilizan si ya existen (no se regeneran)

## Ejemplo de Salida

```
=== BENCHMARKS DEL SISTEMA DISTRIBUIDO ===
Registros a procesar: 1000000

Verificando conexión con master...
✓ Master conectado. Workers disponibles: 3

=== BENCHMARK 1: Operación Map ===
Ejecutando: Map (1000 elementos)...
  Job ID: abc123...
  ✓ Completado en 2.34 segundos

...
```

