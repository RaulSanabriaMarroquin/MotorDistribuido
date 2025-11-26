//! Metrics structures for observability

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Node-level metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetrics {
    pub node_id: String,
    pub cpu_usage_percent: f64, // Approximate CPU usage
    pub memory_usage_mb: f64,  // Memory usage in MB
    pub active_tasks: usize,   // Number of active tasks
    pub avg_latency_ms: f64,    // Average task latency in milliseconds
    pub retry_count: usize,     // Number of retries
    pub timestamp: u64,         // Unix timestamp
}

/// Job/Topology-level metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobMetrics {
    pub job_id: String,
    pub name: String,
    pub total_time_secs: f64,   // Total execution time
    pub stages: usize,           // Number of stages
    pub throughput_events_per_sec: Option<f64>, // For streaming
    pub failure_count: usize,    // Number of failures
    pub start_time: u64,         // Unix timestamp
    pub end_time: Option<u64>,   // Unix timestamp (None if still running)
}

/// Metrics response
#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub version: String,
    pub node_metrics: Option<Vec<NodeMetrics>>,
    pub job_metrics: Option<Vec<JobMetrics>>,
}

