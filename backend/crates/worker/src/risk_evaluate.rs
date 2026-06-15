use anyhow::Result;
use arbitek_config::AppConfig;
use arbitek_db::{
    connect_worker as connect, AlertRepository, CopyIntentForExecution, CopyIntentRepository,
    FollowerRepository, OrderRepository,
};
use arbitek_domain::copy_config::CopyConfig;
use arbitek_domain::stable_market_id;
use rust_decimal::Decimal;
use time::{OffsetDateTime, Time};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Record a risk-rejection alert (unless the follower muted notifications) and return
/// `Ok(false)` so the caller can `return reject(...).await` to reject the intent.
async fn reject(
    alert_repo: &AlertRepository,
    intent_id: Uuid,
    muted: bool,
    severity: &str,
    message: String,
) -> Result<bool> {
    if !muted {
        alert_repo
            .insert(
                severity,
                "risk_rejection",
                &message,
                Some("copy_intent"),
                Some(intent_id),
            )
            .await
            .ok();
    }
    Ok(false)
}

/// Evaluate copy intents against follower risk configs.
pub async fn run_risk_evaluate(config: &AppConfig) -> Result<()> {
    let pool = connect(&config.database_url).await?;
    let copy_repo = CopyIntentRepository::new(pool.clone());
    let follower_repo = FollowerRepository::new(pool.clone());
    let order_repo = OrderRepository::new(pool.clone());
    let alert_repo = AlertRepository::new(pool.clone());

    info!("starting risk evaluation worker");

    let interval =
        std::time::Duration::from_millis(config.worker.risk_evaluate_interval_ms.max(50));
    loop {
        let intents = copy_repo.list_pending_for_risk_eval(100).await?;
        if intents.is_empty() {
            debug!("no copy intents pending risk evaluation");
            tokio::time::sleep(interval).await;
            continue;
        }

        info!(count = intents.len(), "evaluating copy intents");

        for intent in &intents {
            match evaluate_intent(intent, &follower_repo, &order_repo, &alert_repo).await {
                Ok(true) => {
                    copy_repo.update_status(intent.id, "approved").await?;
                    info!(intent_id = %intent.id, "approved copy intent");
                }
                Ok(false) => {
                    copy_repo.update_status(intent.id, "rejected").await?;
                    info!(intent_id = %intent.id, "rejected copy intent");
                }
                Err(e) => {
                    warn!(intent_id = %intent.id, error = %e, "failed to evaluate copy intent");
                }
            }
        }
    }
}

async fn evaluate_intent(
    intent: &CopyIntentForExecution,
    follower_repo: &FollowerRepository,
    order_repo: &OrderRepository,
    alert_repo: &AlertRepository,
) -> Result<bool> {
    let follower = match follower_repo.get_by_id(intent.follower_account_id).await? {
        Some(f) => f,
        None => {
            warn!(follower_id = %intent.follower_account_id, "follower not found");
            return Ok(false);
        }
    };

    let cfg = CopyConfig::from_value(&follower.risk_config);
    let muted = cfg.notifications_muted();

    if follower.status != "active" {
        info!(follower_id = %follower.id, status = %follower.status, "follower not active");
        return reject(
            alert_repo,
            intent.id,
            muted,
            "warning",
            format!(
                "Copy intent {} rejected: follower {} is not active (status: {})",
                intent.id, follower.id, follower.status
            ),
        )
        .await;
    }

    let notional = intent.target_contracts * intent.price_limit;
    let is_buy = intent.action.eq_ignore_ascii_case("buy");

    // --- per-trade size bounds (min_buy / max_buy, legacy min/max_notional) ---
    if let Some(min) = cfg.min_order_notional() {
        if notional < min {
            info!(intent_id = %intent.id, notional = %notional, min = %min, "below minimum buy");
            return reject(
                alert_repo,
                intent.id,
                muted,
                "info",
                format!(
                    "Copy intent {} rejected: notional {:.2} below min buy {:.2}",
                    intent.id, notional, min
                ),
            )
            .await;
        }
    }
    if let Some(max) = cfg.max_order_notional() {
        if notional > max {
            info!(intent_id = %intent.id, notional = %notional, max = %max, "above maximum buy");
            return reject(
                alert_repo,
                intent.id,
                muted,
                "warning",
                format!(
                    "Copy intent {} rejected: notional {:.2} above max buy {:.2}",
                    intent.id, notional, max
                ),
            )
            .await;
        }
    }

    // --- per-trade % of bankroll ---
    if let (Some(pct), Some(bankroll)) = (cfg.max_trade_bankroll_pct, cfg.bankroll) {
        let max_trade = bankroll * (pct / Decimal::from(100));
        if notional > max_trade {
            info!(intent_id = %intent.id, notional = %notional, max_trade = %max_trade, "exceeds max trade size");
            return reject(
                alert_repo,
                intent.id,
                muted,
                "warning",
                format!(
                    "Copy intent {} rejected: notional {:.2} exceeds max trade size {:.2} ({}% of bankroll {})",
                    intent.id, notional, max_trade, pct, bankroll
                ),
            )
            .await;
        }
    }

    // --- cumulative spend caps (buys only; sells reduce exposure) ---
    if is_buy {
        let market_id = stable_market_id(intent.market_id, &intent.signal_platform_market_id);

        if let Some(cap) = cfg.max_spend_total {
            let spent = order_repo.buy_spend(follower.id, None, None).await?;
            if spent + notional > cap {
                return reject(
                    alert_repo,
                    intent.id,
                    muted,
                    "warning",
                    format!(
                        "Copy intent {} rejected: total spend {:.2} + {:.2} would exceed max total {:.2}",
                        intent.id, spent, notional, cap
                    ),
                )
                .await;
            }
        }
        if let Some(cap) = cfg.max_spend_per_market {
            let spent = order_repo
                .buy_spend(follower.id, Some(market_id), None)
                .await?;
            if spent + notional > cap {
                return reject(
                    alert_repo,
                    intent.id,
                    muted,
                    "warning",
                    format!(
                        "Copy intent {} rejected: market spend {:.2} + {:.2} would exceed per-market cap {:.2}",
                        intent.id, spent, notional, cap
                    ),
                )
                .await;
            }
        }
        if let Some(cap) = cfg.daily_spend {
            let start_of_day = OffsetDateTime::now_utc().replace_time(Time::MIDNIGHT);
            let spent = order_repo
                .buy_spend(follower.id, None, Some(start_of_day))
                .await?;
            if spent + notional > cap {
                return reject(
                    alert_repo,
                    intent.id,
                    muted,
                    "info",
                    format!(
                        "Copy intent {} rejected: daily spend {:.2} + {:.2} would exceed daily cap {:.2}",
                        intent.id, spent, notional, cap
                    ),
                )
                .await;
            }
        }
    }

    Ok(true)
}
