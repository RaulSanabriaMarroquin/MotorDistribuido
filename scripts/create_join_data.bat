@echo off
REM Script para crear datos de ejemplo para join en Windows

if not exist scripts\data mkdir scripts\data

REM Dataset izquierdo: pares clave-valor
> scripts\data\left.txt
echo 1,10 >> scripts\data\left.txt
echo 2,20 >> scripts\data\left.txt
echo 3,30 >> scripts\data\left.txt
echo 4,40 >> scripts\data\left.txt
echo 5,50 >> scripts\data\left.txt

REM Dataset derecho: pares clave-valor (algunas claves en común)
> scripts\data\right.txt
echo 2,200 >> scripts\data\right.txt
echo 3,300 >> scripts\data\right.txt
echo 4,400 >> scripts\data\right.txt
echo 6,600 >> scripts\data\right.txt
echo 7,700 >> scripts\data\right.txt

echo Archivos de datos para join creados en scripts\data\

