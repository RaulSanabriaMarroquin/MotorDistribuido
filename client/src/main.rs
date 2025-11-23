use clap::{Parser, Subcommand};
use common::{WorkerListItem, WorkersListResponse};
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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::ListWorkers => list_workers(&cli.master_url).await?,
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
