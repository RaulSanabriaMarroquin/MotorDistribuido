//! Operator implementations for batch processing

use common::Operator;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
// Tracing imports removed as not used in this module

/// Process records through an operator
pub fn execute_operator(
    operator: &Operator,
    input_paths: &[String],
    output_path: &str,
) -> Result<u64, String> {
    // Ensure output directory exists
    if let Some(parent) = Path::new(output_path).parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create output dir: {}", e))?;
    }

    match operator {
        Operator::ReadCsv { path, .. } => read_csv(path, output_path),
        Operator::ReadJsonl { path, .. } => read_jsonl(path, output_path),
        Operator::Map { fn_name } => map_operator(input_paths, output_path, fn_name),
        Operator::FlatMap { fn_name } => flat_map_operator(input_paths, output_path, fn_name),
        Operator::Filter { fn_name } => filter_operator(input_paths, output_path, fn_name),
        Operator::Reduce { fn_name } => reduce_operator(input_paths, output_path, fn_name),
        Operator::ReduceByKey { key, fn_name } => {
            reduce_by_key_operator(input_paths, output_path, key, fn_name)
        }
        Operator::WriteCsv { path } => {
            // Just copy input to output path
            if let Some(input_path) = input_paths.first() {
                fs::copy(input_path, path)
                    .map_err(|e| format!("Failed to write CSV: {}", e))?;
                Ok(0) // Count handled by reading
            } else {
                Err("No input path provided".to_string())
            }
        }
        Operator::WriteJsonl { path } => {
            if let Some(input_path) = input_paths.first() {
                fs::copy(input_path, path)
                    .map_err(|e| format!("Failed to write JSONL: {}", e))?;
                Ok(0)
            } else {
                Err("No input path provided".to_string())
            }
        }
        Operator::Join { key, other_collection } => {
            join_operator(input_paths, output_path, key, other_collection)
        }
        Operator::Shuffle { .. } => Err("Shuffle operator not yet implemented".to_string()),
    }
}

fn read_csv(input_path: &str, output_path: &str) -> Result<u64, String> {
    let file = fs::File::open(input_path).map_err(|e| format!("Failed to open CSV: {}", e))?;
    let reader = BufReader::new(file);
    let mut output = fs::File::create(output_path)
        .map_err(|e| format!("Failed to create output file: {}", e))?;

    let mut count = 0;
    for line in reader.lines() {
        let line = line.map_err(|e| format!("Failed to read line: {}", e))?;
        writeln!(output, "{}", line)
            .map_err(|e| format!("Failed to write line: {}", e))?;
        count += 1;
    }

    Ok(count)
}

fn read_jsonl(input_path: &str, output_path: &str) -> Result<u64, String> {
    let file = fs::File::open(input_path).map_err(|e| format!("Failed to open JSONL: {}", e))?;
    let reader = BufReader::new(file);
    let mut output = fs::File::create(output_path)
        .map_err(|e| format!("Failed to create output file: {}", e))?;

    let mut count = 0;
    for line in reader.lines() {
        let line = line.map_err(|e| format!("Failed to read line: {}", e))?;
        writeln!(output, "{}", line)
            .map_err(|e| format!("Failed to write line: {}", e))?;
        count += 1;
    }

    Ok(count)
}

fn map_operator(input_paths: &[String], output_path: &str, fn_name: &str) -> Result<u64, String> {
    let mut output = fs::File::create(output_path)
        .map_err(|e| format!("Failed to create output file: {}", e))?;
    let mut count = 0;

    for input_path in input_paths {
        let file = fs::File::open(input_path).map_err(|e| format!("Failed to open file: {}", e))?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line.map_err(|e| format!("Failed to read line: {}", e))?;
            let result = apply_map_function(&line, fn_name)?;
            writeln!(output, "{}", result)
                .map_err(|e| format!("Failed to write line: {}", e))?;
            count += 1;
        }
    }

    Ok(count)
}

fn flat_map_operator(
    input_paths: &[String],
    output_path: &str,
    fn_name: &str,
) -> Result<u64, String> {
    let mut output = fs::File::create(output_path)
        .map_err(|e| format!("Failed to create output file: {}", e))?;
    let mut count = 0;

    for input_path in input_paths {
        let file = fs::File::open(input_path).map_err(|e| format!("Failed to open file: {}", e))?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line.map_err(|e| format!("Failed to read line: {}", e))?;
            let results = apply_flat_map_function(&line, fn_name)?;
            for result in results {
                writeln!(output, "{}", result)
                    .map_err(|e| format!("Failed to write line: {}", e))?;
                count += 1;
            }
        }
    }

    Ok(count)
}

