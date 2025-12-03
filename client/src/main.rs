use common::{JobProgress, JobSpec, SubmitJobResponse, WorkerListItem, WorkersListResponse};
use std::env;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args: Vec<String> = env::args().collect();

    // Check for ENABLE_CLAUDE_HAIKU env var
    let enable_claude = env::var("ENABLE_CLAUDE_HAIKU").is_ok()
        || args.contains(&"--enable-claude-haiku".to_string());

    if enable_claude {
        info!("Claude Haiku 4.5 enabled for all requests");
    }

    // Default master URL
    let mut master_url = "http://127.0.0.1:8080".to_string();

    // Parse master_url if provided
    for i in 0..args.len() {
        if args[i] == "--master-url" && i + 1 < args.len() {
            master_url = args[i + 1].clone();
        }
    }

    // Parse command
    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    let command = &args[1];

    match command.as_str() {
        "list-workers" => list_workers(&master_url, enable_claude).await?,
        "submit-job" => submit_job_cmd(&args, &master_url, enable_claude).await?,
        "get-progress" => get_progress_cmd(&args, &master_url, enable_claude).await?,
        _ => {
            eprintln!("Unknown command: {}", command);
            print_usage();
        }
    }

    Ok(())
}

fn print_usage() {
    println!("Usage:");
    println!("  client [--master-url <URL>] [--enable-claude-haiku] <command>");
    println!();
    println!("Commands:");
    println!("  list-workers                                                   List all workers");
    println!();
    println!("  submit-job --name <NAME> --source-inline \"1,2,3\" --stages op:param,op:param,...");
    println!("             Submit a multi-stage job using inline data");
    println!();
    println!("  submit-job --name <NAME> --source-file <PATH> --stages map_add:5,filter_gt:10");
    println!("             Submit a multi-stage job using data from a plain text file");
    println!();
    println!("  submit-job --name <NAME> --source-csv <PATH> --stages map_add:5,filter_gt:10");
    println!("             Submit a multi-stage job using data from a CSV file (valores numéricos)");
    println!();
    println!("  submit-job --name <NAME> --source-jsonl <PATH> --stages map_add:5,filter_gt:10");
    println!("             Submit a multi-stage job using data from a JSONL file (campo \"value\")");
    println!();
    println!("  submit-job --name <NAME> --operation <OP> --param <P> --input <INPUT>");
    println!("             (Legacy mode, week 1 compatibility)");
    println!("             INPUT is comma-separated integers");
    println!();
    println!("  get-progress --job-id <JOB_ID>                                Get job progress");
}

fn get_arg(args: &[String], flag: &str, default: Option<String>) -> Option<String> {
    for i in 0..args.len() {
        if args[i] == flag && i + 1 < args.len() {
            return Some(args[i + 1].clone());
        }
    }
    default
}

// parse --source-inline "1,2,3,4"
fn parse_inline_data(s: &str) -> Result<Vec<i64>, String> {
    s.split(',')
        .map(|x| x.trim().parse::<i64>().map_err(|_| format!("Invalid number '{}'", x)))
        .collect()
}

// parse --stages "map_add:5,filter_gt:10,map_mul:3"
fn parse_stages(s: &str) -> Result<Vec<common::Stage>, String> {
    let mut result = Vec::new();

    for token in s.split(',') {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }

        let parts: Vec<&str> = token.split(':').collect();
        let op = parts[0].trim().to_string();
        let param = if parts.len() > 1 {
            Some(
                parts[1]
                    .trim()
                    .parse::<i64>()
                    .map_err(|_| format!("Invalid param in '{}'", token))?,
            )
        } else {
            None
        };

        result.push(common::Stage { operation: op, param });
    }

    Ok(result)
}

