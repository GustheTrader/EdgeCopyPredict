use anyhow::Result;
use arbitek_config::{init_tracing, AppConfig};
use arbitek_db::{
    AlertRepository, FillFilter, FillRepository, FollowerRepository, MasterRepository,
    OrderRepository, PositionFilter, PositionRepository, RawEventRepository, SignalFilter,
    SignalRepository,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
    Json, Router,
};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::{prelude::FromPrimitive, Decimal};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use std::sync::Arc;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tracing::info;
use uuid::Uuid;

mod auth;
mod sse;

// ---------------------------------------------------------------------------
// AppState
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub config: Arc<AppConfig>,
    pub activity_tx: tokio::sync::broadcast::Sender<SignalDto>,
    pub alerts_tx: tokio::sync::broadcast::Sender<AlertDto>,
    pub system_tx: tokio::sync::broadcast::Sender<SystemStatusDto>,
    pub http: reqwest::Client,
}

// ---------------------------------------------------------------------------
// Error handling
// ---------------------------------------------------------------------------

pub struct ApiError {
    pub status: StatusCode,
    pub inner: anyhow::Error,
}

impl ApiError {
    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            inner: anyhow::anyhow!("{}", msg.into()),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(serde_json::json!({ "error": self.inner.to_string() }));
        (self.status, body).into_response()
    }
}

impl<E: Into<anyhow::Error>> From<E> for ApiError {
    fn from(err: E) -> Self {
        ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            inner: err.into(),
        }
    }
}

type ApiResult<T> = std::result::Result<T, ApiError>;

// ---------------------------------------------------------------------------
// Request / Query DTOs
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct StatusBody {
    status: String,
}

#[derive(Deserialize)]
struct CreateMasterBody {
    display_name: String,
    #[serde(default)]
    status: String,
    #[serde(default = "default_master_weight")]
    master_weight: f64,
    wallet_address: Option<String>,
    #[serde(default)]
    config: Value,
}

fn default_master_weight() -> f64 {
    1.0
}

#[derive(Deserialize)]
struct CreateFollowerBody {
    display_name: String,
    #[serde(default = "default_active")]
    status: String,
    #[serde(default = "default_usd")]
    base_currency: String,
    wallet_address: Option<String>,
    #[serde(default = "default_polymarket")]
    platform: String,
    #[serde(default)]
    risk_config: Value,
}

fn default_active() -> String {
    "active".to_owned()
}

fn default_usd() -> String {
    "USD".to_owned()
}

fn default_polymarket() -> String {
    "polymarket".to_owned()
}

#[derive(Deserialize)]
struct SignalQuery {
    platform: Option<String>,
    master_id: Option<Uuid>,
    status: Option<String>,
    page: Option<i32>,
    per_page: Option<i32>,
}

// ---------------------------------------------------------------------------
// Response DTOs
// ---------------------------------------------------------------------------

#[derive(Serialize, Clone)]
pub struct MasterDto {
    id: Uuid,
    display_name: String,
    status: String,
    master_weight: f64,
    wallet_address: Option<String>,
    config: Value,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    updated_at: OffsetDateTime,
}

#[derive(Serialize, Clone)]
pub struct FollowerDto {
    id: Uuid,
    display_name: String,
    status: String,
    base_currency: String,
    wallet_address: Option<String>,
    platform: String,
    risk_config: Value,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    updated_at: OffsetDateTime,
}

#[derive(Serialize, Clone)]
pub struct SignalDto {
    id: Uuid,
    platform: String,
    master_id: Uuid,
    /// Wallet address of the master trader this signal came from.
    master_wallet: Option<String>,
    market_id: Option<Uuid>,
    outcome: String,
    side: String,
    position_effect: String,
    size_contracts: f64,
    notional: f64,
    currency: String,
    entry_price: f64,
    #[serde(with = "time::serde::rfc3339")]
    event_timestamp: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    detected_at: OffsetDateTime,
    confidence_score: Option<f64>,
    status: String,
    market_title: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct OrderDto {
    id: Uuid,
    copy_intent_id: Option<Uuid>,
    platform: String,
    follower_account_id: Uuid,
    platform_order_id: Option<String>,
    client_order_id: String,
    market_id: Option<Uuid>,
    outcome: String,
    side: String,
    order_type: String,
    price_limit: f64,
    requested_contracts: f64,
    filled_contracts: f64,
    avg_fill_price: Option<f64>,
    status: String,
    #[serde(with = "time::serde::rfc3339::option")]
    submitted_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    last_update_at: OffsetDateTime,
}

#[derive(Serialize, Clone)]
pub struct FillDto {
    id: Uuid,
    order_id: Uuid,
    platform: String,
    platform_fill_id: String,
    follower_account_id: Uuid,
    market_id: Option<Uuid>,
    market_title: Option<String>,
    outcome: String,
    side: String,
    price: f64,
    contracts: f64,
    notional: f64,
    fee: f64,
    is_taker: Option<bool>,
    #[serde(with = "time::serde::rfc3339")]
    filled_at: OffsetDateTime,
    payload: Value,
}

#[derive(Serialize, Clone)]
pub struct PositionDto {
    id: Uuid,
    follower_account_id: Uuid,
    platform: String,
    market_id: Uuid,
    market_title: Option<String>,
    market_status: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    close_time: Option<OffsetDateTime>,
    outcome: String,
    contracts: f64,
    avg_price: f64,
    realized_pnl: f64,
    unrealized_pnl: f64,
    total_pnl: f64,
    exposure: f64,
    #[serde(with = "time::serde::rfc3339")]
    updated_at: OffsetDateTime,
}

#[derive(Serialize, Clone)]
pub struct MarketMappingDto {
    id: Uuid,
    polymarket_condition_id: String,
    polymarket_slug: Option<String>,
    kalshi_ticker: String,
    kalshi_event_ticker: Option<String>,
    confidence: String,
    enabled: bool,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

#[derive(Serialize, Clone)]
pub struct RawEventDto {
    id: Uuid,
    platform: String,
    source: String,
    external_event_id: String,
    raw_payload: Value,
    #[serde(with = "time::serde::rfc3339")]
    detected_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    processed_at: Option<OffsetDateTime>,
    status: String,
    source_entity_id: Option<Uuid>,
    source_entity_type: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct AlertDto {
    id: Uuid,
    severity: String,
    alert_type: String,
    status: String,
    entity_type: Option<String>,
    entity_id: Option<Uuid>,
    message: String,
    payload: Value,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    acknowledged_at: Option<OffsetDateTime>,
}

#[derive(Serialize)]
struct SignalTimelineDto {
    signal: SignalDto,
    orders: Vec<OrderDto>,
}

#[derive(Serialize, Clone)]
pub struct SystemStatusDto {
    live_trading_enabled: bool,
    kalshi_execution_enabled: bool,
    paper_wallet_balance: f64,
    worker_mode: String,
    open_alert_count: i64,
    last_signal_at: Option<String>,
}

#[derive(Serialize)]
struct AccountBalancesDto {
    kalshi_balance: Option<f64>,
    polymarket_balance: Option<f64>,
    kalshi_error: Option<String>,
    polymarket_error: Option<String>,
}

#[derive(Serialize)]
struct PnlSummaryDto {
    follower_account_id: Option<Uuid>,
    total_invested: f64,
    unrealized_pnl: f64,
    realized_pnl: f64,
    total_pnl: f64,
    win_rate: f64,
    trade_count: i64,
    open_positions: i64,
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

/// Sync wallet addresses from the COPY_WALLETS env var into the `masters` table.
///
/// Each wallet is inserted as an active master if it does not already exist.
/// Existing rows are never modified or deleted — this is purely additive so
/// manual edits in the UI are preserved.
async fn sync_copy_wallets(pool: &sqlx::PgPool, copy_wallets: &str) {
    let wallets: Vec<&str> = copy_wallets
        .split(',')
        .map(|w| w.trim())
        .filter(|w| !w.is_empty())
        .collect();

    info!(
        count = wallets.len(),
        "syncing COPY_WALLETS into masters table"
    );

    for wallet in wallets {
        let wallet_lc = wallet.to_lowercase();
        // Check if a master with this wallet address already exists.
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM masters WHERE lower(wallet_address) = lower($1))",
        )
        .bind(&wallet_lc)
        .fetch_one(pool)
        .await
        .unwrap_or(false);

