use anyhow::Result;
use serde::Deserialize;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone, Debug, Deserialize)]
pub struct AppConfig {
    pub database_url: String,
    pub redis_url: String,
    pub bind_addr: String,
    pub environment: String,
    pub log_level: String,
    pub live_trading_enabled: bool,
    pub worker: WorkerConfig,
    pub polymarket: PolymarketConfig,
    pub kalshi: KalshiConfig,
    pub auth: AuthConfig,
    /// Oddpool API key (env var: ODDPOOL_API)
    pub oddpool_api: Option<String>,
    /// Comma-separated list of master Polymarket wallet addresses to copy
    /// (env var: COPY_WALLETS)
    pub copy_wallets: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct WorkerConfig {
    pub mode: String,
    pub run_migrations_on_startup: bool,
    pub execution_batch_size: i64,
    pub polymarket_poll_interval_seconds: u64,
    pub kalshi_poll_interval_seconds: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PolymarketConfig {
    pub data_api_base_url: String,
    pub clob_api_base_url: String,
    pub private_key: Option<String>,
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
    pub api_passphrase: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct KalshiConfig {
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
    /// Demo/paper base URL (used when kalshi_execution_enabled = false)
    pub rest_api_base_url: String,
    pub websocket_base_url: String,
    /// Production base URL (used when kalshi_execution_enabled = true)
    pub rest_api_base_url_live: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub admin_password: String,
    pub token_expiry_hours: u64,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();
        let settings = config::Config::builder()
            .add_source(config::Environment::default().separator("__"))
            .build()?;
        let config: AppConfig = settings.try_deserialize()?;
        Ok(config)
    }
}

pub fn init_tracing(level: &str) -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level)),
        )
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();
    Ok(())
}
