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
// Job and Task Types (Batch DAG)
// ============================================================================

/// Job status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum JobStatus {
    Accepted,
    Running,
    Failed,
    Succeeded,
}

impl JobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobStatus::Accepted => "ACCEPTED",
            JobStatus::Running => "RUNNING",
            JobStatus::Failed => "FAILED",
            JobStatus::Succeeded => "SUCCEEDED",
        }
    }
}

/// Operator types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Operator {
    ReadCsv { path: String, partitions: u32 },
    ReadJsonl { path: String, partitions: u32 },
    Map { fn_name: String },
    FlatMap { fn_name: String },
    Filter { fn_name: String },
    Reduce { fn_name: String },
    ReduceByKey { key: String, fn_name: String },
    Join { key: String, other_collection: String },
    Shuffle { key: String },
    WriteCsv { path: String },
    WriteJsonl { path: String },
}

/// DAG node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagNode {
    pub id: String,
    #[serde(flatten)]
    pub operator: Operator,
}

/// DAG edge (from -> to)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagEdge(pub String, pub String);

/// DAG structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dag {
    pub nodes: Vec<DagNode>,
    pub edges: Vec<DagEdge>,
}

/// Job submission request
#[derive(Debug, Deserialize, Serialize)]
pub struct JobSubmitRequest {
    pub name: String,
    pub dag: Dag,
    pub parallelism: u32,
}

/// Job submission response
#[derive(Debug, Serialize, Deserialize)]
pub struct JobSubmitResponse {
    pub version: String,
    pub job_id: String,
    pub message: String,
}

/// Job status response
#[derive(Debug, Serialize, Deserialize)]
pub struct JobStatusResponse {
    pub version: String,
    pub job_id: String,
    pub status: String,
    pub progress: f64, // 0.0 to 100.0
    pub metrics: JobMetrics,
}

/// Job metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JobMetrics {
    pub total_time_secs: Option<f64>,
    pub stages_completed: u32,
    pub stages_total: u32,
    pub failures: u32,
    pub retries: u32,
}

/// Job results response
#[derive(Debug, Serialize, Deserialize)]
pub struct JobResultsResponse {
    pub version: String,
    pub job_id: String,
    pub output_paths: Vec<String>,
}

// ============================================================================
// Task Types
// ============================================================================

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TaskStatus {
    Pending,
    Assigned,
    Running,
    Completed,
    Failed,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "PENDING",
            TaskStatus::Assigned => "ASSIGNED",
            TaskStatus::Running => "RUNNING",
            TaskStatus::Completed => "COMPLETED",
            TaskStatus::Failed => "FAILED",
        }
    }
}

/// Task assignment from master to worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAssignment {
    pub task_id: String,
    pub job_id: String,
    pub attempt_id: u32, // For idempotency
    pub stage_id: String,
    pub operator: Operator,
    pub input_paths: Vec<String>,
    pub output_path: String,
    pub partition: u32,
}

/// Task status update from worker to master
#[derive(Debug, Serialize, Deserialize)]
pub struct TaskStatusUpdate {
    pub version: Option<String>,
    pub task_id: String,
    pub attempt_id: u32,
    pub status: String,
    pub output_path: Option<String>,
    pub error: Option<String>,
    pub metrics: TaskMetrics,
}

/// Task metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskMetrics {
    pub execution_time_secs: f64,
    pub records_processed: u64,
    pub memory_used_mb: f64,
}

/// Task request from master to worker
#[derive(Debug, Serialize, Deserialize)]
pub struct TaskRequest {
    pub version: String,
    pub assignment: TaskAssignment,
}

/// Task response from worker to master
#[derive(Debug, Serialize, Deserialize)]
pub struct TaskResponse {
    pub version: String,
    pub task_id: String,
    pub attempt_id: u32,
    pub status: String,
    pub output_path: Option<String>,
    pub error: Option<String>,
    pub metrics: TaskMetrics,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_status_serialization() {
        let status = JobStatus::Running;
        assert_eq!(status.as_str(), "RUNNING");
    }

    #[test]
    fn test_operator_serialization() {
        let op = Operator::Map {
            fn_name: "to_lower".to_string(),
        };
        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains("map"));
        assert!(json.contains("to_lower"));
    }
}

