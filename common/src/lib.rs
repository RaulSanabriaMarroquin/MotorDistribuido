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

#[cfg(test)]
mod tests {
    // puedes agregar tests útiles luego,
    // pero quita por ahora el test con Message inexistente
}

