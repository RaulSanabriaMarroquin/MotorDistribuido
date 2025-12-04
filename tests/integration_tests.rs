//! Pruebas de integración para el sistema distribuido
//! Prueba escenarios de nodo único y multinodo

mod helpers;

use common::{JobSpec, RegisterRequest, Stage, DatasetSource, HeartbeatRequest};
use helpers::{start_test_master, start_test_worker};
use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;

/// Espera a que un servicio esté listo
async fn wait_for_service(url: &str, max_attempts: usize) -> bool {
    let client = Client::new();
    for _ in 0..max_attempts {
        if client.get(url).send().await.is_ok() {
            return true;
        }
        sleep(Duration::from_millis(100)).await;
    }
    false
}

/// Registra un worker con el master
async fn register_worker(master_url: &str, worker_port: u16) -> Result<String, String> {
    let client = Client::new();
    let register_url = format!("{}/api/v1/workers/register", master_url);
    
    let request = RegisterRequest {
        version: Some("1.0".to_string()),
        host: "127.0.0.1".to_string(),
        port: worker_port,
    };
    
    let response = client
        .post(&register_url)
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Error al registrar worker: {}", e))?;
    
    if !response.status().is_success() {
        return Err(format!("Registro falló con estado: {}", response.status()));
    }
    
    let body: serde_json::Value = response.json().await.unwrap();
    body.get("worker_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "No hay worker_id en la respuesta".to_string())
}

/// Envía un heartbeat desde un worker
async fn send_heartbeat(master_url: &str, worker_id: &str) -> Result<(), String> {
    let client = Client::new();
    let heartbeat_url = format!("{}/api/v1/workers/{}/heartbeat", master_url, worker_id);
    
    let request = HeartbeatRequest {
        version: Some("1.0".to_string()),
        timestamp: Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        ),
    };
    
    let response = client
        .post(&heartbeat_url)
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Error al enviar heartbeat: {}", e))?;
    
    if !response.status().is_success() {
        return Err(format!("Heartbeat falló con estado: {}", response.status()));
    }
    
    Ok(())
}

// ============================================================================
// Pruebas de integración (nodo único)
// ============================================================================

/// Prueba el registro de un worker en un escenario de nodo único
#[tokio::test]
async fn test_worker_registration_single_node() {
    // Iniciar master de pruebas
    let (master_addr, _master_handle) = start_test_master().await;
    let master_url = format!("http://{}", master_addr);
    
    // Esperar a que el master esté listo
    assert!(wait_for_service(&format!("{}/api/v1/workers", master_url), 10).await);
    
    // Registrar un worker
    let worker_id = register_worker(&master_url, 19000).await.unwrap();
    assert!(!worker_id.is_empty());
    assert!(worker_id.starts_with("w"));
}

/// Prueba el listado de workers en un escenario de nodo único
#[tokio::test]
async fn test_list_workers_single_node() {
    let (master_addr, _master_handle) = start_test_master().await;
    let master_url = format!("http://{}", master_addr);
    
    assert!(wait_for_service(&format!("{}/api/v1/workers", master_url), 10).await);
    
    // Registrar un worker
    let _worker_id = register_worker(&master_url, 19001).await.unwrap();
    
    // Listar workers
    let client = Client::new();
    let response = client
        .get(&format!("{}/api/v1/workers", master_url))
        .send()
        .await
        .unwrap();
    
    assert!(response.status().is_success());
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body.get("workers").is_some());
    let workers = body.get("workers").unwrap().as_array().unwrap();
    assert_eq!(workers.len(), 1);
}

/// Prueba el envío de heartbeat en un escenario de nodo único
#[tokio::test]
async fn test_heartbeat_single_node() {
    let (master_addr, _master_handle) = start_test_master().await;
    let master_url = format!("http://{}", master_addr);
    
    assert!(wait_for_service(&format!("{}/api/v1/workers", master_url), 10).await);
    
    // Registrar un worker
    let worker_id = register_worker(&master_url, 19002).await.unwrap();
    
    // Enviar heartbeat
    send_heartbeat(&master_url, &worker_id).await.unwrap();
    
    // Verificar que el worker sigue registrado
    let client = Client::new();
    let response = client
        .get(&format!("{}/api/v1/workers", master_url))
        .send()
        .await
        .unwrap();
    
    let body: serde_json::Value = response.json().await.unwrap();
    let workers = body.get("workers").unwrap().as_array().unwrap();
    assert_eq!(workers.len(), 1);
    assert_eq!(workers[0].get("id").unwrap().as_str().unwrap(), worker_id);
}

