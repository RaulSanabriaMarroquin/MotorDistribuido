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

/// Información del worker almacenada en el registro
#[derive(Debug, Clone)]
struct WorkerInfo {
    id: String,
    host: String,
    port: u16,
    last_heartbeat: SystemTime,
    status: WorkerStatus,
    active_tasks: usize, // Número de tareas activas
    retry_count: usize,  // Número de reintentos
    task_latencies: Vec<Duration>, // Latencias de tareas completadas (para cálculo de promedio)
}

/// Información de tarea para seguimiento
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

/// Información del job almacenada en el registro
#[derive(Debug, Clone)]
struct JobInfo {
    job_id: String,
    name: String,
    total_tasks: usize,
    completed_tasks: usize,
    failed_tasks: usize,
    status: String, // "running", "completed", "failed"
    tasks: HashMap<String, TaskInfo>, // Seguimiento de tareas individuales
    start_time: SystemTime, // Tiempo de inicio del job
    end_time: Option<SystemTime>, // Tiempo de finalización del job
    stages: usize, // Número de etapas en el DAG
    result_paths: Vec<String>, // Rutas a archivos de salida
}

/// Estado compartido de la aplicación
#[derive(Clone)]
struct AppState {
    registry: Arc<RwLock<HashMap<String, WorkerInfo>>>,
    jobs: Arc<RwLock<HashMap<String, JobInfo>>>,
    tasks: Arc<RwLock<HashMap<String, TaskInfo>>>, // Registro global de tareas
}

// Constantes de configuración
const HEARTBEAT_TIMEOUT_SECS: u64 = 15;
const MONITOR_INTERVAL_SECS: u64 = 3;
const MAX_TASK_RETRIES: usize = 1; // Al menos 1 reintento según la especificación

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Nodo master iniciando...");

    // Crear estado compartido
    let state = AppState {
        registry: Arc::new(RwLock::new(HashMap::new())),
        jobs: Arc::new(RwLock::new(HashMap::new())),
        tasks: Arc::new(RwLock::new(HashMap::new())),
    };

    // Crear tarea de monitoreo en segundo plano
    let registry_monitor = state.registry.clone();
    let state_monitor = state.clone();
    tokio::spawn(async move {
        monitor_workers(registry_monitor, state_monitor).await;
    });

    // Construir router
    // Nota: Mantenemos /api/v1/jobs/submit para compatibilidad hacia atrás pero también agregamos /api/v1/jobs
    let app = Router::new()
        .route("/api/v1/workers/register", post(register_worker))
        .route("/api/v1/workers/:id/heartbeat", post(heartbeat))
        .route("/api/v1/workers", get(list_workers))
        .route("/api/v1/jobs", post(submit_job)) // Ruta exacta según especificación sección 5.1
        .route("/api/v1/jobs/submit", post(submit_job)) // Compatibilidad hacia atrás
        .route("/api/v1/jobs/:id", get(get_job_status)) // Ruta exacta según especificación sección 5.1
        .route("/api/v1/jobs/:id/progress", get(get_job_progress)) // Compatibilidad hacia atrás
        .route("/api/v1/jobs/:id/results", get(get_job_results))
        .route("/api/v1/jobs/:id/task_result", post(job_task_result))
        .route("/api/v1/metrics", get(get_metrics))
        .route("/api/v1/metrics/nodes", get(get_node_metrics))
        .route("/api/v1/metrics/jobs", get(get_job_metrics))
        .with_state(state);

    // Iniciar servidor con apagado ordenado (según especificación sección 12)
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    info!("Nodo master escuchando en 127.0.0.1:8080");

    // Configurar manejo de señales para apagado ordenado
    let shutdown_signal = async {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};
            let mut sigterm = signal(SignalKind::terminate()).ok();
            let mut sigint = signal(SignalKind::interrupt()).ok();
            
            tokio::select! {
                _ = async {
                    if let Some(mut sigterm) = sigterm {
                        sigterm.recv().await
                    } else {
                        futures::future::pending().await
                    }
                } => {},
                _ = async {
                    if let Some(mut sigint) = sigint {
                        sigint.recv().await
                    } else {
                        futures::future::pending().await
                    }
                } => {},
            }
        }
        #[cfg(windows)]
        {
            use tokio::signal::windows::{ctrl_c, ctrl_break};
            let mut ctrl_c_stream = ctrl_c().ok();
            let mut ctrl_break_stream = ctrl_break().ok();
            tokio::select! {
                _ = async {
                    if let Some(mut stream) = ctrl_c_stream {
                        stream.recv().await
                    } else {
                        futures::future::pending().await
                    }
                } => {},
                _ = async {
                    if let Some(mut stream) = ctrl_break_stream {
                        stream.recv().await
                    } else {
                        futures::future::pending().await
                    }
                } => {},
            }
        }
        #[cfg(not(any(unix, windows)))]
        {
            futures::future::pending().await
        }
    };

    // Crear manejador de apagado
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    
    // Crear manejador de señales
    tokio::spawn(async move {
        shutdown_signal.await;
        info!("Señal de apagado recibida, iniciando apagado ordenado...");
        let _ = shutdown_tx.send(());
    });

    // Servir con apagado ordenado
    let server = axum::serve(listener, app);
    
    tokio::select! {
        result = server => {
            if let Err(e) = result {
                error!("Error del servidor: {}", e);
            }
        }
        _ = shutdown_rx => {
            info!("Señal de apagado recibida, deteniendo servidor...");
        }
    }

    info!("Nodo master apagándose ordenadamente...");
    Ok(())
}