fn filter_operator(
    input_paths: &[String],
    output_path: &str,
    fn_name: &str,
) -> Result<u64, String> {
    let mut output = fs::File::create(output_path)
        .map_err(|e| format!("Failed to create output file: {}", e))?;
    let mut count = 0;

    for input_path in input_paths {
        let file = fs::File::open(input_path).map_err(|e| format!("Failed to open file: {}", e))?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line.map_err(|e| format!("Failed to read line: {}", e))?;
            if apply_filter_function(&line, fn_name)? {
                writeln!(output, "{}", line)
                    .map_err(|e| format!("Failed to write line: {}", e))?;
                count += 1;
            }
        }
    }

    Ok(count)
}

fn reduce_operator(
    input_paths: &[String],
    output_path: &str,
    fn_name: &str,
) -> Result<u64, String> {
    let mut accumulator: Option<Value> = None;

    for input_path in input_paths {
        let file = fs::File::open(input_path).map_err(|e| format!("Failed to open file: {}", e))?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line.map_err(|e| format!("Failed to read line: {}", e))?;
            let value: Value = serde_json::from_str(&line)
                .map_err(|e| format!("Failed to parse JSON: {}", e))?;

            accumulator = match accumulator {
                Some(acc) => Some(apply_reduce_function(&acc, &value, fn_name)?),
                None => Some(value),
            };
        }
    }

    if let Some(result) = accumulator {
        let mut output = fs::File::create(output_path)
            .map_err(|e| format!("Failed to create output file: {}", e))?;
        writeln!(output, "{}", serde_json::to_string(&result).unwrap())
            .map_err(|e| format!("Failed to write result: {}", e))?;
        Ok(1)
    } else {
        Ok(0)
    }
}

fn reduce_by_key_operator(
    input_paths: &[String],
    output_path: &str,
    key: &str,
    fn_name: &str,
) -> Result<u64, String> {
    let mut groups: HashMap<String, Vec<Value>> = HashMap::new();

    // Group by key
    for input_path in input_paths {
        let file = fs::File::open(input_path).map_err(|e| format!("Failed to open file: {}", e))?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line.map_err(|e| format!("Failed to read line: {}", e))?;
            let value: Value = serde_json::from_str(&line)
                .map_err(|e| format!("Failed to parse JSON: {}", e))?;

            let key_value = value
                .get(key)
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("Key '{}' not found or not a string", key))?
                .to_string();

            groups.entry(key_value).or_insert_with(Vec::new).push(value);
        }
    }

    // Reduce each group
    let mut output = fs::File::create(output_path)
        .map_err(|e| format!("Failed to create output file: {}", e))?;
    let mut count = 0;

    for (_, values) in groups {
        let mut accumulator: Option<Value> = None;
        for value in values {
            accumulator = match accumulator {
                Some(acc) => Some(apply_reduce_function(&acc, &value, fn_name)?),
                None => Some(value),
            };
        }

        if let Some(result) = accumulator {
            writeln!(output, "{}", serde_json::to_string(&result).unwrap())
                .map_err(|e| format!("Failed to write result: {}", e))?;
            count += 1;
        }
    }

    Ok(count)
}