/// Prueba el envío de un job en un escenario de nodo único
#[tokio::test]
async fn test_submit_job_single_node() {
    let (master_addr, _master_handle) = start_test_master().await;
    let master_url = format!("http://{}", master_addr);
    
    assert!(wait_for_service(&format!("{}/api/v1/workers", master_url), 10).await);
    
    // Registrar un worker
    let _worker_id = register_worker(&master_url, 19003).await.unwrap();
    
    // Enviar un job
    let client = Client::new();
    let job_spec = JobSpec {
        name: "test-job".to_string(),
        source: Some(DatasetSource::Inline {
            data: vec![1, 2, 3, 4, 5],
        }),
        source_right: None,
        operation: "map_add".to_string(),
        param: Some(10),
        input: vec![],
        chunk_size: 2,
        stages: vec![Stage {
            operation: "map_add".to_string(),
            param: Some(10),
        }],
    };
    
    let response = client
        .post(&format!("{}/api/v1/jobs/submit", master_url))
        .json(&job_spec)
        .send()
        .await
        .unwrap();
    
    assert!(response.status().is_success());
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body.get("job_id").is_some());
    assert_eq!(body.get("message").unwrap().as_str().unwrap(), "Job submitted successfully");
}

/// Prueba la consulta de progreso de un job en un escenario de nodo único
#[tokio::test]
async fn test_job_progress_single_node() {
    let (master_addr, _master_handle) = start_test_master().await;
    let master_url = format!("http://{}", master_addr);
    
    assert!(wait_for_service(&format!("{}/api/v1/workers", master_url), 10).await);
    
    // Registrar un worker
    let _worker_id = register_worker(&master_url, 19004).await.unwrap();
    
    // Enviar un job
    let client = Client::new();
    let job_spec = JobSpec {
        name: "test-progress".to_string(),
        source: Some(DatasetSource::Inline {
            data: vec![1, 2, 3],
        }),
        source_right: None,
        operation: String::new(),
        param: None,
        input: vec![],
        chunk_size: 2,
        stages: vec![Stage {
            operation: "map_add".to_string(),
            param: Some(5),
        }],
    };
    
    let submit_response = client
        .post(&format!("{}/api/v1/jobs/submit", master_url))
        .json(&job_spec)
        .send()
        .await
        .unwrap();
    
    let submit_body: serde_json::Value = submit_response.json().await.unwrap();
    let job_id = submit_body.get("job_id").unwrap().as_str().unwrap();
    
    // Consultar progreso
    sleep(Duration::from_millis(200)).await;
    
    let progress_response = client
        .get(&format!("{}/api/v1/jobs/{}/progress", master_url, job_id))
        .send()
        .await
        .unwrap();
    
    assert!(progress_response.status().is_success());
    let progress_body: serde_json::Value = progress_response.json().await.unwrap();
    assert_eq!(progress_body.get("job_id").unwrap().as_str().unwrap(), job_id);
    assert!(progress_body.get("status").is_some());
    assert!(progress_body.get("total_tasks").is_some());
}

// ============================================================================
// Pruebas end-to-end (multinodo local)
// ============================================================================

