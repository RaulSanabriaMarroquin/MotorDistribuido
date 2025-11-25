mod operators;
mod cache;

use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use common::{
    HeartbeatRequest, RegisterRequest, RegisterResponse, TaskRequest, TaskResponse,
    TaskStatusUpdate, MESSAGE_VERSION,
};
use reqwest::Client;
use tokio::sync::RwLock;
use tokio::{net::TcpListener, time::sleep};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

/// Worker state
#[derive(Clone)]
struct WorkerState {
    master_url: String,
    worker_id: String,
    active_tasks: Arc<RwLock<u32>>,
}

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

    info!(%master_url, %host, port, "Worker node starting");

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

    // 2) Worker state
    let worker_state = WorkerState {
        master_url: master_url.clone(),
        worker_id: worker_id.clone(),
        active_tasks: Arc::new(RwLock::new(0)),
    };

    // 3) Servidor HTTP local
    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/api/v1/tasks/execute", post(execute_task))
        .with_state(worker_state);
    let addr = format!("{}:{}", host, port);
    let listener = TcpListener::bind(&addr).await?;
    info!(%addr, "Starting worker HTTP server");

    // 4) Task de heartbeats
    let hb_client = http_client.clone();
    let hb_master_url = master_url.clone();
    let hb_worker_id = worker_id.clone();
    tokio::spawn(async move {
        let hb_url = format!("{}/api/v1/workers/{}/heartbeat", hb_master_url, hb_worker_id);
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

    // 5) Mantener el worker corriendo con graceful shutdown
    let shutdown_signal = async {
        #[cfg(unix)]
        {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("Failed to install SIGTERM handler")
                .recv()
                .await;
        }
        #[cfg(windows)]
        {
            tokio::signal::windows::ctrl_c()
                .expect("Failed to install Ctrl+C handler")
                .recv()
                .await;
        }
        #[cfg(not(any(unix, windows)))]
        {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
        }
    };

    let server = axum::serve(listener, app);
    let graceful = server.with_graceful_shutdown(shutdown_signal);

    info!("Press Ctrl+C to shutdown gracefully...");
    graceful.await?;

    info!("Worker shutting down gracefully...");
    Ok(())
}

/// Execute a task assigned by the master
async fn execute_task(
    State(state): State<WorkerState>,
    Json(payload): Json<TaskRequest>,
) -> Result<Json<TaskResponse>, StatusCode> {
    let assignment = &payload.assignment;
    info!(
        task_id = %assignment.task_id,
        job_id = %assignment.job_id,
        "Received task assignment"
    );

    // Update active tasks count
    {
        let mut active = state.active_tasks.write().await;
        *active += 1;
    }

    let start_time = Instant::now();
    let mut records_processed = 0u64;
    let mut error_msg = None;

    // Idempotency: Check if output already exists for this attempt
    // If output exists and is valid, skip execution (idempotency)
    let output_exists = std::path::Path::new(&assignment.output_path).exists();
    let should_execute = if output_exists {
        // Check if file is valid (non-empty)
        if let Ok(metadata) = std::fs::metadata(&assignment.output_path) {
            if metadata.len() > 0 {
                info!(
                    task_id = %assignment.task_id,
                    attempt_id = assignment.attempt_id,
                    output_path = %assignment.output_path,
                    "Output already exists, skipping execution (idempotency)"
                );
                false
            } else {
                true // File exists but is empty, re-execute
            }
        } else {
            true // Can't check, re-execute
        }
    } else {
        true // Output doesn't exist, execute
    };

    // Execute task in a blocking thread pool to avoid blocking async runtime
    let assignment_clone = assignment.clone();
    let result = if should_execute {
        tokio::task::spawn_blocking(move || {
            operators::execute_operator(
                &assignment_clone.operator,
                &assignment_clone.input_paths,
                &assignment_clone.output_path,
            )
        })
        .await
    } else {
        // Skip execution, return success with existing output
        Ok(Ok(0)) // Count will be updated from file if needed
    };

    match result {
        Ok(Ok(count)) => {
            records_processed = count;
            info!(
                task_id = %assignment.task_id,
                records = count,
                "Task completed successfully"
            );
        }
        Ok(Err(e)) => {
            error_msg = Some(e.clone());
            error!(task_id = %assignment.task_id, error = %e, "Task failed");
        }
        Err(e) => {
            error_msg = Some(format!("Task execution error: {:?}", e));
            error!(task_id = %assignment.task_id, error = ?e, "Task execution panicked");
        }
    }

    let execution_time = start_time.elapsed().as_secs_f64();

    // Update active tasks count
    {
        let mut active = state.active_tasks.write().await;
        if *active > 0 {
            *active -= 1;
        }
    }

    // Send status update to master
    let status_update = TaskStatusUpdate {
        version: Some(MESSAGE_VERSION.to_string()),
        task_id: assignment.task_id.clone(),
        attempt_id: assignment.attempt_id,
        status: if error_msg.is_none() {
            "COMPLETED".to_string()
        } else {
            "FAILED".to_string()
        },
        output_path: if error_msg.is_none() {
            Some(assignment.output_path.clone())
        } else {
            None
        },
        error: error_msg.clone(),
        metrics: common::TaskMetrics {
            execution_time_secs: execution_time,
            records_processed,
            memory_used_mb: 0.0, // TODO: implement actual memory tracking
        },
    };

    // Send update to master
    let http_client = Client::new();
    let update_url = format!(
        "{}/api/v1/tasks/{}/status",
        state.master_url, assignment.task_id
    );

    match http_client.post(&update_url).json(&status_update).send().await {
        Ok(resp) if resp.status().is_success() => {
            info!(task_id = %assignment.task_id, "Status update sent to master");
        }
        Ok(resp) => {
            warn!(
                task_id = %assignment.task_id,
                status = ?resp.status(),
                "Failed to send status update"
            );
        }
        Err(e) => {
            warn!(task_id = %assignment.task_id, error = ?e, "Error sending status update");
        }
    }

    Ok(Json(TaskResponse {
        version: MESSAGE_VERSION.to_string(),
        task_id: assignment.task_id.clone(),
        attempt_id: assignment.attempt_id,
        status: if error_msg.is_none() {
            "COMPLETED".to_string()
        } else {
            "FAILED".to_string()
        },
        output_path: if error_msg.is_none() {
            Some(assignment.output_path.clone())
        } else {
            None
        },
        error: error_msg,
        metrics: common::TaskMetrics {
            execution_time_secs: execution_time,
            records_processed,
            memory_used_mb: 0.0,
        },
    }))
}
