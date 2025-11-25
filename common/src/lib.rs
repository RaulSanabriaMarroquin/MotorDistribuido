//! Common types and utilities shared across the distributed system.

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
// Job/Task para semana 2
// ============================================================================

/// Job submission request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSpec {
    /// Job name/description
    pub name: String,
    /// Operation: "map_add", "map_mul", "filter_gt", "filter_lt"
    pub operation: String,
    /// Parameter for the operation (e.g., factor for map_mul, threshold for filter_gt)
    pub param: Option<i64>,
    /// Input data (vector of integers)
    pub input: Vec<i64>,
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
    pub operation: String,
    pub param: Option<i64>,
    pub input: Vec<i64>,
}

/// Task result returned by worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub job_id: String,
    pub task_id: String,
    pub output: Vec<i64>,
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
