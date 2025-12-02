//! Estructuras para métricas del sistema

use serde::{Deserialize, Serialize};

/// Métricas de un nodo (worker)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetrics {
    pub node_id: String,
    pub cpu_usage_percent: f64,        // Porcentaje de CPU (0-100)
    pub memory_usage_mb: f64,          // Uso de memoria en MB
    pub active_tasks: usize,           // Número de tareas activas
    pub avg_latency_ms: f64,           // Latencia promedio en milisegundos
    pub retry_count: usize,            // Número de reintentos
    pub timestamp: u64,                // Timestamp Unix
}

/// Métricas de un job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobMetrics {
    pub job_id: String,
    pub name: String,
    pub total_time_secs: f64,          // Tiempo total en segundos
    pub stages: usize,                 // Número de etapas
    #[serde(skip_serializing_if = "Option::is_none")]
    pub throughput_events_per_sec: Option<f64>, // Para streaming
    pub failure_count: usize,          // Número de fallos
    pub start_time: u64,               // Timestamp Unix de inicio
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<u64>,         // Timestamp Unix de fin
}

/// Respuesta con todas las métricas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub node_metrics: Vec<NodeMetrics>,
    pub job_metrics: Vec<JobMetrics>,
}

