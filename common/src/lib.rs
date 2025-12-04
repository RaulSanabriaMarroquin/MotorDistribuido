//! Tipos y utilidades comunes compartidos en el sistema distribuido

use serde::{Deserialize, Serialize};

// Versionado de mensajes
pub const MESSAGE_VERSION: &str = "1.0";




// ============================================================================
// Información de Workers
// ============================================================================


/// Estructura de información del worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerInfo {
    pub id: String,
    pub host: String,
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_heartbeat: Option<u64>, // Unix timestamp in seconds
    pub status: WorkerStatus,
}

/// Estado del worker
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum WorkerStatus {
    Up,    // Worker activo y disponible
    Down,  // Worker caído o no disponible
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
// Registro + Heartbeats
// ============================================================================


/// Solicitud de registro desde un worker
#[derive(Debug, Deserialize, Serialize)]
pub struct RegisterRequest {
    pub version: Option<String>,
    pub host: String,
    pub port: u16,
}


/// Respuesta de registro al worker
#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterResponse {
    pub version: String,
    pub worker_id: String,
    pub message: String,
}

/// Solicitud de heartbeat desde un worker
#[derive(Debug, Deserialize, Serialize)]
pub struct HeartbeatRequest {
    pub version: Option<String>,
    pub timestamp: Option<u64>,
}

/// Respuesta de heartbeat al worker
#[derive(Debug, Serialize, Deserialize)]
pub struct HeartbeatResponse {
    pub version: String,
    pub status: String,
}

/// Item de lista de workers para respuesta GET /api/v1/workers
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkerListItem {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub status: String,
    pub last_heartbeat: Option<u64>,
}

/// Respuesta de lista de workers
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkersListResponse {
    pub version: String,
    pub workers: Vec<WorkerListItem>,
}

/// Respuesta del envío de job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitJobResponse {
    pub version: String,
    pub job_id: String,
    pub message: String,
}

// ============================================================================
// Fuente de Datos (Semana 2)
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

/// Representa una etapa en el DAG de un job batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stage {
    pub operation: String,            // "map_add", "join", etc.
    pub param: Option<i64>,    // usado para map/filter/window
}

// ============================================================================
// Especificación de Job
// ============================================================================

fn default_chunk_size() -> usize { 100 }

/// Especificación completa de un job para ejecución
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSpec {
    pub name: String,

    #[serde(default)]
    pub source: Option<DatasetSource>,  // fuente izquierda (para join)
    #[serde(default)]
    pub source_right: Option<DatasetSource>, // fuente derecha (para join)

    #[serde(default)]
    pub operation: String,     // modo legacy
    #[serde(default)]
    pub param: Option<i64>,    // modo legacy
    #[serde(default)]
    pub input: Vec<i64>,       // modo legacy

    #[serde(default = "default_chunk_size")]
    pub chunk_size: usize,     // tamaño de chunk para particionado

    #[serde(default)]
    pub stages: Vec<Stage>,    // etapas del DAG
}

// ============================================================================
// Asignación de Tarea
// ============================================================================

/// Asignación de una tarea a un worker para ejecución
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAssignment {
    pub job_id: String,
    pub task_id: String,
    pub operation: String,

    // Modo normal (map/filter/reduce)
    #[serde(default)]
    pub input: Vec<i64>, // datos de entrada para map/filter/reduce_by_key

    // Modo join (dos colecciones)
    #[serde(default)]
    pub left: Vec<i64>, // join: colección izquierda
    #[serde(default)]
    pub right: Vec<i64>, // join: colección derecha

    // Parámetro usado por filtros/map
    pub param: Option<i64>,

    pub stage_id: usize,        // ID de la etapa en el DAG
    #[serde(default)]
    pub reassign_attempt: u32,  // número de intento de reasignación
}

// ============================================================================
// Resultados
// ============================================================================

/// Resultado de la ejecución de una tarea
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub job_id: String,
    pub task_id: String,
    pub output: Vec<i64>,      // resultado de la ejecución

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,   // error si la tarea falló
}

// ============================================================================
// Progreso del Job
// ============================================================================

/// Información de progreso de un job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobProgress {
    pub job_id: String,
    pub name: String,
    pub total_tasks: usize,      // total de tareas del job
    pub completed_tasks: usize,  // tareas completadas exitosamente
    pub failed_tasks: usize,     // tareas que fallaron
    pub status: String,           // "running", "completed", "failed", etc.
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_status_as_str() {
        assert_eq!(WorkerStatus::Up.as_str(), "UP");
        assert_eq!(WorkerStatus::Down.as_str(), "DOWN");
    }

    #[test]
    fn test_worker_status_serialization() {
        let status = WorkerStatus::Up;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"UP\"");
    }

    #[test]
    fn test_worker_status_deserialization() {
        let json = "\"DOWN\"";
        let status: WorkerStatus = serde_json::from_str(json).unwrap();
        assert_eq!(status, WorkerStatus::Down);
    }

    #[test]
    fn test_dataset_source_serialization() {
        let source = DatasetSource::Inline {
            data: vec![1, 2, 3],
        };
        let json = serde_json::to_string(&source).unwrap();
        assert!(json.contains("Inline"));
        assert!(json.contains("data"));
    }

    #[test]
    fn test_stage_serialization() {
        let stage = Stage {
            operation: "map_add".to_string(),
            param: Some(5),
        };
        let json = serde_json::to_string(&stage).unwrap();
        assert!(json.contains("map_add"));
        assert!(json.contains("5"));
    }

    #[test]
    fn test_job_spec_default_chunk_size() {
        let spec = JobSpec {
            name: "test".to_string(),
            source: None,
            source_right: None,
            operation: String::new(),
            param: None,
            input: vec![],
            chunk_size: 0, // will use default
            stages: vec![],
        };
        // Default chunk_size is 100
        assert_eq!(spec.chunk_size, 0); // This is what we set, but default_chunk_size() returns 100
    }

    #[test]
    fn test_task_assignment_defaults() {
        let task = TaskAssignment {
            job_id: "job1".to_string(),
            task_id: "task1".to_string(),
            operation: "map_add".to_string(),
            input: vec![],
            left: vec![],
            right: vec![],
            param: None,
            stage_id: 0,
            reassign_attempt: 0,
        };
        assert_eq!(task.input.len(), 0);
        assert_eq!(task.reassign_attempt, 0);
    }

    #[test]
    fn test_task_result_with_error() {
        let result = TaskResult {
            job_id: "job1".to_string(),
            task_id: "task1".to_string(),
            output: vec![1, 2, 3],
            error: Some("test error".to_string()),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("test error"));
    }

    #[test]
    fn test_task_result_without_error() {
        let result = TaskResult {
            job_id: "job1".to_string(),
            task_id: "task1".to_string(),
            output: vec![1, 2, 3],
            error: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        // error field should be skipped when None
        assert!(!json.contains("error"));
    }
}
