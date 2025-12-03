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
    WorkersListResponse, Stage, DatasetSource,  MESSAGE_VERSION,
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
    /// Track outstanding task assignments: (task_id) -> worker_id
    outstanding_tasks: HashMap<String, String>,
    /// Queue of tasks waiting to be reassigned (task_id, chunk_data, operation, param)
    reassign_queue: Vec<(String, Vec<i64>, String, Option<i64>)>,
    /// Stored payloads for tasks so we can reconstruct on reassignment
    task_payloads: HashMap<String, (Vec<i64>, String, Option<i64>)>,
    /// Multi-stage DAG support
    stages: Vec<Stage>,
    /// Current stage being executed (0-indexed)
    current_stage: usize,
    /// Output from previous stage (used as input for next stage)
    stage_output: Vec<i64>,
    // NUEVO: salida derecha para JOIN
    pub stage_output_left: Vec<i64>,
    pub stage_output_right: Vec<i64>,

}

/// Shared application state
#[derive(Clone)]
struct AppState {
    registry: Arc<RwLock<HashMap<String, WorkerInfo>>>,
    jobs: Arc<RwLock<HashMap<String, JobInfo>>>,
    metrics: Arc<RwLock<Metrics>>,
}

#[derive(Debug, Default)]
struct Metrics {
    jobs_submitted: u64,
    tasks_completed: u64,
    tasks_failed: u64,
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
        metrics: Arc::new(RwLock::new(Metrics::default())),
    };

    // Spawn background monitoring task
    let registry_monitor = state.registry.clone();
    let jobs_monitor = state.jobs.clone();
    let metrics_monitor = state.metrics.clone();
    let client_monitor = Client::new();
    tokio::spawn(async move {
        monitor_workers(registry_monitor, jobs_monitor, metrics_monitor, client_monitor).await;
    });

    // Build router
    let app = Router::new()
        .route("/api/v1/workers/register", post(register_worker))
        .route("/api/v1/workers/:id/heartbeat", post(heartbeat))
        .route("/api/v1/workers", get(list_workers))
        .route("/api/v1/jobs/submit", post(submit_job))
        .route("/api/v1/jobs/:id/progress", get(get_job_progress))
        .route("/api/v1/jobs/:id/task_result", post(job_task_result))
        .route("/api/v1/jobs/:id/next_stage", post(next_stage))
        .route("/api/v1/metrics", get(get_metrics))
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


/// Get global metrics
async fn get_metrics(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let metrics = state.metrics.read().await;
    let registry = state.registry.read().await;

    let workers_up = registry
        .values()
        .filter(|w| w.status == WorkerStatus::Up)
        .count();

    let workers_down = registry
        .values()
        .filter(|w| w.status == WorkerStatus::Down)
        .count();

    Ok(Json(json!({
        "jobs_submitted": metrics.jobs_submitted,
        "tasks_completed": metrics.tasks_completed,
        "tasks_failed": metrics.tasks_failed,
        "workers_up": workers_up,
        "workers_down": workers_down
    })))
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

async fn load_dataset(source: &DatasetSource) -> Vec<i64> {
    match source {
        DatasetSource::Inline { data } => data.clone(),

        DatasetSource::File { path } => {
            let content = tokio::fs::read_to_string(path)
                .await
                .unwrap_or_else(|_| "".to_string());

            content
                .split(|c: char| c == ',' || c == ' ' || c == '\n')
                .filter_map(|s| s.trim().parse::<i64>().ok())
                .collect()
        }

        DatasetSource::Csv { path } => {
            let content = tokio::fs::read_to_string(path)
                .await
                .unwrap_or_else(|_| "".to_string());

            let mut result = Vec::<i64>::new();

            for line in content.lines() {
                for token in line.split(',') {
                    if let Ok(v) = token.trim().parse::<i64>() {
                        result.push(v);
                    }
                }
            }
            result
        }

        DatasetSource::Jsonl { path } => {
            // Cada línea es un JSON independiente
            // Ejemplo:
            //   {"value": 10}
            //   {"value": 20}

            let content = tokio::fs::read_to_string(path)
                .await
                .unwrap_or_else(|_| "".to_string());

            let mut result = Vec::<i64>::new();

            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                // Esperamos que cada línea tenga un campo "value"
                match serde_json::from_str::<serde_json::Value>(trimmed) {
                    Ok(json_line) => {
                        if let Some(v) = json_line.get("value").and_then(|x| x.as_i64()) {
                            result.push(v);
                        }
                    }
                    Err(_) => {
                        // ignoramos líneas malas
                        continue;
                    }
                }
            }

            result
        }
    }
}

