use anyhow::Result;
use arbitek_config::AppConfig;
use arbitek_db::{
    connect_worker as connect, AlertRepository, CopyIntentForExecution, CopyIntentRepository,
    FollowerRepository,
};
use rust_decimal::Decimal;
use tracing::{info, warn};

/// Risk configuration for a follower.
#[derive(Clone, Debug, serde::Deserialize)]
struct ParsedRiskConfig {
    bankroll: Option<Decimal>,
    copy_scale: Option<Decimal>,
    min_notional: Option<Decimal>,
    max_notional: Option<Decimal>,
    max_trade_bankroll_pct: Option<Decimal>,
}

/// Evaluate copy intents against follower risk configs.
pub async fn run_risk_evaluate(config: &AppConfig) -> Result<()> {
    let pool = connect(&config.database_url).await?;
    let copy_repo = CopyIntentRepository::new(pool.clone());
    let follower_repo = FollowerRepository::new(pool.clone());

    info!("starting risk evaluation worker");

    let interval = std::time::Duration::from_secs(5);
    loop {
        let intents = copy_repo.list_pending_for_risk_eval(100).await?;
        if intents.is_empty() {
            info!("no copy intents pending risk evaluation");
            tokio::time::sleep(interval).await;
            continue;
        }

        info!(count = intents.len(), "evaluating copy intents");

        for intent in &intents {
            match evaluate_intent(intent, &follower_repo, &AlertRepository::new(pool.clone())).await
            {
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
    alert_repo: &AlertRepository,
) -> Result<bool> {
    let follower = match follower_repo.get_by_id(intent.follower_account_id).await? {
        Some(f) => f,
        None => {
            warn!(follower_id = %intent.follower_account_id, "follower not found");
            return Ok(false);
        }
    };

    // Parse risk config
    let risk: ParsedRiskConfig =
        match serde_json::from_value::<ParsedRiskConfig>(follower.risk_config.clone()) {
            Ok(cfg) => cfg,
            Err(e) => {
                warn!(follower_id = %follower.id, error = %e, "failed to parse risk config");
                return Ok(false);
            }
        };

    // Check if follower is active
    if follower.status != "active" {
        info!(follower_id = %follower.id, status = %follower.status, "follower not active");
        alert_repo
            .insert(
                "warning",
                "risk_rejection",
                &format!(
                    "Copy intent {} rejected: follower {} is not active (status: {})",
                    intent.id, follower.id, follower.status
                ),
                Some("copy_intent"),
                Some(intent.id),
            )
            .await
            .ok();
        return Ok(false);
    }

    // Check minimum notional
    if let Some(min) = risk.min_notional {
        let notional = intent.target_contracts * intent.price_limit;
        if notional < min {
            info!(intent_id = %intent.id, notional = %notional, min = %min, "below minimum notional");
            alert_repo
                .insert(
                    "info",
                    "risk_rejection",
                    &format!(
                        "Copy intent {} rejected: notional {} below minimum {}",
                        intent.id, notional, min
                    ),
                    Some("copy_intent"),
                    Some(intent.id),
                )
                .await
                .ok();
            return Ok(false);
        }
    }

    // Check maximum notional
    if let Some(max) = risk.max_notional {
        let notional = intent.target_contracts * intent.price_limit;
        if notional > max {
            info!(intent_id = %intent.id, notional = %notional, max = %max, "above maximum notional");
            alert_repo
                .insert(
                    "warning",
                    "risk_rejection",
                    &format!(
                        "Copy intent {} rejected: notional {} above maximum {}",
                        intent.id, notional, max
                    ),
                    Some("copy_intent"),
                    Some(intent.id),
                )
                .await
                .ok();
            return Ok(false);
        }
    }

    // Check max trade bankroll percentage
    if let Some(pct) = risk.max_trade_bankroll_pct {
        if let Some(bankroll) = risk.bankroll {
            let max_trade = bankroll * (pct / Decimal::new(100, 0));
            let notional = intent.target_contracts * intent.price_limit;
            if notional > max_trade {
                info!(intent_id = %intent.id, notional = %notional, max_trade = %max_trade, "exceeds max trade size");
                alert_repo.insert(
                    "warning", "risk_rejection",
                    &format!("Copy intent {} rejected: notional {} exceeds max trade size {} ({}% of bankroll {})", intent.id, notional, max_trade, pct, bankroll),
                    Some("copy_intent"), Some(intent.id),
                ).await.ok();
                return Ok(false);
            }
        }
    }

    // Check copy scale (if copy_scale is very low, might indicate a pause)
    if let Some(scale) = risk.copy_scale {
        if scale <= Decimal::ZERO {
            info!(intent_id = %intent.id, scale = %scale, "copy scale is zero or negative");
            alert_repo
                .insert(
                    "info",
                    "risk_rejection",
                    &format!(
                        "Copy intent {} rejected: copy scale is zero or negative ({})",
                        intent.id, scale
                    ),
                    Some("copy_intent"),
                    Some(intent.id),
                )
                .await
                .ok();
            return Ok(false);
        }
    }

    Ok(true)
}