/// Registrar un nuevo worker
async fn register_worker(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, StatusCode> {
    info!(
        "Solicitud de registro recibida de {}:{}",
        payload.host, payload.port
    );

    // Validar versión si se proporciona
    if let Some(version) = &payload.version {
        if version != MESSAGE_VERSION {
            warn!(
                "Incompatibilidad de versión: esperada {}, recibida {}",
                MESSAGE_VERSION, version
            );
        }
    }

    // Generar ID único de worker
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

    // Registrar worker
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
            "Worker registrado: {} (total de workers: {})",
            worker_id,
            registry.len()
        );
    }

    Ok(Json(RegisterResponse {
        version: MESSAGE_VERSION.to_string(),
        worker_id,
        message: "Registro exitoso".to_string(),
    }))
}

/// Manejar heartbeat del worker
async fn heartbeat(
    State(state): State<AppState>,
    Path(worker_id): Path<String>,
    Json(payload): Json<HeartbeatRequest>,
) -> Result<Json<HeartbeatResponse>, StatusCode> {
    // Validar versión si se proporciona
    if let Some(version) = &payload.version {
        if version != MESSAGE_VERSION {
            warn!(
                "Incompatibilidad de versión: esperada {}, recibida {}",
                MESSAGE_VERSION, version
            );
        }
    }

    let mut registry = state.registry.write().await;

    // Buscar worker
    let worker = registry.get_mut(&worker_id).ok_or_else(|| {
        warn!("Heartbeat de worker desconocido: {}", worker_id);
        StatusCode::NOT_FOUND
    })?;

    // Actualizar heartbeat
    worker.last_heartbeat = SystemTime::now();

    // Si el worker estaba DOWN, marcarlo como UP nuevamente
    if worker.status == WorkerStatus::Down {
        info!("Worker {} recuperado, marcado como UP", worker_id);
        worker.status = WorkerStatus::Up;
    }

    drop(registry); // Liberar lock antes de registrar
    info!("Heartbeat recibido del worker: {}", worker_id);

    Ok(Json(HeartbeatResponse {
        version: MESSAGE_VERSION.to_string(),
        status: "ok".to_string(),
    }))
}

/// Listar todos los workers registrados
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

    info!("Listados {} workers", workers.len());

    Ok(Json(WorkersListResponse {
        version: MESSAGE_VERSION.to_string(),
        workers,
    }))
}

