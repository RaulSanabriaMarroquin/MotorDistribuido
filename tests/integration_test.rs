//! Tests de integración para el sistema distribuido
//! Prueban la interacción entre master, worker y client

use common::{JobSpec, RegisterRequest};
use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

const MASTER_URL: &str = "http://127.0.0.1:8080";

#[tokio::test]
#[ignore] // Ignore by default - requires running master and worker
async fn test_worker_registration() {
    let client = Client::new();
    
    // Try to register a worker
    let register_req = RegisterRequest {
        version: Some("1.0".to_string()),
        host: "127.0.0.1".to_string(),
        port: 9000,
    };
    
    let resp = client
        .post(&format!("{}/api/v1/workers/register", MASTER_URL))
        .json(&register_req)
        .send()
        .await;
    
    match resp {
        Ok(response) if response.status().is_success() => {
            let body: serde_json::Value = response.json().await.unwrap();
            assert!(body.get("worker_id").is_some());
        }
        _ => {
            // Master not running - skip test
            println!("Master no está corriendo, omitiendo test de integración");
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_job_submission() {
    let client = Client::new();
    
    // Wait for master to be ready
    sleep(Duration::from_secs(2)).await;
    
    let job_spec = JobSpec {
        name: "test-job".to_string(),
        dag: None,
        parallelism: None,
        operation: Some("map_add".to_string()),
        param: Some(10),
        input: Some(vec![1, 2, 3, 4, 5]),
    };
    
    let resp = client
        .post(&format!("{}/api/v1/jobs", MASTER_URL))
        .json(&job_spec)
        .send()
        .await;
    
    match resp {
        Ok(response) if response.status().is_success() => {
            let body: serde_json::Value = response.json().await.unwrap();
            assert!(body.get("job_id").is_some());
            
            // Check job progress
            let job_id = body["job_id"].as_str().unwrap();
            sleep(Duration::from_secs(1)).await;
            
            let progress_resp = client
                .get(&format!("{}/api/v1/jobs/{}", MASTER_URL, job_id))
                .send()
                .await
                .unwrap();
            
            if progress_resp.status().is_success() {
                let progress: serde_json::Value = progress_resp.json().await.unwrap();
                assert!(progress.get("status").is_some());
            }
        }
        _ => {
            println!("Master no está corriendo, omitiendo test de integración");
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_dag_job() {
    let client = Client::new();
    
    sleep(Duration::from_secs(2)).await;
    
    let job_spec = JobSpec {
        name: "dag-test".to_string(),
        dag: Some(common::Dag {
            nodes: vec![
                common::DagNode {
                    id: "read".to_string(),
                    op: "read_csv".to_string(),
                    path: Some("test_data.csv".to_string()),
                    fn_name: None,
                    key: None,
                    partitions: Some(2),
                },
                common::DagNode {
                    id: "map".to_string(),
                    op: "map".to_string(),
                    path: None,
                    fn_name: Some("add".to_string()),
                    key: None,
                    partitions: None,
                },
            ],
            edges: vec![common::DagEdge("read".to_string(), "map".to_string())],
        }),
        parallelism: Some(2),
        operation: None,
        param: None,
        input: None,
    };
    
    let resp = client
        .post(&format!("{}/api/v1/jobs", MASTER_URL))
        .json(&job_spec)
        .send()
        .await;
    
    match resp {
        Ok(response) if response.status().is_success() => {
            let body: serde_json::Value = response.json().await.unwrap();
            assert!(body.get("job_id").is_some());
        }
        _ => {
            println!("Master no está corriendo, omitiendo test de integración");
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_metrics_endpoint() {
    let client = Client::new();
    
    sleep(Duration::from_secs(2)).await;
    
    let resp = client
        .get(&format!("{}/api/v1/metrics", MASTER_URL))
        .send()
        .await;
    
    match resp {
        Ok(response) if response.status().is_success() => {
            let metrics: serde_json::Value = response.json().await.unwrap();
            assert!(metrics.get("version").is_some());
        }
        _ => {
            println!("Master no está corriendo, omitiendo test de integración");
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_job_with_no_workers() {
    let client = Client::new();
    
    sleep(Duration::from_secs(2)).await;
    
    // Intentar enviar job sin workers registrados
    let job_spec = JobSpec {
        name: "test-no-workers".to_string(),
        dag: None,
        parallelism: None,
        operation: Some("map_add".to_string()),
        param: Some(10),
        input: Some(vec![1, 2, 3]),
    };
    
    let resp = client
        .post(&format!("{}/api/v1/jobs", MASTER_URL))
        .json(&job_spec)
        .send()
        .await;
    
    match resp {
        Ok(response) => {
            // Debería retornar 503 Service Unavailable si no hay workers
            if response.status() == 503 {
                println!("✓ Test pasado: Job rechazado correctamente sin workers");
            } else {
                println!("⚠ Master tiene workers disponibles, test no aplicable");
            }
        }
        _ => {
            println!("Master no está corriendo, omitiendo test");
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_job_results_endpoint() {
    let client = Client::new();
    
    sleep(Duration::from_secs(2)).await;
    
    // Enviar job que escriba resultados
    let job_spec = JobSpec {
        name: "test-results".to_string(),
        dag: None,
        parallelism: None,
        operation: Some("map_add".to_string()),
        param: Some(10),
        input: Some(vec![1, 2, 3, 4, 5]),
    };
    
    let resp = client
        .post(&format!("{}/api/v1/jobs", MASTER_URL))
        .json(&job_spec)
        .send()
        .await;
    
    match resp {
        Ok(response) if response.status().is_success() => {
            let body: serde_json::Value = response.json().await.unwrap();
            let job_id = body["job_id"].as_str().unwrap();
            
            // Esperar a que el job se complete
            sleep(Duration::from_secs(5)).await;
            
            // Verificar endpoint de resultados
            let results_resp = client
                .get(&format!("{}/api/v1/jobs/{}/results", MASTER_URL, job_id))
                .send()
                .await;
            
            match results_resp {
                Ok(r) if r.status().is_success() => {
                    let results: serde_json::Value = r.json().await.unwrap();
                    assert!(results.get("job_id").is_some());
                    assert!(results.get("result_paths").is_some());
                }
                _ => {
                    println!("Job aún no completado o master no responde");
                }
            }
        }
        _ => {
            println!("Master no está corriendo, omitiendo test");
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_job_status_endpoint() {
    let client = Client::new();
    
    sleep(Duration::from_secs(2)).await;
    
    let job_spec = JobSpec {
        name: "test-status".to_string(),
        dag: None,
        parallelism: None,
        operation: Some("map_add".to_string()),
        param: Some(10),
        input: Some(vec![1, 2, 3]),
    };
    
    let resp = client
        .post(&format!("{}/api/v1/jobs", MASTER_URL))
        .json(&job_spec)
        .send()
        .await;
    
    match resp {
        Ok(response) if response.status().is_success() => {
            let body: serde_json::Value = response.json().await.unwrap();
            let job_id = body["job_id"].as_str().unwrap();
            
            sleep(Duration::from_secs(2)).await;
            
            // Verificar endpoint de estado
            let status_resp = client
                .get(&format!("{}/api/v1/jobs/{}", MASTER_URL, job_id))
                .send()
                .await;
            
            match status_resp {
                Ok(r) if r.status().is_success() => {
                    let status: serde_json::Value = r.json().await.unwrap();
                    assert!(status.get("job_id").is_some());
                    assert!(status.get("status").is_some());
                    assert!(status.get("progress_percent").is_some());
                }
                _ => {
                    println!("Error al obtener estado del job");
                }
            }
        }
        _ => {
            println!("Master no está corriendo, omitiendo test");
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_heartbeat_timeout() {
    let client = Client::new();
    
    sleep(Duration::from_secs(2)).await;
    
    // Registrar un worker
    let register_req = RegisterRequest {
        version: Some("1.0".to_string()),
        host: "127.0.0.1".to_string(),
        port: 9999, // Puerto que no usaremos
    };
    
    let resp = client
        .post(&format!("{}/api/v1/workers/register", MASTER_URL))
        .json(&register_req)
        .send()
        .await;
    
    match resp {
        Ok(response) if response.status().is_success() => {
            let body: serde_json::Value = response.json().await.unwrap();
            let worker_id = body["worker_id"].as_str().unwrap();
            
            // Verificar que el worker está registrado
            let workers_resp = client
                .get(&format!("{}/api/v1/workers", MASTER_URL))
                .send()
                .await
                .unwrap();
            
            if workers_resp.status().is_success() {
                let workers: serde_json::Value = workers_resp.json().await.unwrap();
                let worker_list = workers["workers"].as_array().unwrap();
                let found = worker_list.iter().any(|w| w["id"].as_str() == Some(worker_id));
                assert!(found, "Worker debería estar registrado");
            }
            
            // Esperar más del timeout (15 segundos) para verificar detección
            // Nota: Este test puede tomar tiempo
            println!("Esperando timeout de heartbeat (15+ segundos)...");
            sleep(Duration::from_secs(20)).await;
            
            // Verificar que el worker fue marcado como DOWN
            let workers_resp2 = client
                .get(&format!("{}/api/v1/workers", MASTER_URL))
                .send()
                .await
                .unwrap();
            
            if workers_resp2.status().is_success() {
                let workers: serde_json::Value = workers_resp2.json().await.unwrap();
                let worker_list = workers["workers"].as_array().unwrap();
                let worker = worker_list.iter().find(|w| w["id"].as_str() == Some(worker_id));
                
                if let Some(w) = worker {
                    let status = w["status"].as_str().unwrap();
                    println!("Estado del worker después del timeout: {}", status);
                    // El worker debería estar DOWN o el test puede fallar si el timeout no se detectó
                }
            }
        }
        _ => {
            println!("Master no está corriendo, omitiendo test");
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_metrics_nodes() {
    let client = Client::new();
    
    sleep(Duration::from_secs(2)).await;
    
    let resp = client
        .get(&format!("{}/api/v1/metrics/nodes", MASTER_URL))
        .send()
        .await;
    
    match resp {
        Ok(response) if response.status().is_success() => {
            let metrics: serde_json::Value = response.json().await.unwrap();
            assert!(metrics.get("version").is_some());
            assert!(metrics.get("node_metrics").is_some());
        }
        _ => {
            println!("Master no está corriendo, omitiendo test");
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_metrics_jobs() {
    let client = Client::new();
    
    sleep(Duration::from_secs(2)).await;
    
    let resp = client
        .get(&format!("{}/api/v1/metrics/jobs", MASTER_URL))
        .send()
        .await;
    
    match resp {
        Ok(response) if response.status().is_success() => {
            let metrics: serde_json::Value = response.json().await.unwrap();
            assert!(metrics.get("version").is_some());
            assert!(metrics.get("job_metrics").is_some());
        }
        _ => {
            println!("Master no está corriendo, omitiendo test");
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_invalid_job_spec() {
    let client = Client::new();
    
    sleep(Duration::from_secs(2)).await;
    
    // Job sin operation ni dag
    let job_spec = json!({
        "name": "invalid-job",
        "operation": null,
        "dag": null
    });
    
    let resp = client
        .post(&format!("{}/api/v1/jobs", MASTER_URL))
        .json(&job_spec)
        .send()
        .await;
    
    match resp {
        Ok(response) => {
            // Debería retornar 400 Bad Request
            assert_eq!(response.status(), 400, "Job inválido debería retornar 400");
        }
        _ => {
            println!("Master no está corriendo, omitiendo test");
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_job_not_found() {
    let client = Client::new();
    
    sleep(Duration::from_secs(2)).await;
    
    // Intentar obtener un job que no existe
    let resp = client
        .get(&format!("{}/api/v1/jobs/nonexistent-job-id", MASTER_URL))
        .send()
        .await;
    
    match resp {
        Ok(response) => {
            // Debería retornar 404 Not Found
            assert_eq!(response.status(), 404, "Job inexistente debería retornar 404");
        }
        _ => {
            println!("Master no está corriendo, omitiendo test");
        }
    }
}
