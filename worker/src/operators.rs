//! Operadores para tareas de procesamiento de datos
//! Implementa map, flat_map, filter, reduce_by_key, join, read_csv, read_jsonl, write_csv, write_jsonl

use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;
use tokio::io::AsyncWriteExt;

/// Ejecutar una operación map
pub fn map(input: &[i64], fn_name: Option<&str>, param: Option<i64>) -> Result<Vec<i64>, String> {
    match fn_name {
        Some("add") | None => {
            let param = param.unwrap_or(0);
            Ok(input.iter().map(|x| x + param).collect())
        }
        Some("mul") => {
            let param = param.unwrap_or(1);
            Ok(input.iter().map(|x| x * param).collect())
        }
        Some("to_lower") => {
            // Para operaciones de cadena, necesitaremos manejar de manera diferente
            // Por ahora, esto es un placeholder
            Err("to_lower requiere entrada de cadena".to_string())
        }
        Some(fn_name) => Err(format!("Función map desconocida: {}", fn_name)),
    }
}

/// Ejecutar una operación flat_map
/// flat_map aplica una función a cada elemento y aplana el resultado
pub fn flat_map(
    input: &[i64],
    fn_name: Option<&str>,
) -> Result<Vec<i64>, String> {
    match fn_name {
        Some("tokenize") | None => {
            // Ejemplo: tokenizar números en dígitos
            // Para un tokenize real, dividiríamos cadenas, pero para enteros dividiremos dígitos
            let mut result = Vec::new();
            for num in input {
                let num_str = num.to_string();
                for ch in num_str.chars() {
                    if let Some(digit) = ch.to_digit(10) {
                        result.push(digit as i64);
                    }
                }
            }
            Ok(result)
        }
        Some("split") => {
            // Dividir cada número en sus dígitos
            let mut result = Vec::new();
            for num in input {
                let mut n = num.abs();
                if n == 0 {
                    result.push(0);
                } else {
                    let mut digits = Vec::new();
                    while n > 0 {
                        digits.push((n % 10) as i64);
                        n /= 10;
                    }
                    digits.reverse();
                    result.extend(digits);
                }
            }
            Ok(result)
        }
        Some(fn_name) => Err(format!("Función flat_map desconocida: {}", fn_name)),
    }
}

/// Ejecutar una operación filter
pub fn filter(
    input: &[i64],
    fn_name: Option<&str>,
    param: Option<i64>,
) -> Result<Vec<i64>, String> {
    match fn_name {
        Some("gt") | None => {
            let threshold = param.unwrap_or(0);
            Ok(input.iter().copied().filter(|x| *x > threshold).collect())
        }
        Some("lt") => {
            let threshold = param.unwrap_or(0);
            Ok(input.iter().copied().filter(|x| *x < threshold).collect())
        }
        Some("eq") => {
            let value = param.unwrap_or(0);
            Ok(input.iter().copied().filter(|x| *x == value).collect())
        }
        Some(fn_name) => Err(format!("Función filter desconocida: {}", fn_name)),
    }
}

/// Ejecutar una operación reduce_by_key
/// Agrupa elementos por una clave y aplica una función de agregación
/// Por ahora, usaremos el valor mismo como clave (para enteros)
pub fn reduce_by_key(
    input: &[i64],
    fn_name: Option<&str>,
) -> Result<HashMap<i64, i64>, String> {
    let mut result = HashMap::new();
    
    match fn_name {
        Some("sum") | None => {
            // Agrupar por valor y sumar conteos
            for &value in input {
                *result.entry(value).or_insert(0) += 1;
            }
        }
        Some("count") => {
            // Contar ocurrencias de cada valor
            for &value in input {
                *result.entry(value).or_insert(0) += 1;
            }
        }
        Some("max") => {
            // Mantener valor máximo para cada clave
            for &value in input {
                let entry = result.entry(value).or_insert(value);
                *entry = (*entry).max(value);
            }
        }
        Some("min") => {
            // Mantener valor mínimo para cada clave
            for &value in input {
                let entry = result.entry(value).or_insert(value);
                *entry = (*entry).min(value);
            }
        }
        Some(fn_name) => return Err(format!("Función reduce_by_key desconocida: {}", fn_name)),
    }
    
    Ok(result)
}

/// Convertir resultado de reduce_by_key a formato vector para salida
pub fn reduce_by_key_to_vec(result: &HashMap<i64, i64>) -> Vec<Value> {
    let mut vec_result: Vec<(i64, i64)> = result.iter().map(|(k, v)| (*k, *v)).collect();
    vec_result.sort_by_key(|(k, _)| *k);
    
    vec_result
        .into_iter()
        .map(|(key, value)| {
            serde_json::json!({
                "key": key,
                "value": value
            })
        })
        .collect()
}

