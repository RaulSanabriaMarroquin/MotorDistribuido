//! Persistence layer for jobs and tasks using SQLite

use common::{JobMetrics, JobStatus, Operator, TaskMetrics, TaskStatus};
use rusqlite::{params, Connection, Result as SqliteResult};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::info;

const DB_PATH: &str = "master_state.db";

/// Initialize the database and create tables if they don't exist
pub fn init_db() -> SqliteResult<Connection> {
    let conn = Connection::open(DB_PATH)?;
    
    // Create jobs table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS jobs (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            status TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            started_at INTEGER,
            completed_at INTEGER,
            progress REAL NOT NULL,
            metrics_stages_completed INTEGER NOT NULL,
            metrics_stages_total INTEGER NOT NULL,
            metrics_failures INTEGER NOT NULL,
            metrics_retries INTEGER NOT NULL,
            metrics_total_time_secs REAL,
            output_paths TEXT NOT NULL
        )",
        [],
    )?;

    // Create tasks table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tasks (
            task_id TEXT PRIMARY KEY,
            job_id TEXT NOT NULL,
            attempt_id INTEGER NOT NULL,
            stage_id TEXT NOT NULL,
            operator TEXT NOT NULL,
            status TEXT NOT NULL,
            assigned_worker TEXT,
            input_paths TEXT NOT NULL,
            output_path TEXT NOT NULL,
            partition INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            started_at INTEGER,
            completed_at INTEGER,
            error TEXT,
            metrics_records_processed INTEGER NOT NULL,
            metrics_execution_time_secs REAL NOT NULL,
            metrics_memory_used_mb REAL NOT NULL
            FOREIGN KEY (job_id) REFERENCES jobs(id)
        )",
        [],
    )?;

    info!("Database initialized at {}", DB_PATH);
    Ok(conn)
}

/// Convert SystemTime to Unix timestamp (seconds)
fn system_time_to_timestamp(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Convert Unix timestamp to SystemTime
fn timestamp_to_system_time(ts: i64) -> SystemTime {
    UNIX_EPOCH + std::time::Duration::from_secs(ts as u64)
}

/// Save a job to the database
pub fn save_job(
    conn: &Connection,
    job_id: &str,
    name: &str,
    status: JobStatus,
    created_at: SystemTime,
    started_at: Option<SystemTime>,
    completed_at: Option<SystemTime>,
    progress: f64,
    metrics: &JobMetrics,
    output_paths: &[String],
) -> SqliteResult<()> {
    conn.execute(
        "INSERT OR REPLACE INTO jobs (
            id, name, status, created_at, started_at, completed_at, progress,
            metrics_stages_completed, metrics_stages_total, metrics_failures,
            metrics_retries, metrics_total_time_secs, output_paths
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            job_id,
            name,
            status.as_str(),
            system_time_to_timestamp(created_at),
            started_at.map(system_time_to_timestamp),
            completed_at.map(system_time_to_timestamp),
            progress,
            metrics.stages_completed,
            metrics.stages_total,
            metrics.failures,
            metrics.retries,
            metrics.total_time_secs,
            serde_json::to_string(output_paths).unwrap_or_default(),
        ],
    )?;
    Ok(())
}

/// Update job status
pub fn update_job_status(
    conn: &Connection,
    job_id: &str,
    status: JobStatus,
    progress: f64,
    metrics: &JobMetrics,
    output_paths: &[String],
) -> SqliteResult<()> {
    conn.execute(
        "UPDATE jobs SET 
            status = ?1,
            progress = ?2,
            metrics_stages_completed = ?3,
            metrics_stages_total = ?4,
            metrics_failures = ?5,
            metrics_retries = ?6,
            metrics_total_time_secs = ?7,
            output_paths = ?8,
            started_at = COALESCE(started_at, CASE WHEN ?1 = 'RUNNING' THEN ?9 ELSE NULL END),
            completed_at = CASE WHEN ?1 IN ('SUCCEEDED', 'FAILED') THEN ?9 ELSE completed_at END
        WHERE id = ?10",
        params![
            status.as_str(),
            progress,
            metrics.stages_completed,
            metrics.stages_total,
            metrics.failures,
            metrics.retries,
            metrics.total_time_secs,
            serde_json::to_string(output_paths).unwrap_or_default(),
            system_time_to_timestamp(SystemTime::now()),
            job_id,
        ],
    )?;
    Ok(())
}

