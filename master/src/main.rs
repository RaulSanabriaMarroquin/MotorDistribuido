mod persistence;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use common::{
    HeartbeatRequest, HeartbeatResponse, JobMetrics, JobResultsResponse, JobStatus,
    JobStatusResponse, JobSubmitRequest, JobSubmitResponse, Operator, RegisterRequest,
    RegisterResponse, TaskAssignment, TaskMetrics, TaskRequest, TaskResponse, TaskStatus,
    TaskStatusUpdate, WorkerListItem, WorkerStatus, WorkersListResponse, MESSAGE_VERSION,
};
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::sync::RwLock;
use tokio::signal;
use tracing::{info, warn, error};
use rusqlite::Connection;
use std::sync::Mutex;

/// Worker information stored in registry
#[derive(Debug, Clone)]
struct WorkerInfo {
    id: String,
    host: String,
    port: u16,
    last_heartbeat: SystemTime,
    status: WorkerStatus,
    active_tasks: u32,
    total_tasks_completed: u64,
}

/// Job information
#[derive(Debug, Clone)]
struct JobInfo {
    id: String,
    name: String,
    status: JobStatus,
    created_at: SystemTime,
    started_at: Option<SystemTime>,
    completed_at: Option<SystemTime>,
    progress: f64,
    metrics: JobMetrics,
    output_paths: Vec<String>,
}

/// Task information
#[derive(Debug, Clone)]
struct TaskInfo {
    task_id: String,
    job_id: String,
    attempt_id: u32,
    stage_id: String,
    operator: Operator,
    status: TaskStatus,
    assigned_worker: Option<String>,
    input_paths: Vec<String>,
    output_path: String,
    partition: u32,
    created_at: SystemTime,
    started_at: Option<SystemTime>,
    completed_at: Option<SystemTime>,
    error: Option<String>,
    metrics: TaskMetrics,
}

/// Shared application state
#[derive(Clone)]
struct AppState {
    registry: Arc<RwLock<HashMap<String, WorkerInfo>>>,
    jobs: Arc<RwLock<HashMap<String, JobInfo>>>,
    tasks: Arc<RwLock<HashMap<String, TaskInfo>>>,
    task_queue: Arc<RwLock<VecDeque<String>>>, // Queue of task IDs
    db: Arc<Mutex<Connection>>, // Database connection (Mutex for sync access)
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

    // Initialize database
    let db = persistence::init_db()
        .map_err(|e| format!("Failed to initialize database: {}", e))?;
    let db = Arc::new(Mutex::new(db));

    // Load persisted state
    let (loaded_jobs, loaded_tasks, loaded_queue) = {
        let db_guard = db.lock().unwrap();
        let jobs_data = persistence::load_jobs(&db_guard)
            .unwrap_or_else(|e| {
                warn!("Failed to load jobs: {}", e);
                Vec::new()
            });
        
        let mut jobs = HashMap::new();
        let mut tasks = HashMap::new();
        let mut task_queue = VecDeque::new();

        for (id, name, status, created_at, started_at, completed_at, progress, metrics, output_paths) in jobs_data {
            jobs.insert(id.clone(), JobInfo {
                id: id.clone(),
                name,
                status,
                created_at,
                started_at,
                completed_at,
                progress,
                metrics,
                output_paths,
            });

            // Load tasks for this job
            if let Ok(tasks_data) = persistence::load_tasks(&db_guard, &id) {
                for (task_id, job_id, attempt_id, stage_id, operator, status, assigned_worker, input_paths, output_path, partition, created_at, started_at, completed_at, error, metrics) in tasks_data {
                    let task_id_clone = task_id.clone();
                    tasks.insert(task_id_clone.clone(), TaskInfo {
                        task_id: task_id_clone.clone(),
                        job_id,
                        attempt_id,
                        stage_id,
                        operator,
                        status,
                        assigned_worker,
                        input_paths,
                        output_path,
                        partition,
                        created_at,
                        started_at,
                        completed_at,
                        error,
                        metrics,
                    });
                    
                    // Re-queue pending/assigned tasks
                    if matches!(status, TaskStatus::Pending | TaskStatus::Assigned) {
                        task_queue.push_back(task_id_clone);
                    }
                }
            }
        }

        info!("Loaded {} jobs and {} tasks from database", jobs.len(), tasks.len());
        (jobs, tasks, task_queue)
    };