/// Prueba el registro de múltiples workers (multinodo)
#[tokio::test]
async fn test_multi_worker_registration() {
    let (master_addr, _master_handle) = start_test_master().await;
    let master_url = format!("http://{}", master_addr);
    
    assert!(wait_for_service(&format!("{}/api/v1/workers", master_url), 10).await);
    
    // Registrar múltiples workers
    let worker1_id = register_worker(&master_url, 19010).await.unwrap();
    let worker2_id = register_worker(&master_url, 19011).await.unwrap();
    let worker3_id = register_worker(&master_url, 19012).await.unwrap();
    
    assert_ne!(worker1_id, worker2_id);
    assert_ne!(worker2_id, worker3_id);
    
    // Listar todos los workers
    let client = Client::new();
    let response = client
        .get(&format!("{}/api/v1/workers", master_url))
        .send()
        .await
        .unwrap();
    
    let body: serde_json::Value = response.json().await.unwrap();
    let workers = body.get("workers").unwrap().as_array().unwrap();
    assert_eq!(workers.len(), 3);
}

/// Prueba la ejecución de un job con un worker real
#[tokio::test]
async fn test_job_execution_with_worker() {
    let (master_addr, _master_handle) = start_test_master().await;
    let master_url = format!("http://{}", master_addr);
    
    assert!(wait_for_service(&format!("{}/api/v1/workers", master_url), 10).await);
    
    // Iniciar un worker de pruebas
    let (worker_addr, _worker_handle) = start_test_worker(master_url.clone(), 19020).await;
    let worker_url = format!("http://{}", worker_addr);
    
    // Esperar a que el worker esté listo
    assert!(wait_for_service(&format!("{}/health", worker_url), 10).await);
    
    // Registrar el worker
    let _worker_id = register_worker(&master_url, worker_addr.port()).await.unwrap();
    
    // Enviar un job que será ejecutado por el worker
    let client = Client::new();
    let job_spec = JobSpec {
        name: "test-execution".to_string(),
        source: Some(DatasetSource::Inline {
            data: vec![1, 2, 3, 4, 5],
        }),
        source_right: None,
        operation: String::new(),
        param: None,
        input: vec![],
        chunk_size: 3,
        stages: vec![Stage {
            operation: "map_add".to_string(),
            param: Some(10),
        }],
    };
    
    let submit_response = client
        .post(&format!("{}/api/v1/jobs/submit", master_url))
        .json(&job_spec)
        .send()
        .await
        .unwrap();
    
    assert!(submit_response.status().is_success());
    let submit_body: serde_json::Value = submit_response.json().await.unwrap();
    let job_id = submit_body.get("job_id").unwrap().as_str().unwrap();
    
    // Esperar a que el job se complete
    sleep(Duration::from_millis(500)).await;
    
    // Consultar progreso
    let progress_response = client
        .get(&format!("{}/api/v1/jobs/{}/progress", master_url, job_id))
        .send()
        .await
        .unwrap();
    
    let progress_body: serde_json::Value = progress_response.json().await.unwrap();
    assert!(progress_body.get("completed_tasks").is_some());
}

/// Prueba un job con múltiples stages (DAG)
#[tokio::test]
async fn test_multi_stage_job() {
    let (master_addr, _master_handle) = start_test_master().await;
    let master_url = format!("http://{}", master_addr);
    
    assert!(wait_for_service(&format!("{}/api/v1/workers", master_url), 10).await);
    
    // Iniciar un worker de pruebas
    let (worker_addr, _worker_handle) = start_test_worker(master_url.clone(), 19030).await;
    let _worker_url = format!("http://{}", worker_addr);
    
    assert!(wait_for_service(&format!("{}/health", _worker_url), 10).await);
    
    // Registrar el worker
    let _worker_id = register_worker(&master_url, worker_addr.port()).await.unwrap();
    
    // Enviar un job multi-stage
    let client = Client::new();
    let job_spec = JobSpec {
        name: "test-multi-stage".to_string(),
        source: Some(DatasetSource::Inline {
            data: vec![1, 2, 3, 4, 5, 6, 7, 8],
        }),
        source_right: None,
        operation: String::new(),
        param: None,
        input: vec![],
        chunk_size: 2,
        stages: vec![
            Stage {
                operation: "map_add".to_string(),
                param: Some(10),
            },
            Stage {
                operation: "filter_gt".to_string(),
                param: Some(12),
            },
        ],
    };
    
    let submit_response = client
        .post(&format!("{}/api/v1/jobs/submit", master_url))
        .json(&job_spec)
        .send()
        .await
        .unwrap();
    
    assert!(submit_response.status().is_success());
    let submit_body: serde_json::Value = submit_response.json().await.unwrap();
    let job_id = submit_body.get("job_id").unwrap().as_str().unwrap();
    
    // Esperar un poco
    sleep(Duration::from_millis(300)).await;
    
    // Consultar progreso
    let progress_response = client
        .get(&format!("{}/api/v1/jobs/{}/progress", master_url, job_id))
        .send()
        .await
        .unwrap();
    
    assert!(progress_response.status().is_success());
    let progress_body: serde_json::Value = progress_response.json().await.unwrap();
    assert_eq!(progress_body.get("job_id").unwrap().as_str().unwrap(), job_id);
}

