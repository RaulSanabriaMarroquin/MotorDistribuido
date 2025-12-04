@echo off
REM Script para matar un worker especifico en Windows
REM Uso: kill_worker.bat <worker_number>

set WORKER_NUM=%1
if "%WORKER_NUM%"=="" set WORKER_NUM=1

set /a PORT=8080+%WORKER_NUM%

echo Intentando matar el worker %WORKER_NUM% (puerto %PORT%)...

REM Buscar el PID del proceso que esta usando el puerto
set PID=
for /f "tokens=5" %%a in ('netstat -ano ^| findstr ":%PORT%" ^| findstr "LISTENING"') do (
    set PID=%%a
    goto :found
)

:found
if defined PID (
    echo Encontrado proceso con PID %PID% en puerto %PORT%
    echo Matando worker con PID %PID%...
    taskkill /F /PID %PID% 2>nul
    if %ERRORLEVEL% EQU 0 (
        echo Worker %WORKER_NUM% terminado exitosamente
    ) else (
        echo Error al terminar el proceso con PID %PID%
    )
) else (
    echo No se encontro ningun proceso usando el puerto %PORT%
    echo El worker %WORKER_NUM% puede no estar ejecutandose
)