    // Create shared state with loaded data
    let state = AppState {
        registry: Arc::new(RwLock::new(HashMap::new())),
        jobs: Arc::new(RwLock::new(loaded_jobs)),
        tasks: Arc::new(RwLock::new(loaded_tasks)),
        task_queue: Arc::new(RwLock::new(loaded_queue)),
        db,
    };

    // Spawn background monitoring task
    let registry_monitor = state.registry.clone();
    let tasks_monitor = state.tasks.clone();
    let task_queue_monitor = state.task_queue.clone();
    tokio::spawn(async move {
        monitor_workers(registry_monitor, tasks_monitor, task_queue_monitor).await;
    });

    // Spawn task scheduler
    let scheduler_state = state.clone();
    tokio::spawn(async move {
        task_scheduler(scheduler_state).await;
    });

    // Build router
    let app = Router::new()
        .route("/api/v1/workers/register", post(register_worker))
        .route("/api/v1/workers/:id/heartbeat", post(heartbeat))
        .route("/api/v1/workers", get(list_workers))
        .route("/api/v1/jobs", post(submit_job))
        .route("/api/v1/jobs/:id", get(get_job_status))
        .route("/api/v1/jobs/:id/results", get(get_job_results))
        .route("/api/v1/tasks/:task_id/status", post(update_task_status))
        .with_state(state);

    // Start server with graceful shutdown
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    info!("Master node listening on 127.0.0.1:8080");

    // Setup graceful shutdown
    let shutdown_signal = async {
        #[cfg(unix)]
        {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("Failed to install SIGTERM handler")
                .recv()
                .await;
        }
        #[cfg(windows)]
        {
            signal::windows::ctrl_c()
                .expect("Failed to install Ctrl+C handler")
                .recv()
                .await;
        }
        #[cfg(not(any(unix, windows)))]
        {
            signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
        }
    };

    let server = axum::serve(listener, app);
    let graceful = server.with_graceful_shutdown(shutdown_signal);

    info!("Press Ctrl+C to shutdown gracefully...");
    graceful.await?;

    info!("Master node shutting down gracefully...");
    Ok(())
}

