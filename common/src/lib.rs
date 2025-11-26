//! Common types and utilities shared across the distributed system.

pub mod metrics;

use serde::{Deserialize, Serialize};

// Message versioning
pub const MESSAGE_VERSION: &str = "1.0";

/// Worker information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerInfo {
    pub id: String,
    pub host: String,
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_heartbeat: Option<u64>, // Unix timestamp in seconds
    pub status: WorkerStatus,
}

/// Worker status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum WorkerStatus {
    Up,
    Down,
}

impl WorkerStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkerStatus::Up => "UP",
            WorkerStatus::Down => "DOWN",
        }
    }
}

/// Register request from worker
#[derive(Debug, Deserialize, Serialize)]
pub struct RegisterRequest {
    pub version: Option<String>,
    pub host: String,
    pub port: u16,
}

/// Register response to worker
#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterResponse {
    pub version: String,
    pub worker_id: String,
    pub message: String,
}

/// Heartbeat request from worker
#[derive(Debug, Deserialize, Serialize)]
pub struct HeartbeatRequest {
    pub version: Option<String>,
    pub timestamp: Option<u64>,
}

/// Heartbeat response to worker
#[derive(Debug, Serialize, Deserialize)]
pub struct HeartbeatResponse {
    pub version: String,
    pub status: String,
}

/// Worker list item for GET /api/v1/workers response
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkerListItem {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub status: String,
    pub last_heartbeat: Option<u64>,
}

/// List workers response
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkersListResponse {
    pub version: String,
    pub workers: Vec<WorkerListItem>,
}

// ============================================================================
// Job/Task para semana 2 y 3
// ============================================================================

/// DAG Node - represents an operation in the DAG
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagNode {
    pub id: String,
    pub op: String, // "read_csv", "map", "flat_map", "filter", "reduce_by_key", "join", etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>, // For read operations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fn_name: Option<String>, // Function name (e.g., "tokenize", "to_lower", "sum")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>, // For reduce_by_key, join operations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partitions: Option<usize>, // Number of partitions
}

/// DAG Edge - represents dependency between nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagEdge(pub String, pub String); // (from, to)

/// DAG structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dag {
    pub nodes: Vec<DagNode>,
    pub edges: Vec<DagEdge>,
}

/// Job submission request - supports both old format and new DAG format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSpec {
    /// Job name/description
    pub name: String,
    /// DAG structure (new format according to enunciado)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dag: Option<Dag>,
    /// Parallelism level
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallelism: Option<usize>,
    // Legacy fields for backward compatibility
    /// Operation: "map_add", "map_mul", "filter_gt", "filter_lt" (legacy)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,
    /// Parameter for the operation (legacy)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param: Option<i64>,
    /// Input data (vector of integers) (legacy)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<Vec<i64>>,
}

/// Response from job submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitJobResponse {
    pub version: String,
    pub job_id: String,
    pub message: String,
}

/// Task assigned to a worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAssignment {
    pub job_id: String,
    pub task_id: String,
    pub node_id: String, // ID of the DAG node this task belongs to
    pub operation: String, // Operation type: "map", "flat_map", "filter", "reduce_by_key", "join", etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fn_name: Option<String>, // Function name for the operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>, // Key field for reduce_by_key, join
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param: Option<i64>, // Legacy parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<Vec<i64>>, // Input data (legacy format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_path: Option<String>, // Input file path (for read operations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt_id: Option<usize>, // Task attempt for idempotency (0 = first attempt, 1+ = retries)
}

/// Task result returned by worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub job_id: String,
    pub task_id: String,
    pub node_id: String, // ID of the DAG node
    pub attempt_id: usize, // Task attempt number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Vec<i64>>, // Output data (legacy format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_path: Option<String>, // Output file path (for write operations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_data: Option<serde_json::Value>, // Generic output data (for complex types)
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>, // Error message if task failed
}

/// Job progress/status (used by master)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobProgress {
    pub job_id: String,
    pub name: String,
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub status: String, // "running", "completed", "failed"
}

#[cfg(test)]
mod tests {
    // puedes agregar tests útiles luego,
    // pero quita por ahora el test con Message inexistente
}
