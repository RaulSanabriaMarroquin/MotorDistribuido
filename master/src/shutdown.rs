use tokio::signal;
use tracing::info;

/// Espera señales externas (Ctrl+C o SIGTERM)
/// para permitir apagado ordenado del proceso master.
pub async fn wait_for_shutdown_signal() {
    // Escucha Ctrl+C (SIGINT)
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Error al esperar Ctrl+C");
    };

    // Escucha SIGTERM si está disponible
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
        _ = ctrl_c => info!("Señal Ctrl+C recibida."),
        _ = terminate => info!("Señal SIGTERM recibida."),
    }

    info!("Master: iniciando apagado ordenado…");
}