/// Enviar un job para ejecución
async fn submit_job(
    State(state): State<AppState>,
    Json(payload): Json<JobSpec>,
) -> Result<Json<SubmitJobResponse>, StatusCode> {
    info!("Envío de job recibido: {}", payload.name);

    let job_id = Uuid::new_v4().to_string();

    // Obtener workers disponibles
    let workers = {
        let registry = state.registry.read().await;
        registry
            .values()
            .filter(|w| w.status == WorkerStatus::Up)
            .cloned()
            .collect::<Vec<_>>()
    };

    if workers.is_empty() {
        warn!("No hay workers disponibles para ejecutar el job {}", job_id);
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    // Determinar número de tareas y despachar
    let total_tasks = if let Some(ref dag) = payload.dag {
        // Procesar formato DAG
        info!("Procesando DAG para el job: {}", job_id);
        let stages = match parse_dag(dag) {
            Ok(s) => s,
            Err(e) => {
                error!("Error al analizar DAG: {}", e);
                return Err(StatusCode::BAD_REQUEST);
            }
        };

        let parallelism = payload.parallelism.unwrap_or(workers.len());
        let mut task_count = 0;
        let client = Client::new();

        // Despachar tareas para cada etapa
        for (stage_idx, stage) in stages.iter().enumerate() {
            let num_tasks = stage.partitions.unwrap_or(parallelism);
            task_count += num_tasks;

            for task_idx in 0..num_tasks {
                // Round-robin + awareness de carga (según especificación sección 4.1 y 7)
                // Primero usar round-robin, pero considerar carga (active_tasks) al seleccionar
                let worker = {
                    let registry = state.registry.read().await;
                    let available_workers: Vec<_> = registry
                        .values()
                        .filter(|w| w.status == WorkerStatus::Up)
                        .cloned()
                        .collect();
                    
                    if available_workers.is_empty() {
                        return Err(StatusCode::SERVICE_UNAVAILABLE);
                    }
                    
                    // Round-robin: ciclo a través de workers
                    let round_robin_idx = task_idx % available_workers.len();
                    let round_robin_worker = &available_workers[round_robin_idx];
                    
                    // Awareness de carga: si el worker de round-robin tiene carga alta, encontrar uno con menos carga
                    // Umbral: si el worker de round-robin tiene >2x carga promedio, usar balanceo de carga
                    let avg_load: usize = available_workers.iter()
                        .map(|w| w.active_tasks)
                        .sum::<usize>() / available_workers.len().max(1);
                    
                    if round_robin_worker.active_tasks > avg_load * 2 && avg_load > 0 {
                        // Usar balanceo de carga en su lugar
                        available_workers
                            .iter()
                            .min_by_key(|w| w.active_tasks)
                            .unwrap()
                            .clone()
                    } else {
                        // Usar round-robin
                        round_robin_worker.clone()
                    }
                };
                
                let task_id = format!("{}-stage{}-task{}", job_id, stage_idx, task_idx);
                
                // Para shuffle: si esta etapa depende de etapas anteriores,
                // necesitamos pasar información de partición para redistribución de datos
                let input_path = if stage_idx > 0 && !stage.dependencies.is_empty() {
                    // Shuffle: datos de la etapa anterior necesitan ser redistribuidos
                    // En una implementación real, recogeríamos resultados de la etapa anterior
                    // y redistribuiríamos basado en hash de clave
                    // Por ahora, usaremos un patrón de ruta que indica shuffle
                    Some(format!("shuffle/{}/stage{}/partition{}", job_id, stage_idx - 1, task_idx))
                } else {
                    stage.path.clone()
                };
                
                let task_assignment = TaskAssignment {
                    job_id: job_id.clone(),
                    task_id: task_id.clone(),
                    node_id: stage.node_id.clone(),
                    operation: stage.operation.clone(),
                    fn_name: stage.fn_name.clone(),
                    key: stage.key.clone(),
                    param: None, // Las operaciones DAG no usan param
                    input: None, // La entrada vendrá de la etapa anterior o archivo
                    input_path,
                    input2: None,
                    input_path2: None,
                    attempt_id: Some(0),
                };
                
                // Registrar tarea
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
                // Actualizar conteo de tareas activas del worker
                {
                    let mut registry = state.registry.write().await;
                    if let Some(worker_info) = registry.get_mut(&worker.id) {
                        worker_info.active_tasks += 1;
                    }
                }
                
                // Actualizar estado de tarea a Running
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
                            info!("Tarea {} despachada al worker {}", task_id, worker_url);
                        }
                        Ok(resp) => {
                            error!(
                                "Error al despachar tarea {} al worker {}: {}",
                                task_id, worker_url, resp.status()
                            );
                        }
                        Err(e) => {
                            error!(
                                "Error al despachar tarea {} al worker {}: {:?}",
                                task_id, worker_url, e
                            );
                        }
                    }
                });
            }
        }

        task_count
    } else if let Some(ref operation) = payload.operation {
        // Formato legacy (operación simple)
        info!("Procesando job en formato legacy: {}", job_id);
        let client = Client::new();
        for (idx, worker) in workers.iter().enumerate() {
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
                input2: None,
                input_path2: None,
                attempt_id: Some(0),
            };
            
            // Registrar tarea para formato legacy
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
                        info!("Tarea despachada exitosamente al worker {}", worker_url);
                    }
                    Ok(resp) => {
                        error!(
                            "Error al despachar tarea al worker {}: {}",
                            worker_url,
                            resp.status()
                        );
                    }
                    Err(e) => {
                        error!("Error al despachar tarea al worker {}: {:?}", worker_url, e);
                    }
                }
            });
        }
        1 // Formato legacy: 1 tarea por job
    } else {
        error!("La especificación del job debe tener el campo 'dag' o 'operation'");
        return Err(StatusCode::BAD_REQUEST);
    };

    // Calcular número de etapas
    let stages = if let Some(ref dag) = payload.dag {
        match parse_dag(dag) {
            Ok(parsed_stages) => parsed_stages.len(),
            Err(_) => 1, // Fallback a 1 etapa
        }
    } else {
        1 // Formato legacy: 1 etapa
    };

    // Crear información del job
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
        result_paths: Vec::new(),
    };

    // Almacenar job
    {
        let mut jobs = state.jobs.write().await;
        jobs.insert(job_id.clone(), job_info);
        info!("Job creado: {} con id: {} ({} tareas)", payload.name, job_id, total_tasks);
    }

    Ok(Json(SubmitJobResponse {
        version: MESSAGE_VERSION.to_string(),
        job_id,
        message: "Job enviado exitosamente".to_string(),
    }))
}

