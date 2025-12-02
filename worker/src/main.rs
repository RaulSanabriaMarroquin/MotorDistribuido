mod cache;
mod operators;

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
use operators::{
    filter, flat_map, join, map, read_csv, read_jsonl, reduce_by_key, reduce_by_key_to_vec,
    write_csv, write_jsonl,
};
use reqwest::Client;
use serde_json::Value;
use tokio::{net::TcpListener, time::sleep};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configurar logging: si RUST_LOG no está configurado, usar "info" por defecto
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
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

    info!(%master_url, %host, port, "Nodo worker iniciando");

    let http_client = Client::new();

    // 1) Registro en el master
    // Generar un worker_id temporal (el master puede asignar uno nuevo)
    let temp_worker_id = format!("worker-{}", uuid::Uuid::new_v4());
    let register_req = RegisterRequest {
        worker_id: temp_worker_id,
        host: host.clone(),
        port,
        version: MESSAGE_VERSION.to_string(),
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
                info!(worker_id = %body.worker_id, "Registro exitoso");
                break body.worker_id;
            }
            Ok(resp) => {
                error!(status=?resp.status(), "Registro fallido");
            }
            Err(e) => {
                error!(error=?e, "Error al registrar worker");
            }
        }

        let backoff = Duration::from_secs(3);
        info!(?backoff, "Reintentando registro después de backoff");
        sleep(backoff).await;
    };

    // 2) Servidor HTTP local con /health y /api/v1/tasks/execute
    let master_url_clone = master_url.clone();
    let http_client_clone = http_client.clone();
    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/api/v1/tasks/execute", post(execute_task))
        .with_state((master_url_clone, http_client_clone));

    let addr = format!("{}:{}", host, port);
    let listener = TcpListener::bind(&addr).await?;
    info!(%addr, "Iniciando servidor HTTP del worker (/health, /api/v1/tasks/execute)");

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
                worker_id: hb_worker_id.clone(),
                active_tasks: 0, // TODO: rastrear tareas activas
                version: Some(MESSAGE_VERSION.to_string()),
                timestamp: Some(now),
            };

            match hb_client.post(&hb_url).json(&hb_req).send().await {
                Ok(resp) if resp.status().is_success() => {
                    info!("Heartbeat enviado");
                }
                Ok(resp) => {
                    error!(status=?resp.status(), "Heartbeat fallido");
                }
                Err(e) => {
                    error!(error=?e, "Error al enviar heartbeat");
                }
            }

            sleep(Duration::from_secs(heartbeat_interval_secs)).await;
        }
    });

    // 4) Mantener el worker corriendo con graceful shutdown (per specification section 12)
    let shutdown_signal = async {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};
            let sigterm = signal(SignalKind::terminate()).ok();
            let sigint = signal(SignalKind::interrupt()).ok();
            
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
                    if let Some(stream) = &mut ctrl_c_stream {
                        stream.recv().await
                    } else {
                        futures::future::pending().await
                    }
                } => {},
                _ = async {
                    if let Some(stream) = &mut ctrl_break_stream {
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

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    
    tokio::spawn(async move {
        shutdown_signal.await;
        info!("Señal de apagado recibida, iniciando apagado ordenado...");
        let _ = shutdown_tx.send(());
    });

    let server = axum::serve(listener, app);
    
    tokio::select! {
        result = server => {
            if let Err(e) = result {
                error!("Error del servidor: {}", e);
            }
        }
        _ = shutdown_rx => {
            info!("Señal de apagado recibida, deteniendo worker...");
        }
    }

    info!("Worker apagándose ordenadamente...");
    Ok(())
}

/// Ejecutar una tarea y retornar resultado
async fn execute_task(
    axum::extract::State((master_url, http_client)): axum::extract::State<(String, Client)>,
    Json(payload): Json<TaskAssignment>,
) -> Result<AxumJson<serde_json::Value>, axum::http::StatusCode> {
    info!(
        "Ejecutando tarea: {} para el job: {} (op: {})",
        payload.task_id, payload.job_id, payload.operation
    );

    // Obtener datos de entrada (soporta formato legacy y nuevo)
    // Si se proporciona input_path, leer desde archivo; de lo contrario usar campo input
    let input_data = if let Some(ref input_path) = payload.input_path {
        // Leer desde archivo basado en extensión
        if input_path.ends_with(".csv") {
            match read_csv(input_path).await {
                Ok(data) => data,
                Err(e) => {
                    error!("Error al leer CSV {}: {}", input_path, e);
                    // Enviar error al master antes de retornar
                    let error_result = TaskResult {
                        job_id: payload.job_id.clone(),
                        task_id: payload.task_id.clone(),
                        attempt_id: payload.attempt_id.unwrap_or(0),
                        success: false,
                        node_id: Some(payload.node_id.clone()),
                        result: None,
                        result_path: None,
                        output: None,
                        output_data: None,
                        output_path: None,
                        error: Some(format!("Error al leer CSV {}: {}", input_path, e)),
                    };
                    let result_url = format!("{}/api/v1/jobs/{}/task_result", master_url, payload.job_id);
                    let _ = http_client.post(&result_url).json(&error_result).send().await;
                    return Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
                }
            }
        } else if input_path.ends_with(".jsonl") {
            match read_jsonl(input_path).await {
                Ok(data) => data,
                Err(e) => {
                    error!("Error al leer JSONL {}: {}", input_path, e);
                    // Enviar error al master antes de retornar
                    let error_result = TaskResult {
                        job_id: payload.job_id.clone(),
                        task_id: payload.task_id.clone(),
                        attempt_id: payload.attempt_id.unwrap_or(0),
                        success: false,
                        node_id: Some(payload.node_id.clone()),
                        result: None,
                        result_path: None,
                        output: None,
                        output_data: None,
                        output_path: None,
                        error: Some(format!("Error al leer JSONL {}: {}", input_path, e)),
                    };
                    let result_url = format!("{}/api/v1/jobs/{}/task_result", master_url, payload.job_id);
                    let _ = http_client.post(&result_url).json(&error_result).send().await;
                    return Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
                }
            }
        } else {
            return Err(axum::http::StatusCode::BAD_REQUEST);
        }
    } else if let Some(ref input) = payload.input {
        input.clone()
    } else {
        return Err(axum::http::StatusCode::BAD_REQUEST);
    };

    // Ejecutar la operación
    let (output, output_data) = match payload.operation.as_str() {
        // Operaciones legacy (para compatibilidad hacia atrás)
        "map_add" => {
            let result = map(&input_data, Some("add"), payload.param)
                .map_err(|e| {
                    error!("Operación map falló: {}", e);
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR
                })?;
            (Some(result), None)
        }
        "map_mul" => {
            let result = map(&input_data, Some("mul"), payload.param)
                .map_err(|e| {
                    error!("Operación map falló: {}", e);
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR
                })?;
            (Some(result), None)
        }
        "filter_gt" => {
            let result = filter(&input_data, Some("gt"), payload.param)
                .map_err(|e| {
                    error!("Operación filter falló: {}", e);
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR
                })?;
            (Some(result), None)
        }
        "filter_lt" => {
            let result = filter(&input_data, Some("lt"), payload.param)
                .map_err(|e| {
                    error!("Operación filter falló: {}", e);
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR
                })?;
            (Some(result), None)
        }
        // Operaciones DAG nuevas
        "map" => {
            let result = map(&input_data, payload.fn_name.as_deref(), payload.param)
                .map_err(|e| {
                    error!("Operación map falló: {}", e);
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR
                })?;
            (Some(result), None)
        }
        "flat_map" => {
            let result = flat_map(&input_data, payload.fn_name.as_deref())
                .map_err(|e| {
                    error!("Operación flat_map falló: {}", e);
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR
                })?;
            (Some(result), None)
        }
        "filter" => {
            let result = filter(&input_data, payload.fn_name.as_deref(), payload.param)
                .map_err(|e| {
                    error!("Operación filter falló: {}", e);
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR
                })?;
            (Some(result), None)
        }
        "reduce_by_key" => {
            let result = reduce_by_key(&input_data, payload.fn_name.as_deref())
                .map_err(|e| {
                    error!("Operación reduce_by_key falló: {}", e);
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR
                })?;
            let vec_result = reduce_by_key_to_vec(&result);
            (None, Some(Value::Array(vec_result)))
        }
        "read_csv" => {
            // Esto se maneja arriba en la lectura de entrada, pero también podemos manejarlo aquí
            if let Some(ref path) = payload.input_path {
                let result = read_csv(path)
                    .await
                    .map_err(|e| {
                        error!("Error al leer CSV: {}", e);
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR
                    })?;
                (Some(result), None)
            } else {
                return Err(axum::http::StatusCode::BAD_REQUEST);
            }
        }
        "read_jsonl" => {
            if let Some(ref path) = payload.input_path {
                let result = read_jsonl(path)
                    .await
                    .map_err(|e| {
                        error!("Error al leer JSONL: {}", e);
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR
                    })?;
                (Some(result), None)
            } else {
                return Err(axum::http::StatusCode::BAD_REQUEST);
            }
        }
        "write_csv" => {
            // Para operaciones de escritura, necesitamos una ruta de salida
            // Por ahora, generar una ruta basada en task_id
            let output_path = format!("output/{}/result.csv", payload.job_id);
            let path = write_csv(&output_path, &input_data)
                .await
                .map_err(|e| {
                    error!("Error al escribir CSV: {}", e);
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR
                })?;
            (None, Some(Value::String(path)))
        }
        "write_jsonl" => {
            let output_path = format!("output/{}/result.jsonl", payload.job_id);
            let path = write_jsonl(&output_path, &input_data)
                .await
                .map_err(|e| {
                    error!("Error al escribir JSONL: {}", e);
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR
                })?;
            (None, Some(Value::String(path)))
        }
        "join" => {
            // Join requiere dos entradas
            let right_data = if let Some(ref input_path2) = payload.input_path2 {
                if input_path2.ends_with(".csv") {
                    read_csv(input_path2)
                        .await
                        .map_err(|e| {
                            error!("Error al leer CSV para join: {}", e);
                            axum::http::StatusCode::INTERNAL_SERVER_ERROR
                        })?
                } else if input_path2.ends_with(".jsonl") {
                    read_jsonl(input_path2)
                        .await
                        .map_err(|e| {
                            error!("Error al leer JSONL para join: {}", e);
                            axum::http::StatusCode::INTERNAL_SERVER_ERROR
                        })?
                } else {
                    return Err(axum::http::StatusCode::BAD_REQUEST);
                }
            } else if let Some(ref input2) = payload.input2 {
                input2.clone()
            } else {
                error!("Operación join requiere segunda entrada (input2 o input_path2)");
                return Err(axum::http::StatusCode::BAD_REQUEST);
            };

            let result = join(&input_data, &right_data, payload.key.as_deref())
                .map_err(|e| {
                    error!("Operación join falló: {}", e);
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR
                })?;
            (None, Some(Value::Array(result)))
        }
        _ => {
            error!("Operación desconocida: {}", payload.operation);
            return Err(axum::http::StatusCode::BAD_REQUEST);
        }
    };

    let output_size = output.as_ref().map(|v| v.len()).unwrap_or(0);
    let output_data_size = output_data.as_ref().and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
    
    info!(
        "Tarea {} completada: input_size={}, output_size={} (output_data_size={})",
        payload.task_id,
        input_data.len(),
        output_size,
        output_data_size
    );

    // Determinar ruta de salida si es una operación de escritura
    let output_path = if payload.operation == "write_csv" || payload.operation == "write_jsonl" {
        output_data.as_ref().and_then(|v| v.as_str()).map(|s| s.to_string())
    } else {
        None
    };

    // Enviar resultado de vuelta al master
    let attempt_id = payload.attempt_id.unwrap_or(0);
    let task_result = TaskResult {
        job_id: payload.job_id.clone(),
        task_id: payload.task_id.clone(),
        attempt_id,
        success: true,
        node_id: Some(payload.node_id.clone()),
        result: output.clone(),
        result_path: output_path.clone(),
        output: output.clone(),
        output_data: output_data.clone(),
        output_path,
        error: None,
    };

    let result_url = format!("{}/api/v1/jobs/{}/task_result", master_url, payload.job_id);

    match http_client
        .post(&result_url)
        .json(&task_result)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            info!("Resultado de tarea enviado al master para la tarea: {}", payload.task_id);
        }
        Ok(resp) => {
            error!(
                "Error al enviar resultado de tarea al master: {} (estado: {})",
                result_url,
                resp.status()
            );
        }
        Err(e) => {
            error!(
                "Error al enviar resultado de tarea al master: {} (error: {:?})",
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
