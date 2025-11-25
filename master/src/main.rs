use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use common::{
    HeartbeatRequest, HeartbeatResponse, JobProgress, JobSpec, RegisterRequest, RegisterResponse,
    SubmitJobResponse, TaskAssignment, TaskResult, WorkerListItem, WorkerStatus,
    WorkersListResponse, MESSAGE_VERSION,
};
use reqwest::Client;
use serde_json::json;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Worker information stored in registry
#[derive(Debug, Clone)]
struct WorkerInfo {
    id: String,
    host: String,
    port: u16,
    last_heartbeat: SystemTime,
    status: WorkerStatus,
}

/// Job information stored in registry
#[derive(Debug, Clone)]
struct JobInfo {
    job_id: String,
    name: String,
    total_tasks: usize,
    completed_tasks: usize,
    failed_tasks: usize,
    status: String, // "running", "completed", "failed"
}

/// Shared application state
#[derive(Clone)]
struct AppState {
    registry: Arc<RwLock<HashMap<String, WorkerInfo>>>,
    jobs: Arc<RwLock<HashMap<String, JobInfo>>>,
}

// Configuration constants
const HEARTBEAT_TIMEOUT_SECS: u64 = 15;
const MONITOR_INTERVAL_SECS: u64 = 3;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Master node starting...");

    // Create shared state
    let state = AppState {
        registry: Arc::new(RwLock::new(HashMap::new())),
        jobs: Arc::new(RwLock::new(HashMap::new())),
    };

    // Spawn background monitoring task
    let registry_monitor = state.registry.clone();
    tokio::spawn(async move {
        monitor_workers(registry_monitor).await;
    });

    // Build router
    let app = Router::new()
        .route("/api/v1/workers/register", post(register_worker))
        .route("/api/v1/workers/:id/heartbeat", post(heartbeat))
        .route("/api/v1/workers", get(list_workers))
        .route("/api/v1/jobs/submit", post(submit_job))
        .route("/api/v1/jobs/:id/progress", get(get_job_progress))
        .route("/api/v1/jobs/:id/task_result", post(job_task_result))
        .with_state(state);

    // Start server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    info!("Master node listening on 127.0.0.1:8080");

    axum::serve(listener, app).await?;

    info!("Master node shutting down...");
    Ok(())
}

