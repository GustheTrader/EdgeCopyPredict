use anyhow::Result;
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::Value;
pub use sqlx::PgPool as DbPool;
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    PgPool, Row,
};
use std::str::FromStr;
use time::OffsetDateTime;
use uuid::Uuid;

pub async fn connect(database_url: &str) -> Result<PgPool> {
    connect_pool(database_url, 2).await
}

pub async fn connect_api(database_url: &str) -> Result<PgPool> {
    connect_pool(database_url, 5).await
}

/// Single-connection pool per worker — prevents concurrent prepared statement name
/// conflicts (two connections with the same per-connection counter both create
/// "sqlx_s_0" simultaneously and one fails with "already exists").
pub async fn connect_worker(database_url: &str) -> Result<PgPool> {
    connect_pool(database_url, 1).await
}

pub async fn connect_execution_worker(database_url: &str) -> Result<PgPool> {
    connect_pool(database_url, 1).await
}

async fn connect_pool(database_url: &str, max_connections: u32) -> Result<PgPool> {
    let base_url = database_url.split('?').next().unwrap_or(database_url);
    let opts = PgConnectOptions::from_str(base_url)?.statement_cache_capacity(0);

    // Retry up to 12 times (60s total) — Supabase session-mode pooler caps at 15
    // connections; stale connections from recently-killed processes hold slots
    // until TCP keepalives time them out.
    let mut last_err = anyhow::anyhow!("no connection attempts made");
    for attempt in 1u32..=12 {
        match PgPoolOptions::new()
            .max_connections(max_connections)
            .min_connections(0)
            .idle_timeout(std::time::Duration::from_secs(30))
            .connect_with(opts.clone())
            .await
        {
            Ok(pool) => return Ok(pool),
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("EMAXCONNSESSION") || msg.contains("max clients") {
                    eprintln!("[db] pool full ({attempt}/12), retrying in 5s…");
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    last_err = e.into();
                } else {
                    return Err(e.into());
                }
            }
        }
    }
    Err(last_err)
}

pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    sqlx::migrate!("../../migrations").run(pool).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Records
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct MasterRecord {
    pub id: Uuid,
    pub display_name: String,
    pub status: String,
    pub master_weight: Decimal,
    pub wallet_address: Option<String>,
    pub config: Value,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FollowerRecord {
    pub id: Uuid,
    pub display_name: String,
    pub status: String,
    pub base_currency: String,
    pub wallet_address: Option<String>,
    pub platform: String,
    pub risk_config: Value,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SignalRecord {
    pub id: Uuid,
    pub platform: String,
    pub master_id: Uuid,
    pub market_id: Option<Uuid>,
    pub outcome: String,
    pub side: String,
    pub position_effect: String,
    pub size_contracts: Decimal,
    pub notional: Decimal,
    pub currency: String,
    pub entry_price: Decimal,
    pub event_timestamp: OffsetDateTime,
    pub detected_at: OffsetDateTime,
    pub confidence_score: Option<Decimal>,
    pub status: String,
    pub market_title: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OrderRecord {
    pub id: Uuid,
    pub copy_intent_id: Option<Uuid>,
    pub platform: String,
    pub follower_account_id: Uuid,
    pub platform_order_id: Option<String>,
    pub client_order_id: String,
    pub market_id: Option<Uuid>,
    pub outcome: String,
    pub side: String,
    pub order_type: String,
    pub price_limit: Decimal,
    pub requested_contracts: Decimal,
    pub filled_contracts: Decimal,
    pub avg_fill_price: Option<Decimal>,
    pub status: String,
    pub submitted_at: Option<OffsetDateTime>,
    pub last_update_at: OffsetDateTime,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FillRecord {
    pub id: Uuid,
    pub order_id: Uuid,
    pub platform: String,
    pub platform_fill_id: String,
    pub follower_account_id: Uuid,
    pub market_id: Option<Uuid>,
    pub market_title: Option<String>,
    pub outcome: String,
    pub side: String,
    pub price: Decimal,
    pub contracts: Decimal,
    pub notional: Decimal,
    pub fee: Decimal,
    pub is_taker: Option<bool>,
    pub filled_at: OffsetDateTime,
    pub payload: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PositionRecord {
    pub id: Uuid,
    pub follower_account_id: Uuid,
    pub platform: String,
    pub market_id: Uuid,
    pub market_title: Option<String>,
    pub market_status: Option<String>,
    pub close_time: Option<OffsetDateTime>,
    pub outcome: String,
    pub contracts: Decimal,
    pub avg_price: Decimal,
    pub realized_pnl: Decimal,
    pub unrealized_pnl: Decimal,
    pub exposure: Decimal,
    pub updated_at: OffsetDateTime,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AlertRecord {
    pub id: Uuid,
    pub severity: String,
    pub alert_type: String,
    pub status: String,
    pub entity_type: Option<String>,
    pub entity_id: Option<Uuid>,
    pub message: String,
    pub payload: Value,
    pub created_at: OffsetDateTime,
    pub acknowledged_at: Option<OffsetDateTime>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RawEventRecord {
    pub id: Uuid,
    pub platform: String,
    pub source: String,
    pub external_event_id: String,
    pub raw_payload: Value,
    pub detected_at: OffsetDateTime,
    pub processed_at: Option<OffsetDateTime>,
    pub status: String,
    pub source_entity_id: Option<Uuid>,
    pub source_entity_type: Option<String>,
}

// ---------------------------------------------------------------------------
// Filters / Queries
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Deserialize)]
pub struct SignalFilter<'a> {
    pub platform: Option<&'a str>,
    pub master_id: Option<Uuid>,
    pub status: Option<&'a str>,
    pub page: i32,
    pub per_page: i32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FillFilter<'a> {
    pub follower_account_id: Option<Uuid>,
    pub platform: Option<&'a str>,
    pub market_id: Option<Uuid>,
    pub page: i32,
    pub per_page: i32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PositionFilter<'a> {
    pub follower_account_id: Option<Uuid>,
    pub platform: Option<&'a str>,
    pub market_id: Option<Uuid>,
    pub open_only: Option<bool>,
    pub page: i32,
    pub per_page: i32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AlertQuery {
    pub status: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FillQuery {
    pub follower_account_id: Option<Uuid>,
    pub platform: Option<String>,
    pub market_id: Option<Uuid>,
    pub page: Option<i32>,
    pub per_page: Option<i32>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PositionQuery {
    pub follower_account_id: Option<Uuid>,
    pub platform: Option<String>,
    pub market_id: Option<Uuid>,
    pub open_only: Option<bool>,
    pub page: Option<i32>,
    pub per_page: Option<i32>,
}

// ---------------------------------------------------------------------------
// Paginated result
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct PaginatedResult<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i32,
    pub per_page: i32,
}

// ---------------------------------------------------------------------------
// Repositories
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct MasterRepository {
    pool: PgPool,
}

impl MasterRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list(&self) -> Result<Vec<MasterRecord>> {
        let rows = sqlx::query(
            "select id, display_name, status, master_weight, wallet_address, config, created_at, updated_at from masters where status <> 'deleted' order by created_at desc"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(master_from_row).collect())
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<MasterRecord>> {
        let row = sqlx::query(
            "select id, display_name, status, master_weight, wallet_address, config, created_at, updated_at from masters where id = $1 and status <> 'deleted'"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(master_from_row))
    }

    pub async fn update_status(&self, id: Uuid, status: &str) -> Result<()> {
        sqlx::query("update masters set status = $1, updated_at = now() where id = $2")
            .bind(status)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_wallet_address(
        &self,
        id: Uuid,
        wallet_address: Option<&str>,
    ) -> Result<()> {
        sqlx::query("update masters set wallet_address = $1, updated_at = now() where id = $2")
            .bind(wallet_address)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn insert(
        &self,
        display_name: &str,
        status: &str,
        master_weight: Decimal,
        wallet_address: Option<&str>,
        config: &Value,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO masters (id, display_name, status, master_weight, wallet_address, config)
               VALUES ($1, $2, $3, $4, $5, $6)"#,
        )
        .bind(id)
        .bind(display_name)
        .bind(status)
        .bind(master_weight)
        .bind(wallet_address)
        .bind(config)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }
}

#[derive(Clone)]
pub struct FollowerRepository {
    pool: PgPool,
}

impl FollowerRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list(&self) -> Result<Vec<FollowerRecord>> {
        let rows = sqlx::query(
            "select id, display_name, status, base_currency, wallet_address, platform, risk_config, created_at, updated_at from followers where status <> 'deleted' order by created_at desc"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(follower_from_row).collect())
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<FollowerRecord>> {
        let row = sqlx::query(
            "select id, display_name, status, base_currency, wallet_address, platform, risk_config, created_at, updated_at from followers where id = $1 and status <> 'deleted'"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(follower_from_row))
    }

    pub async fn update_status(&self, id: Uuid, status: &str) -> Result<()> {
        sqlx::query("update followers set status = $1, updated_at = now() where id = $2")
            .bind(status)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_risk_config(&self, id: Uuid, config: &Value) -> Result<()> {
        sqlx::query("update followers set risk_config = $1, updated_at = now() where id = $2")
            .bind(config)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_wallet_address(
        &self,
        id: Uuid,
        wallet_address: Option<&str>,
    ) -> Result<()> {
        sqlx::query("update followers set wallet_address = $1, updated_at = now() where id = $2")
            .bind(wallet_address)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_platform(&self, id: Uuid, platform: &str) -> Result<()> {
        sqlx::query("update followers set platform = $1, updated_at = now() where id = $2")
            .bind(platform)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn insert(
        &self,
        display_name: &str,
        status: &str,
        base_currency: &str,
        wallet_address: Option<&str>,
        platform: &str,
        risk_config: &Value,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO followers (id, display_name, status, base_currency, wallet_address, platform, risk_config)
               VALUES ($1, $2, $3, $4, $5, $6, $7)"#
        )
        .bind(id)
        .bind(display_name)
        .bind(status)
        .bind(base_currency)
        .bind(wallet_address)
        .bind(platform)
        .bind(risk_config)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }
}

#[derive(Clone)]
pub struct SignalRepository {
    pool: PgPool,
}

impl SignalRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_paginated(
        &self,
        filter: &SignalFilter<'_>,
    ) -> Result<PaginatedResult<SignalRecord>> {
        let limit = filter.per_page.max(1).min(200) as i64;
        let offset = ((filter.page.saturating_sub(1)) as i64) * limit;

        let rows = sqlx::query(
            "select id, platform, master_id, market_id, outcome, side, position_effect, size_contracts, notional, currency, entry_price, event_timestamp, detected_at, confidence_score, status, market_title from signals order by detected_at desc limit $1 offset $2"
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total: i64 = sqlx::query_scalar("select count(*) from signals")
            .fetch_one(&self.pool)
            .await?;

        Ok(PaginatedResult {
            items: rows.into_iter().map(signal_from_row).collect(),
            total,
            page: filter.page,
            per_page: filter.per_page,
        })
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<SignalRecord>> {
        let row = sqlx::query(
            "select id, platform, master_id, market_id, outcome, side, position_effect, size_contracts, notional, currency, entry_price, event_timestamp, detected_at, confidence_score, status, market_title from signals where id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(signal_from_row))
    }

    pub async fn insert(&self, record: &SignalRecord) -> Result<Uuid> {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO signals (
                id, platform, master_id, market_id, outcome, side, position_effect,
                size_contracts, notional, currency, entry_price, event_timestamp,
                detected_at, confidence_score, status, market_title
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)"#,
        )
        .bind(id)
        .bind(&record.platform)
        .bind(record.master_id)
        .bind(record.market_id)
        .bind(&record.outcome)
        .bind(&record.side)
        .bind(&record.position_effect)
        .bind(record.size_contracts)
        .bind(record.notional)
        .bind(&record.currency)
        .bind(record.entry_price)
        .bind(record.event_timestamp)
        .bind(record.detected_at)
        .bind(record.confidence_score)
        .bind(&record.status)
        .bind(&record.market_title)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }
}

#[derive(Clone)]
pub struct OrderRepository {
    pool: PgPool,
}

impl OrderRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_for_signal(&self, signal_id: Uuid) -> Result<Vec<OrderRecord>> {
        let rows = sqlx::query(
            "select id, copy_intent_id, platform, follower_account_id, platform_order_id, client_order_id, market_id, outcome, side, order_type, price_limit, requested_contracts, filled_contracts, avg_fill_price, status, submitted_at, last_update_at from orders where copy_intent_id = (select id from copy_intents where signal_id = $1) order by last_update_at desc"
        )
        .bind(signal_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(order_from_row).collect())
    }

    pub async fn list_recent(&self, limit: i64) -> Result<Vec<OrderRecord>> {
        let rows = sqlx::query(
            "select id, copy_intent_id, platform, follower_account_id, platform_order_id, client_order_id, market_id, outcome, side, order_type, price_limit, requested_contracts, filled_contracts, avg_fill_price, status, submitted_at, last_update_at from orders order by last_update_at desc limit $1"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(order_from_row).collect())
    }

    pub async fn list_by_status(&self, status: &str, limit: i64) -> Result<Vec<OrderRecord>> {
        let rows = sqlx::query(
            "select id, copy_intent_id, platform, follower_account_id, platform_order_id, client_order_id, market_id, outcome, side, order_type, price_limit, requested_contracts, filled_contracts, avg_fill_price, status, submitted_at, last_update_at from orders where status = $1 order by last_update_at desc limit $2"
        )
        .bind(status)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(order_from_row).collect())
    }

    pub async fn update_status(
        &self,
        id: Uuid,
        status: &str,
        filled_contracts: Option<Decimal>,
        avg_fill_price: Option<Decimal>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE orders SET status = $1, filled_contracts = $2, avg_fill_price = $3, last_update_at = NOW() WHERE id = $4"
        )
        .bind(status)
        .bind(filled_contracts)
        .bind(avg_fill_price)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn order_exists_for_intent(&self, copy_intent_id: Uuid) -> Result<bool> {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM orders WHERE copy_intent_id = $1)")
                .bind(copy_intent_id)
                .fetch_one(&self.pool)
                .await?;
        Ok(exists)
    }
}

#[derive(Clone)]
pub struct FillRepository {
    pool: PgPool,
}

impl FillRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_paginated(
        &self,
        filter: &FillFilter<'_>,
    ) -> Result<PaginatedResult<FillRecord>> {
        let limit = filter.per_page.max(1).min(200) as i64;
        let offset = ((filter.page.saturating_sub(1)) as i64) * limit;

        let rows = sqlx::query(
            "select id, order_id, platform, platform_fill_id, follower_account_id, market_id, market_title, outcome, side, price, contracts, notional, fee, is_taker, filled_at, payload from fills order by filled_at desc limit $1 offset $2"
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total: i64 = sqlx::query_scalar("select count(*) from fills")
            .fetch_one(&self.pool)
            .await?;

        Ok(PaginatedResult {
            items: rows.into_iter().map(fill_from_row).collect(),
            total,
            page: filter.page,
            per_page: filter.per_page,
        })
    }

    pub async fn insert(
        &self,
        order_id: Uuid,
        platform: &str,
        platform_fill_id: &str,
        follower_account_id: Uuid,
        market_id: Option<Uuid>,
        market_title: Option<&str>,
        outcome: &str,
        side: &str,
        price: Decimal,
        contracts: Decimal,
        notional: Decimal,
        fee: Decimal,
        is_taker: Option<bool>,
        filled_at: OffsetDateTime,
        payload: &Value,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO fills (
                id, order_id, platform, platform_fill_id, follower_account_id,
                market_id, market_title, outcome, side, price, contracts, notional,
                fee, is_taker, filled_at, payload
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)"#,
        )
        .bind(id)
        .bind(order_id)
        .bind(platform)
        .bind(platform_fill_id)
        .bind(follower_account_id)
        .bind(market_id)
        .bind(market_title)
        .bind(outcome)
        .bind(side)
        .bind(price)
        .bind(contracts)
        .bind(notional)
        .bind(fee)
        .bind(is_taker)
        .bind(filled_at)
        .bind(payload)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }
}

#[derive(Clone)]
pub struct PositionRepository {
    pool: PgPool,
}

impl PositionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_paginated(
        &self,
        filter: &PositionFilter<'_>,
    ) -> Result<PaginatedResult<PositionRecord>> {
        let limit = filter.per_page.max(1).min(200) as i64;
        let offset = ((filter.page.saturating_sub(1)) as i64) * limit;

        let rows = sqlx::query(
            "select id, follower_account_id, platform, market_id, market_title, market_status, close_time, outcome, contracts, avg_price, realized_pnl, unrealized_pnl, exposure, updated_at from positions order by updated_at desc limit $1 offset $2"
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total: i64 = sqlx::query_scalar("select count(*) from positions")
            .fetch_one(&self.pool)
            .await?;

        Ok(PaginatedResult {
            items: rows.into_iter().map(position_from_row).collect(),
            total,
            page: filter.page,
            per_page: filter.per_page,
        })
    }

    pub async fn upsert(
        &self,
        follower_account_id: Uuid,
        platform: &str,
        market_id: Uuid,
        market_title: Option<&str>,
        market_status: Option<&str>,
        outcome: &str,
        contracts_delta: Decimal,
        price: Decimal,
        fee: Decimal,
    ) -> Result<()> {
        // All SET expressions use the pre-update row values (Postgres semantics).
        //
        // Realized P&L is only crystalized when closing (selling) contracts:
        //   pnl = contracts_sold * (sell_price - avg_entry_price) - fee
        // On a buy (delta > 0) we only deduct the fee (entry cost); the gain/loss is
        // unrealized until the market resolves or we sell.
        //
        // avg_price is a weighted average that only updates on buys; selling does not
        // change the entry basis of the remaining contracts.
        //
        // Exposure = open contracts * avg_entry_price.
        let updated = sqlx::query(
            r#"UPDATE positions SET
                realized_pnl = realized_pnl + CASE
                    WHEN $1 < 0 THEN (-$1) * ($2 - avg_price) - $3
                    ELSE -$3
                END,
                contracts = contracts + $1,
                avg_price = CASE
                    WHEN contracts + $1 > 0 AND $1 > 0 THEN
                        ((avg_price * contracts) + ($2 * $1)) / (contracts + $1)
                    WHEN contracts + $1 > 0 THEN avg_price
                    ELSE avg_price
                END,
                exposure = CASE
                    WHEN contracts + $1 > 0 THEN
                        (contracts + $1) * CASE
                            WHEN $1 > 0 THEN ((avg_price * contracts) + ($2 * $1)) / (contracts + $1)
                            ELSE avg_price
                        END
                    ELSE 0
                END,
                market_title = COALESCE($4, market_title),
                market_status = COALESCE($5, market_status),
                updated_at = NOW()
            WHERE follower_account_id = $6 AND market_id = $7 AND outcome = $8"#
        )
        .bind(contracts_delta)
        .bind(price)
        .bind(fee)
        .bind(market_title)
        .bind(market_status)
        .bind(follower_account_id)
        .bind(market_id)
        .bind(outcome)
        .execute(&self.pool)
        .await?;

        if updated.rows_affected() == 0 {
            // First trade on this position — realized P&L starts at zero minus any entry fee.
            // avg_price only set for buys; sell-first is an edge case we treat as zero basis.
            let is_buy = contracts_delta > Decimal::ZERO;
            sqlx::query(
                r#"INSERT INTO positions (
                    id, follower_account_id, platform, market_id, market_title,
                    market_status, outcome, contracts, avg_price, realized_pnl,
                    unrealized_pnl, exposure, updated_at
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, NOW())"#,
            )
            .bind(Uuid::new_v4())
            .bind(follower_account_id)
            .bind(platform)
            .bind(market_id)
            .bind(market_title)
            .bind(market_status)
            .bind(outcome)
            .bind(contracts_delta)
            .bind(if is_buy { price } else { Decimal::ZERO })
            .bind(-fee)
            .bind(Decimal::ZERO)
            .bind(if is_buy {
                contracts_delta * price
            } else {
                Decimal::ZERO
            })
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    pub async fn refresh_unrealized_pnl(&self, id: Uuid, current_price: Decimal) -> Result<()> {
        sqlx::query(
            r#"UPDATE positions SET
                unrealized_pnl = contracts * ($1 - avg_price),
                updated_at = NOW()
            WHERE id = $2"#,
        )
        .bind(current_price)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_open(&self, limit: i64) -> Result<Vec<PositionRecord>> {
        let rows = sqlx::query(
            r#"SELECT id, follower_account_id, platform, market_id, market_title,
                      market_status, close_time, outcome, contracts, avg_price,
                      realized_pnl, unrealized_pnl, exposure, updated_at
               FROM positions WHERE contracts <> 0 ORDER BY updated_at DESC LIMIT $1"#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(position_from_row).collect())
    }

    pub async fn delete_for_follower(&self, follower_account_id: Uuid) -> Result<u64> {
        let result = sqlx::query("DELETE FROM positions WHERE follower_account_id = $1")
            .bind(follower_account_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }
}

#[derive(Clone)]
pub struct AlertRepository {
    pool: PgPool,
}

impl AlertRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list(&self, status: Option<&str>) -> Result<Vec<AlertRecord>> {
        let rows = match status {
            Some(s) => {
                sqlx::query(
                    "select id, severity, type, status, entity_type, entity_id, message, payload, created_at, acknowledged_at from alerts where status = $1 order by created_at desc limit 200"
                )
                .bind(s)
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query(
                    "select id, severity, type, status, entity_type, entity_id, message, payload, created_at, acknowledged_at from alerts order by created_at desc limit 200"
                )
                .fetch_all(&self.pool)
                .await?
            }
        };
        Ok(rows.into_iter().map(alert_from_row).collect())
    }

    pub async fn ack(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query(
            "update alerts set status = 'acknowledged', acknowledged_at = now() where id = $1 and status = 'open'"
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn ack_all(&self) -> Result<u64> {
        let result = sqlx::query(
            "update alerts set status = 'acknowledged', acknowledged_at = now() where status = 'open'"
        )
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    pub async fn count_open(&self) -> Result<i64> {
        let row = sqlx::query("select count(*) as n from alerts where status = 'open'")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.try_get("n")?)
    }

    pub async fn insert(
        &self,
        severity: &str,
        alert_type: &str,
        message: &str,
        entity_type: Option<&str>,
        entity_id: Option<Uuid>,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO alerts (id, severity, type, status, message, entity_type, entity_id) VALUES ($1, $2, $3, 'open', $4, $5, $6)"
        )
        .bind(Uuid::new_v4())
        .bind(severity)
        .bind(alert_type)
        .bind(message)
        .bind(entity_type)
        .bind(entity_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Row converters
// ---------------------------------------------------------------------------

fn master_from_row(row: sqlx::postgres::PgRow) -> MasterRecord {
    MasterRecord {
        id: row.try_get("id").unwrap_or_default(),
        display_name: row.try_get("display_name").unwrap_or_default(),
        status: row.try_get("status").unwrap_or_default(),
        master_weight: row.try_get("master_weight").unwrap_or_default(),
        wallet_address: row.try_get("wallet_address").ok(),
        config: row.try_get("config").unwrap_or_default(),
        created_at: row
            .try_get("created_at")
            .unwrap_or(OffsetDateTime::now_utc()),
        updated_at: row
            .try_get("updated_at")
            .unwrap_or(OffsetDateTime::now_utc()),
    }
}

fn follower_from_row(row: sqlx::postgres::PgRow) -> FollowerRecord {
    FollowerRecord {
        id: row.try_get("id").unwrap_or_default(),
        display_name: row.try_get("display_name").unwrap_or_default(),
        status: row.try_get("status").unwrap_or_default(),
        base_currency: row.try_get("base_currency").unwrap_or_default(),
        wallet_address: row.try_get("wallet_address").ok(),
        platform: row.try_get("platform").unwrap_or_default(),
        risk_config: row.try_get("risk_config").unwrap_or_default(),
        created_at: row
            .try_get("created_at")
            .unwrap_or(OffsetDateTime::now_utc()),
        updated_at: row
            .try_get("updated_at")
            .unwrap_or(OffsetDateTime::now_utc()),
    }
}

fn signal_from_row(row: sqlx::postgres::PgRow) -> SignalRecord {
    SignalRecord {
        id: row.try_get("id").unwrap_or_default(),
        platform: row.try_get("platform").unwrap_or_default(),
        master_id: row.try_get("master_id").unwrap_or_default(),
        market_id: row.try_get("market_id").ok(),
        outcome: row.try_get("outcome").unwrap_or_default(),
        side: row.try_get("side").unwrap_or_default(),
        position_effect: row.try_get("position_effect").unwrap_or_default(),
        size_contracts: row.try_get("size_contracts").unwrap_or_default(),
        notional: row.try_get("notional").unwrap_or_default(),
        currency: row.try_get("currency").unwrap_or_default(),
        entry_price: row.try_get("entry_price").unwrap_or_default(),
        event_timestamp: row
            .try_get("event_timestamp")
            .unwrap_or(OffsetDateTime::now_utc()),
        detected_at: row
            .try_get("detected_at")
            .unwrap_or(OffsetDateTime::now_utc()),
        confidence_score: row.try_get("confidence_score").ok(),
        status: row.try_get("status").unwrap_or_default(),
        market_title: row.try_get("market_title").ok(),
    }
}

fn order_from_row(row: sqlx::postgres::PgRow) -> OrderRecord {
    OrderRecord {
        id: row.try_get("id").unwrap_or_default(),
        copy_intent_id: row.try_get("copy_intent_id").ok(),
        platform: row.try_get("platform").unwrap_or_default(),
        follower_account_id: row.try_get("follower_account_id").unwrap_or_default(),
        platform_order_id: row.try_get("platform_order_id").ok(),
        client_order_id: row.try_get("client_order_id").unwrap_or_default(),
        market_id: row.try_get("market_id").ok(),
        outcome: row.try_get("outcome").unwrap_or_default(),
        side: row.try_get("side").unwrap_or_default(),
        order_type: row.try_get("order_type").unwrap_or_default(),
        price_limit: row.try_get("price_limit").unwrap_or_default(),
        requested_contracts: row.try_get("requested_contracts").unwrap_or_default(),
        filled_contracts: row.try_get("filled_contracts").unwrap_or_default(),
        avg_fill_price: row.try_get("avg_fill_price").ok(),
        status: row.try_get("status").unwrap_or_default(),
        submitted_at: row.try_get("submitted_at").ok(),
        last_update_at: row
            .try_get("last_update_at")
            .unwrap_or(OffsetDateTime::now_utc()),
    }
}

fn fill_from_row(row: sqlx::postgres::PgRow) -> FillRecord {
    FillRecord {
        id: row.try_get("id").unwrap_or_default(),
        order_id: row.try_get("order_id").unwrap_or_default(),
        platform: row.try_get("platform").unwrap_or_default(),
        platform_fill_id: row.try_get("platform_fill_id").unwrap_or_default(),
        follower_account_id: row.try_get("follower_account_id").unwrap_or_default(),
        market_id: row.try_get("market_id").ok(),
        market_title: row.try_get("market_title").ok(),
        outcome: row.try_get("outcome").unwrap_or_default(),
        side: row.try_get("side").unwrap_or_default(),
        price: row.try_get("price").unwrap_or_default(),
        contracts: row.try_get("contracts").unwrap_or_default(),
        notional: row.try_get("notional").unwrap_or_default(),
        fee: row.try_get("fee").unwrap_or_default(),
        is_taker: row.try_get("is_taker").ok(),
        filled_at: row
            .try_get("filled_at")
            .unwrap_or(OffsetDateTime::now_utc()),
        payload: row.try_get("payload").unwrap_or_default(),
    }
}

fn position_from_row(row: sqlx::postgres::PgRow) -> PositionRecord {
    PositionRecord {
        id: row.try_get("id").unwrap_or_default(),
        follower_account_id: row.try_get("follower_account_id").unwrap_or_default(),
        platform: row.try_get("platform").unwrap_or_default(),
        market_id: row.try_get("market_id").unwrap_or_default(),
        market_title: row.try_get("market_title").ok(),
        market_status: row.try_get("market_status").ok(),
        close_time: row.try_get("close_time").ok(),
        outcome: row.try_get("outcome").unwrap_or_default(),
        contracts: row.try_get("contracts").unwrap_or_default(),
        avg_price: row.try_get("avg_price").unwrap_or_default(),
        realized_pnl: row.try_get("realized_pnl").unwrap_or_default(),
        unrealized_pnl: row.try_get("unrealized_pnl").unwrap_or_default(),
        exposure: row.try_get("exposure").unwrap_or_default(),
        updated_at: row
            .try_get("updated_at")
            .unwrap_or(OffsetDateTime::now_utc()),
    }
}

fn alert_from_row(row: sqlx::postgres::PgRow) -> AlertRecord {
    AlertRecord {
        id: row.try_get("id").unwrap_or_default(),
        severity: row.try_get("severity").unwrap_or_default(),
        alert_type: row.try_get("type").unwrap_or_default(),
        status: row.try_get("status").unwrap_or_default(),
        entity_type: row.try_get("entity_type").ok(),
        entity_id: row.try_get("entity_id").ok(),
        message: row.try_get("message").unwrap_or_default(),
        payload: row.try_get("payload").unwrap_or_default(),
        created_at: row
            .try_get("created_at")
            .unwrap_or(OffsetDateTime::now_utc()),
        acknowledged_at: row.try_get("acknowledged_at").ok(),
    }
}

// ---------------------------------------------------------------------------
// Copy intent records (for live execution)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct CopyIntentForExecution {
    pub id: Uuid,
    pub follower_account_id: Uuid,
    pub platform: String,
    pub signal_platform_market_id: String,
    pub market_id: Option<Uuid>,
    pub outcome: String,
    pub action: String,
    pub target_contracts: Decimal,
    pub price_limit: Decimal,
}

pub struct CopyIntentRepository {
    pool: PgPool,
}

impl CopyIntentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_approved_for_execution(
        &self,
        limit: i64,
    ) -> Result<Vec<CopyIntentForExecution>> {
        // Exclude the paper follower — paper intents are handled by the paper-execution worker.
        let paper_follower = Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap();
        let rows = sqlx::query(
            r#"SELECT id, follower_account_id, platform, signal_platform_market_id,
                      market_id, outcome, action, target_contracts, price_limit
               FROM copy_intents
               WHERE status = 'approved' AND follower_account_id != $1
               ORDER BY created_at ASC
               LIMIT $2"#,
        )
        .bind(paper_follower)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(copy_intent_from_row).collect())
    }

    pub async fn list_approved_for_follower(
        &self,
        follower_id: Uuid,
        limit: i64,
    ) -> Result<Vec<CopyIntentForExecution>> {
        let rows = sqlx::query(
            r#"SELECT id, follower_account_id, platform, signal_platform_market_id,
                      market_id, outcome, action, target_contracts, price_limit
               FROM copy_intents
               WHERE status = 'approved' AND follower_account_id = $1
               ORDER BY created_at ASC
               LIMIT $2"#,
        )
        .bind(follower_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(copy_intent_from_row).collect())
    }

    pub async fn list_pending_for_risk_eval(
        &self,
        limit: i64,
    ) -> Result<Vec<CopyIntentForExecution>> {
        let rows = sqlx::query(
            r#"SELECT id, follower_account_id, platform, signal_platform_market_id,
                      market_id, outcome, action, target_contracts, price_limit
               FROM copy_intents
               WHERE status = 'pending'
               ORDER BY created_at ASC
               LIMIT $1"#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(copy_intent_from_row).collect())
    }

    pub async fn update_status(&self, id: Uuid, status: &str) -> Result<()> {
        sqlx::query("UPDATE copy_intents SET status = $1 WHERE id = $2")
            .bind(status)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn insert(
        &self,
        follower_account_id: Uuid,
        platform: &str,
        signal_platform_market_id: &str,
        market_id: Option<Uuid>,
        outcome: &str,
        action: &str,
        target_contracts: Decimal,
        price_limit: Decimal,
        signal_id: Uuid,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO copy_intents (
                id, follower_account_id, platform, signal_platform_market_id,
                market_id, outcome, action, target_contracts, price_limit, status, signal_id
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'pending', $10)"#,
        )
        .bind(id)
        .bind(follower_account_id)
        .bind(platform)
        .bind(signal_platform_market_id)
        .bind(market_id)
        .bind(outcome)
        .bind(action)
        .bind(target_contracts)
        .bind(price_limit)
        .bind(signal_id)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }
}

fn copy_intent_from_row(r: sqlx::postgres::PgRow) -> CopyIntentForExecution {
    CopyIntentForExecution {
        id: r.try_get("id").unwrap_or_default(),
        follower_account_id: r.try_get("follower_account_id").unwrap_or_default(),
        platform: r.try_get("platform").unwrap_or_default(),
        signal_platform_market_id: r.try_get("signal_platform_market_id").unwrap_or_default(),
        market_id: r.try_get("market_id").ok(),
        outcome: r.try_get("outcome").unwrap_or_default(),
        action: r.try_get("action").unwrap_or_default(),
        target_contracts: r.try_get("target_contracts").unwrap_or_default(),
        price_limit: r.try_get("price_limit").unwrap_or_default(),
    }
}

// ---------------------------------------------------------------------------
// OrderRepository extension for live execution
// ---------------------------------------------------------------------------

impl OrderRepository {
    pub async fn insert_from_live_output(
        &self,
        output: arbitek_execution::live::LiveExecutionOutput,
        intent: &CopyIntentForExecution,
    ) -> Result<Uuid> {
        let order_id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO orders (
                id, copy_intent_id, platform, follower_account_id,
                platform_order_id, client_order_id, market_id, outcome, side,
                order_type, price_limit, requested_contracts, filled_contracts,
                avg_fill_price, status, submitted_at, last_update_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9,
                'market', $10, $11, $12, $13, $14, $15, $16
            )"#,
        )
        .bind(order_id)
        .bind(intent.id)
        .bind(&intent.platform)
        .bind(intent.follower_account_id)
        .bind(output.platform_order_id)
        .bind(format!("live:{}", order_id))
        .bind(intent.market_id)
        .bind(&intent.outcome)
        .bind(&intent.action)
        .bind(intent.price_limit)
        .bind(intent.target_contracts)
        .bind(output.filled_contracts)
        .bind(output.avg_fill_price)
        .bind(output.status)
        .bind(output.submitted_at)
        .bind(OffsetDateTime::now_utc())
        .execute(&self.pool)
        .await?;
        Ok(order_id)
    }

    /// Insert a simulated paper order (no real exchange interaction).
    /// The order is immediately marked as 'filled' at the given price.
    pub async fn insert_paper_order(
        &self,
        intent: &CopyIntentForExecution,
        fill_price: Decimal,
        fill_contracts: Decimal,
    ) -> Result<Uuid> {
        let order_id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO orders (
                id, copy_intent_id, platform, follower_account_id,
                platform_order_id, client_order_id, market_id, outcome, side,
                order_type, price_limit, requested_contracts, filled_contracts,
                avg_fill_price, status, submitted_at, last_update_at
            ) VALUES (
                $1, $2, $3, $4, NULL, $5, $6, $7, $8,
                'limit', $9, $10, $11, $12, 'filled', NOW(), NOW()
            )"#,
        )
        .bind(order_id)
        .bind(intent.id)
        .bind(&intent.platform)
        .bind(intent.follower_account_id)
        .bind(format!("paper:{}", order_id))
        .bind(intent.market_id)
        .bind(&intent.outcome)
        .bind(&intent.action)
        .bind(intent.price_limit)
        .bind(intent.target_contracts)
        .bind(fill_contracts)
        .bind(fill_price)
        .execute(&self.pool)
        .await?;
        Ok(order_id)
    }
}

pub struct RawEventRepository {
    pool: PgPool,
}

impl RawEventRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_by_external_event_id(
        &self,
        platform: &str,
        external_event_id: &str,
    ) -> Result<Option<RawEventRecord>> {
        let row = sqlx::query(
            r#"SELECT id, platform, source, external_event_id, raw_payload, detected_at, processed_at, status, source_entity_id, source_entity_type
               FROM raw_events WHERE platform = $1 AND external_event_id = $2"#
        )
        .bind(platform)
        .bind(external_event_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(raw_event_from_row))
    }

    pub async fn insert_or_ignore(&self, record: &RawEventRecord) -> Result<bool> {
        let result = sqlx::query(
            r#"INSERT INTO raw_events (id, platform, source, external_event_id, raw_payload, detected_at, status, source_entity_id, source_entity_type)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
               ON CONFLICT (platform, external_event_id) DO NOTHING"#
        )
        .bind(record.id)
        .bind(&record.platform)
        .bind(&record.source)
        .bind(&record.external_event_id)
        .bind(&record.raw_payload)
        .bind(record.detected_at)
        .bind(&record.status)
        .bind(record.source_entity_id)
        .bind(record.source_entity_type.as_deref())
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn list_unprocessed(&self, limit: i64) -> Result<Vec<RawEventRecord>> {
        let rows = sqlx::query(
            r#"SELECT id, platform, source, external_event_id, raw_payload, detected_at, processed_at, status, source_entity_id, source_entity_type
               FROM raw_events WHERE status = 'new' ORDER BY detected_at ASC LIMIT $1"#
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(raw_event_from_row).collect())
    }

    pub async fn mark_processed(&self, id: Uuid) -> Result<()> {
        sqlx::query(
            "UPDATE raw_events SET status = 'processed', processed_at = NOW() WHERE id = $1",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Recover the Polymarket condition_id for a position whose market_id was derived
    /// from the first 16 bytes of the condition_id hex. Searches all raw_events by
    /// matching the condition_id prefix against the UUID's hex representation.
    pub async fn find_condition_id_by_market_uuid(
        &self,
        market_id: Uuid,
    ) -> Result<Option<String>> {
        let prefix = market_id.as_simple().to_string(); // 32 hex chars, no dashes
        let pattern = format!("0x{}%", prefix);
        // The serialised PolymarketDataApiTrade stores condition_id at the top level
        // (snake_case field name from serde).  The nested `raw` field mirrors the
        // original API payload which uses camelCase conditionId — check both.
        let row: Option<String> = sqlx::query_scalar(
            r#"SELECT coalesce(
                   raw_payload->>'condition_id',
                   raw_payload->'raw'->>'conditionId'
               )
               FROM raw_events
               WHERE lower(coalesce(
                   raw_payload->>'condition_id',
                   raw_payload->'raw'->>'conditionId'
               )) LIKE lower($1)
               LIMIT 1"#,
        )
        .bind(pattern)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn delete_by_source(&self, source: &str) -> Result<u64> {
        let result = sqlx::query("DELETE FROM raw_events WHERE source = $1")
            .bind(source)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn list_paginated(
        &self,
        status: Option<&str>,
        page: i32,
        per_page: i32,
    ) -> Result<PaginatedResult<RawEventRecord>> {
        let limit = per_page.max(1).min(200) as i64;
        let offset = ((page.saturating_sub(1)) as i64) * limit;

        // Always 3 params: ($1 = optional status filter or NULL, $2 = limit, $3 = offset).
        // Using a single stable query shape avoids prepared-statement cache mismatches
        // when pgBouncer session pooler reuses server connections.
        let rows = sqlx::query(
            "SELECT id, platform, source, external_event_id, raw_payload, detected_at, processed_at, status, source_entity_id, source_entity_type FROM raw_events WHERE ($1::text IS NULL OR status = $1::text) ORDER BY detected_at DESC LIMIT $2 OFFSET $3"
        )
        .bind(status).bind(limit).bind(offset)
        .fetch_all(&self.pool).await?;

        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM raw_events WHERE ($1::text IS NULL OR status = $1::text)",
        )
        .bind(status)
        .fetch_one(&self.pool)
        .await?;

        Ok(PaginatedResult {
            items: rows.into_iter().map(raw_event_from_row).collect(),
            total,
            page,
            per_page,
        })
    }
}

fn raw_event_from_row(row: sqlx::postgres::PgRow) -> RawEventRecord {
    RawEventRecord {
        id: row.try_get("id").unwrap_or_default(),
        platform: row.try_get("platform").unwrap_or_default(),
        source: row.try_get("source").unwrap_or_default(),
        external_event_id: row.try_get("external_event_id").unwrap_or_default(),
        raw_payload: row.try_get("raw_payload").unwrap_or_default(),
        detected_at: row
            .try_get("detected_at")
            .unwrap_or(OffsetDateTime::now_utc()),
        processed_at: row.try_get("processed_at").ok(),
        status: row.try_get("status").unwrap_or_default(),
        source_entity_id: row.try_get("source_entity_id").ok(),
        source_entity_type: row.try_get("source_entity_type").ok(),
    }
}

// ---------------------------------------------------------------------------
// Market mapping records
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct MarketMappingRecord {
    pub id: Uuid,
    pub polymarket_condition_id: String,
    pub polymarket_slug: Option<String>,
    pub kalshi_ticker: String,
    pub kalshi_event_ticker: Option<String>,
    pub confidence: String,
    pub enabled: bool,
    pub created_at: OffsetDateTime,
}

#[derive(Clone)]
pub struct MarketMappingRepository {
    pool: PgPool,
}

impl MarketMappingRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list(&self, enabled_only: Option<bool>) -> Result<Vec<MarketMappingRecord>> {
        let rows = match enabled_only {
            Some(true) => {
                sqlx::query(
                    "select id, polymarket_condition_id, polymarket_slug, kalshi_ticker, kalshi_event_ticker, confidence, enabled, created_at from market_mappings where enabled = true order by created_at desc"
                )
                .fetch_all(&self.pool)
                .await?
            }
            Some(false) => {
                sqlx::query(
                    "select id, polymarket_condition_id, polymarket_slug, kalshi_ticker, kalshi_event_ticker, confidence, enabled, created_at from market_mappings where enabled = false order by created_at desc"
                )
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query(
                    "select id, polymarket_condition_id, polymarket_slug, kalshi_ticker, kalshi_event_ticker, confidence, enabled, created_at from market_mappings order by created_at desc"
                )
                .fetch_all(&self.pool)
                .await?
            }
        };
        Ok(rows.into_iter().map(market_mapping_from_row).collect())
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<MarketMappingRecord>> {
        let row = sqlx::query(
            "select id, polymarket_condition_id, polymarket_slug, kalshi_ticker, kalshi_event_ticker, confidence, enabled, created_at from market_mappings where id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(market_mapping_from_row))
    }

    pub async fn insert(
        &self,
        polymarket_condition_id: &str,
        polymarket_slug: Option<&str>,
        kalshi_ticker: &str,
        kalshi_event_ticker: Option<&str>,
        confidence: &str,
        enabled: bool,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO market_mappings (id, polymarket_condition_id, polymarket_slug, kalshi_ticker, kalshi_event_ticker, confidence, enabled)
               VALUES ($1, $2, $3, $4, $5, $6, $7)"#
        )
        .bind(id)
        .bind(polymarket_condition_id)
        .bind(polymarket_slug)
        .bind(kalshi_ticker)
        .bind(kalshi_event_ticker)
        .bind(confidence)
        .bind(enabled)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    pub async fn update(
        &self,
        id: Uuid,
        polymarket_condition_id: &str,
        polymarket_slug: Option<&str>,
        kalshi_ticker: &str,
        kalshi_event_ticker: Option<&str>,
        confidence: &str,
        enabled: bool,
    ) -> Result<()> {
        sqlx::query(
            r#"UPDATE market_mappings SET
                polymarket_condition_id = $1,
                polymarket_slug = $2,
                kalshi_ticker = $3,
                kalshi_event_ticker = $4,
                confidence = $5,
                enabled = $6,
                created_at = NOW()
            WHERE id = $7"#,
        )
        .bind(polymarket_condition_id)
        .bind(polymarket_slug)
        .bind(kalshi_ticker)
        .bind(kalshi_event_ticker)
        .bind(confidence)
        .bind(enabled)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_enabled(&self, id: Uuid, enabled: bool) -> Result<()> {
        sqlx::query("UPDATE market_mappings SET enabled = $1, created_at = NOW() WHERE id = $2")
            .bind(enabled)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM market_mappings WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

fn market_mapping_from_row(row: sqlx::postgres::PgRow) -> MarketMappingRecord {
    MarketMappingRecord {
        id: row.try_get("id").unwrap_or_default(),
        polymarket_condition_id: row.try_get("polymarket_condition_id").unwrap_or_default(),
        polymarket_slug: row.try_get("polymarket_slug").ok(),
        kalshi_ticker: row.try_get("kalshi_ticker").unwrap_or_default(),
        kalshi_event_ticker: row.try_get("kalshi_event_ticker").ok(),
        confidence: row.try_get("confidence").unwrap_or_default(),
        enabled: row.try_get("enabled").unwrap_or(false),
        created_at: row
            .try_get("created_at")
            .unwrap_or(OffsetDateTime::now_utc()),
    }
}

// ---------------------------------------------------------------------------
// System settings
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct SystemSettingsRepository {
    pool: PgPool,
}

impl SystemSettingsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get(&self, key: &str) -> Result<Option<String>> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT value FROM system_settings WHERE key = $1")
                .bind(key)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.map(|r| r.0))
    }

    pub async fn set(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO system_settings (key, value, updated_at) VALUES ($1, $2, NOW())
             ON CONFLICT (key) DO UPDATE SET value = $2, updated_at = NOW()",
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_bool(&self, key: &str, default: bool) -> Result<bool> {
        match self.get(key).await? {
            Some(v) => Ok(v.parse::<bool>().unwrap_or(default)),
            None => Ok(default),
        }
    }

    pub async fn set_bool(&self, key: &str, value: bool) -> Result<()> {
        self.set(key, if value { "true" } else { "false" }).await
    }

    pub async fn get_decimal(
        &self,
        key: &str,
        default: rust_decimal::Decimal,
    ) -> Result<rust_decimal::Decimal> {
        match self.get(key).await? {
            Some(v) => Ok(v.parse::<rust_decimal::Decimal>().unwrap_or(default)),
            None => Ok(default),
        }
    }

    pub async fn set_decimal(&self, key: &str, value: rust_decimal::Decimal) -> Result<()> {
        self.set(key, &value.to_string()).await
    }
}
