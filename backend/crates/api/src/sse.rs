use crate::{AlertDto, AppState, SignalDto, SystemStatusDto};
use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures_util::stream::{self, Stream};
use rust_decimal::prelude::ToPrimitive;
use serde_json;
use sqlx::Row;
use std::convert::Infallible;
use std::time::Duration;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use tokio::sync::broadcast;
use tokio::time::sleep;
use tracing::{error, warn};

// ---------------------------------------------------------------------------
// SSE handlers
// ---------------------------------------------------------------------------

pub async fn sse_activity_handler(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.activity_tx.subscribe();
    let stream = stream::unfold(rx, |mut rx| async {
        match rx.recv().await {
            Ok(signal) => {
                let payload = serde_json::to_string(&signal).unwrap_or_default();
                Some((Ok(Event::default().data(payload)), rx))
            }
            Err(_) => None,
        }
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

pub async fn sse_alerts_handler(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.alerts_tx.subscribe();
    let stream = stream::unfold(rx, |mut rx| async {
        match rx.recv().await {
            Ok(alert) => {
                let payload = serde_json::to_string(&alert).unwrap_or_default();
                Some((Ok(Event::default().data(payload)), rx))
            }
            Err(_) => None,
        }
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

pub async fn sse_system_handler(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.system_tx.subscribe();
    let stream = stream::unfold(rx, |mut rx| async {
        match rx.recv().await {
            Ok(status) => {
                let payload = serde_json::to_string(&status).unwrap_or_default();
                Some((Ok(Event::default().data(payload)), rx))
            }
            Err(_) => None,
        }
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

// ---------------------------------------------------------------------------
// Background broadcast tasks (raw SQL to avoid modifying db crate)
// ---------------------------------------------------------------------------

pub async fn broadcast_activity(pool: sqlx::PgPool, tx: broadcast::Sender<SignalDto>) {
    let mut last_check = OffsetDateTime::now_utc() - Duration::from_secs(60);
    loop {
        sleep(Duration::from_secs(2)).await;

        let rows = sqlx::query(
            r#"SELECT s.id, s.platform, s.master_id, s.market_id, s.outcome, s.side,
                      s.position_effect, s.size_contracts, s.notional, s.currency,
                      s.entry_price, s.event_timestamp, s.detected_at,
                      s.confidence_score, s.status, s.market_title,
                      m.wallet_address AS master_wallet
               FROM signals s
               LEFT JOIN masters m ON m.id = s.master_id
               WHERE s.detected_at > $1
               ORDER BY s.detected_at ASC LIMIT $2"#,
        )
        .bind(last_check)
        .bind(50i64)
        .fetch_all(&pool)
        .await;

        match rows {
            Ok(records) => {
                if !records.is_empty() {
                    last_check = records
                        .iter()
                        .filter_map(|r| r.try_get::<OffsetDateTime, _>("detected_at").ok())
                        .max()
                        .unwrap_or(last_check);
                    for row in records {
                        if let Ok(dto) = signal_row_to_dto(row) {
                            if tx.send(dto).is_err() {
                                warn!("activity broadcast channel has no receivers");
                            }
                        }
                    }
                }
            }
            Err(e) => {
                error!("activity broadcast query failed: {}", e);
            }
        }
    }
}

pub async fn broadcast_alerts(pool: sqlx::PgPool, tx: broadcast::Sender<AlertDto>) {
    let mut last_check = OffsetDateTime::now_utc() - Duration::from_secs(60);
    loop {
        sleep(Duration::from_secs(3)).await;

        let rows = sqlx::query(
            r#"select id, severity, type, status, entity_type, entity_id, message,
                      payload, created_at, acknowledged_at
               from alerts where created_at > $1 order by created_at asc limit $2"#,
        )
        .bind(last_check)
        .bind(50i64)
        .fetch_all(&pool)
        .await;

        match rows {
            Ok(records) => {
                if !records.is_empty() {
                    last_check = records
                        .iter()
                        .filter_map(|r| r.try_get::<OffsetDateTime, _>("created_at").ok())
                        .max()
                        .unwrap_or(last_check);
                    for row in records {
                        if let Ok(dto) = alert_row_to_dto(row) {
                            if tx.send(dto).is_err() {
                                warn!("alerts broadcast channel has no receivers");
                            }
                        }
                    }
                }
            }
            Err(e) => {
                error!("alerts broadcast query failed: {}", e);
            }
        }
    }
}

pub async fn broadcast_system(
    pool: sqlx::PgPool,
    config: std::sync::Arc<arbitek_config::AppConfig>,
    tx: broadcast::Sender<SystemStatusDto>,
) {
    let settings = arbitek_db::SystemSettingsRepository::new(pool.clone());
    loop {
        sleep(Duration::from_secs(10)).await;

        let live_trading = settings
            .get_bool("live_trading_enabled", false)
            .await
            .unwrap_or(false);
        let kalshi_exec = settings
            .get_bool("kalshi_execution_enabled", false)
            .await
            .unwrap_or(false);
        let paper_balance = settings
            .get_decimal("paper_wallet_balance", rust_decimal::Decimal::new(1000, 0))
            .await
            .unwrap_or(rust_decimal::Decimal::new(1000, 0))
            .to_f64()
            .unwrap_or(1000.0);

        let open_count: i64 =
            sqlx::query_scalar("select count(*) as n from alerts where status = 'open'")
                .fetch_one(&pool)
                .await
                .unwrap_or(0);

        let last_ts: Option<OffsetDateTime> =
            sqlx::query("select max(detected_at) as ts from signals")
                .fetch_one(&pool)
                .await
                .ok()
                .and_then(|row| row.try_get("ts").ok());

        let dto = SystemStatusDto {
            live_trading_enabled: live_trading,
            kalshi_execution_enabled: kalshi_exec,
            paper_wallet_balance: paper_balance,
            worker_mode: config.worker.mode.clone(),
            open_alert_count: open_count,
            last_signal_at: last_ts.and_then(|dt| dt.format(&Rfc3339).ok()),
        };

        if tx.send(dto).is_err() {
            warn!("system broadcast channel has no receivers");
        }
    }
}

// ---------------------------------------------------------------------------
// Row-to-DTO helpers (duplicated from db crate to avoid dependency issues)
// ---------------------------------------------------------------------------

fn signal_row_to_dto(row: sqlx::postgres::PgRow) -> Result<SignalDto, sqlx::Error> {
    let size_contracts: rust_decimal::Decimal = row.try_get("size_contracts")?;
    let notional: rust_decimal::Decimal = row.try_get("notional")?;
    let entry_price: rust_decimal::Decimal = row.try_get("entry_price")?;
    let confidence_score: Option<rust_decimal::Decimal> = row.try_get("confidence_score")?;

    Ok(SignalDto {
        id: row.try_get("id")?,
        platform: row.try_get("platform")?,
        master_id: row.try_get("master_id")?,
        master_wallet: row.try_get("master_wallet").ok().flatten(),
        market_id: row.try_get("market_id")?,
        outcome: row.try_get("outcome")?,
        side: row.try_get("side")?,
        position_effect: row.try_get("position_effect")?,
        size_contracts: size_contracts.to_f64().unwrap_or(0.0),
        notional: notional.to_f64().unwrap_or(0.0),
        currency: row.try_get("currency")?,
        entry_price: entry_price.to_f64().unwrap_or(0.0),
        event_timestamp: row.try_get("event_timestamp")?,
        detected_at: row.try_get("detected_at")?,
        confidence_score: confidence_score.and_then(|d| d.to_f64()),
        status: row.try_get("status")?,
        market_title: row.try_get("market_title")?,
    })
}

fn alert_row_to_dto(row: sqlx::postgres::PgRow) -> Result<AlertDto, sqlx::Error> {
    Ok(AlertDto {
        id: row.try_get("id")?,
        severity: row.try_get("severity")?,
        alert_type: row.try_get("type")?,
        status: row.try_get("status")?,
        entity_type: row.try_get("entity_type")?,
        entity_id: row.try_get("entity_id")?,
        message: row.try_get("message")?,
        payload: row.try_get("payload")?,
        created_at: row.try_get("created_at")?,
        acknowledged_at: row.try_get("acknowledged_at")?,
    })
}

pub async fn broadcast_from_redis(
    redis_url: String,
    pool: sqlx::PgPool,
    activity_tx: broadcast::Sender<SignalDto>,
    _alerts_tx: broadcast::Sender<AlertDto>,
    _system_tx: broadcast::Sender<SystemStatusDto>,
) {
    use redis::streams::{StreamReadOptions, StreamReadReply};
    use redis::AsyncCommands;
    use tracing::info;

    let client = match redis::Client::open(redis_url) {
        Ok(c) => c,
        Err(e) => {
            error!("failed to open Redis client: {}", e);
            return;
        }
    };
    let mut conn = match client.get_connection_manager().await {
        Ok(c) => c,
        Err(e) => {
            error!("failed to connect Redis manager: {}", e);
            return;
        }
    };

    let group_name = "arbitek-api";
    let consumer_name = "api-consumer-1";
    let streams = [
        "raw.events",
        "signals.normalized",
        "copy_intents",
        "execution.reports",
    ];

    // Ensure consumer groups exist
    for stream in &streams {
        let _: Result<String, redis::RedisError> = redis::cmd("XGROUP")
            .arg("CREATE")
            .arg(*stream)
            .arg(group_name)
            .arg("$")
            .arg("MKSTREAM")
            .query_async(&mut conn)
            .await;
    }

    info!("Redis stream consumer started for SSE broadcasts");

    loop {
        let keys: Vec<&str> = streams.iter().map(|s| *s).collect();
        let ids: Vec<&str> = std::iter::repeat(">").take(streams.len()).collect();
        let opts = StreamReadOptions::default()
            .group(group_name, consumer_name)
            .count(10)
            .block(5000);

        let reply: StreamReadReply = match conn.xread_options(&keys, &ids, &opts).await {
            Ok(r) => r,
            Err(_) => {
                sleep(Duration::from_secs(5)).await;
                continue;
            }
        };

        for stream_key in reply.keys {
            for stream_id in stream_key.ids {
                let map = stream_id.map;
                let payload_str = match map.get("payload") {
                    Some(redis::Value::SimpleString(s)) => s.clone(),
                    Some(redis::Value::BulkString(b)) => String::from_utf8_lossy(b).to_string(),
                    _ => continue,
                };

                let envelope: serde_json::Value = match serde_json::from_str(&payload_str) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                match stream_key.key.as_str() {
                    "signals.normalized" => {
                        let signal_id = envelope
                            .get("payload")
                            .and_then(|p| p.get("signal_id"))
                            .and_then(|v| v.as_str())
                            .and_then(|s| s.parse::<uuid::Uuid>().ok());

                        if let Some(sid) = signal_id {
                            let row = sqlx::query(
                                r#"select id,platform,master_id,market_id,outcome,side,position_effect,
                                 size_contracts,notional,currency,entry_price,event_timestamp,detected_at,
                                 confidence_score,status,market_title from signals where id = $1"#
                            )
                            .bind(sid)
                            .fetch_optional(&pool)
                            .await;

                            match row {
                                Ok(Some(r)) => {
                                    if let Ok(dto) = signal_row_to_dto(r) {
                                        if activity_tx.send(dto).is_err() {
                                            warn!("activity broadcast channel has no receivers");
                                        }
                                    }
                                }
                                Ok(None) => {
                                    warn!(signal_id = %sid, "signal not found in DB for Redis event")
                                }
                                Err(e) => error!("failed to fetch signal {} from DB: {}", sid, e),
                            }
                        }
                    }
                    "execution.reports" => {
                        info!(
                            "received execution report: {}",
                            envelope
                                .get("event_type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                        );
                    }
                    _ => {}
                }
            }
        }
    }
}
