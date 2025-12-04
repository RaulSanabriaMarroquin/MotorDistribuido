@echo off
REM Script para limpiar procesos en Windows

echo Limpiando procesos...

REM Matar procesos del master y workers
taskkill /F /FI "WINDOWTITLE eq *master*" 2>nul
taskkill /F /FI "WINDOWTITLE eq *worker*" 2>nul

echo Limpieza completada
echo.
echo Nota: Los archivos en scripts\data\ se mantienen para futuras ejecuciones
echo       Si quieres eliminarlos, ejecuta: rmdir /S /Q scripts\data

