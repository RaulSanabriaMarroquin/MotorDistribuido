//! Librería del worker con lógica de ejecución de tareas

use common::TaskAssignment;

/// Ejecuta una operación de tarea y retorna el resultado
pub fn execute_operation(payload: &TaskAssignment) -> Result<Vec<i64>, String> {
    let output: Vec<i64>;

    match payload.operation.as_str() {
        "map_add" => {
            let param = payload.param.unwrap_or(0);
            output = payload.input.iter().map(|x| x + param).collect();
        }
        "map_mul" => {
            let param = payload.param.unwrap_or(1);
            output = payload.input.iter().map(|x| x * param).collect();
        }
        "filter_gt" => {
            let threshold = payload.param.unwrap_or(0);
            output = payload
                .input
                .iter()
                .copied()
                .filter(|x| *x > threshold)
                .collect();
        }
        "filter_lt" => {
            let threshold = payload.param.unwrap_or(0);
            output = payload
                .input
                .iter()
                .copied()
                .filter(|x| *x < threshold)
                .collect();
        }
        "reduce_by_key" => {
            // Asegurar que la longitud sea par (pares clave-valor)
            if payload.input.len() % 2 != 0 {
                return Err("reduce_by_key: la longitud de entrada no es par (desajuste clave/valor)".into());
            } else {
                let mut map = std::collections::HashMap::<i64, i64>::new();

                // Acumular valores por clave
                for pair in payload.input.chunks(2) {
                    let key = pair[0];
                    let val = pair[1];
                    *map.entry(key).or_insert(0) += val;
                }

                // Producir salida intercalada ordenada [k1, v1, k2, v2, ...]
                let mut kv_pairs: Vec<(i64, i64)> = map.into_iter().collect();
                kv_pairs.sort_by_key(|p| p.0);

                output = kv_pairs
                    .into_iter()
                    .flat_map(|(k, v)| vec![k, v])
                    .collect();
            }
        }
        "join" => {
            // Validar entrada: ambas colecciones deben tener longitud par
            if payload.left.len() % 2 != 0 || payload.right.len() % 2 != 0 {
                return Err("JOIN: la longitud izquierda o derecha no es par (pares k,v malformados)".into());
            } else {
                // Construir mapas para las colecciones izquierda y derecha
                let mut left_map = std::collections::HashMap::<i64, i64>::new();
                let mut right_map = std::collections::HashMap::<i64, i64>::new();

                for pair in payload.left.chunks(2) {
                    left_map.insert(pair[0], pair[1]);
                }
                for pair in payload.right.chunks(2) {
                    right_map.insert(pair[0], pair[1]);
                }

                // Calcular claves de intersección (claves presentes en ambas colecciones)
                let mut result: Vec<i64> = Vec::new();
                let mut keys: Vec<i64> = left_map
                    .keys()
                    .filter(|k| right_map.contains_key(k))
                    .cloned()
                    .collect();

                // Ordenar claves para salida determinística
                keys.sort();

                // Salida del join: [k, left_val, right_val, k2, left_val2, right_val2, ...]
                for k in keys {
                    let lv = left_map[&k];
                    let rv = right_map[&k];
                    result.push(k);
                    result.push(lv);
                    result.push(rv);
                }

                output = result;
            }
        }
        "flat_map" => {
            // Transformación plana: cada elemento produce múltiples valores
            let mut out = Vec::new();
            for x in &payload.input {
                // Ejemplo sencillo: cada elemento produce dos valores [x, x*2]
                out.push(*x);
                out.push(*x * 2);
            }
            output = out;
        }
        _ => {
            return Err(format!("Operación desconocida: {}", payload.operation));
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::TaskAssignment;

    fn create_task(operation: &str, input: Vec<i64>, param: Option<i64>) -> TaskAssignment {
        TaskAssignment {
            job_id: "test_job".to_string(),
            task_id: "test_task".to_string(),
            operation: operation.to_string(),
            input,
            left: vec![],
            right: vec![],
            param,
            stage_id: 0,
            reassign_attempt: 0,
        }
    }

    #[test]
    fn test_map_add() {
        let task = create_task("map_add", vec![1, 2, 3, 4], Some(5));
        let result = execute_operation(&task).unwrap();
        assert_eq!(result, vec![6, 7, 8, 9]);
    }

    #[test]
    fn test_map_add_default_param() {
        let task = create_task("map_add", vec![1, 2, 3], None);
        let result = execute_operation(&task).unwrap();
        assert_eq!(result, vec![1, 2, 3]); // parámetro por defecto es 0
    }

    #[test]
    fn test_map_mul() {
        let task = create_task("map_mul", vec![1, 2, 3, 4], Some(3));
        let result = execute_operation(&task).unwrap();
        assert_eq!(result, vec![3, 6, 9, 12]);
    }

    #[test]
    fn test_map_mul_default_param() {
        let task = create_task("map_mul", vec![1, 2, 3], None);
        let result = execute_operation(&task).unwrap();
        assert_eq!(result, vec![1, 2, 3]); // parámetro por defecto es 1
    }

    #[test]
    fn test_filter_gt() {
        let task = create_task("filter_gt", vec![1, 5, 10, 15, 20], Some(10));
        let result = execute_operation(&task).unwrap();
        assert_eq!(result, vec![15, 20]);
    }

    #[test]
    fn test_filter_gt_default() {
        let task = create_task("filter_gt", vec![-1, 0, 1, 2], None);
        let result = execute_operation(&task).unwrap();
        assert_eq!(result, vec![1, 2]); // umbral por defecto es 0
    }

    #[test]
    fn test_filter_lt() {
        let task = create_task("filter_lt", vec![1, 5, 10, 15, 20], Some(10));
        let result = execute_operation(&task).unwrap();
        assert_eq!(result, vec![1, 5]);
    }

    #[test]
    fn test_filter_lt_default() {
        let task = create_task("filter_lt", vec![-2, -1, 0, 1], None);
        let result = execute_operation(&task).unwrap();
        assert_eq!(result, vec![-2, -1]); // umbral por defecto es 0
    }

    #[test]
    fn test_reduce_by_key() {
        // Input: [k1, v1, k2, v2, k1, v3] -> Output: [k1, v1+v3, k2, v2]
        let task = create_task("reduce_by_key", vec![1, 10, 2, 20, 1, 5], None);
        let result = execute_operation(&task).unwrap();
        assert_eq!(result, vec![1, 15, 2, 20]); // clave 1: 10+5=15, clave 2: 20
    }

    #[test]
    fn test_reduce_by_key_empty() {
        let task = create_task("reduce_by_key", vec![], None);
        let result = execute_operation(&task).unwrap();
        assert_eq!(result, vec![]);
    }

    #[test]
    fn test_reduce_by_key_odd_length() {
        let task = create_task("reduce_by_key", vec![1, 10, 2], None);
        let result = execute_operation(&task);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no es par"));
    }

    #[test]
    fn test_reduce_by_key_multiple_keys() {
        // Multiple values for same key
        let task = create_task("reduce_by_key", vec![1, 5, 1, 10, 2, 3, 1, 2], None);
        let result = execute_operation(&task).unwrap();
        assert_eq!(result, vec![1, 17, 2, 3]); // clave 1: 5+10+2=17
    }

    #[test]
    fn test_join() {
        let mut task = create_task("join", vec![], None);
        // left: [(1, 10), (2, 20), (3, 30)]
        task.left = vec![1, 10, 2, 20, 3, 30];
        // right: [(2, 200), (3, 300), (4, 400)]
        task.right = vec![2, 200, 3, 300, 4, 400];
        
        let result = execute_operation(&task).unwrap();
        // Salida: [k, left_val, right_val, ...] para claves que intersectan
        assert_eq!(result, vec![2, 20, 200, 3, 30, 300]);
    }

    #[test]
    fn test_join_no_intersection() {
        let mut task = create_task("join", vec![], None);
        task.left = vec![1, 10, 2, 20];
        task.right = vec![3, 30, 4, 40];
        
        let result = execute_operation(&task).unwrap();
        assert_eq!(result, vec![]);
    }

    #[test]
    fn test_join_odd_length_left() {
        let mut task = create_task("join", vec![], None);
        task.left = vec![1, 10, 2]; // longitud impar
        task.right = vec![1, 100];
        
        let result = execute_operation(&task);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no es par"));
    }

    #[test]
    fn test_join_odd_length_right() {
        let mut task = create_task("join", vec![], None);
        task.left = vec![1, 10];
        task.right = vec![1, 100, 2]; // longitud impar
        
        let result = execute_operation(&task);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no es par"));
    }

    #[test]
    fn test_flat_map() {
        let task = create_task("flat_map", vec![1, 2, 3], None);
        let result = execute_operation(&task).unwrap();
        // flat_map produce [x, x*2] para cada x
        assert_eq!(result, vec![1, 2, 2, 4, 3, 6]);
    }

    #[test]
    fn test_flat_map_empty() {
        let task = create_task("flat_map", vec![], None);
        let result = execute_operation(&task).unwrap();
        assert_eq!(result, vec![]);
    }

    #[test]
    fn test_unknown_operation() {
        let task = create_task("unknown_op", vec![1, 2, 3], None);
        let result = execute_operation(&task);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("desconocida"));
    }
}

