//! Biblioteca común compartida entre master, worker y client
//! Define estructuras de datos y constantes compartidas

pub mod metrics;

pub use metrics::{JobMetrics, MetricsResponse, NodeMetrics};

use serde::{Deserialize, Serialize};

/// Versión del protocolo de mensajes
pub const MESSAGE_VERSION: &str = "1.0";

// ============================================================================
// Estructuras DAG
// ============================================================================

/// Nodo en un DAG
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagNode {
    pub id: String,
    pub op: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fn_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partitions: Option<usize>,
}

/// Arista en un DAG (from, to)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct DagEdge(pub String, pub String);

/// Estructura completa de un DAG
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dag {
    pub nodes: Vec<DagNode>,
    pub edges: Vec<DagEdge>,
}

// ============================================================================
// Estructuras de Jobs
// ============================================================================

/// Especificación de un job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSpec {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fn_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dag: Option<Dag>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallelism: Option<usize>,
}

/// Respuesta al enviar un job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitJobResponse {
    pub job_id: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Progreso de un job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobProgress {
    pub job_id: String,
    pub status: String, // "running", "completed", "failed"
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

// ============================================================================
// Estructuras de Workers
// ============================================================================

/// Estado de un worker
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum WorkerStatus {
    Up,
    Down,
}

impl WorkerStatus {
    pub fn as_str(&self) -> &str {
        match self {
            WorkerStatus::Up => "UP",
            WorkerStatus::Down => "DOWN",
        }
    }
}

/// Información de un worker para listado
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerListItem {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub status: WorkerStatus,
    pub active_tasks: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_heartbeat: Option<u64>,
}

/// Respuesta al listar workers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkersListResponse {
    pub workers: Vec<WorkerListItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

// ============================================================================
// Estructuras de Registro y Heartbeat
// ============================================================================

/// Solicitud de registro de worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub worker_id: String,
    pub host: String,
    pub port: u16,
    pub version: String,
}

/// Respuesta al registro de worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterResponse {
    pub success: bool,
    pub message: String,
    pub worker_id: String,
    pub version: String,
}

/// Solicitud de heartbeat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatRequest {
    pub worker_id: String,
    pub active_tasks: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
}

/// Respuesta al heartbeat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

// ============================================================================
// Estructuras de Tareas
// ============================================================================

/// Asignación de tarea a un worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAssignment {
    pub job_id: String,
    pub task_id: String,
    pub node_id: String,
    pub operation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fn_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_data: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_paths: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input2: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_path2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt_id: Option<usize>,
}

/// Resultado de una tarea
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub job_id: String,
    pub task_id: String,
    pub attempt_id: usize,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

