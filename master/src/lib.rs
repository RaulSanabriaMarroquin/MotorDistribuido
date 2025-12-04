//! Librería del master con funciones de parsing y utilidades

/// Parsea contenido CSV y extrae valores numéricos
pub fn parse_csv_numbers(content: &str) -> Vec<i64> {
    let mut result = Vec::new();
    for line in content.lines() {
        for token in line.split(',') {
            if let Ok(v) = token.trim().parse::<i64>() {
                result.push(v);
            }
        }
    }
    result
}

/// Parsea contenido JSONL y extrae valores numéricos del campo "value"
pub fn parse_jsonl_numbers(content: &str) -> Vec<i64> {
    let mut result = Vec::<i64>::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Ok(json_line) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if let Some(v) = json_line.get("value").and_then(|x| x.as_i64()) {
                result.push(v);
            }
        }
    }

    result
}

/// Parsea contenido de texto plano y extrae valores numéricos
pub fn parse_plain_numbers(content: &str) -> Vec<i64> {
    content
        .split(|c: char| c == ',' || c == '\n' || c == ' ' || c == '\t')
        .filter_map(|s| s.trim().parse::<i64>().ok())
        .collect()
}

/// Divide los datos de entrada en chunks para ejecución paralela de tareas
pub fn split_into_chunks(input: Vec<i64>, chunk_size: usize) -> Vec<Vec<i64>> {
    if chunk_size == 0 {
        return vec![input];
    }

    input
        .chunks(chunk_size)
        .map(|chunk| chunk.to_vec())
        .collect()
}

