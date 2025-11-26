mod dag;

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
use common::metrics::{JobMetrics, MetricsResponse, NodeMetrics};
use dag::parse_dag;
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
    active_tasks: usize, // Number of active tasks
    retry_count: usize,  // Number of retries
    task_latencies: Vec<Duration>, // Latencies of completed tasks (for avg calculation)
}

/// Task information for tracking
#[derive(Debug, Clone)]
struct TaskInfo {
    task_id: String,
    node_id: String,
    job_id: String,
    worker_id: Option<String>,
    attempt_id: usize,
    status: TaskStatus,
    assignment: TaskAssignment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
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
    tasks: HashMap<String, TaskInfo>, // Track individual tasks
    start_time: SystemTime, // Job start time
    end_time: Option<SystemTime>, // Job end time
    stages: usize, // Number of stages in DAG
}

/// Shared application state
#[derive(Clone)]
struct AppState {
    registry: Arc<RwLock<HashMap<String, WorkerInfo>>>,
    jobs: Arc<RwLock<HashMap<String, JobInfo>>>,
    tasks: Arc<RwLock<HashMap<String, TaskInfo>>>, // Global task registry
}

// Configuration constants
const HEARTBEAT_TIMEOUT_SECS: u64 = 15;
const MONITOR_INTERVAL_SECS: u64 = 3;
const MAX_TASK_RETRIES: usize = 1; // At least 1 retry as per specification

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
        tasks: Arc::new(RwLock::new(HashMap::new())),
    };

    // Spawn background monitoring task
    let registry_monitor = state.registry.clone();
    let state_monitor = state.clone();
    tokio::spawn(async move {
        monitor_workers(registry_monitor, state_monitor).await;
    });

    // Build router
    let app = Router::new()
        .route("/api/v1/workers/register", post(register_worker))
        .route("/api/v1/workers/:id/heartbeat", post(heartbeat))
        .route("/api/v1/workers", get(list_workers))
        .route("/api/v1/jobs/submit", post(submit_job))
        .route("/api/v1/jobs/:id/progress", get(get_job_progress))
        .route("/api/v1/jobs/:id/task_result", post(job_task_result))
        .route("/api/v1/metrics", get(get_metrics))
        .route("/api/v1/metrics/nodes", get(get_node_metrics))
        .route("/api/v1/metrics/jobs", get(get_job_metrics))
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
        active_tasks: 0,
        retry_count: 0,
        task_latencies: Vec::new(),
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
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    // Determine number of tasks and dispatch
    let total_tasks = if let Some(ref dag) = payload.dag {
        // Process DAG format
        info!("Processing DAG for job: {}", job_id);
        let stages = match parse_dag(dag) {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to parse DAG: {}", e);
                return Err(StatusCode::BAD_REQUEST);
            }
        };

        let parallelism = payload.parallelism.unwrap_or(workers.len());
        let mut task_count = 0;
        let client = Client::new();

        // Dispatch tasks for each stage
        for (stage_idx, stage) in stages.iter().enumerate() {
            let num_tasks = stage.partitions.unwrap_or(parallelism);
            task_count += num_tasks;

            for task_idx in 0..num_tasks {
                let worker = &workers[task_idx % workers.len()];
                let task_id = format!("{}-stage{}-task{}", job_id, stage_idx, task_idx);
                
                let task_assignment = TaskAssignment {
                    job_id: job_id.clone(),
                    task_id: task_id.clone(),
                    node_id: stage.node_id.clone(),
                    operation: stage.operation.clone(),
                    fn_name: stage.fn_name.clone(),
                    key: stage.key.clone(),
                    param: None, // DAG operations don't use param
                    input: None, // Input will come from previous stage or file
                    input_path: stage.path.clone(),
                    attempt_id: Some(0),
                };
                
                // Register task
                let task_info = TaskInfo {
                    task_id: task_id.clone(),
                    node_id: stage.node_id.clone(),
                    job_id: job_id.clone(),
                    worker_id: Some(worker.id.clone()),
                    attempt_id: 0,
                    status: TaskStatus::Pending,
                    assignment: task_assignment.clone(),
                };
                
                {
                    let mut jobs = state.jobs.write().await;
                    if let Some(job) = jobs.get_mut(&job_id) {
                        job.tasks.insert(task_id.clone(), task_info);
                    }
                }

                let worker_url = format!(
                    "http://{}:{}/api/v1/tasks/execute",
                    worker.host, worker.port
                );

                let client_clone = client.clone();
                let task_clone = task_assignment.clone();
                // Update worker active tasks count
                {
                    let mut registry = state.registry.write().await;
                    if let Some(worker_info) = registry.get_mut(&worker.id) {
                        worker_info.active_tasks += 1;
                    }
                }
                
                // Update task status to Running
                {
                    let mut jobs = state.jobs.write().await;
                    if let Some(job) = jobs.get_mut(&job_id) {
                        if let Some(task) = job.tasks.get_mut(&task_id) {
                            task.status = TaskStatus::Running;
                        }
                    }
                }

                tokio::spawn(async move {
                    match client_clone
                        .post(&worker_url)
                        .json(&task_clone)
                        .send()
                        .await
                    {
                        Ok(resp) if resp.status().is_success() => {
                            info!("Task {} dispatched to worker {}", task_id, worker_url);
                        }
                        Ok(resp) => {
                            error!(
                                "Failed to dispatch task {} to worker {}: {}",
                                task_id, worker_url, resp.status()
                            );
                        }
                        Err(e) => {
                            error!(
                                "Error dispatching task {} to worker {}: {:?}",
                                task_id, worker_url, e
                            );
                        }
                    }
                });
            }
        }

        task_count
    } else if let Some(ref operation) = payload.operation {
        // Legacy format (simple operation)
        info!("Processing legacy format job: {}", job_id);
        let client = Client::new();
        for (idx, worker) in workers.iter().enumerate() {
            let task_id = format!("task-{}", idx);
            let task_id = format!("task-{}", idx);
            let task_assignment = TaskAssignment {
                job_id: job_id.clone(),
                task_id: task_id.clone(),
                node_id: format!("node-{}", idx),
                operation: operation.clone(),
                fn_name: None,
                key: None,
                param: payload.param,
                input: payload.input.clone(),
                input_path: None,
                attempt_id: Some(0),
            };
            
            // Register task for legacy format
            let task_info = TaskInfo {
                task_id: task_id.clone(),
                node_id: format!("node-{}", idx),
                job_id: job_id.clone(),
                worker_id: Some(worker.id.clone()),
                attempt_id: 0,
                status: TaskStatus::Pending,
                assignment: task_assignment.clone(),
            };
            
            {
                let mut jobs = state.jobs.write().await;
                if let Some(job) = jobs.get_mut(&job_id) {
                    job.tasks.insert(task_id.clone(), task_info);
                }
            }

            let worker_url = format!(
                "http://{}:{}/api/v1/tasks/execute",
                worker.host, worker.port
            );

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
        1 // Legacy format: 1 task per job
    } else {
        error!("Job spec must have either 'dag' or 'operation' field");
        return Err(StatusCode::BAD_REQUEST);
    };

    // Calculate number of stages
    let stages = if let Some(ref dag) = payload.dag {
        match parse_dag(dag) {
            Ok(parsed_stages) => parsed_stages.len(),
            Err(_) => 1, // Fallback to 1 stage
        }
    } else {
        1 // Legacy format: 1 stage
    };

    // Create job info
    let job_info = JobInfo {
        job_id: job_id.clone(),
        name: payload.name.clone(),
        total_tasks,
        completed_tasks: 0,
        failed_tasks: 0,
        status: "running".to_string(),
        tasks: HashMap::new(),
        start_time: SystemTime::now(),
        end_time: None,
        stages,
    };

    // Store job
    {
        let mut jobs = state.jobs.write().await;
        jobs.insert(job_id.clone(), job_info);
        info!("Created job: {} with id: {} ({} tasks)", payload.name, job_id, total_tasks);
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

    // Update job progress based on task result
    if payload.success {
        // Update task status
        if let Some(task) = job.tasks.get_mut(&payload.task_id) {
            task.status = TaskStatus::Completed;
            
            // Decrement worker active tasks
            if let Some(ref worker_id) = task.worker_id {
                let mut registry = state.registry.write().await;
                if let Some(worker_info) = registry.get_mut(worker_id) {
                    if worker_info.active_tasks > 0 {
                        worker_info.active_tasks -= 1;
                    }
                    // Record task latency (simplified: use current time)
                    // In a real system, we'd track start/end times per task
                }
            }
        }
        
        if job.completed_tasks < job.total_tasks {
            job.completed_tasks += 1;
            info!(
                "Job {} progress: {}/{} (task {} completed, attempt {})",
                job_id, job.completed_tasks, job.total_tasks, payload.task_id, payload.attempt_id
            );

            if job.completed_tasks == job.total_tasks {
                job.status = "completed".to_string();
                job.end_time = Some(SystemTime::now());
                info!("Job {} completed", job_id);
            }
        }
    } else {
        // Task failed - check if we should retry
        let task = job.tasks.get_mut(&payload.task_id);
        if let Some(task_info) = task {
            if task_info.attempt_id < MAX_TASK_RETRIES {
                // Retry the task
                let new_attempt_id = task_info.attempt_id + 1;
                info!(
                    "Retrying task {} for job {} (attempt {}/{})",
                    payload.task_id, job_id, new_attempt_id, MAX_TASK_RETRIES
                );
                
                task_info.attempt_id = new_attempt_id;
                task_info.status = TaskStatus::Pending;
                
                // Decrement worker active tasks (task failed)
                if let Some(ref worker_id) = task_info.worker_id {
                    let mut registry = state.registry.write().await;
                    if let Some(worker_info) = registry.get_mut(worker_id) {
                        if worker_info.active_tasks > 0 {
                            worker_info.active_tasks -= 1;
                        }
                        worker_info.retry_count += 1;
                    }
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
                
                if !workers.is_empty() {
                    let mut assignment = task_info.assignment.clone();
                    assignment.attempt_id = Some(new_attempt_id);
                    
                    let worker = &workers[0]; // Simple round-robin
                    let worker_url = format!(
                        "http://{}:{}/api/v1/tasks/execute",
                        worker.host, worker.port
                    );
                    
                    let client = Client::new();
                    let task_clone = assignment.clone();
                    tokio::spawn(async move {
                        match client.post(&worker_url).json(&task_clone).send().await {
                            Ok(resp) if resp.status().is_success() => {
                                info!("Retry task dispatched to worker {}", worker_url);
                            }
                            Ok(resp) => {
                                error!("Failed to retry task: {}", resp.status());
                            }
                            Err(e) => {
                                error!("Error retrying task: {:?}", e);
                            }
                        }
                    });
                }
            } else {
                // Max retries exceeded
                task_info.status = TaskStatus::Failed;
                job.failed_tasks += 1;
                error!(
                    "Task {} failed permanently for job {} after {} attempts: {}",
                    payload.task_id, job_id, MAX_TASK_RETRIES,
                    payload.error.as_deref().unwrap_or("Unknown error")
                );
                
                // Mark job as failed if too many tasks failed
                if job.failed_tasks > job.total_tasks / 2 {
                    job.status = "failed".to_string();
                    warn!("Job {} marked as failed due to too many task failures", job_id);
                }
            }
        } else {
            // Task not found in job tasks (legacy format)
            job.failed_tasks += 1;
            error!(
                "Task {} failed for job {} (attempt {}): {}",
                payload.task_id, job_id, payload.attempt_id,
                payload.error.as_deref().unwrap_or("Unknown error")
            );
        }
    }

    Ok(Json(json!({
        "status": "ack",
        "message": "Task result received"
    })))
}

/// Get all metrics (nodes and jobs)
async fn get_metrics(
    State(state): State<AppState>,
) -> Result<Json<MetricsResponse>, StatusCode> {
    let node_metrics = get_node_metrics_internal(&state).await;
    let job_metrics = get_job_metrics_internal(&state).await;

    Ok(Json(MetricsResponse {
        version: MESSAGE_VERSION.to_string(),
        node_metrics: Some(node_metrics),
        job_metrics: Some(job_metrics),
    }))
}

/// Get node metrics
async fn get_node_metrics(
    State(state): State<AppState>,
) -> Result<Json<MetricsResponse>, StatusCode> {
    let node_metrics = get_node_metrics_internal(&state).await;

    Ok(Json(MetricsResponse {
        version: MESSAGE_VERSION.to_string(),
        node_metrics: Some(node_metrics),
        job_metrics: None,
    }))
}

/// Get job metrics
async fn get_job_metrics(
    State(state): State<AppState>,
) -> Result<Json<MetricsResponse>, StatusCode> {
    let job_metrics = get_job_metrics_internal(&state).await;

    Ok(Json(MetricsResponse {
        version: MESSAGE_VERSION.to_string(),
        node_metrics: None,
        job_metrics: Some(job_metrics),
    }))
}

/// Internal function to get node metrics
async fn get_node_metrics_internal(state: &AppState) -> Vec<NodeMetrics> {
    let registry = state.registry.read().await;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    registry
        .values()
        .map(|worker| {
            // Calculate average latency
            let avg_latency_ms = if worker.task_latencies.is_empty() {
                0.0
            } else {
                let total: Duration = worker.task_latencies.iter().sum();
                total.as_millis() as f64 / worker.task_latencies.len() as f64
            };

            // Approximate CPU usage (simplified: based on active tasks)
            let cpu_usage_percent = (worker.active_tasks as f64 * 10.0).min(100.0);

            // Approximate memory usage (simplified: based on active tasks)
            let memory_usage_mb = worker.active_tasks as f64 * 50.0; // 50MB per task estimate

            NodeMetrics {
                node_id: worker.id.clone(),
                cpu_usage_percent,
                memory_usage_mb,
                active_tasks: worker.active_tasks,
                avg_latency_ms,
                retry_count: worker.retry_count,
                timestamp: now,
            }
        })
        .collect()
}

/// Internal function to get job metrics
async fn get_job_metrics_internal(state: &AppState) -> Vec<JobMetrics> {
    let jobs = state.jobs.read().await;
    let now = SystemTime::now();

    jobs.values()
        .map(|job| {
            let start_time_secs = job
                .start_time
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let end_time_secs = job.end_time.and_then(|et| {
                et.duration_since(SystemTime::UNIX_EPOCH)
                    .ok()
                    .map(|d| d.as_secs())
            });

            let total_time_secs = if let Some(end) = job.end_time {
                end.duration_since(job.start_time)
                    .unwrap_or_default()
                    .as_secs_f64()
            } else {
                now.duration_since(job.start_time)
                    .unwrap_or_default()
                    .as_secs_f64()
            };

            JobMetrics {
                job_id: job.job_id.clone(),
                name: job.name.clone(),
                total_time_secs,
                stages: job.stages,
                throughput_events_per_sec: None, // For streaming, would calculate based on events
                failure_count: job.failed_tasks,
                start_time: start_time_secs,
                end_time: end_time_secs,
            }
        })
        .collect()
}

/// Background task to monitor workers and mark them as DOWN if they exceed timeout
async fn monitor_workers(registry: Arc<RwLock<HashMap<String, WorkerInfo>>>, state: AppState) {
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
            
            // Replanificar tareas de workers que cayeron
            for worker_id in &marked_down {
                replanify_worker_tasks(&state, worker_id).await;
            }
        }
    }
}

