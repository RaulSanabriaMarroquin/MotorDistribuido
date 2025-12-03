//! Common types and utilities shared across the distributed system.

use serde::{Deserialize, Serialize};

// Message versioning
pub const MESSAGE_VERSION: &str = "1.0";




// ============================================================================
// Worker Info
// ============================================================================


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



// ============================================================================
// Registration + Heartbeats
// ============================================================================


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

/// Response from job submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitJobResponse {
    pub version: String,
    pub job_id: String,
    pub message: String,
}

// ============================================================================
// Dataset Source (Semana 2)
// ============================================================================


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DatasetSource {
    Inline { data: Vec<i64> },
    File { path: String },
    Csv { path: String },
    Jsonl { path: String },
}

// ============================================================================
// Stage (DAG Batch)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stage {
    pub operation: String,            // "map_add", "join", etc.
    pub param: Option<i64>,    // used for map/filter/window
}

// ============================================================================
// Job Specification
// ============================================================================

fn default_chunk_size() -> usize { 100 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSpec {
    pub name: String,

    #[serde(default)]
    pub source: Option<DatasetSource>,  // left
    #[serde(default)]
    pub source_right: Option<DatasetSource>, // right (join)

    #[serde(default)]
    pub operation: String,     // legacy mode
    #[serde(default)]
    pub param: Option<i64>,    // legacy mode
    #[serde(default)]
    pub input: Vec<i64>,       // legacy mode

    #[serde(default = "default_chunk_size")]
    pub chunk_size: usize,

    #[serde(default)]
    pub stages: Vec<Stage>,
}

// ============================================================================
// Task Assignment
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAssignment {
    pub job_id: String,
    pub task_id: String,
    pub operation: String,

    // modo normal
    #[serde(default)]
    pub input: Vec<i64>, // map/filter/reduce_by_key

    // modo join (dos colecciones)
    #[serde(default)]
    pub left: Vec<i64>, // join: colección A
    #[serde(default)]
    pub right: Vec<i64>, // join: colección B

    // usado por filtros/map
    pub param: Option<i64>,

    pub stage_id: usize,
    #[serde(default)]
    pub reassign_attempt: u32,
}

// ============================================================================
// Resultados
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub job_id: String,
    pub task_id: String,
    pub output: Vec<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ============================================================================
// Job Progress
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobProgress {
    pub job_id: String,
    pub name: String,
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub status: String,
}


#[cfg(test)]
mod tests {
    // puedes agregar tests útiles luego,
    // pero quita por ahora el test con Message inexistente
}