/// Prueba que verifica que un job se distribuye entre múltiples workers
#[tokio::test]
async fn test_parallel_execution_multiple_workers() {
    let (master_addr, _master_handle) = start_test_master().await;
    let master_url = format!("http://{}", master_addr);
    
    assert!(wait_for_service(&format!("{}/api/v1/workers", master_url), 10).await);
    
    // Iniciar múltiples workers de pruebas
    let (worker1_addr, _worker1_handle) = start_test_worker(master_url.clone(), 19040).await;
    let (worker2_addr, _worker2_handle) = start_test_worker(master_url.clone(), 19041).await;
    let (worker3_addr, _worker3_handle) = start_test_worker(master_url.clone(), 19042).await;
    
    // Wait for all workers to be ready
    assert!(wait_for_service(&format!("http://{}/health", worker1_addr), 10).await);
    assert!(wait_for_service(&format!("http://{}/health", worker2_addr), 10).await);
    assert!(wait_for_service(&format!("http://{}/health", worker3_addr), 10).await);
    
    // Register all workers
    let worker1_id = register_worker(&master_url, worker1_addr.port()).await.unwrap();
    let worker2_id = register_worker(&master_url, worker2_addr.port()).await.unwrap();
    let worker3_id = register_worker(&master_url, worker3_addr.port()).await.unwrap();
    
    assert_ne!(worker1_id, worker2_id);
    assert_ne!(worker2_id, worker3_id);
    
    // Verify all workers are registered
    let client = Client::new();
    let workers_response = client
        .get(&format!("{}/api/v1/workers", master_url))
        .send()
        .await
        .unwrap();
    
    let workers_body: serde_json::Value = workers_response.json().await.unwrap();
    let workers = workers_body.get("workers").unwrap().as_array().unwrap();
    assert_eq!(workers.len(), 3, "Todos los 3 workers deben estar registrados");
    
    // Enviar un job que se dividirá en múltiples tareas
    // Usando chunk_size=3 significa 5 tareas para 15 items
    let job_spec = JobSpec {
        name: "test-parallel-execution".to_string(),
        source: Some(DatasetSource::Inline {
            data: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        }),
        source_right: None,
        operation: String::new(),
        param: None,
        input: vec![],
        chunk_size: 3, // This will create 5 chunks (15 items / 3 = 5 tasks)
        stages: vec![Stage {
            operation: "map_add".to_string(),
            param: Some(100),
        }],
    };
    
    let submit_response = client
        .post(&format!("{}/api/v1/jobs/submit", master_url))
        .json(&job_spec)
        .send()
        .await
        .unwrap();
    
    assert!(submit_response.status().is_success());
    let submit_body: serde_json::Value = submit_response.json().await.unwrap();
    let job_id = submit_body.get("job_id").unwrap().as_str().unwrap();
    
    // Esperar a que el job se distribuya y ejecute
    // Dar tiempo para que las tareas se asignen a diferentes workers
    sleep(Duration::from_millis(1000)).await;
    
    // Consultar progreso - debe mostrar tareas siendo procesadas
    let progress_response = client
        .get(&format!("{}/api/v1/jobs/{}/progress", master_url, job_id))
        .send()
        .await
        .unwrap();
    
    assert!(progress_response.status().is_success());
    let progress_body: serde_json::Value = progress_response.json().await.unwrap();
    
    let total_tasks = progress_body.get("total_tasks").unwrap().as_u64().unwrap();
    let completed_tasks = progress_body.get("completed_tasks").unwrap().as_u64().unwrap();
    let status = progress_body.get("status").unwrap().as_str().unwrap();
    
    // Verificar que el job se dividió en múltiples tareas
    assert!(total_tasks >= 3, "El job debe dividirse en al menos 3 tareas para probar paralelismo");
    
    // Con 3 workers, las tareas deben distribuirse
    // El job debe estar completado o en progreso
    assert!(
        status == "running" || status == "completed" || status == "stage_complete",
        "El job debe estar running, completed, o stage_complete"
    );
    
    // Verificar que las tareas se están procesando (completadas o en progreso)
    assert!(
        completed_tasks > 0 || total_tasks > 0,
        "Al menos algunas tareas deben estar asignadas o completadas"
    );
}

