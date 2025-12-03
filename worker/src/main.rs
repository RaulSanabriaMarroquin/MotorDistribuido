use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::{
    extract::Json,
    response::Json as AxumJson,
    routing::{get, post},
    Router,
};
use common::{
    HeartbeatRequest, RegisterRequest, RegisterResponse, TaskAssignment, TaskResult,
    MESSAGE_VERSION,
};
use reqwest::Client;
use tokio::{net::TcpListener, time::sleep};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // Config desde env
    let master_url =
        std::env::var("MASTER_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());
    let host = std::env::var("WORKER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: u16 = std::env::var("WORKER_PORT")
        .unwrap_or_else(|_| "9000".to_string())
        .parse()
        .expect("Invalid WORKER_PORT");
    let heartbeat_interval_secs: u64 = std::env::var("HEARTBEAT_INTERVAL_SECS")
        .unwrap_or_else(|_| "3".to_string())
        .parse()
        .expect("Invalid HEARTBEAT_INTERVAL_SECS");
    let task_execution_delay_secs: u64 = std::env::var("TASK_EXECUTION_DELAY_SECS")
        .unwrap_or_else(|_| "0".to_string())
        .parse()
        .expect("Invalid TASK_EXECUTION_DELAY_SECS");

    info!(%master_url, %host, port, task_execution_delay_secs, "Worker node starting");

    let http_client = Client::new();

    // 1) Registro en el master
    let register_req = RegisterRequest {
        version: Some(MESSAGE_VERSION.to_string()),
        host: host.clone(),
        port,
    };

    let register_url = format!("{}/api/v1/workers/register", master_url);
    let worker_id = loop {
        match http_client
            .post(&register_url)
            .json(&register_req)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                let body: RegisterResponse = resp.json().await?;
                info!(worker_id = %body.worker_id, "Registration successful");
                break body.worker_id;
            }
            Ok(resp) => {
                error!(status=?resp.status(), "Registration failed");
            }
            Err(e) => {
                error!(error=?e, "Error registering worker");
            }
        }

        let backoff = Duration::from_secs(3);
        info!(?backoff, "Retrying registration after backoff");
        sleep(backoff).await;
    };

    // 2) Servidor HTTP local con /health y /api/v1/tasks/execute
    let master_url_clone = master_url.clone();
    let http_client_clone = http_client.clone();
    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/api/v1/tasks/execute", post(execute_task))
        .with_state((master_url_clone, http_client_clone, task_execution_delay_secs));

    let addr = format!("{}:{}", host, port);
    let listener = TcpListener::bind(&addr).await?;
    info!(%addr, "Starting worker HTTP server (/health, /api/v1/tasks/execute)");

    // 3) Task de heartbeats
    let hb_client = http_client.clone();
    let hb_master_url = master_url.clone();
    let hb_worker_id = worker_id.clone();
    tokio::spawn(async move {
        let hb_url = format!(
            "{}/api/v1/workers/{}/heartbeat",
            hb_master_url, hb_worker_id
        );
        loop {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_else(|_| Duration::from_secs(0))
                .as_secs();

            let hb_req = HeartbeatRequest {
                version: Some(MESSAGE_VERSION.to_string()),
                timestamp: Some(now),
            };

            match hb_client.post(&hb_url).json(&hb_req).send().await {
                Ok(resp) if resp.status().is_success() => {
                    info!("Heartbeat sent");
                }
                Ok(resp) => {
                    error!(status=?resp.status(), "Heartbeat failed");
                }
                Err(e) => {
                    error!(error=?e, "Error sending heartbeat");
                }
            }

            sleep(Duration::from_secs(heartbeat_interval_secs)).await;
        }
    });

    // 4) Mantener el worker corriendo
    axum::serve(listener, app).await?;
    Ok(())
}