/// Replanificar tareas de un worker que cayó
async fn replanify_worker_tasks(state: &AppState, failed_worker_id: &str) {
    info!("Replanifying tasks for failed worker: {}", failed_worker_id);
    
    // Get available workers
    let available_workers = {
        let registry = state.registry.read().await;
        registry
            .values()
            .filter(|w| w.status == WorkerStatus::Up && w.id != failed_worker_id)
            .cloned()
            .collect::<Vec<_>>()
    };

    if available_workers.is_empty() {
        warn!("No available workers to replanify tasks from worker {}", failed_worker_id);
        return;
    }

    // Find tasks assigned to the failed worker
    let mut tasks_to_replanify = Vec::new();
    {
        let jobs = state.jobs.read().await;
        for job in jobs.values() {
            for task in job.tasks.values() {
                if let Some(ref worker_id) = task.worker_id {
                    if worker_id == failed_worker_id && task.status == TaskStatus::Running {
                        tasks_to_replanify.push((job.job_id.clone(), task.clone()));
                    }
                }
            }
        }
    }

    if tasks_to_replanify.is_empty() {
        info!("No tasks to replanify for worker {}", failed_worker_id);
        return;
    }

    info!("Replanifying {} tasks from worker {}", tasks_to_replanify.len(), failed_worker_id);

    let client = Client::new();
    let mut worker_idx = 0;

    for (job_id, mut task_info) in tasks_to_replanify {
        // Update task status to Pending
        {
            let mut jobs = state.jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                if let Some(task) = job.tasks.get_mut(&task_info.task_id) {
                    task.status = TaskStatus::Pending;
                    task.worker_id = None; // Clear old worker assignment
                }
            }
        }

        // Assign to new worker
        let worker = &available_workers[worker_idx % available_workers.len()];
        worker_idx += 1;

        let mut assignment = task_info.assignment.clone();
        assignment.attempt_id = Some(task_info.attempt_id); // Keep attempt_id for idempotency

        let worker_url = format!(
            "http://{}:{}/api/v1/tasks/execute",
            worker.host, worker.port
        );

        // Update worker assignment
        {
            let mut jobs = state.jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                if let Some(task) = job.tasks.get_mut(&task_info.task_id) {
                    task.worker_id = Some(worker.id.clone());
                    task.status = TaskStatus::Running;
                }
            }
        }

        // Update worker active tasks count
        {
            let mut registry = state.registry.write().await;
            if let Some(worker_info) = registry.get_mut(&worker.id) {
                worker_info.active_tasks += 1;
            }
        }

        // Dispatch task to new worker
        let client_clone = client.clone();
        let task_clone = assignment.clone();
        let task_id = task_info.task_id.clone();
        tokio::spawn(async move {
            match client_clone.post(&worker_url).json(&task_clone).send().await {
                Ok(resp) if resp.status().is_success() => {
                    info!("Replanified task {} to worker {}", task_id, worker_url);
                }
                Ok(resp) => {
                    error!("Failed to replanify task {}: {}", task_id, resp.status());
                }
                Err(e) => {
                    error!("Error replanifying task {}: {:?}", task_id, e);
                }
            }
        });
    }
}