/// Obtener estado del job (endpoint exacto según especificación sección 5.1)
/// Retorna: estado (ACCEPTED/RUNNING/FAILED/SUCCEEDED), progreso (%), métricas
async fn get_job_status(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let jobs = state.jobs.read().await;

    let job = jobs.get(&job_id).ok_or_else(|| {
        warn!("Job no encontrado: {}", job_id);
        StatusCode::NOT_FOUND
    })?;

    // Calcular porcentaje de progreso
    let progress_percent = if job.total_tasks > 0 {
        (job.completed_tasks as f64 / job.total_tasks as f64) * 100.0
    } else {
        0.0
    };

    // Mapear estado al formato de especificación (ACCEPTED/RUNNING/FAILED/SUCCEEDED)
    let status = match job.status.as_str() {
        "running" => "RUNNING",
        "completed" => "SUCCEEDED",
        "failed" => "FAILED",
        _ => "ACCEPTED",
    };

    info!(
        "Estado recuperado para el job: {} ({}/{}, {}%)",
        job_id, job.completed_tasks, job.total_tasks, progress_percent
    );

    Ok(Json(json!({
        "job_id": job.job_id,
        "name": job.name,
        "status": status,
        "progress_percent": progress_percent,
        "total_tasks": job.total_tasks,
        "completed_tasks": job.completed_tasks,
        "failed_tasks": job.failed_tasks,
        "stages": job.stages
    })))
}