/// Split input data into chunks for parallel task execution
fn split_into_chunks(input: Vec<i64>, chunk_size: usize) -> Vec<Vec<i64>> {
    if chunk_size == 0 {
        return vec![input];
    }

    input
        .chunks(chunk_size)
        .map(|chunk| chunk.to_vec())
        .collect()
}

/// Carga el dataset inicial de un job, ya sea desde `source` (nuevo)
/// o desde `input` (modo viejo).
fn load_input_data(payload: &JobSpec) -> Result<Vec<i64>, std::io::Error> {
    if let Some(source) = &payload.source {
    match source {
        DatasetSource::Inline { data } => Ok(data.clone()),

        DatasetSource::File { path } => {
            let content = std::fs::read_to_string(path)?;
            Ok(parse_plain_numbers(&content))
        }

        DatasetSource::Csv { path } => {
            let content = std::fs::read_to_string(path)?;
            Ok(parse_csv_numbers(&content))
        }

        DatasetSource::Jsonl { path } => {
            let content = std::fs::read_to_string(path)?;
            Ok(parse_jsonl_numbers(&content))
        }
    }
} else {
        // Modo viejo: el cliente envía `input` directo
        Ok(payload.input.clone())
    }
}

fn parse_plain_numbers(content: &str) -> Vec<i64> {
    content
        .split(|c: char| c == ',' || c == '\n' || c == ' ' || c == '\t')
        .filter_map(|s| s.trim().parse::<i64>().ok())
        .collect()
}

