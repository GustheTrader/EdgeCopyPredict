/// Paper execution worker — simulates Kalshi trades against the virtual Supabase wallet.
///
/// Follows the same pipeline as the live execution worker:
///   approved copy_intent (paper follower) → simulate fill → insert order + fill + position
///   → deduct/credit paper wallet balance → mark copy_intent "executed"
///
/// No exchange API calls are made. All state lives in Postgres.
use anyhow::Result;
use arbitek_config::AppConfig;
use arbitek_db::{
    connect_execution_worker as connect, AlertRepository, CopyIntentRepository, DbPool,
    FillRepository, OrderRepository, PositionRepository, SystemSettingsRepository,
};
use rust_decimal::Decimal;
use serde_json::json;
use time::OffsetDateTime;
use tracing::{info, warn};
use uuid::Uuid;

const PAPER_FOLLOWER_ID: &str = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";

/// Kalshi taker fee estimate: 7% of the smaller side's value per contract.
fn estimate_fee(contracts: Decimal, price: Decimal) -> Decimal {
    let price_cents = price * Decimal::new(100, 0);
    let hundred = Decimal::new(100, 0);
    let other = hundred - price_cents;
    let smaller = if price_cents < other {
        price_cents
    } else {
        other
    };
    contracts * smaller * Decimal::new(7, 2) / hundred
}

pub async fn run_paper_execution_once(config: &AppConfig) -> Result<()> {
    let pool = connect(&config.database_url).await?;
    run_paper_execution_pass(config, &pool).await
}

async fn run_paper_execution_pass(config: &AppConfig, pool: &DbPool) -> Result<()> {
    let copy_repo = CopyIntentRepository::new(pool.clone());
    let paper_follower_id = Uuid::parse_str(PAPER_FOLLOWER_ID).unwrap();

    let intents = copy_repo
        .list_approved_for_follower(paper_follower_id, config.worker.execution_batch_size)
        .await?;

    if intents.is_empty() {
        info!("no approved paper copy intents pending execution");
        return Ok(());
    }

    let settings = SystemSettingsRepository::new(pool.clone());
    let order_repo = OrderRepository::new(pool.clone());
    let fill_repo = FillRepository::new(pool.clone());
    let position_repo = PositionRepository::new(pool.clone());
    let alert_repo = AlertRepository::new(pool.clone());

    let mut executed = 0usize;
    let mut rejected = 0usize;

    for intent in &intents {
        if order_repo.order_exists_for_intent(intent.id).await? {
            warn!(
                intent_id = %intent.id,
                "[PAPER] copy intent already has an order; marking executed without another wallet debit"
            );
            copy_repo.update_status(intent.id, "executed").await?;
            continue;
        }

        let contracts = intent.target_contracts;
        let price = intent.price_limit;
        let notional = contracts * price;
        let fee = estimate_fee(contracts, price);
        let is_buy = intent.action == "buy";

        let balance = settings
            .get_decimal("paper_wallet_balance", Decimal::new(1000, 0))
            .await?;

        // Reject if insufficient paper balance for a buy
        if is_buy && notional + fee > balance {
            warn!(
                intent_id = %intent.id,
                cost = %(notional + fee),
                balance = %balance,
                "[PAPER] insufficient wallet balance — rejecting copy intent"
            );
            copy_repo.update_status(intent.id, "rejected").await?;
            let _ = alert_repo
                .insert(
                    "warning",
                    "paper_insufficient_balance",
                    &format!(
                        "Paper wallet insufficient for intent {}: need ${:.2}, have ${:.2}",
                        intent.id,
                        notional + fee,
                        balance
                    ),
                    Some("copy_intent"),
                    Some(intent.id),
                )
                .await;
            rejected += 1;
            continue;
        }

        let cash_delta = if is_buy {
            -(notional + fee)
        } else {
            notional - fee
        };
        let movement_payload = json!({
            "copy_intent_id": intent.id,
            "platform": intent.platform.as_str(),
            "market_id": intent.market_id,
            "signal_platform_market_id": intent.signal_platform_market_id.as_str(),
            "market_title": intent.market_title.as_deref(),
            "outcome": intent.outcome.as_str(),
            "side": intent.action.as_str(),
            "price": price,
            "contracts": contracts,
            "notional": notional,
            "fee": fee
        });
        let movement_description = format!(
            "{} {} {} @ {}",
            intent.action.to_uppercase(),
            contracts,
            intent.outcome,
            price
        );
        let wallet_movement = settings
            .record_paper_wallet_movement(
                intent.follower_account_id,
                if is_buy { "paper_buy" } else { "paper_sell" },
                cash_delta,
                Some("copy_intent"),
                Some(intent.id),
                Some(&movement_description),
                &movement_payload,
                Decimal::new(1000, 0),
            )
            .await?;
        let new_balance = wallet_movement.balance_after;

        // Insert paper order (immediately "filled")
        let order_id = order_repo
            .insert_paper_order(intent, price, contracts)
            .await?;

        // Insert fill record
        let _ = fill_repo
            .insert(
                order_id,
                &intent.platform,
                &format!("paper:fill:{}", Uuid::new_v4()),
                intent.follower_account_id,
                intent.market_id,
                None,
                &intent.outcome,
                &intent.action,
                price,
                contracts,
                notional,
                fee,
                Some(true),
                OffsetDateTime::now_utc(),
                &json!({}),
            )
            .await;

        // Stable market UUID for position tracking.
        // For Polymarket condition_ids (0x + 64 hex chars), the first 16 bytes of the
        // condition_id are used directly as the UUID — this makes pnl-refresh's
        // LIKE-pattern lookup (market_uuid_hex% matches condition_id) work correctly.
        let market_id = intent.market_id.unwrap_or_else(|| {
            let mid = &intent.signal_platform_market_id;
            if mid.starts_with("0x") && mid.len() >= 34 {
                if let Ok(bytes) = hex::decode(&mid[2..34]) {
                    if let Ok(arr) = <[u8; 16]>::try_from(bytes.as_slice()) {
                        return Uuid::from_bytes(arr);
                    }
                }
            }
            Uuid::new_v5(&Uuid::NAMESPACE_URL, mid.as_bytes())
        });
        let contracts_delta = if is_buy { contracts } else { -contracts };

        if let Err(e) = position_repo
            .upsert(
                intent.follower_account_id,
                &intent.platform,
                market_id,
                intent.market_title.as_deref(),
                None,
                &intent.outcome,
                contracts_delta,
                price,
                fee,
            )
            .await
        {
            warn!(intent_id = %intent.id, "position upsert failed: {}", e);
        }

        copy_repo.update_status(intent.id, "executed").await?;

        info!(
            intent_id = %intent.id,
            platform = %intent.platform,
            action = %intent.action,
            contracts = %contracts,
            price = %price,
            fee = %fee,
            new_balance = %new_balance,
            "[PAPER] executed copy intent"
        );

        executed += 1;

        if let Err(e) = crate::queue_publish::publish_execution(config, order_id, "executed").await
        {
            warn!("failed to publish paper execution event: {}", e);
        }
    }

    info!(executed, rejected, "paper execution pass completed");
    Ok(())
}