/// Obtener progreso del job (endpoint de compatibilidad hacia atrás)
async fn get_job_progress(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> Result<Json<JobProgress>, StatusCode> {
    let jobs = state.jobs.read().await;

    let job = jobs.get(&job_id).ok_or_else(|| {
        warn!("Job no encontrado: {}", job_id);
        StatusCode::NOT_FOUND
    })?;

    info!(
        "Progreso recuperado para el job: {} ({}/{})",
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

/// Obtener resultados del job (rutas de archivos de salida)
async fn get_job_results(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let jobs = state.jobs.read().await;

    let job = jobs.get(&job_id).ok_or_else(|| {
        warn!("Job no encontrado: {}", job_id);
        StatusCode::NOT_FOUND
    })?;

    info!("Resultados recuperados para el job: {}", job_id);

    Ok(Json(json!({
        "job_id": job.job_id,
        "name": job.name,
        "status": job.status,
        "result_paths": job.result_paths
    })))
}

/// Recibir resultado de tarea del worker
async fn job_task_result(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
    Json(payload): Json<TaskResult>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!(
        "Resultado de tarea recibido para el job: {}, tarea: {}",
        job_id, payload.task_id
    );

    let mut jobs = state.jobs.write().await;

    let job = jobs.get_mut(&job_id).ok_or_else(|| {
        warn!("Job no encontrado: {}", job_id);
        StatusCode::NOT_FOUND
    })?;

    // Actualizar progreso del job basado en resultado de tarea
    if payload.success {
        // Actualizar estado de tarea
        if let Some(task) = job.tasks.get_mut(&payload.task_id) {
            task.status = TaskStatus::Completed;
            
            // Almacenar ruta de salida si está presente
            if let Some(ref output_path) = payload.output_path {
                if !job.result_paths.contains(output_path) {
                    job.result_paths.push(output_path.clone());
                }
            }
            
            // Decrementar tareas activas del worker
            if let Some(ref worker_id) = task.worker_id {
                let mut registry = state.registry.write().await;
                if let Some(worker_info) = registry.get_mut(worker_id) {
                    if worker_info.active_tasks > 0 {
                        worker_info.active_tasks -= 1;
                    }
                    // Registrar latencia de tarea (simplificado: usar tiempo actual)
                    // En un sistema real, rastrearíamos tiempos de inicio/fin por tarea
                }
            }
        }
        
        if job.completed_tasks < job.total_tasks {
            job.completed_tasks += 1;
            info!(
                "Progreso del job {}: {}/{} (tarea {} completada, intento {})",
                job_id, job.completed_tasks, job.total_tasks, payload.task_id, payload.attempt_id
            );

            if job.completed_tasks == job.total_tasks {
                job.status = "completed".to_string();
                job.end_time = Some(SystemTime::now());
                info!("Job {} completado", job_id);
            }
        }
    } else {
        // Tarea falló - verificar si debemos reintentar
        let task = job.tasks.get_mut(&payload.task_id);
        if let Some(task_info) = task {
            if task_info.attempt_id < MAX_TASK_RETRIES {
                // Reintentar la tarea
                let new_attempt_id = task_info.attempt_id + 1;
                info!(
                    "Reintentando tarea {} para el job {} (intento {}/{})",
                    payload.task_id, job_id, new_attempt_id, MAX_TASK_RETRIES
                );
                
                task_info.attempt_id = new_attempt_id;
                task_info.status = TaskStatus::Pending;
                
                // Decrementar tareas activas del worker (tarea falló)
                if let Some(ref worker_id) = task_info.worker_id {
                    let mut registry = state.registry.write().await;
                    if let Some(worker_info) = registry.get_mut(worker_id) {
                        if worker_info.active_tasks > 0 {
                            worker_info.active_tasks -= 1;
                        }
                        worker_info.retry_count += 1;
                    }
                }
                
                // Obtener workers disponibles
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
                    
                    // Seleccionar worker con menos tareas activas para reintento
                    let worker = {
                        let registry = state.registry.read().await;
                        let available_workers: Vec<_> = registry
                            .values()
                            .filter(|w| w.status == WorkerStatus::Up)
                            .cloned()
                            .collect();
                        
                        available_workers
                            .iter()
                            .min_by_key(|w| w.active_tasks)
                            .unwrap()
                            .clone()
                    };
                    let worker_url = format!(
                        "http://{}:{}/api/v1/tasks/execute",
                        worker.host, worker.port
                    );
                    
                    let client = Client::new();
                    let task_clone = assignment.clone();
                    tokio::spawn(async move {
                        match client.post(&worker_url).json(&task_clone).send().await {
                            Ok(resp) if resp.status().is_success() => {
                                info!("Tarea de reintento despachada al worker {}", worker_url);
                            }
                            Ok(resp) => {
                                error!("Error al reintentar tarea: {}", resp.status());
                            }
                            Err(e) => {
                                error!("Error al reintentar tarea: {:?}", e);
                            }
                        }
                    });
                }
            } else {
                // Máximo de reintentos excedido
                task_info.status = TaskStatus::Failed;
                job.failed_tasks += 1;
                error!(
                    "Tarea {} falló permanentemente para el job {} después de {} intentos: {}",
                    payload.task_id, job_id, MAX_TASK_RETRIES,
                    payload.error.as_deref().unwrap_or("Error desconocido")
                );
                
                // Marcar job como fallido si demasiadas tareas fallaron
                if job.failed_tasks > job.total_tasks / 2 {
                    job.status = "failed".to_string();
                    warn!("Job {} marcado como fallido debido a demasiadas fallas de tareas", job_id);
                }
            }
        } else {
            // Tarea no encontrada en tareas del job (formato legacy)
            job.failed_tasks += 1;
            error!(
                "Tarea {} falló para el job {} (intento {}): {}",
                payload.task_id, job_id, payload.attempt_id,
                payload.error.as_deref().unwrap_or("Error desconocido")
            );
        }
    }

    Ok(Json(json!({
        "status": "ack",
        "message": "Resultado de tarea recibido"
    })))
}

/// Obtener todas las métricas (nodos y jobs)
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

/// Obtener métricas de nodos
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

/// Obtener métricas de jobs
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

/// Función interna para obtener métricas de nodos
async fn get_node_metrics_internal(state: &AppState) -> Vec<NodeMetrics> {
    let registry = state.registry.read().await;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    registry
        .values()
        .map(|worker| {
            // Calcular latencia promedio
            let avg_latency_ms = if worker.task_latencies.is_empty() {
                0.0
            } else {
                let total: Duration = worker.task_latencies.iter().sum();
                total.as_millis() as f64 / worker.task_latencies.len() as f64
            };

            // Aproximar uso de CPU (simplificado: basado en tareas activas)
            let cpu_usage_percent = (worker.active_tasks as f64 * 10.0).min(100.0);

            // Aproximar uso de memoria (simplificado: basado en tareas activas)
            let memory_usage_mb = worker.active_tasks as f64 * 50.0; // Estimación de 50MB por tarea

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

/// Función interna para obtener métricas de jobs
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
                throughput_events_per_sec: None, // Para streaming, calcularíamos basado en eventos
                failure_count: job.failed_tasks,
                start_time: start_time_secs,
                end_time: end_time_secs,
            }
        })
        .collect()
}

/// Tarea en segundo plano para monitorear workers y marcarlos como DOWN si exceden el timeout
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
                "Worker {} excedió timeout de heartbeat, marcado como DOWN",
                worker_id
            );
        }

        if !marked_down.is_empty() {
            info!("Marcados {} worker(s) como DOWN", marked_down.len());
            
            // Replanificar tareas de workers que cayeron
            for worker_id in &marked_down {
                replanify_worker_tasks(&state, worker_id).await;
            }
        }
    }
}