fn parse_csv_numbers(content: &str) -> Vec<i64> {
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

fn parse_jsonl_numbers(content: &str) -> Vec<i64> {
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


/// Submit a job for execution
async fn submit_job(
    State(state): State<AppState>,
    Json(payload): Json<JobSpec>,
) -> Result<Json<SubmitJobResponse>, StatusCode> {
    info!("Received job submission: {}", payload.name);

    let job_id = Uuid::new_v4().to_string();

    // 1) Construir stages (soportar modo legacy operation+param)
    let stages = if !payload.stages.is_empty() {
        payload.stages.clone()
    } else {
        vec![Stage {
            operation: payload.operation.clone(),
            param: payload.param,
        }]
    };

    let first_stage = &stages[0];

    // 2) Preparar chunks según el tipo de operación
    let mut chunks_normal: Vec<Vec<i64>> = Vec::new();
    let mut chunks_join: Vec<(Vec<i64>, Vec<i64>)> = Vec::new();
    let is_join = first_stage.operation == "join";

    if is_join {
        // ───── JOIN: particionar dos colecciones por clave ─────
        // izquierda: sacada de source o de input (legacy)
        let left_raw = if let Some(src) = &payload.source {
            load_dataset(src).await
        } else {
            payload.input.clone()
        };

        // derecha: si definiste source_right en JobSpec, úsalo; si no, por ahora vacío
        let right_raw = if let Some(src_right) = &payload.source_right {
            load_dataset(src_right).await
        } else {
            Vec::new()
        };

        // número de particiones (puedes usar workers luego, pero aquí usamos 4 fijo o 1 si nada)
        let num_partitions = 4usize;

        let mut left_parts = vec![Vec::<i64>::new(); num_partitions];
        let mut right_parts = vec![Vec::<i64>::new(); num_partitions];

        // Particionar izquierda: asumimos [k,v,k,v,...]
        for i in (0..left_raw.len()).step_by(2) {
            if i + 1 < left_raw.len() {
                let k = left_raw[i];
                let v = left_raw[i + 1];
                let pid = (k as usize % num_partitions) as usize;
                left_parts[pid].push(k);
                left_parts[pid].push(v);
            }
        }

        // Particionar derecha: asumimos [k,v,k,v,...]
        for i in (0..right_raw.len()).step_by(2) {
            if i + 1 < right_raw.len() {
                let k = right_raw[i];
                let v = right_raw[i + 1];
                let pid = (k as usize % num_partitions) as usize;
                right_parts[pid].push(k);
                right_parts[pid].push(v);
            }
        }

        for pid in 0..num_partitions {
            chunks_join.push((left_parts[pid].clone(), right_parts[pid].clone()));
        }
    } else {
        // ───── MODO NORMAL: map/filter/reduce etc ─────
        let input_data = match load_input_data(&payload) {
            Ok(data) => data,
            Err(e) => {
                error!(
                    "Error cargando datos de entrada para el job {}: {:?}",
                    payload.name, e
                );
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        };

        let chunk_size = if payload.chunk_size == 0 {
            100
        } else {
            payload.chunk_size
        };

        chunks_normal = split_into_chunks(input_data, chunk_size);
    }

    let total_tasks = if is_join {
        chunks_join.len()
    } else {
        chunks_normal.len()
    };

    info!(
        "Job {} split into {} task(s) (operation={})",
        job_id, total_tasks, first_stage.operation
    );

    // 3) Crear JobInfo y guardarlo
    let job_info = JobInfo {
        job_id: job_id.clone(),
        name: payload.name.clone(),
        total_tasks,
        completed_tasks: 0,
        failed_tasks: 0,
        status: "running".to_string(),
        outstanding_tasks: HashMap::new(),
        reassign_queue: Vec::new(),
        task_payloads: HashMap::new(),
        stages: stages.clone(),
        current_stage: 0,
        stage_output: Vec::new(),
        //JOIN
        stage_output_left: Vec::new(),
        stage_output_right: Vec::new(),
    };

    {
        let mut jobs = state.jobs.write().await;
        jobs.insert(job_id.clone(), job_info);
        info!(
            "Created job: {} with id: {} (total_tasks: {})",
            payload.name, job_id, total_tasks
        );
        {
        let mut m = state.metrics.write().await;
        m.jobs_submitted += 1;
    }
    }

    // 4) Obtener workers disponibles
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
        info!(
            "Dispatching job {} ({} tasks) to {} worker(s)",
            job_id,
            total_tasks,
            workers.len()
        );

        // Registrar tareas pendientes en JobInfo
        {
            let mut jobs = state.jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                if is_join {
                    for task_idx in 0..chunks_join.len() {
                        let task_id = format!("task-{}", task_idx);
                        let worker = workers.get(task_idx % workers.len()).unwrap();
                        job.outstanding_tasks
                            .insert(task_id, worker.id.clone());
                    }
                } else {
                    for task_idx in 0..chunks_normal.len() {
                        let task_id = format!("task-{}", task_idx);
                        let worker = workers.get(task_idx % workers.len()).unwrap();
                        job.outstanding_tasks
                            .insert(task_id, worker.id.clone());
                    }
                }
            }
        }

        // 5) Despachar tareas
        let client = Client::new();
        let state_clone_for_payload = state.clone();
        let job_id_for_payload = job_id.clone();

        if is_join {
            for (task_idx, (left_chunk, right_chunk)) in chunks_join.into_iter().enumerate() {
                let worker = workers.get(task_idx % workers.len()).unwrap().clone();
                let task_id = format!("task-{}", task_idx);

                let task_assignment = TaskAssignment {
                    job_id: job_id.clone(),
                    task_id: task_id.clone(),
                    operation: "join".to_string(),
                    input: Vec::new(),
                    left: left_chunk.clone(),
                    right: right_chunk.clone(),
                    param: None,
                    stage_id: 0,
                    reassign_attempt: 0,
                };

                // Guardar payload para reintentos (aunque input esté vacío en join)
                {
                    let mut jobs = state_clone_for_payload.jobs.write().await;
                    if let Some(job) = jobs.get_mut(&job_id_for_payload) {
                        job.task_payloads.insert(
                            task_id.clone(),
                            (
                                task_assignment.input.clone(),
                                task_assignment.operation.clone(),
                                task_assignment.param,
                            ),
                        );
                    }
                }

                let worker_url = format!(
                    "http://{}:{}/api/v1/tasks/execute",
                    worker.host, worker.port
                );

                let client_clone = client.clone();
                let task_clone = task_assignment.clone();
                let state_clone = state.clone();
                let worker_url_clone = worker_url.clone();
                let job_id_clone = job_id.clone();
                let task_id_clone = task_id.clone();
                let worker_id_clone = worker.id.clone();

                tokio::spawn(async move {
                    let max_retries = 3;
                    let mut attempt = 0;
                    let mut backoff = Duration::from_secs(1);
                    let mut success = false;

                    while attempt < max_retries {
                        attempt += 1;
                        match client_clone.post(&worker_url_clone).json(&task_clone).send().await {
                            Ok(resp) if resp.status().is_success() => {
                                info!(
                                    "Task {} dispatched successfully to worker {} (attempt {})",
                                    task_clone.task_id, worker_url_clone, attempt
                                );
                                success = true;
                                break;
                            }
                            Ok(resp) => {
                                error!(
                                    "Failed to dispatch task {} to worker {}: {} (attempt {})",
                                    task_clone.task_id,
                                    worker_url_clone,
                                    resp.status(),
                                    attempt
                                );
                            }
                            Err(e) => {
                                error!(
                                    "Error dispatching task {} to worker {}: {:?} (attempt {})",
                                    task_clone.task_id, worker_url_clone, e, attempt
                                );
                            }
                        }

                        tokio::time::sleep(backoff).await;
                        backoff = backoff + backoff;
                    }

                    if !success {
                        let mut jobs = state_clone.jobs.write().await;
                        if let Some(job) = jobs.get_mut(&job_id_clone) {
                            job.outstanding_tasks.remove(&task_id_clone);
                            job.reassign_queue.push((
                                task_id_clone.clone(),
                                task_clone.input.clone(),
                                task_clone.operation.clone(),
                                task_clone.param,
                            ));
                            info!(
                                "Task {} failed dispatch from worker {}, added to reassign queue for job {}",
                                task_id_clone, worker_id_clone, job_id_clone
                            );
                        }
                    }
                });
            }
        } else {
            for (task_idx, chunk) in chunks_normal.into_iter().enumerate() {
                let worker = workers.get(task_idx % workers.len()).unwrap().clone();
                let task_id = format!("task-{}", task_idx);

                let stage0 = &stages[0];
                let task_assignment = TaskAssignment {
                    job_id: job_id.clone(),
                    task_id: task_id.clone(),
                    operation: stage0.operation.clone(),
                    input: chunk.clone(),
                    left: Vec::new(),
                    right: Vec::new(),
                    param: stage0.param,
                    stage_id: 0,
                    reassign_attempt: 0,
                };

                {
                    let mut jobs = state_clone_for_payload.jobs.write().await;
                    if let Some(job) = jobs.get_mut(&job_id_for_payload) {
                        job.task_payloads.insert(
                            task_id.clone(),
                            (
                                task_assignment.input.clone(),
                                task_assignment.operation.clone(),
                                task_assignment.param,
                            ),
                        );
                    }
                }

                let worker_url = format!(
                    "http://{}:{}/api/v1/tasks/execute",
                    worker.host, worker.port
                );

                let client_clone = client.clone();
                let task_clone = task_assignment.clone();
                let state_clone = state.clone();
                let worker_url_clone = worker_url.clone();
                let job_id_clone = job_id.clone();
                let task_id_clone = task_id.clone();
                let worker_id_clone = worker.id.clone();

                tokio::spawn(async move {
                    let max_retries = 3;
                    let mut attempt = 0;
                    let mut backoff = Duration::from_secs(1);
                    let mut success = false;

                    while attempt < max_retries {
                        attempt += 1;
                        match client_clone.post(&worker_url_clone).json(&task_clone).send().await {
                            Ok(resp) if resp.status().is_success() => {
                                info!(
                                    "Task {} dispatched successfully to worker {} (attempt {})",
                                    task_clone.task_id, worker_url_clone, attempt
                                );
                                success = true;
                                break;
                            }
                            Ok(resp) => {
                                error!(
                                    "Failed to dispatch task {} to worker {}: {} (attempt {})",
                                    task_clone.task_id,
                                    worker_url_clone,
                                    resp.status(),
                                    attempt
                                );
                            }
                            Err(e) => {
                                error!(
                                    "Error dispatching task {} to worker {}: {:?} (attempt {})",
                                    task_clone.task_id, worker_url_clone, e, attempt
                                );
                            }
                        }

                        tokio::time::sleep(backoff).await;
                        backoff = backoff + backoff;
                    }

                    if !success {
                        let mut jobs = state_clone.jobs.write().await;
                        if let Some(job) = jobs.get_mut(&job_id_clone) {
                            job.outstanding_tasks.remove(&task_id_clone);
                            job.reassign_queue.push((
                                task_id_clone.clone(),
                                task_clone.input.clone(),
                                task_clone.operation.clone(),
                                task_clone.param,
                            ));
                            info!(
                                "Task {} failed dispatch from worker {}, added to reassign queue for job {}",
                                task_id_clone, worker_id_clone, job_id_clone
                            );
                        }
                    }
                });
            }
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
        "Received task result for job: {}, task: {} (stage: {})",
        job_id, payload.task_id, payload.task_id.split('-').nth(1).unwrap_or("0")
    );

    let mut jobs = state.jobs.write().await;

    let job = jobs.get_mut(&job_id).ok_or_else(|| {
        warn!("Job not found: {}", job_id);
        StatusCode::NOT_FOUND
    })?;

    // Remove from outstanding tasks
    job.outstanding_tasks.remove(&payload.task_id);

    // Update job progress
    if payload.error.is_some() {
        // Task reported an error
        job.failed_tasks += 1;
        info!("Job {} task {} reported error: {:?} (failed_tasks={})", job_id, payload.task_id, payload.error, job.failed_tasks);
    } else {
        if job.completed_tasks < job.total_tasks {
            job.completed_tasks += 1;
            info!("Job {} progress: {}/{}", job_id, job.completed_tasks, job.total_tasks);
            // Accumulate output from this stage
            // Accumulation depends on stage type
            let op = job.stages[job.current_stage].operation.as_str();

            match op {
                "join" => {
                    // JOIN outputs are full (k,l,r) triples already merged in worker
                    // store them in stage_output
                    job.stage_output.extend(payload.output.clone());
                }
                _ => {
                    // Regular map/filter/reduce
                    job.stage_output.extend(payload.output.clone());
                }
            }
        }
        {
            let mut m = state.metrics.write().await;
            if payload.error.is_some() {
                m.tasks_failed += 1;
            } else {
                // solo contamos si efectivamente incrementamos completed_tasks
                m.tasks_completed += 1;
            }
        }
    }

    // If all tasks of current stage finished (either succeeded or failed)
    if job.completed_tasks + job.failed_tasks == job.total_tasks {
        if job.failed_tasks > 0 {
            // Stage failed
            job.status = "failed".to_string();
            info!("Job {} stage {} completed with failures (failed_tasks={})", job_id, job.current_stage, job.failed_tasks);
        } else if job.current_stage + 1 < job.stages.len() {
            // Stage completed successfully; mark ready for next stage (do not advance current_stage here)
            info!("Job {} stage {} completed successfully, ready for next stage", job_id, job.current_stage);
            job.completed_tasks = 0;
            job.failed_tasks = 0;
            // The next stage will be dispatched by a manual trigger or an automated scheduler
            job.status = "stage_complete".to_string();
        } else {
            // All stages completed
            job.status = "completed".to_string();
            info!("Job {} all stages completed successfully", job_id);
        }
    }

    Ok(Json(json!({
        "status": "ack",
        "message": "Task result received"
    })))
}

/// Trigger execution of the next stage in a multi-stage job
async fn next_stage(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Delegate to shared start_stage function so it can be reused by the monitor
    start_stage(state, job_id, Client::new()).await;

    Ok(Json(json!({
        "status": "ok",
        "message": "next_stage triggered"
    })))
}

/// Start (dispatch) a specific stage for a job. Re-usable from endpoint and monitor.
/// Start (dispatch) a specific stage for a job. Re-usable from endpoint and monitor.
async fn start_stage(state: AppState, job_id: String, client: Client) {
    info!("Starting stage dispatch for job: {}", job_id);

    // 1) Lock job metadata
    let mut jobs = state.jobs.write().await;
    let job = match jobs.get_mut(&job_id) {
        Some(j) => j,
        None => {
            warn!("start_stage: Job not found: {}", job_id);
            return;
        }
    };

    // 2) Check that a next stage exists
    if job.current_stage >= job.stages.len() - 1 {
        warn!("start_stage: Job {} has no next stage", job_id);
        return;
    }

    // 3) Extract metadata we need BEFORE releasing lock
    let next_stage_index = job.current_stage + 1;
    let stage = job.stages[next_stage_index].clone();
    let previous_output = job.stage_output.clone();
    let job_id_clone = job.job_id.clone();

    // 4) Collect available workers (needed to know num_partitions)
    let workers = {
        let registry = state.registry.read().await;
        registry
            .values()
            .filter(|w| w.status == WorkerStatus::Up)
            .cloned()
            .collect::<Vec<_>>()
    };

    if workers.is_empty() {
        warn!(
            "start_stage: No available workers for next stage of job {}",
            job_id
        );
        return;
    }

    // Number of partitions = number of workers
    let num_partitions = workers.len().max(1);

    // 5) Prepare SHUFFLE partitions from previous_output
    // We assume [k1,v1,k2,v2,...] coming from previous stage
    let mut partitions = vec![Vec::<i64>::new(); num_partitions];

    for i in (0..previous_output.len()).step_by(2) {
        if i + 1 < previous_output.len() {
            let key = previous_output[i];
            let val = previous_output[i + 1];

            let pid = (key.abs() as usize) % num_partitions;
            partitions[pid].push(key);
            partitions[pid].push(val);
        }
    }

    let chunks = partitions;
    let total_tasks = chunks.len();

    info!(
        "start_stage: Job {} preparing SHUFFLE for stage {} → {} partitions (op={})",
        job_id,
        next_stage_index,
        total_tasks,
        stage.operation
    );

    // 6) Update job metadata for new stage
    job.current_stage = next_stage_index;
    job.total_tasks = total_tasks;
    job.completed_tasks = 0;
    job.failed_tasks = 0;
    job.outstanding_tasks.clear();
    job.reassign_queue.clear();
    job.task_payloads.clear();
    job.stage_output = Vec::new();
    job.stage_output_left = Vec::new();
    job.stage_output_right = Vec::new();

    // 7) Release lock BEFORE assigning tasks (very important)
    drop(jobs);

    // 8) Dispatch tasks
    for (task_idx, chunk) in chunks.into_iter().enumerate() {
        let worker = workers.get(task_idx % workers.len()).unwrap().clone();
        let task_id = format!("task-{}", task_idx);

        // Select operation mode
        let task_assignment = if stage.operation == "join" {
            // JOIN: dividimos el chunk en dos mitades (left/right) de forma trivial
            let mid = chunk.len() / 2;
            let (left_raw, right_raw) = chunk.split_at(mid);

            TaskAssignment {
                job_id: job_id.clone(),
                task_id: task_id.clone(),
                operation: "join".to_string(),
                input: Vec::new(),
                left: left_raw.to_vec(),
                right: right_raw.to_vec(),
                param: None,
                stage_id: next_stage_index,
                reassign_attempt: 0,
            }
        } else {
            // Normal map/filter/reduce
            TaskAssignment {
                job_id: job_id.clone(),
                task_id: task_id.clone(),
                operation: stage.operation.clone(),
                input: chunk.clone(),
                left: Vec::new(),
                right: Vec::new(),
                param: stage.param,
                stage_id: next_stage_index,
                reassign_attempt: 0,
            }
        };

        // 9) Store payload for retry + mark as outstanding
        {
            let mut jobs = state.jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id_clone) {
                job.task_payloads.insert(
                    task_id.clone(),
                    (
                        task_assignment.input.clone(),
                        task_assignment.operation.clone(),
                        task_assignment.param,
                    ),
                );
                job.outstanding_tasks.insert(task_id.clone(), worker.id.clone());
            }
        }

        // 10) Dispatch task asynchronously with retries
        let worker_url = format!(
            "http://{}:{}/api/v1/tasks/execute",
            worker.host, worker.port
        );

        let client_clone = client.clone();
        let task_clone = task_assignment.clone();
        let state_clone = state.clone();
        let worker_url_clone = worker_url.clone();
        let task_id_clone = task_id.clone();
        let job_id_dispatch = job_id.clone();
        let worker_id_clone = worker.id.clone();

        tokio::spawn(async move {
            let max_retries = 3;
            let mut attempt = 0;
            let mut backoff = Duration::from_secs(1);
            let mut success = false;

            while attempt < max_retries {
                attempt += 1;
                match client_clone
                    .post(&worker_url_clone)
                    .json(&task_clone)
                    .send()
                    .await
                {
                    Ok(resp) if resp.status().is_success() => {
                        info!(
                            "Task {} dispatched successfully to worker {} (attempt {})",
                            task_clone.task_id, worker_url_clone, attempt
                        );
                        success = true;
                        break;
                    }
                    Ok(resp) => {
                        error!(
                            "Failed dispatch task {} → {}: {} (attempt {})",
                            task_clone.task_id,
                            worker_url_clone,
                            resp.status(),
                            attempt
                        );
                    }
                    Err(e) => {
                        error!(
                            "Error dispatching task {} → {}: {:?} (attempt {})",
                            task_clone.task_id,
                            worker_url_clone,
                            e,
                            attempt
                        );
                    }
                }

                tokio::time::sleep(backoff).await;
                backoff *= 2;
            }

            if !success {
                let mut jobs = state_clone.jobs.write().await;
                if let Some(job) = jobs.get_mut(&job_id_dispatch) {
                    job.outstanding_tasks.remove(&task_id_clone);
                    job.reassign_queue.push((
                        task_id_clone.clone(),
                        task_clone.input.clone(),
                        task_clone.operation.clone(),
                        task_clone.param,
                    ));
                }
            }
        });
    }
}




/// Background task to monitor workers and mark them as DOWN if they exceed timeout
async fn monitor_workers(
    registry: Arc<RwLock<HashMap<String, WorkerInfo>>>,
    jobs: Arc<RwLock<HashMap<String, JobInfo>>>,
    metrics: Arc<RwLock<Metrics>>,
    client: Client,
) {
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

            // Reassign tasks from DOWN workers
            let mut jobs_write = jobs.write().await;

            for job in jobs_write.values_mut() {
                // Collect tasks from DOWN workers
                let mut tasks_to_reassign = Vec::new();
                for (task_id, assigned_worker) in job.outstanding_tasks.iter() {
                    if marked_down.contains(assigned_worker) {
                        tasks_to_reassign.push(task_id.clone());
                    }
                }

                // Move to reassign queue, using stored payloads if available
                for task_id in tasks_to_reassign {
                    if let Some(worker_id) = job.outstanding_tasks.remove(&task_id) {
                        if let Some((input, operation, param)) = job.task_payloads.remove(&task_id) {
                            job.reassign_queue.push((task_id.clone(), input.clone(), operation.clone(), param));
                        } else {
                            // Fallback placeholder (shouldn't usually happen)
                            job.reassign_queue.push((task_id.clone(), Vec::new(), "".to_string(), None));
                        }
                        info!(
                            "Task {} from DOWN worker {} moved to reassign queue for job {}",
                            task_id, worker_id, job.job_id
                        );
                    }
                }
            }

            // Attempt to dispatch tasks from reassign queues to available UP workers
            // For each job, try to assign queued tasks to currently UP workers
            for job in jobs_write.values_mut() {
                if job.reassign_queue.is_empty() {
                    continue;
                }

                // Build current list of UP workers (host/port + id)
                let up_workers: Vec<WorkerInfo> = registry
                    .read()
                    .await
                    .values()
                    .filter(|w| w.status == WorkerStatus::Up)
                    .cloned()
                    .collect();

                if up_workers.is_empty() {
                    // No available workers now; leave tasks in queue
                    continue;
                }

                // Dispatch queued tasks round-robin to UP workers
                let mut rr_idx = 0usize;
                // We will iterate while there are tasks in the queue and workers available
                let mut i = 0;
                while i < job.reassign_queue.len() && !up_workers.is_empty() {
                    // Pop the task from the front
                    let (task_id, input, operation, param) = job.reassign_queue.remove(0);

                    let worker = up_workers.get(rr_idx % up_workers.len()).unwrap().clone();
                    rr_idx = rr_idx.wrapping_add(1);

                    let task_assignment = TaskAssignment {
                        job_id: job.job_id.clone(),
                        task_id: task_id.clone(),
                        operation: operation.clone(),
                        param,
                        input: input.clone(),
                        left: Vec::new(),
                        right: Vec::new(),
                        reassign_attempt: 1,
                        stage_id: job.current_stage,
                    };

                    let worker_url = format!("http://{}:{}/api/v1/tasks/execute", worker.host, worker.port);
                    let client_clone = client.clone();
                    // Try dispatch with retries, spawn a task to avoid blocking the monitor loop
                    let task_id_clone = task_id.clone();
                    let worker_id_clone = worker.id.clone();

                    tokio::spawn(async move {
                        let max_retries = 3;
                        let mut attempt = 0;
                        let mut backoff = Duration::from_secs(1);
                        let mut dispatched = false;

                        while attempt < max_retries {
                            attempt += 1;
                            match client_clone.post(&worker_url).json(&task_assignment).send().await {
                                Ok(resp) if resp.status().is_success() => {
                                    info!("Reassigned task {} dispatched to worker {} (attempt {})", task_id_clone, worker_id_clone, attempt);
                                    dispatched = true;
                                    break;
                                }
                                Ok(resp) => {
                                    error!("Failed to dispatch reassigned task {} to {}: {} (attempt {})", task_id_clone, worker_url, resp.status(), attempt);
                                }
                                Err(e) => {
                                    error!("Error dispatching reassigned task {} to {}: {:?} (attempt {})", task_id_clone, worker_url, e, attempt);
                                }
                            }

                            tokio::time::sleep(backoff).await;
                            backoff = backoff + backoff;
                        }

                        if !dispatched {
                            error!("Reassigned task {} could not be dispatched after retries; pushing back to queue", task_id_clone);
                            // The monitor loop cannot mutate job state here; it will be left untracked and should be pushed back by monitor on next tick
                        }
                    });

                    // Update outstanding mapping immediately (we assume dispatch will be attempted)
                    job.outstanding_tasks.insert(task_id.clone(), worker.id.clone());
                    // do not increment i because we removed the front element
                }
            }

            // Collect jobs that are ready to advance to the next stage (stage_complete and no outstanding tasks)
            let mut jobs_ready: Vec<String> = Vec::new();
            for (job_id, job) in jobs_write.iter_mut() {
                if job.status == "stage_complete" && job.outstanding_tasks.is_empty() {
                    // Ensure there is a next stage
                    if job.current_stage + 1 < job.stages.len() {
                        jobs_ready.push(job_id.clone());
                        // mark as running to avoid duplicate triggers while we dispatch
                        job.status = "running".to_string();
                    } else {
                        // no next stage, finalize
                        job.status = "completed".to_string();
                        info!("Job {} had stage_complete but no further stages; marking completed", job_id);
                    }
                }
            }

            // Spawn start_stage for each ready job (release jobs_write lock before dispatch)
            for job_id in jobs_ready {
                let app_state = AppState {
                    registry: registry.clone(),
                    jobs: jobs.clone(),
                    metrics: metrics.clone(),
                };
                let client_clone = client.clone();
                tokio::spawn(async move {
                    start_stage(app_state, job_id, client_clone).await;
                });
            }
        }
    }
}