/// Register a new worker
async fn register_worker(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, StatusCode> {
    info!(
        "Received registration request from {}:{}",
        payload.host, payload.port
    );

    // Validate version if provided
    if let Some(version) = &payload.version {
        if version != MESSAGE_VERSION {
            warn!(
                "Version mismatch: expected {}, got {}",
                MESSAGE_VERSION, version
            );
        }
    }

    // Generate unique worker ID
    let worker_id = {
        let registry = state.registry.read().await;
        let mut id_num = 1;
        loop {
            let id = format!("w{:03}", id_num);
            if !registry.contains_key(&id) {
                break id;
            }
            id_num += 1;
        }
    };

    // Register worker
    let worker_info = WorkerInfo {
        id: worker_id.clone(),
        host: payload.host,
        port: payload.port,
        last_heartbeat: SystemTime::now(),
        status: WorkerStatus::Up,
    };

    {
        let mut registry = state.registry.write().await;
        registry.insert(worker_id.clone(), worker_info);
        info!(
            "Registered worker: {} (total workers: {})",
            worker_id,
            registry.len()
        );
    }

    Ok(Json(RegisterResponse {
        version: MESSAGE_VERSION.to_string(),
        worker_id,
        message: "Registration successful".to_string(),
    }))
}

/// Handle worker heartbeat
async fn heartbeat(
    State(state): State<AppState>,
    Path(worker_id): Path<String>,
    Json(payload): Json<HeartbeatRequest>,
) -> Result<Json<HeartbeatResponse>, StatusCode> {
    // Validate version if provided
    if let Some(version) = &payload.version {
        if version != MESSAGE_VERSION {
            warn!(
                "Version mismatch: expected {}, got {}",
                MESSAGE_VERSION, version
            );
        }
    }

    let mut registry = state.registry.write().await;

    // Find worker
    let worker = registry.get_mut(&worker_id).ok_or_else(|| {
        warn!("Heartbeat from unknown worker: {}", worker_id);
        StatusCode::NOT_FOUND
    })?;

    // Update heartbeat
    worker.last_heartbeat = SystemTime::now();

    // If worker was DOWN, mark it as UP again
    if worker.status == WorkerStatus::Down {
        info!("Worker {} recovered, marking as UP", worker_id);
        worker.status = WorkerStatus::Up;
    }

    drop(registry); // Release lock before logging
    info!("Received heartbeat from worker: {}", worker_id);

    Ok(Json(HeartbeatResponse {
        version: MESSAGE_VERSION.to_string(),
        status: "ok".to_string(),
    }))
}

/// List all registered workers
async fn list_workers(
    State(state): State<AppState>,
) -> Result<Json<WorkersListResponse>, StatusCode> {
    let registry = state.registry.read().await;

    let workers: Vec<WorkerListItem> = registry
        .values()
        .map(|worker| {
            let last_heartbeat_secs = worker
                .last_heartbeat
                .duration_since(SystemTime::UNIX_EPOCH)
                .ok()
                .map(|d| d.as_secs());

            WorkerListItem {
                id: worker.id.clone(),
                host: worker.host.clone(),
                port: worker.port,
                status: worker.status.as_str().to_string(),
                last_heartbeat: last_heartbeat_secs,
            }
        })
        .collect();

    info!("Listed {} workers", workers.len());

    Ok(Json(WorkersListResponse {
        version: MESSAGE_VERSION.to_string(),
        workers,
    }))
}

/// Submit a job for execution
async fn submit_job(
    State(state): State<AppState>,
    Json(payload): Json<JobSpec>,
) -> Result<Json<SubmitJobResponse>, StatusCode> {
    info!("Received job submission: {}", payload.name);

    let job_id = Uuid::new_v4().to_string();

    // Determine number of tasks (for now, 1 task per job)
    let total_tasks = 1;

    // Create job info
    let job_info = JobInfo {
        job_id: job_id.clone(),
        name: payload.name.clone(),
        total_tasks,
        completed_tasks: 0,
        failed_tasks: 0,
        status: "running".to_string(),
    };

    // Store job
    {
        let mut jobs = state.jobs.write().await;
        jobs.insert(job_id.clone(), job_info);
        info!("Created job: {} with id: {}", payload.name, job_id);
    }

    // Get available workers
    let workers = {
        let registry = state.registry.read().await;
        registry
            .values()
            .filter(|w| w.status == WorkerStatus::Up)
            .cloned()
            .collect::<Vec<_>>()
    };

    if workers.is_empty() {
        warn!("No available workers to execute job {}", job_id);
    } else {
        info!("Dispatching job {} to {} worker(s)", job_id, workers.len());
        // Dispatch tasks to workers
        let client = Client::new();
        for (idx, worker) in workers.iter().enumerate() {
            let task_id = format!("task-{}", idx);
            let task_assignment = TaskAssignment {
                job_id: job_id.clone(),
                task_id,
                operation: payload.operation.clone(),
                param: payload.param,
                input: payload.input.clone(),
            };

            let worker_url = format!(
                "http://{}:{}/api/v1/tasks/execute",
                worker.host, worker.port
            );

            // Send task to worker asynchronously
            let client_clone = client.clone();
            let task_clone = task_assignment.clone();
            tokio::spawn(async move {
                match client_clone
                    .post(&worker_url)
                    .json(&task_clone)
                    .send()
                    .await
                {
                    Ok(resp) if resp.status().is_success() => {
                        info!("Task dispatched successfully to worker {}", worker_url);
                    }
                    Ok(resp) => {
                        error!(
                            "Failed to dispatch task to worker {}: {}",
                            worker_url,
                            resp.status()
                        );
                    }
                    Err(e) => {
                        error!("Error dispatching task to worker {}: {:?}", worker_url, e);
                    }
                }
            });
        }
    }

    Ok(Json(SubmitJobResponse {
        version: MESSAGE_VERSION.to_string(),
        job_id,
        message: "Job submitted successfully".to_string(),
    }))
}

/// Get job progress
async fn get_job_progress(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> Result<Json<JobProgress>, StatusCode> {
    let jobs = state.jobs.read().await;

    let job = jobs.get(&job_id).ok_or_else(|| {
        warn!("Job not found: {}", job_id);
        StatusCode::NOT_FOUND
    })?;

    info!(
        "Retrieved progress for job: {} ({}/{})",
        job_id, job.completed_tasks, job.total_tasks
    );

    Ok(Json(JobProgress {
        job_id: job.job_id.clone(),
        name: job.name.clone(),
        total_tasks: job.total_tasks,
        completed_tasks: job.completed_tasks,
        failed_tasks: job.failed_tasks,
        status: job.status.clone(),
    }))
}

/// Receive task result from worker
async fn job_task_result(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
    Json(payload): Json<TaskResult>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!(
        "Received task result for job: {}, task: {}",
        job_id, payload.task_id
    );

    let mut jobs = state.jobs.write().await;

    let job = jobs.get_mut(&job_id).ok_or_else(|| {
        warn!("Job not found: {}", job_id);
        StatusCode::NOT_FOUND
    })?;

    // Update job progress
    if job.completed_tasks < job.total_tasks {
        job.completed_tasks += 1;
        info!(
            "Job {} progress: {}/{}",
            job_id, job.completed_tasks, job.total_tasks
        );

        if job.completed_tasks == job.total_tasks {
            job.status = "completed".to_string();
            info!("Job {} completed", job_id);
        }
    }

    Ok(Json(json!({
        "status": "ack",
        "message": "Task result received"
    })))
}

/// Background task to monitor workers and mark them as DOWN if they exceed timeout
async fn monitor_workers(registry: Arc<RwLock<HashMap<String, WorkerInfo>>>) {
    let mut interval = tokio::time::interval(Duration::from_secs(MONITOR_INTERVAL_SECS));

    loop {
        interval.tick().await;

        let now = SystemTime::now();
        let timeout = Duration::from_secs(HEARTBEAT_TIMEOUT_SECS);

        let mut registry_write = registry.write().await;
        let mut marked_down = Vec::new();

        for (worker_id, worker) in registry_write.iter_mut() {
            if let Ok(elapsed) = now.duration_since(worker.last_heartbeat) {
                if elapsed > timeout && worker.status == WorkerStatus::Up {
                    worker.status = WorkerStatus::Down;
                    marked_down.push(worker_id.clone());
                }
            }
        }

        drop(registry_write); // Release lock before logging

        for worker_id in &marked_down {
            warn!(
                "Worker {} exceeded heartbeat timeout, marked as DOWN",
                worker_id
            );
        }

        if !marked_down.is_empty() {
            info!("Marked {} worker(s) as DOWN", marked_down.len());
        }
    }
}
