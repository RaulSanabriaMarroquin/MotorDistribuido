use clap::{Parser, Subcommand};
use common::{JobProgress, JobSpec, SubmitJobResponse, WorkerListItem, WorkersListResponse};
use std::env;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "cluster-client")]
#[command(about = "CLI client for the mini distributed system")]
struct Cli {
    /// Base URL of the master
    #[arg(long, default_value = "http://127.0.0.1:8080")]
    master_url: String,

    /// Enable Claude Haiku 4.5
    #[arg(long)]
    enable_claude_haiku: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// List registered workers
    ListWorkers,
    /// Submit a job for execution
    SubmitJob {
        /// Job name
        #[arg(long)]
        name: String,
        /// Operation: map_add, map_mul, filter_gt, filter_lt
        #[arg(long)]
        operation: String,
        /// Parameter for the operation
        #[arg(long)]
        param: Option<i64>,
        /// Input data (comma-separated integers)
        #[arg(long)]
        input: String,
    },
    /// Get job progress
    GetProgress {
        /// Job ID
        #[arg(long)]
        job_id: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    // Check for ENABLE_CLAUDE_HAIKU env var as fallback
    let enable_claude = cli.enable_claude_haiku || env::var("ENABLE_CLAUDE_HAIKU").is_ok();

    if enable_claude {
        info!("Claude Haiku 4.5 enabled for all requests");
    }

    match cli.command {
        Command::ListWorkers => list_workers(&cli.master_url, enable_claude).await?,
        Command::SubmitJob {
            name,
            operation,
            param,
            input,
        } => {
            submit_job(
                &cli.master_url,
                &name,
                &operation,
                param,
                &input,
                enable_claude,
            )
            .await?
        }
        Command::GetProgress { job_id } => {
            get_progress(&cli.master_url, &job_id, enable_claude).await?
        }
    }

    Ok(())
}

async fn list_workers(
    master_url: &str,
    enable_claude: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("{}/api/v1/workers", master_url);
    info!(%url, "Requesting worker list");

    let mut req = reqwest::Client::new().get(&url);
    if enable_claude {
        req = req.header("X-Enable-Claude-Haiku", "4.5");
    }

    let resp = req.send().await?;
    if !resp.status().is_success() {
        eprintln!("Request failed with status {}", resp.status());
        return Ok(());
    }

    let body: WorkersListResponse = resp.json().await?;

    println!("Workers (version {}):", body.version);
    if body.workers.is_empty() {
        println!("  (no workers registered)");
    } else {
        for WorkerListItem {
            id,
            host,
            port,
            status,
            last_heartbeat,
        } in body.workers
        {
            println!(
                "- {} @ {}:{} [{}] last_heartbeat={:?}",
                id, host, port, status, last_heartbeat
            );
        }
    }

    Ok(())
}

async fn submit_job(
    master_url: &str,
    name: &str,
    operation: &str,
    param: Option<i64>,
    input_str: &str,
    enable_claude: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("{}/api/v1/jobs/submit", master_url);
    info!(%url, "Submitting job: {}", name);

    // Parse input
    let input: Vec<i64> = input_str
        .split(',')
        .map(|s| s.trim().parse::<i64>())
        .collect::<Result<_, _>>()?;

    let job_spec = JobSpec {
        name: name.to_string(),
        dag: None,
        parallelism: None,
        operation: Some(operation.to_string()),
        param,
        input: Some(input),
    };

    let mut req = reqwest::Client::new().post(&url).json(&job_spec);
    if enable_claude {
        req = req.header("X-Enable-Claude-Haiku", "4.5");
    }

    let resp = req.send().await?;
    if !resp.status().is_success() {
        eprintln!("Request failed with status {}", resp.status());
        return Ok(());
    }

    let body: SubmitJobResponse = resp.json().await?;
    println!("Job submitted successfully!");
    println!("Job ID: {}", body.job_id);
    println!("Message: {}", body.message);

    Ok(())
}

async fn get_progress(
    master_url: &str,
    job_id: &str,
    enable_claude: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("{}/api/v1/jobs/{}/progress", master_url, job_id);
    info!(%url, "Getting job progress");

    let mut req = reqwest::Client::new().get(&url);
    if enable_claude {
        req = req.header("X-Enable-Claude-Haiku", "4.5");
    }

    let resp = req.send().await?;
    if !resp.status().is_success() {
        eprintln!("Request failed with status {}", resp.status());
        return Ok(());
    }

    let body: JobProgress = resp.json().await?;
    println!("Job Progress for: {}", body.job_id);
    println!("Name: {}", body.name);
    println!("Status: {}", body.status);
    println!("Progress: {}/{}", body.completed_tasks, body.total_tasks);
    println!("Failed: {}", body.failed_tasks);

    Ok(())
}
