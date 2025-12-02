use clap::{Parser, Subcommand};
use common::{JobProgress, JobSpec, SubmitJobResponse, WorkerListItem, WorkersListResponse};
use std::env;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "cluster-client")]
#[command(about = "CLI client for the mini distributed system")]
struct Cli {
    /// URL base del master
    #[arg(long, default_value = "http://127.0.0.1:8080")]
    master_url: String,

    /// Habilitar Claude Haiku 4.5
    #[arg(long)]
    enable_claude_haiku: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Listar workers registrados
    ListWorkers,
    /// Enviar un job para ejecución
    SubmitJob {
        /// Nombre del job
        #[arg(long)]
        name: String,
        /// Operación: map_add, map_mul, filter_gt, filter_lt
        #[arg(long)]
        operation: String,
        /// Parámetro para la operación
        #[arg(long)]
        param: Option<i64>,
        /// Datos de entrada (enteros separados por comas)
        #[arg(long)]
        input: String,
    },
    /// Obtener progreso del job
    GetProgress {
        /// ID del job
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

    // Verificar variable de entorno ENABLE_CLAUDE_HAIKU como respaldo
    let enable_claude = cli.enable_claude_haiku || env::var("ENABLE_CLAUDE_HAIKU").is_ok();

    if enable_claude {
        info!("Claude Haiku 4.5 habilitado para todas las solicitudes");
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
    info!(%url, "Solicitando lista de workers");

    let mut req = reqwest::Client::new().get(&url);
    if enable_claude {
        req = req.header("X-Enable-Claude-Haiku", "4.5");
    }

    let resp = req.send().await?;
    if !resp.status().is_success() {
        eprintln!("Solicitud falló con estado {}", resp.status());
        return Ok(());
    }

    let body: WorkersListResponse = resp.json().await?;

    println!("Workers registrados:");
    if body.workers.is_empty() {
        println!("  (no hay workers registrados)");
    } else {
        for WorkerListItem {
            id,
            host,
            port,
            status,
            active_tasks,
            last_heartbeat: _,
        } in body.workers
        {
            let status_str = match status {
                common::WorkerStatus::Up => "UP",
                common::WorkerStatus::Down => "DOWN",
            };
            println!(
                "- {} @ {}:{} [{}] tareas activas: {}",
                id, host, port, status_str, active_tasks
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
    info!(%url, "Enviando job: {}", name);

    // Analizar entrada
    let input: Vec<i64> = input_str
        .split(',')
        .map(|s| s.trim().parse::<i64>())
        .collect::<Result<_, _>>()?;

    let job_spec = JobSpec {
        name: name.to_string(),
        dag: None,
        parallelism: None,
        operation: Some(operation.to_string()),
        fn_name: None,
        param,
        input: Some(input),
    };

    let mut req = reqwest::Client::new().post(&url).json(&job_spec);
    if enable_claude {
        req = req.header("X-Enable-Claude-Haiku", "4.5");
    }

    let resp = req.send().await?;
    if !resp.status().is_success() {
        eprintln!("Solicitud falló con estado {}", resp.status());
        return Ok(());
    }

    let body: SubmitJobResponse = resp.json().await?;
    println!("¡Job enviado exitosamente!");
    println!("ID del Job: {}", body.job_id);
    println!("Mensaje: {}", body.message);

    Ok(())
}

async fn get_progress(
    master_url: &str,
    job_id: &str,
    enable_claude: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("{}/api/v1/jobs/{}/progress", master_url, job_id);
    info!(%url, "Obteniendo progreso del job");

    let mut req = reqwest::Client::new().get(&url);
    if enable_claude {
        req = req.header("X-Enable-Claude-Haiku", "4.5");
    }

    let resp = req.send().await?;
    if !resp.status().is_success() {
        eprintln!("Solicitud falló con estado {}", resp.status());
        return Ok(());
    }

    let body: JobProgress = resp.json().await?;
    println!("Progreso del Job: {}", body.job_id);
    println!("Estado: {}", body.status);
    println!("Progreso: {}/{}", body.completed_tasks, body.total_tasks);
    println!("Fallidos: {}", body.failed_tasks);

    Ok(())
}
