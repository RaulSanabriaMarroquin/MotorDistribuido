use clap::{Parser, Subcommand};
use common::{
    Dag, DagEdge, DagNode, JobStatusResponse, JobSubmitRequest, JobSubmitResponse,
    Operator, WorkerListItem, WorkersListResponse,
};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "cluster-client")]
#[command(about = "CLI client for the mini distributed system")]
struct Cli {
    /// Base URL of the master
    #[arg(long, default_value = "http://127.0.0.1:8080")]
    master_url: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// List registered workers
    ListWorkers,
    /// Submit a new job
    SubmitJob {
        /// Job name
        name: String,
        /// Input file path
        #[arg(long)]
        input: String,
        /// Output file path
        #[arg(long)]
        output: String,
        /// Parallelism level
        #[arg(long, default_value = "1")]
        parallelism: u32,
    },
    /// Get job status
    JobStatus {
        /// Job ID
        job_id: String,
    },
    /// Get job results
    JobResults {
        /// Job ID
        job_id: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::ListWorkers => list_workers(&cli.master_url).await?,
        Command::SubmitJob {
            name,
            input,
            output,
            parallelism,
        } => submit_job(&cli.master_url, name, input, output, parallelism).await?,
        Command::JobStatus { job_id } => get_job_status(&cli.master_url, &job_id).await?,
        Command::JobResults { job_id } => get_job_results(&cli.master_url, &job_id).await?,
    }

    Ok(())
}

async fn list_workers(master_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("{}/api/v1/workers", master_url);
    info!(%url, "Requesting worker list");

    let resp = reqwest::get(&url).await?;
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
    name: String,
    input: String,
    output: String,
    parallelism: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create a simple wordcount DAG
    let dag = Dag {
        nodes: vec![
            DagNode {
                id: "read".to_string(),
                operator: Operator::ReadCsv {
                    path: input.clone(),
                    partitions: parallelism,
                },
            },
            DagNode {
                id: "flat".to_string(),
                operator: Operator::FlatMap {
                    fn_name: "tokenize".to_string(),
                },
            },
            DagNode {
                id: "map".to_string(),
                operator: Operator::Map {
                    fn_name: "to_lower".to_string(),
                },
            },
            DagNode {
                id: "agg".to_string(),
                operator: Operator::ReduceByKey {
                    key: "token".to_string(),
                    fn_name: "sum".to_string(),
                },
            },
            DagNode {
                id: "write".to_string(),
                operator: Operator::WriteJsonl {
                    path: output.clone(),
                },
            },
        ],
        edges: vec![
            DagEdge("read".to_string(), "flat".to_string()),
            DagEdge("flat".to_string(), "map".to_string()),
            DagEdge("map".to_string(), "agg".to_string()),
            DagEdge("agg".to_string(), "write".to_string()),
        ],
    };

    let request = JobSubmitRequest {
        name,
        dag,
        parallelism,
    };

    let url = format!("{}/api/v1/jobs", master_url);
    info!(%url, "Submitting job");

    let client = reqwest::Client::new();
    let resp = client.post(&url).json(&request).send().await?;

    if !resp.status().is_success() {
        eprintln!("Job submission failed with status {}", resp.status());
        return Ok(());
    }

    let body: JobSubmitResponse = resp.json().await?;
    println!("Job submitted successfully!");
    println!("  Job ID: {}", body.job_id);
    println!("  Message: {}", body.message);

    Ok(())
}

async fn get_job_status(
    master_url: &str,
    job_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("{}/api/v1/jobs/{}", master_url, job_id);
    info!(%url, "Requesting job status");

    let resp = reqwest::get(&url).await?;
    if !resp.status().is_success() {
        eprintln!("Request failed with status {}", resp.status());
        return Ok(());
    }

    let body: JobStatusResponse = resp.json().await?;

    println!("Job Status (version {}):", body.version);
    println!("  Job ID: {}", body.job_id);
    println!("  Status: {}", body.status);
    println!("  Progress: {:.2}%", body.progress);
    println!("  Stages: {}/{}", body.metrics.stages_completed, body.metrics.stages_total);
    println!("  Failures: {}", body.metrics.failures);
    println!("  Retries: {}", body.metrics.retries);

    Ok(())
}

async fn get_job_results(
    master_url: &str,
    job_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("{}/api/v1/jobs/{}/results", master_url, job_id);
    info!(%url, "Requesting job results");

    let resp = reqwest::get(&url).await?;
    if !resp.status().is_success() {
        eprintln!("Request failed with status {}", resp.status());
        return Ok(());
    }

    let body: common::JobResultsResponse = resp.json().await?;

    println!("Job Results (version {}):", body.version);
    println!("  Job ID: {}", body.job_id);
    println!("  Output paths:");
    for path in &body.output_paths {
        println!("    - {}", path);
    }

    Ok(())
}