/// Save a task to the database
pub fn save_task(
    conn: &Connection,
    task_id: &str,
    job_id: &str,
    attempt_id: u32,
    stage_id: &str,
    operator: &Operator,
    status: TaskStatus,
    assigned_worker: Option<&str>,
    input_paths: &[String],
    output_path: &str,
    partition: u32,
    created_at: SystemTime,
    started_at: Option<SystemTime>,
    completed_at: Option<SystemTime>,
    error: Option<&str>,
    metrics: &TaskMetrics,
) -> SqliteResult<()> {
    conn.execute(
        "INSERT OR REPLACE INTO tasks (
            task_id, job_id, attempt_id, stage_id, operator, status, assigned_worker,
            input_paths, output_path, partition, created_at, started_at, completed_at,
            error, metrics_records_processed, metrics_execution_time_secs, metrics_memory_used_mb
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        params![
            task_id,
            job_id,
            attempt_id,
            stage_id,
            serde_json::to_string(operator).unwrap_or_default(),
            status.as_str(),
            assigned_worker,
            serde_json::to_string(input_paths).unwrap_or_default(),
            output_path,
            partition,
            system_time_to_timestamp(created_at),
            started_at.map(system_time_to_timestamp),
            completed_at.map(system_time_to_timestamp),
            error,
            metrics.records_processed,
            metrics.execution_time_secs,
            metrics.memory_used_mb,
        ],
    )?;
    Ok(())
}

/// Load all jobs from database
pub fn load_jobs(conn: &Connection) -> SqliteResult<Vec<(String, String, JobStatus, SystemTime, Option<SystemTime>, Option<SystemTime>, f64, JobMetrics, Vec<String>)>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, status, created_at, started_at, completed_at, progress,
                metrics_stages_completed, metrics_stages_total, metrics_failures,
                metrics_retries, metrics_total_time_secs, output_paths
         FROM jobs"
    )?;
    
    let job_iter = stmt.query_map([], |row| {
        let status_str: String = row.get(2)?;
        let status = match status_str.as_str() {
            "ACCEPTED" => JobStatus::Accepted,
            "RUNNING" => JobStatus::Running,
            "FAILED" => JobStatus::Failed,
            "SUCCEEDED" => JobStatus::Succeeded,
            _ => JobStatus::Accepted,
        };
        
        let output_paths_str: String = row.get(12)?;
        let output_paths: Vec<String> = serde_json::from_str(&output_paths_str).unwrap_or_default();
        
        Ok((
            row.get(0)?, // id
            row.get(1)?, // name
            status,
            timestamp_to_system_time(row.get(3)?), // created_at
            row.get::<_, Option<i64>>(4)?.map(timestamp_to_system_time), // started_at
            row.get::<_, Option<i64>>(5)?.map(timestamp_to_system_time), // completed_at
            row.get(6)?, // progress
            JobMetrics {
                total_time_secs: row.get(11)?,
                stages_completed: row.get(7)?,
                stages_total: row.get(8)?,
                failures: row.get(9)?,
                retries: row.get(10)?,
            },
            output_paths,
        ))
    })?;
    
    let mut jobs = Vec::new();
    for job in job_iter {
        jobs.push(job?);
    }
    Ok(jobs)
}

/// Load all tasks for a job from database
pub fn load_tasks(conn: &Connection, job_id: &str) -> SqliteResult<Vec<(String, String, u32, String, Operator, TaskStatus, Option<String>, Vec<String>, String, u32, SystemTime, Option<SystemTime>, Option<SystemTime>, Option<String>, TaskMetrics)>> {
    let mut stmt = conn.prepare(
        "SELECT task_id, job_id, attempt_id, stage_id, operator, status, assigned_worker,
                input_paths, output_path, partition, created_at, started_at, completed_at,
                error, metrics_records_processed, metrics_execution_time_secs, metrics_memory_used_mb
         FROM tasks WHERE job_id = ?1"
    )?;
    
    let task_iter = stmt.query_map(params![job_id], |row| {
        let status_str: String = row.get(5)?;
        let status = match status_str.as_str() {
            "PENDING" => TaskStatus::Pending,
            "ASSIGNED" => TaskStatus::Assigned,
            "RUNNING" => TaskStatus::Running,
            "COMPLETED" => TaskStatus::Completed,
            "FAILED" => TaskStatus::Failed,
            _ => TaskStatus::Pending,
        };
        
        let operator_str: String = row.get(4)?;
        let operator: Operator = serde_json::from_str(&operator_str)
            .unwrap_or_else(|_| Operator::Map { fn_name: "unknown".to_string() });
        
        let input_paths_str: String = row.get(7)?;
        let input_paths: Vec<String> = serde_json::from_str(&input_paths_str).unwrap_or_default();
        
        Ok((
            row.get(0)?, // task_id
            row.get(1)?, // job_id
            row.get(2)?, // attempt_id
            row.get(3)?, // stage_id
            operator,
            status,
            row.get(6)?, // assigned_worker
            input_paths,
            row.get(8)?, // output_path
            row.get(9)?, // partition
            timestamp_to_system_time(row.get(10)?), // created_at
            row.get::<_, Option<i64>>(11)?.map(timestamp_to_system_time), // started_at
            row.get::<_, Option<i64>>(12)?.map(timestamp_to_system_time), // completed_at
            row.get(13)?, // error
            TaskMetrics {
                records_processed: row.get(14)?,
                execution_time_secs: row.get(15)?,
                memory_used_mb: row.get(16)?,
            },
        ))
    })?;
    
    let mut tasks = Vec::new();
    for task in task_iter {
        tasks.push(task?);
    }
    Ok(tasks)
}