/// Register a new worker
async fn register_worker(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, StatusCode> {
    info!("Received registration request from {}:{}", payload.host, payload.port);

    // Validate version if provided
    if let Some(version) = &payload.version {
        if version != MESSAGE_VERSION {
            warn!("Version mismatch: expected {}, got {}", MESSAGE_VERSION, version);
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
        total_tasks_completed: 0,
    };

    {
        let mut registry = state.registry.write().await;
        registry.insert(worker_id.clone(), worker_info);
        info!("Registered worker: {} (total workers: {})", worker_id, registry.len());
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
            warn!("Version mismatch: expected {}, got {}", MESSAGE_VERSION, version);
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

/// Background task to monitor workers and mark them as DOWN if they exceed timeout
async fn monitor_workers(
    registry: Arc<RwLock<HashMap<String, WorkerInfo>>>,
    tasks: Arc<RwLock<HashMap<String, TaskInfo>>>,
    task_queue: Arc<RwLock<VecDeque<String>>>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(MONITOR_INTERVAL_SECS));

    loop {
        interval.tick().await;

        let now = SystemTime::now();
        let timeout = Duration::from_secs(HEARTBEAT_TIMEOUT_SECS);

        let mut registry_write = registry.write().await;
        let mut tasks_write = tasks.write().await;
        let mut task_queue_write = task_queue.write().await;
        let mut marked_down = Vec::new();

        for (worker_id, worker) in registry_write.iter_mut() {
            if let Ok(elapsed) = now.duration_since(worker.last_heartbeat) {
                if elapsed > timeout && worker.status == WorkerStatus::Up {
                    worker.status = WorkerStatus::Down;
                    marked_down.push(worker_id.clone());
                    
                    // Replanify tasks assigned to this worker
                    let mut replanified = 0;
                    for task in tasks_write.values_mut() {
                        if task.assigned_worker.as_ref() == Some(worker_id)
                            && (task.status == TaskStatus::Assigned || task.status == TaskStatus::Running)
                        {
                            // Reset task for replanification
                            task.status = TaskStatus::Pending;
                            task.assigned_worker = None;
                            task_queue_write.push_back(task.task_id.clone());
                            replanified += 1;
                        }
                    }
                    
                    if replanified > 0 {
                        info!(
                            worker_id = %worker_id,
                            tasks = replanified,
                            "Replanified tasks from failed worker"
                        );
                    }
                }
            }
        }

        drop(registry_write);
        drop(tasks_write);
        drop(task_queue_write);

        for worker_id in &marked_down {
            warn!("Worker {} exceeded heartbeat timeout, marked as DOWN", worker_id);
        }

        if !marked_down.is_empty() {
            info!("Marked {} worker(s) as DOWN", marked_down.len());
        }
    }
}

// ============================================================================
// Job Management
// ============================================================================

/// Submit a new job
async fn submit_job(
    State(state): State<AppState>,
    Json(payload): Json<JobSubmitRequest>,
) -> Result<Json<JobSubmitResponse>, StatusCode> {
    info!("Received job submission: {}", payload.name);

    // Generate unique job ID
    let job_id = {
        let jobs = state.jobs.read().await;
        let mut id_num = 1;
        loop {
            let id = format!("job{:06}", id_num);
            if !jobs.contains_key(&id) {
                break id;
            }
            id_num += 1;
        }
    };

    // Create job info
    let job_info = JobInfo {
        id: job_id.clone(),
        name: payload.name.clone(),
        status: JobStatus::Accepted,
        created_at: SystemTime::now(),
        started_at: None,
        completed_at: None,
        progress: 0.0,
        metrics: JobMetrics::default(),
        output_paths: Vec::new(),
    };

    {
        let mut jobs = state.jobs.write().await;
        jobs.insert(job_id.clone(), job_info.clone());
        
        // Persist job to database
        let db = state.db.lock().unwrap();
        if let Err(e) = persistence::save_job(
            &db,
            &job_id,
            &job_info.name,
            job_info.status,
            job_info.created_at,
            job_info.started_at,
            job_info.completed_at,
            job_info.progress,
            &job_info.metrics,
            &job_info.output_paths,
        ) {
            error!("Failed to persist job {}: {}", job_id, e);
        }
    }

    // Create tasks from DAG with proper partition handling
    let mut task_ids = Vec::new();
    {
        let mut tasks = state.tasks.write().await;
        let mut task_queue = state.task_queue.write().await;

        // Build dependency graph from edges
        let mut node_dependencies: HashMap<String, Vec<String>> = HashMap::new();
        for edge in &payload.dag.edges {
            node_dependencies
                .entry(edge.1.clone()) // target node
                .or_insert_with(Vec::new)
                .push(edge.0.clone()); // source node
        }

        // Create tasks for each node, respecting partitions and parallelism
        for (idx, node) in payload.dag.nodes.iter().enumerate() {
            let stage_id = format!("stage-{}", idx);
            
            // Determine number of partitions for this stage
            let num_partitions = match &node.operator {
                common::Operator::ReadCsv { partitions, .. } => *partitions,
                common::Operator::ReadJsonl { partitions, .. } => *partitions,
                _ => {
                    // For non-read operators, use parallelism from job or inherit from previous stage
                    // For simplicity, use job parallelism
                    payload.parallelism
                }
            };

            // Create one task per partition
            for partition in 0..num_partitions {
                let task_id = format!("{}-{}-p{:03}", job_id, node.id, partition);
                
                // Determine input paths based on dependencies
                let mut input_paths = Vec::new();
                if let Some(deps) = node_dependencies.get(&node.id) {
                    // For each dependency, find its output paths
                    for dep_id in deps {
                        // Find the dependency node
                        if let Some(dep_node) = payload.dag.nodes.iter().find(|n| n.id == *dep_id) {
                            // Get partition count from dependency
                            let dep_partitions = match &dep_node.operator {
                                common::Operator::ReadCsv { partitions, .. } => *partitions,
                                common::Operator::ReadJsonl { partitions, .. } => *partitions,
                                _ => payload.parallelism,
                            };
                            
                            // For shuffle operations, we might need all partitions
                            // For now, use the same partition number
                            let dep_partition = if dep_partitions > 0 {
                                partition % dep_partitions
                            } else {
                                0
                            };
                            
                            // Construct input path from dependency
                            let dep_task_id = format!("{}-{}-p{:03}", job_id, dep_id, dep_partition);
                            input_paths.push(format!("output/{}/{}", job_id, dep_task_id));
                        }
                    }
                } else {
                    // Root node (read operation) - use original path
                    match &node.operator {
                        common::Operator::ReadCsv { path, .. } => {
                            // For partitioned reads, we could split the file
                            // For now, use the same path for all partitions
                            input_paths.push(path.clone());
                        }
                        common::Operator::ReadJsonl { path, .. } => {
                            input_paths.push(path.clone());
                        }
                        _ => {}
                    }
                }

                let task_info = TaskInfo {
                    task_id: task_id.clone(),
                    job_id: job_id.clone(),
                    attempt_id: 0,
                    stage_id: stage_id.clone(),
                    operator: node.operator.clone(),
                    status: TaskStatus::Pending,
                    assigned_worker: None,
                    input_paths: input_paths.clone(),
                    output_path: format!("output/{}/{}-attempt{}", job_id, task_id, 0),
                    partition,
                    created_at: SystemTime::now(),
                    started_at: None,
                    completed_at: None,
                    error: None,
                    metrics: TaskMetrics::default(),
                };

                let task_info_clone = task_info.clone();
                tasks.insert(task_id.clone(), task_info);
                task_queue.push_back(task_id.clone());
                task_ids.push(task_id.clone());
                
                // Persist task to database
                let db = state.db.lock().unwrap();
                if let Err(e) = persistence::save_task(
                    &db,
                    &task_id,
                    &task_info_clone.job_id,
                    task_info_clone.attempt_id,
                    &task_info_clone.stage_id,
                    &task_info_clone.operator,
                    task_info_clone.status,
                    task_info_clone.assigned_worker.as_deref(),
                    &task_info_clone.input_paths,
                    &task_info_clone.output_path,
                    task_info_clone.partition,
                    task_info_clone.created_at,
                    task_info_clone.started_at,
                    task_info_clone.completed_at,
                    task_info_clone.error.as_deref(),
                    &task_info_clone.metrics,
                ) {
                    error!("Failed to persist task {}: {}", task_id, e);
                }
            }
        }
    }

    info!("Created job {} with {} tasks", job_id, task_ids.len());

    Ok(Json(JobSubmitResponse {
        version: MESSAGE_VERSION.to_string(),
        job_id,
        message: "Job submitted successfully".to_string(),
    }))
}

/// Get job status
async fn get_job_status(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> Result<Json<JobStatusResponse>, StatusCode> {
    let jobs = state.jobs.read().await;
    let tasks = state.tasks.read().await;

    let job = jobs.get(&job_id).ok_or(StatusCode::NOT_FOUND)?;

    // Calculate progress from tasks
    let job_tasks: Vec<_> = tasks
        .values()
        .filter(|t| t.job_id == job_id)
        .collect();

    let total = job_tasks.len();
    let completed = job_tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Completed)
        .count();
    let failed = job_tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Failed)
        .count();

    let progress = if total > 0 {
        (completed as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    // Update job status based on tasks
    let mut job_status = job.status;
    if failed > 0 && completed == 0 {
        job_status = JobStatus::Failed;
    } else if completed == total && total > 0 {
        job_status = JobStatus::Succeeded;
    } else if completed > 0 || failed > 0 {
        job_status = JobStatus::Running;
    }

    let mut metrics = job.metrics.clone();
    metrics.stages_completed = completed as u32;
    metrics.stages_total = total as u32;
    metrics.failures = failed as u32;

    Ok(Json(JobStatusResponse {
        version: MESSAGE_VERSION.to_string(),
        job_id: job_id.clone(),
        status: job_status.as_str().to_string(),
        progress,
        metrics,
    }))
}

/// Get job results
async fn get_job_results(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> Result<Json<JobResultsResponse>, StatusCode> {
    let jobs = state.jobs.read().await;
    let job = jobs.get(&job_id).ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(JobResultsResponse {
        version: MESSAGE_VERSION.to_string(),
        job_id: job_id.clone(),
        output_paths: job.output_paths.clone(),
    }))
}

/// Update task status (called by worker)
async fn update_task_status(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
    Json(payload): Json<TaskStatusUpdate>,
) -> Result<Json<TaskResponse>, StatusCode> {
    let mut tasks = state.tasks.write().await;
    let mut jobs = state.jobs.write().await;
    let mut registry = state.registry.write().await;

    let task = tasks.get_mut(&task_id).ok_or(StatusCode::NOT_FOUND)?;

    // Update task status
    task.status = match payload.status.as_str() {
        "COMPLETED" => TaskStatus::Completed,
        "FAILED" => TaskStatus::Failed,
        "RUNNING" => TaskStatus::Running,
        _ => TaskStatus::Pending,
    };

    if let Some(output) = &payload.output_path {
        task.output_path = output.clone();
    }

    if let Some(err) = &payload.error {
        task.error = Some(err.clone());
    }

    task.metrics = payload.metrics.clone();

    // Update worker active tasks count
    if let Some(worker_id) = &task.assigned_worker {
        if let Some(worker) = registry.get_mut(worker_id) {
            match task.status {
                TaskStatus::Running => {
                    worker.active_tasks += 1;
                }
                TaskStatus::Completed | TaskStatus::Failed => {
                    if worker.active_tasks > 0 {
                        worker.active_tasks -= 1;
                    }
                    if task.status == TaskStatus::Completed {
                        worker.total_tasks_completed += 1;
                    }
                }
                _ => {}
            }
        }
    }

    // Handle task failures: retry if attempt_id < max_retries
    const MAX_RETRIES: u32 = 1; // At least 1 retry as per requirements
    if task.status == TaskStatus::Failed && task.attempt_id < MAX_RETRIES {
        // Retry the task - update output path with new attempt_id for idempotency
        task.attempt_id += 1;
        task.status = TaskStatus::Pending;
        task.assigned_worker = None;
        task.error = None;
        
        // Update output path to include attempt_id (idempotency: avoid overwriting previous attempts)
        let old_output = task.output_path.clone();
        let base_path = old_output
            .rsplitn(2, '-')
            .nth(1)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("output/{}/{}", task.job_id, task_id));
        task.output_path = format!("{}-attempt{}", base_path, task.attempt_id);
        
        // Remove old output file if it exists (cleanup failed attempt)
        if std::path::Path::new(&old_output).exists() {
            let _ = std::fs::remove_file(&old_output);
        }
        
        // Re-queue for retry
        let task_queue = state.task_queue.clone();
        let task_id_clone = task_id.clone();
        tokio::spawn(async move {
            let mut queue = task_queue.write().await;
            queue.push_back(task_id_clone);
        });
        
        info!(
            task_id = %task_id,
            attempt = task.attempt_id,
            old_output = %old_output,
            new_output = %task.output_path,
            "Task queued for retry with new output path"
        );
    }

    // Update job output paths if task completed
    if task.status == TaskStatus::Completed {
        if let Some(job) = jobs.get_mut(&task.job_id) {
            if !job.output_paths.contains(&task.output_path) {
                job.output_paths.push(task.output_path.clone());
            }
        }
    }

    // Persist task update
    {
        let db = state.db.lock().unwrap();
        if let Err(e) = persistence::save_task(
            &db,
            &task_id,
            &task.job_id,
            task.attempt_id,
            &task.stage_id,
            &task.operator,
            task.status,
            task.assigned_worker.as_deref(),
            &task.input_paths,
            &task.output_path,
            task.partition,
            task.created_at,
            task.started_at,
            task.completed_at,
            task.error.as_deref(),
            &task.metrics,
        ) {
            error!("Failed to persist task update {}: {}", task_id, e);
        }
    }

    // Update and persist job status if changed
    {
        let jobs_read = state.jobs.read().await;
        if let Some(job) = jobs_read.get(&task.job_id) {
            let tasks_read = state.tasks.read().await;
            let job_tasks: Vec<_> = tasks_read
                .values()
                .filter(|t| t.job_id == task.job_id)
                .collect();
            
            let total = job_tasks.len();
            let completed = job_tasks
                .iter()
                .filter(|t| t.status == TaskStatus::Completed)
                .count();
            let failed = job_tasks
                .iter()
                .filter(|t| t.status == TaskStatus::Failed)
                .count();
            
            let progress = if total > 0 {
                (completed as f64 / total as f64) * 100.0
            } else {
                0.0
            };
            
            let mut new_status = job.status;
            if failed > 0 && completed == 0 {
                new_status = JobStatus::Failed;
            } else if completed == total && total > 0 {
                new_status = JobStatus::Succeeded;
            } else if completed > 0 || failed > 0 {
                new_status = JobStatus::Running;
            }
            
            if new_status != job.status || progress != job.progress {
                drop(jobs_read);
                let mut jobs_write = state.jobs.write().await;
                if let Some(job) = jobs_write.get_mut(&task.job_id) {
                    job.status = new_status;
                    job.progress = progress;
                    job.metrics.stages_completed = completed as u32;
                    job.metrics.stages_total = total as u32;
                    job.metrics.failures = failed as u32;
                    
                    // Persist job update
                    let db = state.db.lock().unwrap();
                    if let Err(e) = persistence::update_job_status(
                        &db,
                        &task.job_id,
                        job.status,
                        job.progress,
                        &job.metrics,
                        &job.output_paths,
                    ) {
                        error!("Failed to persist job update {}: {}", task.job_id, e);
                    }
                }
            }
        }
    }

    Ok(Json(TaskResponse {
        version: MESSAGE_VERSION.to_string(),
        task_id: task_id.clone(),
        attempt_id: payload.attempt_id,
        status: payload.status,
        output_path: payload.output_path,
        error: payload.error,
        metrics: payload.metrics,
    }))
}

// ============================================================================
// Task Scheduler
// ============================================================================

/// Task scheduler: assigns pending tasks to available workers (round-robin with load awareness)
async fn task_scheduler(state: AppState) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));

    loop {
        interval.tick().await;

        let registry = state.registry.read().await;
        let mut tasks = state.tasks.write().await;
        let mut task_queue = state.task_queue.write().await;

        // Get available workers (UP status, sorted by load)
        let mut available_workers: Vec<_> = registry
            .values()
            .filter(|w| w.status == WorkerStatus::Up)
            .map(|w| (w.id.clone(), w.active_tasks))
            .collect();

        // Sort by load (fewer active tasks first)
        available_workers.sort_by_key(|(_, load)| *load);

        if available_workers.is_empty() {
            drop(registry);
            drop(tasks);
            drop(task_queue);
            continue;
        }

        // Round-robin assignment
        let mut worker_idx = 0;
        let mut assigned = 0;

        while let Some(task_id) = task_queue.pop_front() {
            if let Some(task) = tasks.get_mut(&task_id) {
                if task.status == TaskStatus::Pending {
                    let (worker_id, _) = &available_workers[worker_idx % available_workers.len()];
                    task.assigned_worker = Some(worker_id.clone());
                    task.status = TaskStatus::Assigned;

                    // Send task to worker
                    let task_assignment = TaskAssignment {
                        task_id: task.task_id.clone(),
                        job_id: task.job_id.clone(),
                        attempt_id: task.attempt_id,
                        stage_id: task.stage_id.clone(),
                        operator: task.operator.clone(),
                        input_paths: task.input_paths.clone(),
                        output_path: task.output_path.clone(),
                        partition: task.partition,
                    };

                    let worker = registry.get(worker_id).unwrap();
                    let worker_url = format!("http://{}:{}", worker.host, worker.port);

                    // Spawn async task to send assignment
                    let task_req = TaskRequest {
                        version: MESSAGE_VERSION.to_string(),
                        assignment: task_assignment,
                    };

                    let http_client = reqwest::Client::new();
                    let task_id_clone = task_id.clone();
                    let worker_url_clone = worker_url.clone();

                    tokio::spawn(async move {
                        match http_client
                            .post(&format!("{}/api/v1/tasks/execute", worker_url_clone))
                            .json(&task_req)
                            .send()
                            .await
                        {
                            Ok(resp) if resp.status().is_success() => {
                                info!("Task {} assigned to worker", task_id_clone);
                            }
                            Ok(resp) => {
                                error!("Failed to assign task {}: status {}", task_id_clone, resp.status());
                            }
                            Err(e) => {
                                error!("Error assigning task {}: {:?}", task_id_clone, e);
                            }
                        }
                    });

                    assigned += 1;
                    worker_idx += 1;
                } else {
                    // Task already assigned or completed, skip
                }
            }
        }

        drop(registry);
        drop(tasks);
        drop(task_queue);

        if assigned > 0 {
            info!("Assigned {} tasks to workers", assigned);
        }
    }
}