/// Execute a task and return result
async fn execute_task(
    axum::extract::State((master_url, http_client, task_execution_delay_secs)): axum::extract::State<(String, Client, u64)>,
    Json(payload): Json<TaskAssignment>,
) -> Result<AxumJson<serde_json::Value>, axum::http::StatusCode> {
    info!(
    "Executing task: {} for job: {} (op: {}) stage={} attempt={}",
    payload.task_id,
    payload.job_id,
    payload.operation,
    payload.stage_id,
    payload.reassign_attempt,
    );

    // Add optional delay to simulate long-running tasks
    if task_execution_delay_secs > 0 {
        info!(
            "Task {} delaying execution for {} seconds",
            payload.task_id, task_execution_delay_secs
        );
        sleep(Duration::from_secs(task_execution_delay_secs)).await;
    }

    // Execute the operation
    let mut output: Vec<i64> = Vec::new();
    let mut task_error: Option<String> = None;

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
            // ensure even length
            if payload.input.len() % 2 != 0 {
                task_error = Some("reduce_by_key: input length is not even (key/value mismatch)".into());
            } else {
                let mut map = std::collections::HashMap::<i64, i64>::new();

                // accumulate values by key
                for pair in payload.input.chunks(2) {
                    let key = pair[0];
                    let val = pair[1];
                    *map.entry(key).or_insert(0) += val;
                }

                // produce sorted interleaved output
                let mut kv_pairs: Vec<(i64, i64)> = map.into_iter().collect();
                kv_pairs.sort_by_key(|p| p.0);

                output = kv_pairs
                    .into_iter()
                    .flat_map(|(k,v)| vec![k, v])
                    .collect();
            }
        }
            "join" => {
            // Validate input
            if payload.left.len() % 2 != 0 || payload.right.len() % 2 != 0 {
                task_error = Some("JOIN: left or right length is not even (k,v pairs malformed)".into());
            } else {
                // Build maps for left and right collections
                let mut left_map = std::collections::HashMap::<i64, i64>::new();
                let mut right_map = std::collections::HashMap::<i64, i64>::new();

                for pair in payload.left.chunks(2) {
                    left_map.insert(pair[0], pair[1]);
                }
                for pair in payload.right.chunks(2) {
                    right_map.insert(pair[0], pair[1]);
                }

                // Compute intersection keys
                let mut result: Vec<i64> = Vec::new();
                let mut keys: Vec<i64> = left_map
                    .keys()
                    .filter(|k| right_map.contains_key(k))
                    .cloned()
                    .collect();

                // Sort keys for deterministic output
                keys.sort();

                // Join output: [k, left_val, right_val, ...]
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
            let mut out = Vec::new();
            for x in &payload.input {
                // ejemplo sencillo: produce dos valores
                out.push(*x);
                out.push(*x * 2);
            }
            output = out;
        }
        _ => {
            let msg = format!("Unknown operation: {}", payload.operation);
            error!("{}", msg);
            task_error = Some(msg);
        }
    };

    info!(
        "Task {} completed: input_size={}, output_size={}",
        payload.task_id,
        payload.input.len(),
        output.len()
    );

    // Send result back to master
    let task_result = TaskResult {
        job_id: payload.job_id.clone(),
        task_id: payload.task_id.clone(),
        output: output.clone(),
        error: task_error.clone(),
    };

    let result_url = format!("{}/api/v1/jobs/{}/task_result", master_url, payload.job_id);

    match http_client
        .post(&result_url)
        .json(&task_result)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            info!("Task result sent to master for task: {}", payload.task_id);
        }
        Ok(resp) => {
            error!(
                "Failed to send task result to master: {} (status: {})",
                result_url,
                resp.status()
            );
        }
        Err(e) => {
            error!(
                "Error sending task result to master: {} (error: {:?})",
                result_url, e
            );
        }
    }

    Ok(AxumJson(serde_json::json!({
        "status": "ok",
        "task_id": payload.task_id,
        "output": output
    })))
}