/// Particiona pares clave-valor por hash de la clave (para operaciones shuffle)
pub fn partition_by_key(kv_pairs: &[i64], num_partitions: usize) -> Vec<Vec<i64>> {
    if num_partitions == 0 {
        return vec![];
    }

    let mut partitions = vec![Vec::<i64>::new(); num_partitions];

    for i in (0..kv_pairs.len()).step_by(2) {
        if i + 1 < kv_pairs.len() {
            let key = kv_pairs[i];
            let val = kv_pairs[i + 1];
            let pid = (key.abs() as usize) % num_partitions;
            partitions[pid].push(key);
            partitions[pid].push(val);
        }
    }

    partitions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_csv_numbers_simple() {
        let content = "1,2,3\n4,5,6";
        let result = parse_csv_numbers(content);
        assert_eq!(result, vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn test_parse_csv_numbers_with_spaces() {
        let content = " 1 , 2 , 3 \n 4 , 5 ";
        let result = parse_csv_numbers(content);
        assert_eq!(result, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_parse_csv_numbers_empty() {
        let content = "";
        let result = parse_csv_numbers(content);
        assert_eq!(result, Vec::<i64>::new());
    }

    #[test]
    fn test_parse_csv_numbers_ignores_non_numeric() {
        let content = "1,abc,2,def,3";
        let result = parse_csv_numbers(content);
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_parse_csv_numbers_negative() {
        let content = "-1,2,-3,4";
        let result = parse_csv_numbers(content);
        assert_eq!(result, vec![-1, 2, -3, 4]);
    }

    #[test]
    fn test_parse_jsonl_numbers_simple() {
        let content = r#"{"value": 1}
{"value": 2}
{"value": 3}"#;
        let result = parse_jsonl_numbers(content);
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_parse_jsonl_numbers_empty() {
        let content = "";
        let result = parse_jsonl_numbers(content);
        assert_eq!(result, Vec::<i64>::new());
    }

    #[test]
    fn test_parse_jsonl_numbers_empty_lines() {
        let content = r#"{"value": 1}

{"value": 2}
"#;
        let result = parse_jsonl_numbers(content);
        assert_eq!(result, vec![1, 2]);
    }

    #[test]
    fn test_parse_jsonl_numbers_ignores_missing_value() {
        let content = r#"{"value": 1}
{"other": 2}
{"value": 3}"#;
        let result = parse_jsonl_numbers(content);
        assert_eq!(result, vec![1, 3]);
    }

    #[test]
    fn test_parse_jsonl_numbers_negative() {
        let content = r#"{"value": -1}
{"value": 2}
{"value": -3}"#;
        let result = parse_jsonl_numbers(content);
        assert_eq!(result, vec![-1, 2, -3]);
    }

    #[test]
    fn test_parse_jsonl_numbers_large_numbers() {
        let content = r#"{"value": 1000000}
{"value": -1000000}"#;
        let result = parse_jsonl_numbers(content);
        assert_eq!(result, vec![1000000, -1000000]);
    }

    #[test]
    fn test_parse_jsonl_numbers_ignores_invalid_json() {
        let content = r#"{"value": 1}
invalid json
{"value": 2}"#;
        let result = parse_jsonl_numbers(content);
        assert_eq!(result, vec![1, 2]);
    }

    #[test]
    fn test_parse_plain_numbers_comma_separated() {
        let content = "1,2,3,4,5";
        let result = parse_plain_numbers(content);
        assert_eq!(result, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_parse_plain_numbers_newline_separated() {
        let content = "1\n2\n3";
        let result = parse_plain_numbers(content);
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_parse_plain_numbers_space_separated() {
        let content = "1 2 3 4";
        let result = parse_plain_numbers(content);
        assert_eq!(result, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_parse_plain_numbers_mixed_separators() {
        let content = "1,2 3\n4\t5";
        let result = parse_plain_numbers(content);
        assert_eq!(result, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_parse_plain_numbers_empty() {
        let content = "";
        let result = parse_plain_numbers(content);
        assert_eq!(result, Vec::<i64>::new());
    }

    #[test]
    fn test_parse_plain_numbers_ignores_non_numeric() {
        let content = "1,abc,2,def,3";
        let result = parse_plain_numbers(content);
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_split_into_chunks_exact_divisor() {
        let input = vec![1, 2, 3, 4, 5, 6];
        let result = split_into_chunks(input, 2);
        assert_eq!(result, vec![vec![1, 2], vec![3, 4], vec![5, 6]]);
    }

    #[test]
    fn test_split_into_chunks_remainder() {
        let input = vec![1, 2, 3, 4, 5];
        let result = split_into_chunks(input, 2);
        assert_eq!(result, vec![vec![1, 2], vec![3, 4], vec![5]]);
    }

    #[test]
    fn test_split_into_chunks_larger_than_input() {
        let input = vec![1, 2, 3];
        let result = split_into_chunks(input, 10);
        assert_eq!(result, vec![vec![1, 2, 3]]);
    }

    #[test]
    fn test_split_into_chunks_zero_chunk_size() {
        let input = vec![1, 2, 3, 4, 5];
        let result = split_into_chunks(input, 0);
        assert_eq!(result, vec![vec![1, 2, 3, 4, 5]]);
    }

    #[test]
    fn test_split_into_chunks_empty() {
        let input = vec![];
        let result = split_into_chunks(input, 2);
        assert_eq!(result, Vec::<Vec<i64>>::new());
    }

    #[test]
    fn test_split_into_chunks_single_element() {
        let input = vec![42];
        let result = split_into_chunks(input, 1);
        assert_eq!(result, vec![vec![42]]);
    }

    #[test]
    fn test_partition_by_key() {
        // Entrada: [k1, v1, k2, v2, k3, v3] con claves 1, 2, 3
        let kv_pairs = vec![1, 10, 2, 20, 3, 30];
        let result = partition_by_key(&kv_pairs, 2);
        
        // Clave 1 -> partición 1 % 2 = 1
        // Clave 2 -> partición 2 % 2 = 0
        // Clave 3 -> partición 3 % 2 = 1
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], vec![2, 20]); // partición 0
        assert_eq!(result[1], vec![1, 10, 3, 30]); // partición 1
    }

    #[test]
    fn test_partition_by_key_negative_keys() {
        // Las claves negativas usan abs() para particionado
        let kv_pairs = vec![-1, 10, -2, 20, 3, 30];
        let result = partition_by_key(&kv_pairs, 2);
        
        // abs(-1) = 1 -> partición 1 % 2 = 1
        // abs(-2) = 2 -> partición 2 % 2 = 0
        // abs(3) = 3 -> partición 3 % 2 = 1
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], vec![-2, 20]);
        assert_eq!(result[1], vec![-1, 10, 3, 30]);
    }

    #[test]
    fn test_partition_by_key_single_partition() {
        let kv_pairs = vec![1, 10, 2, 20, 3, 30];
        let result = partition_by_key(&kv_pairs, 1);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], vec![1, 10, 2, 20, 3, 30]);
    }

    #[test]
    fn test_partition_by_key_empty() {
        let kv_pairs = vec![];
        let result = partition_by_key(&kv_pairs, 3);
        assert_eq!(result.len(), 3);
        assert!(result.iter().all(|p| p.is_empty()));
    }

    #[test]
    fn test_partition_by_key_odd_length() {
        // Si la entrada tiene longitud impar, la última clave se ignora
        let kv_pairs = vec![1, 10, 2, 20, 3];
        let result = partition_by_key(&kv_pairs, 2);
        // Solo [1,10] y [2,20] se procesan, 3 se ignora
        assert_eq!(result[0], vec![2, 20]);
        assert_eq!(result[1], vec![1, 10]);
    }

    #[test]
    fn test_partition_by_key_zero_partitions() {
        let kv_pairs = vec![1, 10, 2, 20];
        let result = partition_by_key(&kv_pairs, 0);
        assert_eq!(result, Vec::<Vec<i64>>::new());
    }
}