/// Ejecutar una operación join entre dos colecciones
/// Une en claves enteras (el valor mismo como clave por simplicidad)
/// Retorna un vector de registros unidos como objetos JSON
pub fn join(
    left: &[i64],
    right: &[i64],
    _key: Option<&str>, // Campo clave para uso futuro con datos estructurados
) -> Result<Vec<Value>, String> {
    // Para arreglos de enteros, usaremos el valor mismo como clave
    // En una implementación real, analizaríamos datos estructurados y usaríamos el campo clave especificado
    let mut result = Vec::new();
    
    // Construir índice del lado derecho (hash join)
    let mut right_index: HashMap<i64, Vec<i64>> = HashMap::new();
    for &value in right {
        right_index.entry(value).or_insert_with(Vec::new).push(value);
    }
    
    // Unir izquierda con derecha
    for &left_value in left {
        if let Some(right_values) = right_index.get(&left_value) {
            for &right_value in right_values {
                result.push(serde_json::json!({
                    "key": left_value,
                    "left": left_value,
                    "right": right_value
                }));
            }
        }
    }
    
    Ok(result)
}

/// Leer archivo CSV y retornar como vector de enteros
/// Por simplicidad, lee la primera columna como enteros
pub async fn read_csv(path: &str) -> Result<Vec<i64>, String> {
    let content = fs::read_to_string(path)
        .await
        .map_err(|e| format!("Error al leer archivo CSV {}: {}", path, e))?;
    
    let mut reader = csv::Reader::from_reader(content.as_bytes());
    let mut result = Vec::new();
    
    for record in reader.records() {
        let record = record.map_err(|e| format!("Error al analizar CSV: {}", e))?;
        // Leer primera columna como entero
        if let Some(first_field) = record.get(0) {
            if let Ok(value) = first_field.parse::<i64>() {
                result.push(value);
            }
        }
    }
    
    Ok(result)
}

/// Leer archivo JSONL (un objeto JSON por línea) y extraer valores enteros
/// Por simplicidad, extrae el primer campo numérico encontrado
pub async fn read_jsonl(path: &str) -> Result<Vec<i64>, String> {
    let content = fs::read_to_string(path)
        .await
        .map_err(|e| format!("Error al leer archivo JSONL {}: {}", path, e))?;
    
    let mut result = Vec::new();
    
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        
        let json: Value = serde_json::from_str(line)
            .map_err(|e| format!("Error al analizar JSON: {}", e))?;
        
        // Intentar extraer valor entero del JSON
        if let Some(num) = json.as_i64() {
            result.push(num);
        } else if let Some(obj) = json.as_object() {
            // Intentar encontrar primer valor numérico
            for value in obj.values() {
                if let Some(num) = value.as_i64() {
                    result.push(num);
                    break;
                }
            }
        }
    }
    
    Ok(result)
}

/// Escribir datos a archivo CSV
pub async fn write_csv(path: &str, data: &[i64]) -> Result<String, String> {
    // Crear directorio si no existe
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Error al crear directorio: {}", e))?;
    }
    
    let mut writer = csv::Writer::from_path(path)
        .map_err(|e| format!("Error al crear escritor CSV: {}", e))?;
    
    for value in data {
        writer
            .write_record(&[value.to_string()])
            .map_err(|e| format!("Error al escribir registro CSV: {}", e))?;
    }
    
    writer
        .flush()
        .map_err(|e| format!("Error al hacer flush del escritor CSV: {}", e))?;
    
    Ok(path.to_string())
}

/// Escribir datos a archivo JSONL (un objeto JSON por línea)
pub async fn write_jsonl(path: &str, data: &[i64]) -> Result<String, String> {
    // Crear directorio si no existe
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Error al crear directorio: {}", e))?;
    }
    
    let mut file = fs::File::create(path)
        .await
        .map_err(|e| format!("Error al crear archivo JSONL: {}", e))?;
    
    for value in data {
        let json_obj = serde_json::json!({ "value": value });
        let line = serde_json::to_string(&json_obj)
            .map_err(|e| format!("Error al serializar JSON: {}", e))?;
        
        file.write_all(line.as_bytes())
            .await
            .map_err(|e| format!("Error al escribir línea JSONL: {}", e))?;
        file.write_all(b"\n")
            .await
            .map_err(|e| format!("Error al escribir nueva línea: {}", e))?;
    }
    
    file.flush()
        .await
        .map_err(|e| format!("Error al hacer flush del archivo: {}", e))?;
    
    Ok(path.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_add() {
        let input = vec![1, 2, 3, 4, 5];
        let result = map(&input, Some("add"), Some(10)).unwrap();
        assert_eq!(result, vec![11, 12, 13, 14, 15]);
    }

    #[test]
    fn test_flat_map_split() {
        let input = vec![123, 456];
        let result = flat_map(&input, Some("split")).unwrap();
        assert_eq!(result, vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn test_filter_gt() {
        let input = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let result = filter(&input, Some("gt"), Some(5)).unwrap();
        assert_eq!(result, vec![6, 7, 8, 9, 10]);
    }

    #[test]
    fn test_reduce_by_key_sum() {
        let input = vec![1, 2, 2, 3, 3, 3, 4, 4, 4, 4];
        let result = reduce_by_key(&input, Some("sum")).unwrap();
        assert_eq!(result.get(&1), Some(&1));
        assert_eq!(result.get(&2), Some(&2));
        assert_eq!(result.get(&3), Some(&3));
        assert_eq!(result.get(&4), Some(&4));
    }
}