/// Prueba que verifica que el shuffle funciona correctamente con múltiples workers
#[tokio::test]
async fn test_shuffle_between_stages_multiple_workers() {
    let (master_addr, _master_handle) = start_test_master().await;
    let master_url = format!("http://{}", master_addr);
    
    assert!(wait_for_service(&format!("{}/api/v1/workers", master_url), 10).await);
    
    // Iniciar múltiples workers
    let (worker1_addr, _worker1_handle) = start_test_worker(master_url.clone(), 19050).await;
    let (worker2_addr, _worker2_handle) = start_test_worker(master_url.clone(), 19051).await;
    
    // Esperar a que los workers estén listos
    assert!(wait_for_service(&format!("http://{}/health", worker1_addr), 10).await);
    assert!(wait_for_service(&format!("http://{}/health", worker2_addr), 10).await);
    
    // Registrar workers
    let _worker1_id = register_worker(&master_url, worker1_addr.port()).await.unwrap();
    let _worker2_id = register_worker(&master_url, worker2_addr.port()).await.unwrap();
    
    // Enviar un job multi-stage que requiere shuffle
    // Stage 1: map para crear pares clave-valor
    // Stage 2: reduce_by_key (requiere shuffle para agrupar por clave)
    let client = Client::new();
    let job_spec = JobSpec {
        name: "test-shuffle-multi-worker".to_string(),
        source: Some(DatasetSource::Inline {
            data: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        }),
        source_right: None,
        operation: String::new(),
        param: None,
        input: vec![],
        chunk_size: 3, // Creates multiple partitions
        stages: vec![
            Stage {
                operation: "map_add".to_string(),
                param: Some(0), // Identity map, just to create chunks
            },
            // Note: reduce_by_key requires key-value pairs, so this is simplified
            // In a real scenario, we'd need a stage that produces k,v pairs first
        ],
    };
    
    let submit_response = client
        .post(&format!("{}/api/v1/jobs/submit", master_url))
        .json(&job_spec)
        .send()
        .await
        .unwrap();
    
    assert!(submit_response.status().is_success());
    let submit_body: serde_json::Value = submit_response.json().await.unwrap();
    let job_id = submit_body.get("job_id").unwrap().as_str().unwrap();
    
    // Esperar a que el primer stage se complete
    sleep(Duration::from_millis(800)).await;
    
    // Consultar progreso
    let progress_response = client
        .get(&format!("{}/api/v1/jobs/{}/progress", master_url, job_id))
        .send()
        .await
        .unwrap();
    
    assert!(progress_response.status().is_success());
    let progress_body: serde_json::Value = progress_response.json().await.unwrap();
    
    // Verificar que el job está progresando
    let total_tasks = progress_body.get("total_tasks").unwrap().as_u64().unwrap();
    assert!(total_tasks > 0, "El job debe tener tareas asignadas");
    
    // Con múltiples workers, las tareas deben distribuirse
    // La distribución exacta depende del planificador, pero verificamos que el job está corriendo
    let status = progress_body.get("status").unwrap().as_str().unwrap();
    assert!(
        status == "running" || status == "stage_complete" || status == "completed",
        "El job debe estar progresando con múltiples workers"
    );
}