pub async fn run_paper_execution(config: &AppConfig) -> Result<()> {
    let pool = connect(&config.database_url).await?;
    let interval =
        std::time::Duration::from_secs(config.worker.polymarket_poll_interval_seconds.max(5));
    info!(
        "paper-execution worker starting — polling every {}s",
        interval.as_secs()
    );
    loop {
        if let Err(e) = run_paper_execution_pass(config, &pool).await {
            warn!("paper execution pass failed: {}", e);
        }
        tokio::time::sleep(interval).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arbitek_db::CopyIntentForExecution;

    fn make_intent(action: &str, price_f: i64, price_scale: u32) -> CopyIntentForExecution {
        CopyIntentForExecution {
            id: Uuid::new_v4(),
            follower_account_id: Uuid::parse_str(PAPER_FOLLOWER_ID).unwrap(),
            platform: "kalshi".to_owned(),
            signal_platform_market_id: "BTC-5MIN".to_owned(),
            market_id: None,
            outcome: "yes".to_owned(),
            action: action.to_owned(),
            target_contracts: Decimal::new(5, 0),
            price_limit: Decimal::new(price_f, price_scale),
            market_title: None,
        }
    }

    #[test]
    fn paper_buy_deducts_notional_and_fee() {
        let balance = Decimal::new(1000, 0);
        let intent = make_intent("buy", 50, 2); // price=0.50
        let contracts = intent.target_contracts;
        let price = intent.price_limit;
        let notional = contracts * price;
        let fee = estimate_fee(contracts, price);

        assert!(notional + fee <= balance);
        let new_balance = (balance - notional - fee).max(Decimal::ZERO);
        assert!(new_balance < balance, "buy must reduce wallet");
    }

    #[test]
    fn paper_sell_credits_proceeds_minus_fee() {
        let balance = Decimal::new(997, 0); // wallet after a buy
        let intent = make_intent("sell", 80, 2); // selling at 0.80
        let contracts = intent.target_contracts;
        let price = intent.price_limit;
        let notional = contracts * price;
        let fee = estimate_fee(contracts, price);

        let new_balance = balance + notional - fee;
        assert!(new_balance > balance, "sell must increase wallet");
    }

    #[test]
    fn paper_buy_rejected_when_balance_insufficient() {
        let balance = Decimal::new(1, 0); // $1 — not enough
        let intent = make_intent("buy", 50, 2); // 5 @ $0.50 = $2.50 notional
        let contracts = intent.target_contracts;
        let price = intent.price_limit;
        let notional = contracts * price;
        let fee = estimate_fee(contracts, price);

        assert!(notional + fee > balance, "should be rejected");
    }

    #[test]
    fn market_id_stable_from_ticker() {
        let ticker = "BTC-5MIN";
        let id1 = Uuid::new_v5(&Uuid::NAMESPACE_URL, ticker.as_bytes());
        let id2 = Uuid::new_v5(&Uuid::NAMESPACE_URL, ticker.as_bytes());
        assert_eq!(id1, id2, "market UUID must be deterministic per ticker");
        assert_ne!(id1, Uuid::nil());
    }

    #[test]
    fn follower_id_is_paper_sentinel() {
        let intent = make_intent("buy", 50, 2);
        assert_eq!(
            intent.follower_account_id,
            Uuid::parse_str(PAPER_FOLLOWER_ID).unwrap()
        );
    }

    #[test]
    fn estimate_fee_at_50_cents() {
        // price=0.50 → 5*50*0.07/100 = 0.175
        let fee = estimate_fee(Decimal::new(5, 0), Decimal::new(50, 2));
        assert_eq!(fee, Decimal::new(175, 3));
    }
}