/// Replanificar tareas de un worker que cayó
async fn replanify_worker_tasks(state: &AppState, failed_worker_id: &str) {
    info!("Replanificando tareas para el worker fallido: {}", failed_worker_id);
    
    // Obtener workers disponibles
    let available_workers = {
        let registry = state.registry.read().await;
        registry
            .values()
            .filter(|w| w.status == WorkerStatus::Up && w.id != failed_worker_id)
            .cloned()
            .collect::<Vec<_>>()
    };

    if available_workers.is_empty() {
        warn!("No hay workers disponibles para replanificar tareas del worker {}", failed_worker_id);
        return;
    }

    // Encontrar tareas asignadas al worker fallido
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
        info!("No hay tareas para replanificar para el worker {}", failed_worker_id);
        return;
    }

    info!("Replanificando {} tareas del worker {}", tasks_to_replanify.len(), failed_worker_id);

    let client = Client::new();

    for (job_id, task_info) in tasks_to_replanify {
        // Actualizar estado de tarea a Pending
        {
            let mut jobs = state.jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                if let Some(task) = job.tasks.get_mut(&task_info.task_id) {
                    task.status = TaskStatus::Pending;
                    task.worker_id = None; // Limpiar asignación anterior de worker
                }
            }
        }

        // Asignar a nuevo worker con menos tareas activas
        let worker = {
            let registry = state.registry.read().await;
            let available: Vec<_> = registry
                .values()
                .filter(|w| w.status == WorkerStatus::Up && w.id != failed_worker_id)
                .cloned()
                .collect();
            
            available
                .iter()
                .min_by_key(|w| w.active_tasks)
                .unwrap()
                .clone()
        };

        let mut assignment = task_info.assignment.clone();
        assignment.attempt_id = Some(task_info.attempt_id); // Mantener attempt_id para idempotencia

        let worker_url = format!(
            "http://{}:{}/api/v1/tasks/execute",
            worker.host, worker.port
        );

        // Actualizar asignación de worker
        {
            let mut jobs = state.jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                if let Some(task) = job.tasks.get_mut(&task_info.task_id) {
                    task.worker_id = Some(worker.id.clone());
                    task.status = TaskStatus::Running;
                }
            }
        }

        // Actualizar conteo de tareas activas del worker
        {
            let mut registry = state.registry.write().await;
            if let Some(worker_info) = registry.get_mut(&worker.id) {
                worker_info.active_tasks += 1;
            }
        }

        // Despachar tarea a nuevo worker
        let client_clone = client.clone();
        let task_clone = assignment.clone();
        let task_id = task_info.task_id.clone();
        tokio::spawn(async move {
            match client_clone.post(&worker_url).json(&task_clone).send().await {
                Ok(resp) if resp.status().is_success() => {
                    info!("Tarea {} replanificada al worker {}", task_id, worker_url);
                }
                Ok(resp) => {
                    error!("Error al replanificar tarea {}: {}", task_id, resp.status());
                }
                Err(e) => {
                    error!("Error al replanificar tarea {}: {:?}", task_id, e);
                }
            }
        });
    }
}
