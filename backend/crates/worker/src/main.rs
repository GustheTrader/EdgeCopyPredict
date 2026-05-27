use anyhow::{bail, Result};
use arbitek_config::{init_tracing, AppConfig};
use tokio::signal;
use tokio::sync::broadcast;
use tracing::{error, info};

mod ingest;
mod live_execution;
mod paper_execution;
mod pnl_refresh;
mod queue_consume;
mod queue_publish;
mod reconcile;
mod risk_evaluate;
mod signal_normalize;

#[tokio::main]
async fn main() -> Result<()> {
    let config = AppConfig::from_env()?;
    init_tracing(&config.log_level)?;

    info!(mode = %config.worker.mode, "arbitek worker starting");

    let (shutdown_tx, mut shutdown_rx) = broadcast::channel::<()>(1);
    let shutdown_tx2 = shutdown_tx.clone();

    // Spawn a task that waits for shutdown signals and notifies all workers
    tokio::spawn(async move {
        let ctrl_c = async {
            signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {},
            _ = terminate => {},
        }

        info!("shutdown signal received");
        let _ = shutdown_tx2.send(());
    });

    let result = match config.worker.mode.as_str() {
        "paper-execution-once" => paper_execution::run_paper_execution_once(&config).await,
        "paper-execution" => {
            tokio::select! {
                result = paper_execution::run_paper_execution(&config) => result,
                _ = shutdown_rx.recv() => {
                    info!("shutting down paper-execution worker");
                    Ok(())
                }
            }
        }
        "execution-live-once" => live_execution::run_live_execution_once(&config).await,
        "execution-live" => {
            tokio::select! {
                result = live_execution::run_live_execution(&config) => result,
                _ = shutdown_rx.recv() => {
                    info!("shutting down execution-live worker");
                    Ok(())
                }
            }
        }
        "ingest-polymarket" => {
            tokio::select! {
                result = ingest::run_polymarket_ingest(&config) => {
                    result
                }
                _ = shutdown_rx.recv() => {
                    info!("shutting down ingest-polymarket worker");
                    Ok(())
                }
            }
        }
        "ingest-kalshi" => {
            tokio::select! {
                result = ingest::run_kalshi_ingest(&config) => {
                    result
                }
                _ = shutdown_rx.recv() => {
                    info!("shutting down ingest-kalshi worker");
                    Ok(())
                }
            }
        }
        "signal-normalize" => {
            tokio::select! {
                result = signal_normalize::run_signal_normalize(&config) => {
                    result
                }
                _ = shutdown_rx.recv() => {
                    info!("shutting down signal-normalize worker");
                    Ok(())
                }
            }
        }
        "risk-evaluate" => {
            tokio::select! {
                result = risk_evaluate::run_risk_evaluate(&config) => {
                    result
                }
                _ = shutdown_rx.recv() => {
                    info!("shutting down risk-evaluate worker");
                    Ok(())
                }
            }
        }
        "reconcile" => {
            tokio::select! {
                result = reconcile::run_reconcile(&config) => {
                    result
                }
                _ = shutdown_rx.recv() => {
                    info!("shutting down reconcile worker");
                    Ok(())
                }
            }
        }
        "queue-consume" => {
            tokio::select! {
                result = queue_consume::run_queue_consume(&config) => {
                    result
                }
                _ = shutdown_rx.recv() => {
                    info!("shutting down queue-consume worker");
                    Ok(())
                }
            }
        }
        "pnl-refresh" => {
            tokio::select! {
                result = pnl_refresh::run_pnl_refresh(&config) => {
                    result
                }
                _ = shutdown_rx.recv() => {
                    info!("shutting down pnl-refresh worker");
                    Ok(())
                }
            }
        }
        other => {
            bail!("unsupported worker mode '{}'", other)
        }
    };

    if let Err(e) = result {
        error!("worker error: {}", e);
    }

    info!("arbitek worker stopped gracefully");
    Ok(())
}