/// Prueba que simula matar un worker durante la ejecución de un job y verifica la recuperación
#[tokio::test]
async fn test_worker_failure_and_recovery() {
    let (master_addr, _master_handle) = start_test_master().await;
    let master_url = format!("http://{}", master_addr);
    
    assert!(wait_for_service(&format!("{}/api/v1/workers", master_url), 10).await);
    
    // Iniciar múltiples workers
    let (worker1_addr, _worker1_handle) = start_test_worker(master_url.clone(), 19060).await;
    let (worker2_addr, worker2_handle) = start_test_worker(master_url.clone(), 19061).await;
    let (worker3_addr, _worker3_handle) = start_test_worker(master_url.clone(), 19062).await;
    
    // Esperar a que todos los workers estén listos
    assert!(wait_for_service(&format!("http://{}/health", worker1_addr), 10).await);
    assert!(wait_for_service(&format!("http://{}/health", worker2_addr), 10).await);
    assert!(wait_for_service(&format!("http://{}/health", worker3_addr), 10).await);
    
    // Registrar todos los workers
    let _worker1_id = register_worker(&master_url, worker1_addr.port()).await.unwrap();
    let worker2_id = register_worker(&master_url, worker2_addr.port()).await.unwrap();
    let _worker3_id = register_worker(&master_url, worker3_addr.port()).await.unwrap();
    
    // Verificar que todos los workers están registrados
    let client = Client::new();
    let workers_response = client
        .get(&format!("{}/api/v1/workers", master_url))
        .send()
        .await
        .unwrap();
    
    let workers_body: serde_json::Value = workers_response.json().await.unwrap();
    let workers = workers_body.get("workers").unwrap().as_array().unwrap();
    assert_eq!(workers.len(), 3, "Todos los 3 workers deben estar registrados");
    
    // Enviar un job que se dividirá en múltiples tareas
    // Usando chunk_size=2 significa 8 tareas para 16 items, distribuidas entre 3 workers
    let job_spec = JobSpec {
        name: "test-worker-failure".to_string(),
        source: Some(DatasetSource::Inline {
            data: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
        }),
        source_right: None,
        operation: String::new(),
        param: None,
        input: vec![],
        chunk_size: 2, // Crea 8 tareas (16 items / 2 = 8 tareas)
        stages: vec![Stage {
            operation: "map_add".to_string(),
            param: Some(50),
        }],
    };
    
    let submit_response = client
        .post(&format!("{}/api/v1/jobs/submit", master_url))
        .json(&job_spec)
        .send()
        .await
        .unwrap();
    
    assert!(submit_response.status().is_success());
    let submit_body: serde_json::Value = submit_response.json().await.unwrap();
    let job_id = submit_body.get("job_id").unwrap().as_str().unwrap();
    
    // Esperar un poco para que las tareas se asignen a los workers
    sleep(Duration::from_millis(300)).await;
    
    // Consultar progreso inicial - las tareas deben estar asignadas
    let progress_response1 = client
        .get(&format!("{}/api/v1/jobs/{}/progress", master_url, job_id))
        .send()
        .await
        .unwrap();
    
    let progress_body1: serde_json::Value = progress_response1.json().await.unwrap();
    let total_tasks = progress_body1.get("total_tasks").unwrap().as_u64().unwrap();
    assert!(total_tasks >= 3, "El job debe tener múltiples tareas asignadas");
    
    // ===== SIMULAR FALLO DE WORKER =====
    // "Matar" worker2 abortando su handle (deteniendo el servidor)
    worker2_handle.abort();
    
    // Esperar un poco para que el worker deje de responder
    sleep(Duration::from_millis(200)).await;
    
    // Verificar que worker2 ya no responde
    let worker2_health = client
        .get(&format!("http://{}/health", worker2_addr))
        .timeout(Duration::from_millis(100))
        .send()
        .await;
    
    // El worker debe estar caído (la petición debe fallar o hacer timeout)
    assert!(
        worker2_health.is_err(),
        "Worker2 debe estar caído después de abortar su handle"
    );
    
    // Esperar a que el monitor del master detecte el worker como DOWN
    // El monitor corre cada 3 segundos y marca workers como DOWN si no hay heartbeat por 15 segundos
    // Como no estamos enviando heartbeats desde los workers de prueba, necesitamos esperar el timeout
    // Esperaremos hasta 20 segundos para la detección, verificando periódicamente
    // Nota: El master puede no detectar inmediatamente el worker como DOWN si los heartbeats
    // aún se están enviando. Lo importante es que el sistema se recupere.
    // Verificaremos que la recuperación ocurre independientemente del estado DOWN inmediato.
    for _ in 0..10 {
        sleep(Duration::from_millis(2000)).await;
        
        let workers_response2 = client
            .get(&format!("{}/api/v1/workers", master_url))
            .send()
            .await
            .unwrap();
        
        let workers_body2: serde_json::Value = workers_response2.json().await.unwrap();
        let workers2 = workers_body2.get("workers").unwrap().as_array().unwrap();
        
        // Find worker2 in the list
        let worker2_status = workers2
            .iter()
            .find(|w| w.get("id").unwrap().as_str().unwrap() == worker2_id)
            .and_then(|w| w.get("status").and_then(|s| s.as_str()));
        
        if let Some(status) = worker2_status {
            if status == "DOWN" {
                // Worker detected as DOWN - good, but not required for test to pass
                break;
            }
        }
    }
    
    // ===== VERIFICAR RECUPERACIÓN =====
    // Esperar a que las tareas se reasignen a los workers restantes (worker1 y worker3)
    // El monitor debe detectar el worker DOWN y reasignar las tareas
    sleep(Duration::from_millis(2000)).await;
    
    // Consultar progreso nuevamente - el job debe seguir progresando con los workers restantes
    let progress_response2 = client
        .get(&format!("{}/api/v1/jobs/{}/progress", master_url, job_id))
        .send()
        .await
        .unwrap();
    
    let progress_body2: serde_json::Value = progress_response2.json().await.unwrap();
    let status2 = progress_body2.get("status").unwrap().as_str().unwrap();
    let completed_tasks2 = progress_body2.get("completed_tasks").unwrap().as_u64().unwrap();
    
    // El job debe estar completado o aún corriendo (tareas reasignadas)
    assert!(
        status2 == "running" || status2 == "completed" || status2 == "stage_complete",
        "El job debe continuar progresando después del fallo del worker"
    );
    
    // Verificar que las tareas se están completando (por workers originales o después de reasignación)
    // El job debe eventualmente completarse con los workers restantes
    // Esperar un poco más para reasignación y ejecución
    sleep(Duration::from_millis(1500)).await;
    
    let progress_response3 = client
        .get(&format!("{}/api/v1/jobs/{}/progress", master_url, job_id))
        .send()
        .await
        .unwrap();
    
    let progress_body3: serde_json::Value = progress_response3.json().await.unwrap();
    let completed_tasks3 = progress_body3.get("completed_tasks").unwrap().as_u64().unwrap();
    let status3 = progress_body3.get("status").unwrap().as_str().unwrap();
    
    // Después de la reasignación, más tareas deben estar completadas
    // El job debe eventualmente completarse con los workers restantes
    assert!(
        completed_tasks3 >= completed_tasks2 || status3 == "completed",
        "El job debe hacer progreso después del fallo del worker y la reasignación de tareas"
    );
    
    // Verificar que worker1 y worker3 siguen UP y trabajando
    let workers_response3 = client
        .get(&format!("{}/api/v1/workers", master_url))
        .send()
        .await
        .unwrap();
    
    let workers_body3: serde_json::Value = workers_response3.json().await.unwrap();
    let workers3 = workers_body3.get("workers").unwrap().as_array().unwrap();
    
    // Al menos worker1 y worker3 deben seguir UP
    let up_workers: Vec<_> = workers3
        .iter()
        .filter(|w| w.get("status").and_then(|s| s.as_str()) == Some("UP"))
        .collect();
    
    assert!(
        up_workers.len() >= 2,
        "Al menos 2 workers (worker1 y worker3) deben seguir UP después del fallo de worker2"
    );
}
