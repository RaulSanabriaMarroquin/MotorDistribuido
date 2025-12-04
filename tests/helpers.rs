//! Helpers de pruebas para tests de integración y end-to-end

use axum::{
    extract::State,
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
    net::SocketAddr,
    sync::Arc,
    time::SystemTime,
};
use tokio::sync::RwLock;
use tokio::net::TcpListener;
use uuid::Uuid;

/// Información del worker para el master de pruebas
#[derive(Debug, Clone)]
struct TestWorkerInfo {
    id: String,
    host: String,
    port: u16,
    last_heartbeat: SystemTime,
    status: WorkerStatus,
}

/// Información del job para el master de pruebas
#[derive(Debug, Clone)]
struct TestJobInfo {
    job_id: String,
    name: String,
    total_tasks: usize,
    completed_tasks: usize,
    failed_tasks: usize,
    status: String,
    outstanding_tasks: HashMap<String, String>,
}

/// Estado del master de pruebas
#[derive(Clone)]
struct TestMasterState {
    registry: Arc<RwLock<HashMap<String, TestWorkerInfo>>>,
    jobs: Arc<RwLock<HashMap<String, TestJobInfo>>>,
    #[allow(dead_code)]
    client: Client,
}

/// Inicia un servidor master de pruebas en un puerto aleatorio
pub async fn start_test_master() -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let state = TestMasterState {
        registry: Arc::new(RwLock::new(HashMap::new())),
        jobs: Arc::new(RwLock::new(HashMap::new())),
        client: Client::new(),
    };

    let app = Router::new()
        .route("/api/v1/workers/register", post(register_worker))
        .route("/api/v1/workers/:id/heartbeat", post(heartbeat))
        .route("/api/v1/workers", get(list_workers))
        .route("/api/v1/jobs/submit", post(submit_job))
        .route("/api/v1/jobs/:id/progress", get(get_job_progress))
        .route("/api/v1/jobs/:id/task_result", post(task_result))
        .with_state(state);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (addr, handle)
}

async fn register_worker(
    State(state): State<TestMasterState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, StatusCode> {
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

    let worker_info = TestWorkerInfo {
        id: worker_id.clone(),
        host: payload.host,
        port: payload.port,
        last_heartbeat: SystemTime::now(),
        status: WorkerStatus::Up,
    };

    {
        let mut registry = state.registry.write().await;
        registry.insert(worker_id.clone(), worker_info);
    }

    Ok(Json(RegisterResponse {
        version: MESSAGE_VERSION.to_string(),
        worker_id,
        message: "Registration successful".to_string(),
    }))
}

async fn heartbeat(
    State(state): State<TestMasterState>,
    axum::extract::Path(worker_id): axum::extract::Path<String>,
    Json(_payload): Json<HeartbeatRequest>,
) -> Result<Json<HeartbeatResponse>, StatusCode> {
    let mut registry = state.registry.write().await;
    if let Some(worker) = registry.get_mut(&worker_id) {
        worker.last_heartbeat = SystemTime::now();
        worker.status = WorkerStatus::Up;
    }

    Ok(Json(HeartbeatResponse {
        version: MESSAGE_VERSION.to_string(),
        status: "ok".to_string(),
    }))
}

async fn list_workers(
    State(state): State<TestMasterState>,
) -> Result<Json<WorkersListResponse>, StatusCode> {
    let registry = state.registry.read().await;
    let workers: Vec<WorkerListItem> = registry
        .values()
        .map(|w| WorkerListItem {
            id: w.id.clone(),
            host: w.host.clone(),
            port: w.port,
            status: w.status.as_str().to_string(),
            last_heartbeat: Some(
                w.last_heartbeat
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            ),
        })
        .collect();

    Ok(Json(WorkersListResponse {
        version: MESSAGE_VERSION.to_string(),
        workers,
    }))
}

async fn submit_job(
    State(state): State<TestMasterState>,
    Json(payload): Json<JobSpec>,
) -> Result<Json<SubmitJobResponse>, StatusCode> {
    let job_id = Uuid::new_v4().to_string();

    // Calculate total tasks (simplified)
    let total_tasks = if let Some(source) = &payload.source {
        match source {
            common::DatasetSource::Inline { data } => {
                (data.len() + payload.chunk_size - 1) / payload.chunk_size.max(1)
            }
            _ => 1,
        }
    } else {
        (payload.input.len() + payload.chunk_size - 1) / payload.chunk_size.max(1)
    };

    let job_info = TestJobInfo {
        job_id: job_id.clone(),
        name: payload.name,
        total_tasks,
        completed_tasks: 0,
        failed_tasks: 0,
        status: "running".to_string(),
        outstanding_tasks: HashMap::new(),
    };

    {
        let mut jobs = state.jobs.write().await;
        jobs.insert(job_id.clone(), job_info);
    }

    Ok(Json(SubmitJobResponse {
        version: MESSAGE_VERSION.to_string(),
        job_id,
        message: "Job submitted successfully".to_string(),
    }))
}

async fn get_job_progress(
    State(state): State<TestMasterState>,
    axum::extract::Path(job_id): axum::extract::Path<String>,
) -> Result<Json<JobProgress>, StatusCode> {
    let jobs = state.jobs.read().await;
    let job = jobs
        .get(&job_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(JobProgress {
        job_id: job.job_id.clone(),
        name: job.name.clone(),
        total_tasks: job.total_tasks,
        completed_tasks: job.completed_tasks,
        failed_tasks: job.failed_tasks,
        status: job.status.clone(),
        total_stages: None,
        current_stage: None,
        start_time_secs: None,
        end_time_secs: None,
        duration_secs: None,
        total_retries: None,
    }))
}

async fn task_result(
    State(state): State<TestMasterState>,
    axum::extract::Path(job_id): axum::extract::Path<String>,
    Json(payload): Json<TaskResult>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut jobs = state.jobs.write().await;
    if let Some(job) = jobs.get_mut(&job_id) {
        job.outstanding_tasks.remove(&payload.task_id);
        if payload.error.is_none() {
            job.completed_tasks += 1;
        } else {
            job.failed_tasks += 1;
        }

        if job.completed_tasks + job.failed_tasks >= job.total_tasks {
            job.status = if job.failed_tasks == 0 {
                "completed".to_string()
            } else {
                "failed".to_string()
            };
        }
    }

    Ok(Json(json!({"status": "ok"})))
}

/// Inicia un servidor worker de pruebas que puede ejecutar tareas
pub async fn start_test_worker(
    master_url: String,
    worker_port: u16,
) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    use worker::execute_operation;

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route(
            "/api/v1/tasks/execute",
            post(
                |Json(payload): Json<TaskAssignment>| async move {
                    let master_url_clone = master_url.clone();
                    let result = execute_operation(&payload);

                    let (output, error) = match result {
                        Ok(out) => (out, None),
                        Err(err) => (Vec::new(), Some(err)),
                    };

                    // Enviar resultado de vuelta al master
                    let task_result = TaskResult {
                        job_id: payload.job_id.clone(),
                        task_id: payload.task_id.clone(),
                        output: output.clone(),
                        error: error.clone(),
                    };

                    let client = Client::new();
                    let _ = client
                        .post(&format!("{}/api/v1/jobs/{}/task_result", master_url_clone, payload.job_id))
                        .json(&task_result)
                        .send()
                        .await;

                    Json(json!({
                        "status": "ok",
                        "task_id": payload.task_id,
                        "output": output
                    }))
                },
            ),
        );

    let listener = TcpListener::bind(format!("127.0.0.1:{}", worker_port))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();

    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (addr, handle)
}