fn join_operator(
    input_paths: &[String],
    output_path: &str,
    key: &str,
    other_collection: &str,
) -> Result<u64, String> {
    // Join requires two input collections
    // First input_paths[0] is the left collection
    // other_collection should be a path to the right collection
    // For simplicity, we'll use input_paths[0] as left and other_collection as right
    
    if input_paths.is_empty() {
        return Err("Join requires at least one input path".to_string());
    }

    let left_path = &input_paths[0];
    let right_path = other_collection;

    // Build index from right collection (smaller one typically)
    let mut right_index: HashMap<String, Vec<Value>> = HashMap::new();
    
    let right_file = fs::File::open(right_path)
        .map_err(|e| format!("Failed to open right collection: {}", e))?;
    let right_reader = BufReader::new(right_file);

    for line in right_reader.lines() {
        let line = line.map_err(|e| format!("Failed to read line: {}", e))?;
        let value: Value = serde_json::from_str(&line)
            .map_err(|e| format!("Failed to parse JSON: {}", e))?;

        let key_value = value
            .get(key)
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("Key '{}' not found or not a string in right collection", key))?
            .to_string();

        right_index.entry(key_value).or_insert_with(Vec::new).push(value);
    }

    // Join left collection with right index
    let mut output = fs::File::create(output_path)
        .map_err(|e| format!("Failed to create output file: {}", e))?;
    let mut count = 0;

    let left_file = fs::File::open(left_path)
        .map_err(|e| format!("Failed to open left collection: {}", e))?;
    let left_reader = BufReader::new(left_file);

    for line in left_reader.lines() {
        let line = line.map_err(|e| format!("Failed to read line: {}", e))?;
        let left_value: Value = serde_json::from_str(&line)
            .map_err(|e| format!("Failed to parse JSON: {}", e))?;

        let key_value = left_value
            .get(key)
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("Key '{}' not found or not a string in left collection", key))?
            .to_string();

        // Find matching records in right collection
        if let Some(right_values) = right_index.get(&key_value) {
            for right_value in right_values {
                // Merge left and right values
                let mut joined = left_value.clone();
                if let Some(obj) = joined.as_object_mut() {
                    // Add fields from right value, prefixing with "right_" if conflict
                    if let Some(right_obj) = right_value.as_object() {
                        for (k, v) in right_obj {
                            if obj.contains_key(k) && k != key {
                                // Prefix to avoid conflict
                                obj.insert(format!("right_{}", k), v.clone());
                            } else {
                                obj.insert(k.clone(), v.clone());
                            }
                        }
                    }
                }
                writeln!(output, "{}", serde_json::to_string(&joined).unwrap())
                    .map_err(|e| format!("Failed to write joined record: {}", e))?;
                count += 1;
            }
        }
        // Inner join: skip if no match
    }

    Ok(count)
}

// Helper functions to apply user-defined functions
fn apply_map_function(line: &str, fn_name: &str) -> Result<String, String> {
    match fn_name {
        "to_lower" => Ok(line.to_lowercase()),
        "to_upper" => Ok(line.to_uppercase()),
        "trim" => Ok(line.trim().to_string()),
        _ => {
            // Try to parse as JSON and apply function
            if let Ok(mut value) = serde_json::from_str::<Value>(line) {
                match fn_name {
                    "to_lower" => {
                        if let Some(s) = value.as_str() {
                            value = Value::String(s.to_lowercase());
                        }
                    }
                    _ => {}
                }
                Ok(serde_json::to_string(&value).unwrap())
            } else {
                Ok(line.to_string())
            }
        }
    }
}

fn apply_flat_map_function(line: &str, fn_name: &str) -> Result<Vec<String>, String> {
    match fn_name {
        "tokenize" => {
            let tokens: Vec<String> = line
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
            Ok(tokens)
        }
        "split_lines" => Ok(line.lines().map(|s| s.to_string()).collect()),
        _ => Ok(vec![line.to_string()]),
    }
}

fn apply_filter_function(line: &str, fn_name: &str) -> Result<bool, String> {
    match fn_name {
        "non_empty" => Ok(!line.trim().is_empty()),
        "contains_alpha" => Ok(line.chars().any(|c| c.is_alphabetic())),
        _ => Ok(true),
    }
}

fn apply_reduce_function(acc: &Value, value: &Value, fn_name: &str) -> Result<Value, String> {
    match fn_name {
        "sum" => {
            let acc_num = acc.as_f64().or_else(|| acc.as_u64().map(|n| n as f64)).unwrap_or(0.0);
            let val_num = value.as_f64().or_else(|| value.as_u64().map(|n| n as f64)).unwrap_or(0.0);
            let sum = acc_num + val_num;
            // Convert to Number - use integer if possible, otherwise float
            if sum.fract() == 0.0 && sum >= 0.0 {
                Ok(Value::Number(serde_json::Number::from(sum as u64)))
            } else if sum.fract() == 0.0 {
                Ok(Value::Number(serde_json::Number::from(sum as i64)))
            } else {
                // For floats, we need to use Value::Number with from_f64
                // If that fails, fall back to string representation
                match serde_json::Number::from_f64(sum) {
                    Some(n) => Ok(Value::Number(n)),
                    None => Ok(Value::String(sum.to_string())),
                }
            }
        }
        "count" => {
            let acc_num = acc.as_u64().unwrap_or(0);
            Ok(Value::Number((acc_num + 1).into()))
        }
        _ => Ok(value.clone()),
    }
}