async fn submit_job_cmd(
    args: &[String],
    master_url: &str,
    enable_claude: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let name = match get_arg(args, "--name", None) {
        Some(n) => n,
        None => {
            eprintln!("Error: --name is required");
            return Ok(());
        }
    };

    // =====================================================
    // MODO NUEVO (Semana 2+): source-* + stages
    // =====================================================
    let source_inline = get_arg(args, "--source-inline", None);
    let source_file = get_arg(args, "--source-file", None);
    let source_csv = get_arg(args, "--source-csv", None);
    let source_jsonl = get_arg(args, "--source-jsonl", None);
    let stages_arg = get_arg(args, "--stages", None);

    let mut source: Option<common::DatasetSource> = None;
    let mut stages: Vec<common::Stage> = Vec::new();

    // Validar que solo haya UNA fuente (inline/file/csv/jsonl)
    let mut source_count = 0;
    if source_inline.is_some() {
        source_count += 1;
    }
    if source_file.is_some() {
        source_count += 1;
    }
    if source_csv.is_some() {
        source_count += 1;
    }
    if source_jsonl.is_some() {
        source_count += 1;
    }

    if source_count > 1 {
        eprintln!("Error: please specify only ONE of --source-inline, --source-file, --source-csv or --source-jsonl");
        return Ok(());
    }

    // Inline source
    if let Some(s) = source_inline {
        let data =
            parse_inline_data(&s).map_err(|e| format!("Error parsing inline data: {}", e))?;
        source = Some(common::DatasetSource::Inline { data });
    }

    // File source (texto plano)
    if let Some(p) = source_file {
        source = Some(common::DatasetSource::File { path: p });
    }

    // CSV source
    if let Some(p) = source_csv {
        source = Some(common::DatasetSource::Csv { path: p });
    }

    // JSONL source
    if let Some(p) = source_jsonl {
        source = Some(common::DatasetSource::Jsonl { path: p });
    }

    // Parse stages (si vienen)
    if let Some(s) = stages_arg {
        stages = parse_stages(&s).map_err(|e| format!("Error parsing stages: {}", e))?;
    }

    // =====================================================
    // MODO VIEJO (compatible con semana 1)
    // SOLO SI NO VIENE source NI stages
    // =====================================================
    let (operation, param, input) = if source.is_none() && stages.is_empty() {
        let op = match get_arg(args, "--operation", None) {
            Some(o) => o,
            None => {
                eprintln!("Error: --operation or --source-* must be provided");
                return Ok(());
            }
        };

        let p = get_arg(args, "--param", None).and_then(|x| x.parse::<i64>().ok());

        let input_str = match get_arg(args, "--input", None) {
            Some(i) => i,
            None => {
                eprintln!("Error: --input is required for old mode");
                return Ok(());
            }
        };

        let inp =
            parse_inline_data(&input_str).map_err(|e| format!("input parse error: {}", e))?;

        (op, p, inp)
    } else {
        // Modo nuevo → no usar estos valores
        ("".to_string(), None, vec![])
    };

    // chunk size
    let chunk_size: usize = get_arg(args, "--chunk-size", Some("100".to_string()))
        .and_then(|x| x.parse().ok())
        .unwrap_or(100);

    // =====================================================
    // CONSTRUIR JobSpec
    // =====================================================
    let job_spec = common::JobSpec {
        name,
        source,
        source_right: None, // join vendrá después, por ahora solo lado izquierdo
        operation,
        param,
        input,
        chunk_size,
        stages,
    };

    submit_job(master_url, &job_spec, enable_claude).await
}

async fn get_progress_cmd(
    args: &[String],
    master_url: &str,
    enable_claude: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let job_id = match get_arg(args, "--job-id", None) {
        Some(id) => id,
        None => {
            eprintln!("Error: --job-id is required");
            return Ok(());
        }
    };

    get_progress(master_url, &job_id, enable_claude).await
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
    job_spec: &JobSpec,
    enable_claude: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("{}/api/v1/jobs/submit", master_url);
    info!(%url, "Submitting job: {}", job_spec.name);

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
