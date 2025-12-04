use tokio::signal;
use tracing::info;

pub async fn wait_for_shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Error esperando Ctrl+C");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("No se pudo capturar SIGTERM")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Worker: Ctrl+C recibido"),
        _ = terminate => info!("Worker: SIGTERM recibido"),
    }

    info!("Worker iniciando apagado ordenado…");
}
