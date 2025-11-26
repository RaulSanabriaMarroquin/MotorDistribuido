//! Operators for data processing tasks
//! Implements map, flat_map, filter, reduce_by_key according to the specification

use serde_json::Value;
use std::collections::HashMap;

/// Execute a map operation
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
            // For string operations, we'll need to handle differently
            // For now, this is a placeholder
            Err("to_lower requires string input".to_string())
        }
        Some(fn_name) => Err(format!("Unknown map function: {}", fn_name)),
    }
}

/// Execute a flat_map operation
/// flat_map applies a function to each element and flattens the result
pub fn flat_map(
    input: &[i64],
    fn_name: Option<&str>,
) -> Result<Vec<i64>, String> {
    match fn_name {
        Some("tokenize") | None => {
            // Example: tokenize numbers into digits
            // For a real tokenize, we'd split strings, but for integers we'll split digits
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
            // Split each number into its digits
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
        Some(fn_name) => Err(format!("Unknown flat_map function: {}", fn_name)),
    }
}

/// Execute a filter operation
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
        Some(fn_name) => Err(format!("Unknown filter function: {}", fn_name)),
    }
}

/// Execute a reduce_by_key operation
/// Groups elements by a key and applies an aggregation function
/// For now, we'll use the value itself as the key (for integers)
pub fn reduce_by_key(
    input: &[i64],
    fn_name: Option<&str>,
) -> Result<HashMap<i64, i64>, String> {
    let mut result = HashMap::new();
    
    match fn_name {
        Some("sum") | None => {
            // Group by value and sum counts
            for &value in input {
                *result.entry(value).or_insert(0) += 1;
            }
        }
        Some("count") => {
            // Count occurrences of each value
            for &value in input {
                *result.entry(value).or_insert(0) += 1;
            }
        }
        Some("max") => {
            // Keep maximum value for each key
            for &value in input {
                let entry = result.entry(value).or_insert(value);
                *entry = (*entry).max(value);
            }
        }
        Some("min") => {
            // Keep minimum value for each key
            for &value in input {
                let entry = result.entry(value).or_insert(value);
                *entry = (*entry).min(value);
            }
        }
        Some(fn_name) => return Err(format!("Unknown reduce_by_key function: {}", fn_name)),
    }
    
    Ok(result)
}

/// Convert reduce_by_key result to a vector format for output
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