        if exists {
            info!(wallet = %wallet_lc, "master already in DB — skipping");
            continue;
        }

        // Derive a short display name from the wallet address (first 6 + last 4 chars).
        let display_name = if wallet_lc.len() >= 10 {
            format!(
                "Master {}…{}",
                &wallet_lc[..6],
                &wallet_lc[wallet_lc.len() - 4..]
            )
        } else {
            format!("Master {}", wallet_lc)
        };

        let result = sqlx::query(
            r#"INSERT INTO masters (id, display_name, status, master_weight, wallet_address, config)
               VALUES ($1, $2, 'active', 1.0, $3, '{}'::jsonb)
               ON CONFLICT DO NOTHING"#,
        )
        .bind(uuid::Uuid::new_v4())
        .bind(&display_name)
        .bind(&wallet_lc)
        .execute(pool)
        .await;

        match result {
            Ok(r) if r.rows_affected() > 0 => {
                info!(wallet = %wallet_lc, display_name = %display_name, "inserted master from COPY_WALLETS");
            }
            Ok(_) => {
                info!(wallet = %wallet_lc, "master already exists (race) — skipped");
            }
            Err(e) => {
                tracing::warn!(wallet = %wallet_lc, "failed to insert master: {}", e);
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = AppConfig::from_env()?;
    init_tracing(&config.log_level)?;

    info!("connecting database pool");
    let pool = arbitek_db::connect_api(&config.database_url).await?;
    info!("database pool connected");

    // Sync master wallets from COPY_WALLETS env var → masters table.
    // This ensures the worker always polls the correct addresses.
    if let Some(ref wallets) = config.copy_wallets {
        if !wallets.is_empty() {
            sync_copy_wallets(&pool, wallets).await;
        }
    }

    let (activity_tx, _) = tokio::sync::broadcast::channel::<SignalDto>(256);
    let (alerts_tx, _) = tokio::sync::broadcast::channel::<AlertDto>(256);
    let (system_tx, _) = tokio::sync::broadcast::channel::<SystemStatusDto>(64);

    let state = AppState {
        pool: pool.clone(),
        config: Arc::new(config.clone()),
        activity_tx: activity_tx.clone(),
        alerts_tx: alerts_tx.clone(),
        system_tx: system_tx.clone(),
        http: reqwest::Client::new(),
    };

    tokio::spawn(sse::broadcast_activity(pool.clone(), activity_tx.clone()));
    tokio::spawn(sse::broadcast_alerts(pool.clone(), alerts_tx.clone()));
    tokio::spawn(sse::broadcast_system(
        pool.clone(),
        Arc::new(config.clone()),
        system_tx.clone(),
    ));
    tokio::spawn(sse::broadcast_from_redis(
        config.redis_url.clone(),
        pool.clone(),
        activity_tx,
        alerts_tx,
        system_tx,
    ));

    let listener = TcpListener::bind(&config.bind_addr).await?;
    info!(
        bind_addr = %config.bind_addr,
        environment = %config.environment,
        live_trading_enabled = config.live_trading_enabled,
        "arbitek api starting"
    );

    axum::serve(listener, router(state)).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

fn router(state: AppState) -> Router {
    let protected = Router::new()
        .route("/api/v1/system/toggle", post(toggle_system))
        .route(
            "/api/v1/system/kalshi-execution/toggle",
            post(toggle_kalshi_execution),
        )
        .route(
            "/api/v1/system/paper-wallet/reset",
            post(reset_paper_wallet),
        )
        .route("/api/v1/masters/:id/status", patch(update_master_status))
        .route(
            "/api/v1/followers/:id/status",
            patch(update_follower_status),
        )
        .route(
            "/api/v1/followers/:id/risk-config",
            patch(update_follower_risk_config),
        )
        .route("/api/v1/alerts/:id/ack", post(ack_alert))
        .route("/api/v1/alerts/ack-all", post(ack_all_alerts))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth::require_auth,
        ));

    Router::new()
        .route("/health", get(healthz))
        .route("/healthz", get(healthz))
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/system/status", get(system_status))
        .route("/api/v1/masters", get(list_masters))
        .route("/api/v1/masters", post(create_master))
        .route("/api/v1/masters/:id", get(get_master).delete(delete_master))
        .route(
            "/api/v1/masters/:id/wallet-address",
            patch(update_master_wallet_address),
        )
        .route("/api/v1/followers", get(list_followers))
        .route("/api/v1/followers", post(create_follower))
        .route(
            "/api/v1/followers/:id",
            get(get_follower).delete(delete_follower),
        )
        .route(
            "/api/v1/followers/:id/wallet-address",
            patch(update_follower_wallet_address),
        )
        .route(
            "/api/v1/followers/:id/platform",
            patch(update_follower_platform),
        )
        .route("/api/v1/signals", get(list_signals))
        .route("/api/v1/signals/:id", get(get_signal))
        .route("/api/v1/signals/:id/orders", get(list_signal_orders))
        .route("/api/v1/signals/:id/timeline", get(get_signal_timeline))
        .route("/api/v1/orders", get(list_orders))
        .route("/api/v1/fills", get(list_fills))
        .route("/api/v1/positions", get(list_positions))
        .route("/api/v1/market-mappings", get(list_market_mappings))
        .route("/api/v1/market-mappings", post(create_market_mapping))
        .route("/api/v1/market-mappings/:id", get(get_market_mapping))
        .route("/api/v1/market-mappings/:id", patch(update_market_mapping))
        .route(
            "/api/v1/market-mappings/:id/enabled",
            patch(update_market_mapping_enabled),
        )
        .route("/api/v1/market-mappings/:id", delete(delete_market_mapping))
        .route("/api/v1/pnl", get(get_pnl))
        .route("/api/v1/accounts/balance", get(get_account_balances))
        .route("/api/v1/raw-events", get(list_raw_events))
        .route("/api/v1/alerts", get(list_alerts))
        .route("/sse/activity", get(sse::sse_activity_handler))
        .route("/sse/alerts", get(sse::sse_alerts_handler))
        .route("/sse/system", get(sse::sse_system_handler))
        .merge(protected)
        .fallback(spa_fallback)
        .layer(CorsLayer::permissive())
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers — system
// ---------------------------------------------------------------------------

async fn healthz() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "service": "arbitek-api" }))
}

async fn system_status(State(state): State<AppState>) -> ApiResult<Json<SystemStatusDto>> {
    let settings = arbitek_db::SystemSettingsRepository::new(state.pool.clone());
    let live_trading = settings.get_bool("live_trading_enabled", false).await?;
    let kalshi_exec = settings.get_bool("kalshi_execution_enabled", false).await?;
    let paper_balance = settings
        .get_decimal("paper_wallet_balance", rust_decimal::Decimal::new(1000, 0))
        .await
        .unwrap_or(rust_decimal::Decimal::new(1000, 0))
        .to_f64()
        .unwrap_or(1000.0);

    let open_count = AlertRepository::new(state.pool.clone())
        .count_open()
        .await?;
    let row = sqlx::query("select max(detected_at) as ts from signals")
        .fetch_one(&state.pool)
        .await?;
    let last_ts: Option<OffsetDateTime> = row.try_get("ts")?;

    Ok(Json(SystemStatusDto {
        live_trading_enabled: live_trading,
        kalshi_execution_enabled: kalshi_exec,
        paper_wallet_balance: paper_balance,
        worker_mode: state.config.worker.mode.clone(),
        open_alert_count: open_count,
        last_signal_at: last_ts.and_then(|dt| dt.format(&Rfc3339).ok()),
    }))
}

// ---------------------------------------------------------------------------
// Handlers — masters
// ---------------------------------------------------------------------------

async fn list_masters(State(state): State<AppState>) -> ApiResult<Json<Vec<MasterDto>>> {
    let records = MasterRepository::new(state.pool).list().await?;
    Ok(Json(records.into_iter().map(master_to_dto).collect()))
}

async fn get_master(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<Response> {
    match MasterRepository::new(state.pool).get_by_id(id).await? {
        Some(r) => Ok(Json(master_to_dto(r)).into_response()),
        None => Ok(not_found()),
    }
}

async fn create_master(
    State(state): State<AppState>,
    Json(body): Json<CreateMasterBody>,
) -> ApiResult<Response> {
    let repo = MasterRepository::new(state.pool);
    let weight = Decimal::from_f64(body.master_weight).unwrap_or_else(|| Decimal::new(1, 0));
    let id = repo
        .insert(
            &body.display_name,
            &body.status,
            weight,
            body.wallet_address.as_deref(),
            &body.config,
        )
        .await?;
    match repo.get_by_id(id).await? {
        Some(r) => Ok((StatusCode::CREATED, Json(master_to_dto(r))).into_response()),
        None => Ok((
            StatusCode::CREATED,
            Json(master_to_dto(arbitek_db::MasterRecord {
                id,
                display_name: body.display_name,
                status: body.status,
                master_weight: weight,
                wallet_address: body.wallet_address,
                config: body.config,
                created_at: OffsetDateTime::now_utc(),
                updated_at: OffsetDateTime::now_utc(),
            })),
        )
            .into_response()),
    }
}

async fn update_master_status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<StatusBody>,
) -> ApiResult<Response> {
    let repo = MasterRepository::new(state.pool);
    repo.update_status(id, &body.status).await?;
    match repo.get_by_id(id).await? {
        Some(r) => Ok(Json(master_to_dto(r)).into_response()),
        None => Ok(not_found()),
    }
}

#[derive(Deserialize)]
struct WalletAddressBody {
    wallet_address: Option<String>,
}

async fn update_master_wallet_address(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<WalletAddressBody>,
) -> ApiResult<Response> {
    let repo = MasterRepository::new(state.pool);
    repo.update_wallet_address(id, body.wallet_address.as_deref())
        .await?;
    match repo.get_by_id(id).await? {
        Some(r) => Ok(Json(master_to_dto(r)).into_response()),
        None => Ok(not_found()),
    }
}

async fn delete_master(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<Response> {
    let repo = MasterRepository::new(state.pool);
    match repo.get_by_id(id).await? {
        Some(_) => {
            repo.update_status(id, "deleted").await?;
            Ok(Json(serde_json::json!({ "deleted": true })).into_response())
        }
        _ => Ok(not_found()),
    }
}

// ---------------------------------------------------------------------------
// Handlers — followers
// ---------------------------------------------------------------------------

async fn list_followers(State(state): State<AppState>) -> ApiResult<Json<Vec<FollowerDto>>> {
    let records = FollowerRepository::new(state.pool).list().await?;
    Ok(Json(records.into_iter().map(follower_to_dto).collect()))
}

async fn get_follower(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<Response> {
    match FollowerRepository::new(state.pool).get_by_id(id).await? {
        Some(r) => Ok(Json(follower_to_dto(r)).into_response()),
        None => Ok(not_found()),
    }
}

async fn update_follower_status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<StatusBody>,
) -> ApiResult<Response> {
    let repo = FollowerRepository::new(state.pool);
    repo.update_status(id, &body.status).await?;
    match repo.get_by_id(id).await? {
        Some(r) => Ok(Json(follower_to_dto(r)).into_response()),
        None => Ok(not_found()),
    }
}

async fn create_follower(
    State(state): State<AppState>,
    Json(body): Json<CreateFollowerBody>,
) -> ApiResult<Response> {
    let repo = FollowerRepository::new(state.pool);
    let id = repo
        .insert(
            &body.display_name,
            &body.status,
            &body.base_currency,
            body.wallet_address.as_deref(),
            &body.platform,
            &body.risk_config,
        )
        .await?;
    match repo.get_by_id(id).await? {
        Some(r) => Ok((StatusCode::CREATED, Json(follower_to_dto(r))).into_response()),
        None => Ok((
            StatusCode::CREATED,
            Json(follower_to_dto(arbitek_db::FollowerRecord {
                id,
                display_name: body.display_name,
                status: body.status,
                base_currency: body.base_currency,
                wallet_address: body.wallet_address,
                platform: body.platform,
                risk_config: body.risk_config,
                created_at: OffsetDateTime::now_utc(),
                updated_at: OffsetDateTime::now_utc(),
            })),
        )
            .into_response()),
    }
}

async fn update_follower_risk_config(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<Value>,
) -> ApiResult<Response> {
    let repo = FollowerRepository::new(state.pool);
    repo.update_risk_config(id, &body).await?;
    match repo.get_by_id(id).await? {
        Some(r) => Ok(Json(follower_to_dto(r)).into_response()),
        None => Ok(not_found()),
    }
}

async fn update_follower_wallet_address(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<WalletAddressBody>,
) -> ApiResult<Response> {
    let repo = FollowerRepository::new(state.pool);
    repo.update_wallet_address(id, body.wallet_address.as_deref())
        .await?;
    match repo.get_by_id(id).await? {
        Some(r) => Ok(Json(follower_to_dto(r)).into_response()),
        None => Ok(not_found()),
    }
}

#[derive(Deserialize)]
struct PlatformBody {
    platform: String,
}

async fn update_follower_platform(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<PlatformBody>,
) -> ApiResult<Response> {
    let repo = FollowerRepository::new(state.pool);
    repo.update_platform(id, &body.platform).await?;
    match repo.get_by_id(id).await? {
        Some(r) => Ok(Json(follower_to_dto(r)).into_response()),
        None => Ok(not_found()),
    }
}

async fn delete_follower(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    let repo = FollowerRepository::new(state.pool);
    match repo.get_by_id(id).await? {
        Some(_) => {
            repo.update_status(id, "deleted").await?;
            Ok(Json(serde_json::json!({ "deleted": true })).into_response())
        }
        _ => Ok(not_found()),
    }
}

// ---------------------------------------------------------------------------
// Handlers — signals
// ---------------------------------------------------------------------------

async fn list_signals(
    State(state): State<AppState>,
    Query(q): Query<SignalQuery>,
) -> ApiResult<Json<Vec<SignalDto>>> {
    let limit = q.per_page.unwrap_or(50).max(1).min(200) as i64;
    let offset = ((q.page.unwrap_or(1).saturating_sub(1)) as i64) * limit;

    // JOIN masters to include the wallet address with each signal.
    let rows = sqlx::query(
        r#"SELECT s.id, s.platform, s.master_id, s.market_id, s.outcome, s.side,
                  s.position_effect, s.size_contracts, s.notional, s.currency,
                  s.entry_price, s.event_timestamp, s.detected_at,
                  s.confidence_score, s.status, s.market_title,
                  m.wallet_address AS master_wallet
           FROM signals s
           LEFT JOIN masters m ON m.id = s.master_id
           ORDER BY s.detected_at DESC
           LIMIT $1 OFFSET $2"#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let dtos: Vec<SignalDto> = rows
        .into_iter()
        .filter_map(|row| {
            let size_contracts: rust_decimal::Decimal = row.try_get("size_contracts").ok()?;
            let notional: rust_decimal::Decimal = row.try_get("notional").ok()?;
            let entry_price: rust_decimal::Decimal = row.try_get("entry_price").ok()?;
            let confidence_score: Option<rust_decimal::Decimal> =
                row.try_get("confidence_score").ok().flatten();
            Some(SignalDto {
                id: row.try_get("id").ok()?,
                platform: row.try_get("platform").ok()?,
                master_id: row.try_get("master_id").ok()?,
                master_wallet: row.try_get("master_wallet").ok().flatten(),
                market_id: row.try_get("market_id").ok().flatten(),
                outcome: row.try_get("outcome").ok()?,
                side: row.try_get("side").ok()?,
                position_effect: row.try_get("position_effect").ok()?,
                size_contracts: size_contracts.to_f64().unwrap_or(0.0),
                notional: notional.to_f64().unwrap_or(0.0),
                currency: row.try_get("currency").ok()?,
                entry_price: entry_price.to_f64().unwrap_or(0.0),
                event_timestamp: row.try_get("event_timestamp").ok()?,
                detected_at: row.try_get("detected_at").ok()?,
                confidence_score: confidence_score.and_then(|d| d.to_f64()),
                status: row.try_get("status").ok()?,
                market_title: row.try_get("market_title").ok().flatten(),
            })
        })
        .collect();

    Ok(Json(dtos))
}

async fn get_signal(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<Response> {
    match SignalRepository::new(state.pool).get_by_id(id).await? {
        Some(r) => Ok(Json(signal_to_dto(r, None)).into_response()),
        None => Ok(not_found()),
    }
}

async fn list_signal_orders(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<OrderDto>>> {
    let records = OrderRepository::new(state.pool).list_for_signal(id).await?;
    Ok(Json(records.into_iter().map(order_to_dto).collect()))
}

async fn get_signal_timeline(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    let signal = match SignalRepository::new(state.pool.clone())
        .get_by_id(id)
        .await?
    {
        Some(r) => signal_to_dto(r, None),
        None => return Ok(not_found()),
    };
    let orders = OrderRepository::new(state.pool.clone())
        .list_for_signal(id)
        .await?;
    let timeline = SignalTimelineDto {
        signal,
        orders: orders.into_iter().map(order_to_dto).collect(),
    };
    Ok(Json(timeline).into_response())
}

// ---------------------------------------------------------------------------
// Handlers — orders, fills, positions, pnl, alerts
// ---------------------------------------------------------------------------

async fn list_orders(State(state): State<AppState>) -> ApiResult<Json<Vec<OrderDto>>> {
    let records = OrderRepository::new(state.pool).list_recent(200).await?;
    Ok(Json(records.into_iter().map(order_to_dto).collect()))
}

async fn list_fills(
    State(state): State<AppState>,
    Query(q): Query<arbitek_db::FillQuery>,
) -> ApiResult<Json<Vec<FillDto>>> {
    let filter = FillFilter {
        follower_account_id: q.follower_account_id,
        platform: q.platform.as_deref(),
        market_id: q.market_id,
        page: q.page.unwrap_or(1),
        per_page: q.per_page.unwrap_or(50),
    };
    let result = FillRepository::new(state.pool)
        .list_paginated(&filter)
        .await?;
    Ok(Json(result.items.into_iter().map(fill_to_dto).collect()))
}

async fn list_positions(
    State(state): State<AppState>,
    Query(q): Query<arbitek_db::PositionQuery>,
) -> ApiResult<Json<Vec<PositionDto>>> {
    let filter = PositionFilter {
        follower_account_id: q.follower_account_id,
        platform: q.platform.as_deref(),
        market_id: q.market_id,
        open_only: q.open_only,
        page: q.page.unwrap_or(1),
        per_page: q.per_page.unwrap_or(50),
    };
    let result = PositionRepository::new(state.pool)
        .list_paginated(&filter)
        .await?;
    Ok(Json(
        result.items.into_iter().map(position_to_dto).collect(),
    ))
}

async fn get_pnl(State(state): State<AppState>) -> ApiResult<Json<Vec<PnlSummaryDto>>> {
    let rows = sqlx::query(
        r#"
        select
            follower_account_id,
            coalesce(sum(realized_pnl), 0) as realized_pnl,
            coalesce(sum(unrealized_pnl), 0) as unrealized_pnl,
            coalesce(sum(realized_pnl + unrealized_pnl), 0) as total_pnl,
            count(*) as trade_count,
            sum(case when realized_pnl + unrealized_pnl > 0 then 1 else 0 end)::float8 / nullif(count(*), 0) as win_rate,
            sum(case when contracts != 0 then 1 else 0 end) as open_positions,
            coalesce(sum(avg_price * contracts), 0) as total_invested
        from positions
        group by follower_account_id
        "#
    )
    .fetch_all(&state.pool)
    .await?;

    let result: Vec<PnlSummaryDto> = rows
        .into_iter()
        .map(|row| PnlSummaryDto {
            follower_account_id: row.try_get("follower_account_id").ok(),
            total_invested: row
                .try_get::<Decimal, _>("total_invested")
                .unwrap_or_default()
                .to_f64()
                .unwrap_or(0.0),
            unrealized_pnl: row
                .try_get::<Decimal, _>("unrealized_pnl")
                .unwrap_or_default()
                .to_f64()
                .unwrap_or(0.0),
            realized_pnl: row
                .try_get::<Decimal, _>("realized_pnl")
                .unwrap_or_default()
                .to_f64()
                .unwrap_or(0.0),
            total_pnl: row
                .try_get::<Decimal, _>("total_pnl")
                .unwrap_or_default()
                .to_f64()
                .unwrap_or(0.0),
            win_rate: row.try_get::<f64, _>("win_rate").unwrap_or(0.0),
            trade_count: row.try_get::<i64, _>("trade_count").unwrap_or(0),
            open_positions: row.try_get::<i64, _>("open_positions").unwrap_or(0),
        })
        .collect();

    Ok(Json(result))
}

async fn get_account_balances(State(state): State<AppState>) -> Json<AccountBalancesDto> {
    let kalshi = fetch_kalshi_balance(&state).await;
    let poly = fetch_polymarket_balance(&state).await;
    Json(AccountBalancesDto {
        kalshi_balance: kalshi.as_ref().ok().copied().flatten(),
        polymarket_balance: poly.as_ref().ok().copied().flatten(),
        kalshi_error: kalshi.err().map(|e| e.to_string()),
        polymarket_error: poly.err().map(|e| e.to_string()),
    })
}

fn derive_eth_address(private_key_hex: &str) -> anyhow::Result<String> {
    use k256::ecdsa::SigningKey;
    use sha3::{Digest, Keccak256};

    let key_bytes = hex::decode(private_key_hex.trim_start_matches("0x"))
        .map_err(|e| anyhow::anyhow!("invalid hex private key: {e}"))?;
    let signing_key = SigningKey::from_slice(&key_bytes)
        .map_err(|e| anyhow::anyhow!("invalid secp256k1 key: {e}"))?;
    let verifying_key = signing_key.verifying_key();
    let point = verifying_key.to_encoded_point(false);
    let pub_bytes = &point.as_bytes()[1..]; // skip 0x04 prefix
    let mut h = Keccak256::new();
    h.update(pub_bytes);
    let hash = h.finalize();
    Ok(format!("0x{}", hex::encode(&hash[12..])))
}

async fn fetch_kalshi_balance(state: &AppState) -> anyhow::Result<Option<f64>> {
    use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
    use rand::rngs::OsRng;
    use rsa::pkcs1::DecodeRsaPrivateKey;
    use rsa::pss::SigningKey;
    use rsa::signature::{RandomizedSigner, SignatureEncoding};
    use sha2::Sha256;

    let api_key = match state.config.kalshi.api_key.as_deref() {
        Some(k) if !k.is_empty() => k.to_owned(),
        _ => return Ok(None),
    };
    let api_secret = match state.config.kalshi.api_secret.as_deref() {
        Some(s) if !s.is_empty() => s.to_owned(),
        _ => return Ok(None),
    };

    let ts_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    // Build URL: base_url already contains the path prefix (e.g., /trade-api/v2)
    let base_url = state.config.kalshi.rest_api_base_url.trim_end_matches('/');
    let full_url = format!("{}/portfolio/balance", base_url);

    // Extract path for signing (strip scheme + host), then strip query string
    let without_scheme = full_url
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let path_with_query = &without_scheme[without_scheme.find('/').unwrap_or(0)..];
    let sign_path = path_with_query.split('?').next().unwrap_or(path_with_query);

    let msg = format!("{ts_ms}GET{sign_path}");

    // Kalshi uses RSA-PSS with SHA256 and salt_length = digest_length (32)
    let rsa_key = rsa::RsaPrivateKey::from_pkcs1_pem(&api_secret)
        .map_err(|e| anyhow::anyhow!("invalid Kalshi RSA key: {e}"))?;
    let signing_key: SigningKey<Sha256> = SigningKey::new_with_salt_len(rsa_key, 32);
    let sig = signing_key.sign_with_rng(&mut OsRng, msg.as_bytes());
    let sig_b64 = B64.encode(sig.to_bytes().as_ref());

    let resp = state
        .http
        .get(&full_url)
        .header("KALSHI-ACCESS-KEY", &api_key)
        .header("KALSHI-ACCESS-TIMESTAMP", ts_ms.to_string())
        .header("KALSHI-ACCESS-SIGNATURE", sig_b64)
        .header("Content-Type", "application/json")
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        let detail = body
            .pointer("/error/details")
            .or_else(|| body.pointer("/error/message"))
            .and_then(|v| v.as_str())
            .unwrap_or("auth failed");
        return Err(anyhow::anyhow!("Kalshi {status}: {detail}"));
    }

    let body: serde_json::Value = resp.json().await?;
    let balance = body
        .get("balance")
        .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
        .map(|cents| cents as f64 / 100.0);
    Ok(balance)
}

async fn fetch_polymarket_balance(state: &AppState) -> anyhow::Result<Option<f64>> {
    // Derive wallet address from secp256k1 private key
    let priv_key_hex = match state.config.polymarket.private_key.as_deref() {
        Some(k) if !k.is_empty() => k.to_owned(),
        _ => return Ok(None),
    };
    let address = derive_eth_address(&priv_key_hex)?;

    // Check USDC balance on Polygon via public Tenderly RPC
    let addr_padded = format!("000000000000000000000000{}", &address[2..]);
    let call_data = format!("0x70a08231{}", addr_padded); // balanceOf(address)

    // Try native USDC (0x3c4...) and bridged USDC.e (0x279...)
    let native = polygon_usdc_balance(
        &state.http,
        "0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359",
        &call_data,
    )
    .await;
    let bridged = polygon_usdc_balance(
        &state.http,
        "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174",
        &call_data,
    )
    .await;
    let total = (native + bridged) as f64 / 1_000_000.0;
    Ok(Some(total))
}

async fn polygon_usdc_balance(http: &reqwest::Client, contract: &str, call_data: &str) -> u128 {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{ "to": contract, "data": call_data }, "latest"],
        "id": 1
    });
    let Ok(resp) = http
        .post("https://gateway.tenderly.co/public/polygon")
        .json(&body)
        .send()
        .await
    else {
        return 0;
    };
    let Ok(result) = resp.json::<serde_json::Value>().await else {
        return 0;
    };
    result
        .get("result")
        .and_then(|v| v.as_str())
        .and_then(|hex| u128::from_str_radix(hex.trim_start_matches("0x"), 16).ok())
        .unwrap_or(0)
}

async fn list_alerts(
    State(state): State<AppState>,
    Query(q): Query<arbitek_db::AlertQuery>,
) -> ApiResult<Json<Vec<AlertDto>>> {
    let records = AlertRepository::new(state.pool)
        .list(q.status.as_deref())
        .await?;
    Ok(Json(records.into_iter().map(alert_to_dto).collect()))
}

#[derive(Deserialize)]
struct RawEventQuery {
    status: Option<String>,
    page: Option<i32>,
    per_page: Option<i32>,
}

#[derive(Deserialize)]
struct MarketMappingQuery {
    enabled: Option<bool>,
}

#[derive(Deserialize)]
struct CreateMarketMappingBody {
    polymarket_condition_id: String,
    polymarket_slug: Option<String>,
    kalshi_ticker: String,
    kalshi_event_ticker: Option<String>,
    #[serde(default = "default_manual")]
    confidence: String,
    #[serde(default)]
    enabled: bool,
}

fn default_manual() -> String {
    "manual".to_owned()
}

#[derive(Deserialize)]
struct UpdateMarketMappingBody {
    polymarket_condition_id: String,
    polymarket_slug: Option<String>,
    kalshi_ticker: String,
    kalshi_event_ticker: Option<String>,
    confidence: String,
    enabled: bool,
}

#[derive(Deserialize)]
struct UpdateEnabledBody {
    enabled: bool,
}

async fn list_market_mappings(
    State(state): State<AppState>,
    Query(q): Query<MarketMappingQuery>,
) -> ApiResult<Json<Vec<MarketMappingDto>>> {
    let records = arbitek_db::MarketMappingRepository::new(state.pool)
        .list(q.enabled)
        .await?;
    Ok(Json(
        records.into_iter().map(market_mapping_to_dto).collect(),
    ))
}

async fn get_market_mapping(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    match arbitek_db::MarketMappingRepository::new(state.pool)
        .get_by_id(id)
        .await?
    {
        Some(r) => Ok(Json(market_mapping_to_dto(r)).into_response()),
        None => Ok(not_found()),
    }
}

async fn create_market_mapping(
    State(state): State<AppState>,
    Json(body): Json<CreateMarketMappingBody>,
) -> ApiResult<Response> {
    let repo = arbitek_db::MarketMappingRepository::new(state.pool);
    let id = repo
        .insert(
            &body.polymarket_condition_id,
            body.polymarket_slug.as_deref(),
            &body.kalshi_ticker,
            body.kalshi_event_ticker.as_deref(),
            &body.confidence,
            body.enabled,
        )
        .await?;
    match repo.get_by_id(id).await? {
        Some(r) => Ok((StatusCode::CREATED, Json(market_mapping_to_dto(r))).into_response()),
        None => Ok((
            StatusCode::CREATED,
            Json(market_mapping_to_dto(arbitek_db::MarketMappingRecord {
                id,
                polymarket_condition_id: body.polymarket_condition_id,
                polymarket_slug: body.polymarket_slug,
                kalshi_ticker: body.kalshi_ticker,
                kalshi_event_ticker: body.kalshi_event_ticker,
                confidence: body.confidence,
                enabled: body.enabled,
                created_at: OffsetDateTime::now_utc(),
            })),
        )
            .into_response()),
    }
}

async fn update_market_mapping(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateMarketMappingBody>,
) -> ApiResult<Response> {
    let repo = arbitek_db::MarketMappingRepository::new(state.pool);
    repo.update(
        id,
        &body.polymarket_condition_id,
        body.polymarket_slug.as_deref(),
        &body.kalshi_ticker,
        body.kalshi_event_ticker.as_deref(),
        &body.confidence,
        body.enabled,
    )
    .await?;
    match repo.get_by_id(id).await? {
        Some(r) => Ok(Json(market_mapping_to_dto(r)).into_response()),
        None => Ok(not_found()),
    }
}

async fn update_market_mapping_enabled(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateEnabledBody>,
) -> ApiResult<Response> {
    let repo = arbitek_db::MarketMappingRepository::new(state.pool);
    repo.update_enabled(id, body.enabled).await?;
    match repo.get_by_id(id).await? {
        Some(r) => Ok(Json(market_mapping_to_dto(r)).into_response()),
        None => Ok(not_found()),
    }
}

async fn delete_market_mapping(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    arbitek_db::MarketMappingRepository::new(state.pool)
        .delete(id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_raw_events(
    State(state): State<AppState>,
    Query(q): Query<RawEventQuery>,
) -> ApiResult<Json<Vec<RawEventDto>>> {
    let filter_status = q.status.as_deref();
    let page = q.page.unwrap_or(1);
    let per_page = q.per_page.unwrap_or(50);
    let result = arbitek_db::RawEventRepository::new(state.pool)
        .list_paginated(filter_status, page, per_page)
        .await?;
    Ok(Json(
        result.items.into_iter().map(raw_event_to_dto).collect(),
    ))
}

async fn ack_alert(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let acked = AlertRepository::new(state.pool).ack(id).await?;
    Ok(Json(serde_json::json!({ "acked": acked })))
}

async fn ack_all_alerts(State(state): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    let count = AlertRepository::new(state.pool).ack_all().await?;
    Ok(Json(serde_json::json!({ "acked": count })))
}

async fn toggle_system(State(state): State<AppState>) -> ApiResult<Json<SystemStatusDto>> {
    let settings = arbitek_db::SystemSettingsRepository::new(state.pool.clone());
    let current = settings.get_bool("live_trading_enabled", false).await?;
    settings.set_bool("live_trading_enabled", !current).await?;
    let dto = build_system_status_dto(&state.pool, &settings, &state.config.worker.mode).await?;
    let _ = state.system_tx.send(dto.clone());
    Ok(Json(dto))
}

async fn toggle_kalshi_execution(
    State(state): State<AppState>,
) -> ApiResult<Json<SystemStatusDto>> {
    let settings = arbitek_db::SystemSettingsRepository::new(state.pool.clone());
    let current = settings.get_bool("kalshi_execution_enabled", false).await?;
    settings
        .set_bool("kalshi_execution_enabled", !current)
        .await?;
    // Full paper reset (wallet + positions + dedup cache) when switching back to paper mode.
    if current {
        reset_paper_state(&state.pool, &settings).await?;
    }
    let dto = build_system_status_dto(&state.pool, &settings, &state.config.worker.mode).await?;
    let _ = state.system_tx.send(dto.clone());
    Ok(Json(dto))
}

async fn reset_paper_wallet(State(state): State<AppState>) -> ApiResult<Json<SystemStatusDto>> {
    let settings = arbitek_db::SystemSettingsRepository::new(state.pool.clone());
    reset_paper_state(&state.pool, &settings).await?;
    let dto = build_system_status_dto(&state.pool, &settings, &state.config.worker.mode).await?;
    let _ = state.system_tx.send(dto.clone());
    Ok(Json(dto))
}

/// Wipe all paper-trading state and reset the wallet to $1 000.
///
/// Called from toggle (live→paper) and the explicit reset endpoint.
/// Positions are cleared so the ledger matches the fresh wallet.
/// raw_events dedup cache is intentionally kept — deleting it would cause
/// the oddpool_copy worker to immediately re-process all recent Polymarket
/// trades on its next poll, re-draining the wallet within seconds.
/// Both ops run in one transaction so a partial failure leaves no stale state.
async fn reset_paper_state(
    pool: &sqlx::PgPool,
    _settings: &arbitek_db::SystemSettingsRepository,
) -> Result<(), ApiError> {
    const PAPER_FOLLOWER: &str = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
    let follower_id = Uuid::parse_str(PAPER_FOLLOWER).expect("sentinel UUID is valid");

    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO system_settings (key, value, updated_at) VALUES ('paper_wallet_balance', '1000', NOW())
         ON CONFLICT (key) DO UPDATE SET value = '1000', updated_at = NOW()"
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM positions WHERE follower_account_id = $1")
        .bind(follower_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

async fn build_system_status_dto(
    pool: &sqlx::PgPool,
    settings: &arbitek_db::SystemSettingsRepository,
    worker_mode: &str,
) -> Result<SystemStatusDto, ApiError> {
    let live_trading = settings.get_bool("live_trading_enabled", false).await?;
    let kalshi_exec = settings.get_bool("kalshi_execution_enabled", false).await?;
    let paper_balance = settings
        .get_decimal("paper_wallet_balance", rust_decimal::Decimal::new(1000, 0))
        .await
        .unwrap_or(rust_decimal::Decimal::new(1000, 0))
        .to_f64()
        .unwrap_or(1000.0);
    let open_count = AlertRepository::new(pool.clone()).count_open().await?;
    let row = sqlx::query("select max(detected_at) as ts from signals")
        .fetch_one(pool)
        .await?;
    let last_ts: Option<OffsetDateTime> = row.try_get("ts")?;
    Ok(SystemStatusDto {
        live_trading_enabled: live_trading,
        kalshi_execution_enabled: kalshi_exec,
        paper_wallet_balance: paper_balance,
        worker_mode: worker_mode.to_owned(),
        open_alert_count: open_count,
        last_signal_at: last_ts.and_then(|dt| dt.format(&Rfc3339).ok()),
    })
}

// ---------------------------------------------------------------------------
// DTO conversions
// ---------------------------------------------------------------------------

pub fn master_to_dto(r: arbitek_db::MasterRecord) -> MasterDto {
    MasterDto {
        id: r.id,
        display_name: r.display_name,
        status: r.status,
        master_weight: r.master_weight.to_f64().unwrap_or(0.0),
        wallet_address: r.wallet_address,
        config: r.config,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

pub fn follower_to_dto(r: arbitek_db::FollowerRecord) -> FollowerDto {
    FollowerDto {
        id: r.id,
        display_name: r.display_name,
        status: r.status,
        base_currency: r.base_currency,
        wallet_address: r.wallet_address,
        platform: r.platform,
        risk_config: r.risk_config,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

pub fn signal_to_dto(r: arbitek_db::SignalRecord, master_wallet: Option<String>) -> SignalDto {
    SignalDto {
        id: r.id,
        platform: r.platform,
        master_id: r.master_id,
        master_wallet,
        market_id: r.market_id,
        outcome: r.outcome,
        side: r.side,
        position_effect: r.position_effect,
        size_contracts: r.size_contracts.to_f64().unwrap_or(0.0),
        notional: r.notional.to_f64().unwrap_or(0.0),
        currency: r.currency,
        entry_price: r.entry_price.to_f64().unwrap_or(0.0),
        event_timestamp: r.event_timestamp,
        detected_at: r.detected_at,
        confidence_score: r.confidence_score.and_then(|d| d.to_f64()),
        status: r.status,
        market_title: r.market_title,
    }
}

pub fn order_to_dto(r: arbitek_db::OrderRecord) -> OrderDto {
    OrderDto {
        id: r.id,
        copy_intent_id: r.copy_intent_id,
        platform: r.platform,
        follower_account_id: r.follower_account_id,
        platform_order_id: r.platform_order_id,
        client_order_id: r.client_order_id,
        market_id: r.market_id,
        outcome: r.outcome,
        side: r.side,
        order_type: r.order_type,
        price_limit: r.price_limit.to_f64().unwrap_or(0.0),
        requested_contracts: r.requested_contracts.to_f64().unwrap_or(0.0),
        filled_contracts: r.filled_contracts.to_f64().unwrap_or(0.0),
        avg_fill_price: r.avg_fill_price.and_then(|d| d.to_f64()),
        status: r.status,
        submitted_at: r.submitted_at,
        last_update_at: r.last_update_at,
    }
}

pub fn fill_to_dto(r: arbitek_db::FillRecord) -> FillDto {
    FillDto {
        id: r.id,
        order_id: r.order_id,
        platform: r.platform,
        platform_fill_id: r.platform_fill_id,
        follower_account_id: r.follower_account_id,
        market_id: r.market_id,
        market_title: r.market_title,
        outcome: r.outcome,
        side: r.side,
        price: r.price.to_f64().unwrap_or(0.0),
        contracts: r.contracts.to_f64().unwrap_or(0.0),
        notional: r.notional.to_f64().unwrap_or(0.0),
        fee: r.fee.to_f64().unwrap_or(0.0),
        is_taker: r.is_taker,
        filled_at: r.filled_at,
        payload: r.payload,
    }
}

pub fn position_to_dto(r: arbitek_db::PositionRecord) -> PositionDto {
    let total_pnl = r.realized_pnl + r.unrealized_pnl;
    PositionDto {
        id: r.id,
        follower_account_id: r.follower_account_id,
        platform: r.platform,
        market_id: r.market_id,
        market_title: r.market_title,
        market_status: r.market_status,
        close_time: r.close_time,
        outcome: r.outcome,
        contracts: r.contracts.to_f64().unwrap_or(0.0),
        avg_price: r.avg_price.to_f64().unwrap_or(0.0),
        realized_pnl: r.realized_pnl.to_f64().unwrap_or(0.0),
        unrealized_pnl: r.unrealized_pnl.to_f64().unwrap_or(0.0),
        total_pnl: total_pnl.to_f64().unwrap_or(0.0),
        exposure: r.exposure.to_f64().unwrap_or(0.0),
        updated_at: r.updated_at,
    }
}

pub fn market_mapping_to_dto(r: arbitek_db::MarketMappingRecord) -> MarketMappingDto {
    MarketMappingDto {
        id: r.id,
        polymarket_condition_id: r.polymarket_condition_id,
        polymarket_slug: r.polymarket_slug,
        kalshi_ticker: r.kalshi_ticker,
        kalshi_event_ticker: r.kalshi_event_ticker,
        confidence: r.confidence,
        enabled: r.enabled,
        created_at: r.created_at,
    }
}

pub fn raw_event_to_dto(r: arbitek_db::RawEventRecord) -> RawEventDto {
    RawEventDto {
        id: r.id,
        platform: r.platform,
        source: r.source,
        external_event_id: r.external_event_id,
        raw_payload: r.raw_payload,
        detected_at: r.detected_at,
        processed_at: r.processed_at,
        status: r.status,
        source_entity_id: r.source_entity_id,
        source_entity_type: r.source_entity_type,
    }
}

pub fn alert_to_dto(r: arbitek_db::AlertRecord) -> AlertDto {
    AlertDto {
        id: r.id,
        severity: r.severity,
        alert_type: r.alert_type,
        status: r.status,
        entity_type: r.entity_type,
        entity_id: r.entity_id,
        message: r.message,
        payload: r.payload,
        created_at: r.created_at,
        acknowledged_at: r.acknowledged_at,
    }
}

async fn spa_fallback() -> Response {
    // Try multiple paths relative to different possible run locations
    let candidates = [
        "frontend/dist/index.html",
        "../frontend/dist/index.html",
        "../../frontend/dist/index.html",
    ];
    for path in &candidates {
        if let Ok(html) = std::fs::read_to_string(path) {
            return axum::response::Html(html).into_response();
        }
    }
    not_found()
}

fn not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({ "error": "not found" })),
    )
        .into_response()
}
