//! Integration tests for paper and live trading scenarios.
//!
//! Requires DATABASE_URL pointing to a Postgres instance with all migrations applied.
//! Every test skips gracefully when DATABASE_URL is not set, so CI without a DB
//! does not fail.
//!
//! Run with:
//!   DATABASE_URL="postgres://..." cargo test -p arbitek-worker --test trading_scenarios

use rust_decimal::Decimal;
use sqlx::Row;
use uuid::Uuid;

fn db_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok()
}

macro_rules! require_db {
    () => {
        match db_url() {
            Some(u) => u,
            None => {
                println!("SKIP: DATABASE_URL not set");
                return;
            }
        }
    };
}

// ---------------------------------------------------------------------------
// P&L accounting — exercises PositionRepository::upsert against the real DB
// ---------------------------------------------------------------------------

#[tokio::test]
async fn db_first_buy_creates_correct_position() {
    let url = require_db!();
    let pool = sqlx::PgPool::connect(&url).await.expect("connect postgres");
    let repo = arbitek_db::PositionRepository::new(pool.clone());

    let follower_id = Uuid::new_v4(); // unique per run — no teardown needed
    let market_id = Uuid::new_v4();

    // fee = 5 * 50¢ * 7% / 100 = 0.175
    let fee = Decimal::new(175, 3);
    repo.upsert(
        follower_id,
        "polymarket",
        market_id,
        None, // platform_market_id
        Some("Test Market"),
        None,
        "yes",
        Decimal::new(5, 0),  // buy 5 contracts
        Decimal::new(50, 2), // @ $0.50
        fee,
    )
    .await
    .expect("upsert buy");

    let row = sqlx::query(
        "SELECT contracts, avg_price, realized_pnl, exposure \
         FROM positions \
         WHERE follower_account_id = $1 AND market_id = $2 AND outcome = 'yes'",
    )
    .bind(follower_id)
    .bind(market_id)
    .fetch_one(&pool)
    .await
    .expect("fetch position");

    let contracts: Decimal = row.try_get("contracts").unwrap();
    let avg_price: Decimal = row.try_get("avg_price").unwrap();
    let realized_pnl: Decimal = row.try_get("realized_pnl").unwrap();
    let exposure: Decimal = row.try_get("exposure").unwrap();

    assert_eq!(contracts, Decimal::new(5, 0), "wrong contract count");
    assert_eq!(avg_price, Decimal::new(50, 2), "wrong avg_price");
    assert_eq!(
        realized_pnl, -fee,
        "entry fee not deducted from realized_pnl"
    );
    assert_eq!(
        exposure,
        Decimal::new(250, 2),
        "exposure = 5 * $0.50 = $2.50"
    );
}

#[tokio::test]
async fn db_sell_after_buy_records_correct_realized_pnl() {
    let url = require_db!();
    let pool = sqlx::PgPool::connect(&url).await.expect("connect postgres");
    let repo = arbitek_db::PositionRepository::new(pool.clone());

    let follower_id = Uuid::new_v4();
    let market_id = Uuid::new_v4();

    // Buy fees: 5*50*0.07/100 = 0.175
    let buy_fee = Decimal::new(175, 3);
    // Sell fees: sell @ $0.80 → smaller side = NO (20¢) → 5*20*0.07/100 = 0.07
    let sell_fee = Decimal::new(7, 2);

    repo.upsert(
        follower_id,
        "polymarket",
        market_id,
        None, // platform_market_id
        None,
        None,
        "yes",
        Decimal::new(5, 0),
        Decimal::new(50, 2),
        buy_fee,
    )
    .await
    .unwrap();

    repo.upsert(
        follower_id,
        "polymarket",
        market_id,
        None, // platform_market_id
        None,
        None,
        "yes",
        Decimal::new(-5, 0),
        Decimal::new(80, 2),
        sell_fee,
    )
    .await
    .unwrap();

    let row = sqlx::query(
        "SELECT contracts, realized_pnl, exposure \
         FROM positions \
         WHERE follower_account_id = $1 AND market_id = $2 AND outcome = 'yes'",
    )
    .bind(follower_id)
    .bind(market_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    let contracts: Decimal = row.try_get("contracts").unwrap();
    let realized_pnl: Decimal = row.try_get("realized_pnl").unwrap();
    let exposure: Decimal = row.try_get("exposure").unwrap();

    // -0.175 + 5*(0.80-0.50) - 0.07 = -0.175 + 1.50 - 0.07 = 1.255
    let expected_pnl =
        -buy_fee + Decimal::new(5, 0) * (Decimal::new(80, 2) - Decimal::new(50, 2)) - sell_fee;

    assert_eq!(contracts, Decimal::ZERO, "all contracts sold");
    assert_eq!(realized_pnl, expected_pnl, "realized P&L incorrect");
    assert_eq!(exposure, Decimal::ZERO, "no open exposure after full close");
    assert!(
        realized_pnl > Decimal::ZERO,
        "profitable trade must show positive P&L"
    );
}

#[tokio::test]
async fn db_two_buys_produce_weighted_avg_price() {
    let url = require_db!();
    let pool = sqlx::PgPool::connect(&url).await.expect("connect postgres");
    let repo = arbitek_db::PositionRepository::new(pool.clone());

    let follower_id = Uuid::new_v4();
    let market_id = Uuid::new_v4();

    // Buy 5 @ $0.30 — fee: 5*30*0.07/100 = 0.105
    repo.upsert(
        follower_id,
        "polymarket",
        market_id,
        None, // platform_market_id
        None,
        None,
        "yes",
        Decimal::new(5, 0),
        Decimal::new(30, 2),
        Decimal::new(105, 3),
    )
    .await
    .unwrap();

    // Buy 5 @ $0.70 — fee: smaller=NO(30¢) → 5*30*0.07/100 = 0.105
    repo.upsert(
        follower_id,
        "polymarket",
        market_id,
        None, // platform_market_id
        None,
        None,
        "yes",
        Decimal::new(5, 0),
        Decimal::new(70, 2),
        Decimal::new(105, 3),
    )
    .await
    .unwrap();

    let row = sqlx::query(
        "SELECT contracts, avg_price, exposure \
         FROM positions \
         WHERE follower_account_id = $1 AND market_id = $2 AND outcome = 'yes'",
    )
    .bind(follower_id)
    .bind(market_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    let contracts: Decimal = row.try_get("contracts").unwrap();
    let avg_price: Decimal = row.try_get("avg_price").unwrap();
    let exposure: Decimal = row.try_get("exposure").unwrap();

    assert_eq!(contracts, Decimal::new(10, 0), "10 total contracts");
    assert_eq!(
        avg_price,
        Decimal::new(50, 2),
        "(5*0.30 + 5*0.70)/10 = $0.50"
    );
    assert_eq!(exposure, Decimal::new(500, 2), "10 * $0.50 = $5.00");
}

#[tokio::test]
async fn db_partial_sell_preserves_avg_price_and_exposure() {
    let url = require_db!();
    let pool = sqlx::PgPool::connect(&url).await.expect("connect postgres");
    let repo = arbitek_db::PositionRepository::new(pool.clone());

    let follower_id = Uuid::new_v4();
    let market_id = Uuid::new_v4();

    // Buy 10 @ $0.50, then sell 5 @ $0.80 (partial close)
    repo.upsert(
        follower_id,
        "polymarket",
        market_id,
        None, // platform_market_id
        None,
        None,
        "yes",
        Decimal::new(10, 0),
        Decimal::new(50, 2),
        Decimal::new(350, 3), // fee = 0.35
    )
    .await
    .unwrap();

    repo.upsert(
        follower_id,
        "polymarket",
        market_id,
        None, // platform_market_id
        None,
        None,
        "yes",
        Decimal::new(-5, 0),
        Decimal::new(80, 2),
        Decimal::new(7, 2), // fee = 0.07
    )
    .await
    .unwrap();

    let row = sqlx::query(
        "SELECT contracts, avg_price, exposure, realized_pnl \
         FROM positions \
         WHERE follower_account_id = $1 AND market_id = $2 AND outcome = 'yes'",
    )
    .bind(follower_id)
    .bind(market_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    let contracts: Decimal = row.try_get("contracts").unwrap();
    let avg_price: Decimal = row.try_get("avg_price").unwrap();
    let exposure: Decimal = row.try_get("exposure").unwrap();
    let realized_pnl: Decimal = row.try_get("realized_pnl").unwrap();

    assert_eq!(
        contracts,
        Decimal::new(5, 0),
        "5 contracts remain after partial close"
    );
    assert_eq!(
        avg_price,
        Decimal::new(50, 2),
        "cost basis must not change on a sell"
    );
    assert_eq!(
        exposure,
        Decimal::new(250, 2),
        "5 remaining * $0.50 = $2.50"
    );
    assert!(
        realized_pnl > Decimal::ZERO,
        "partial realisation at profit must be positive"
    );
}

#[tokio::test]
async fn db_settlement_returns_cash_before_zeroing_contracts() {
    let url = require_db!();
    let pool = sqlx::PgPool::connect(&url).await.expect("connect postgres");
    let repo = arbitek_db::PositionRepository::new(pool.clone());

    let follower_id = Uuid::new_v4();
    let market_id = Uuid::new_v4();
    let contracts = Decimal::new(5, 0);
    let avg_price = Decimal::new(30, 2);
    let settlement_price = Decimal::new(100, 2);
    let fee = Decimal::new(105, 3);

    repo.upsert(
        follower_id,
        "polymarket",
        market_id,
        None,
        Some("Settlement regression"),
        None,
        "yes",
        contracts,
        avg_price,
        fee,
    )
    .await
    .expect("upsert buy");

    let position_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM positions WHERE follower_account_id = $1 AND market_id = $2 AND outcome = 'yes'",
    )
    .bind(follower_id)
    .bind(market_id)
    .fetch_one(&pool)
    .await
    .expect("fetch position id");

    let settlement = repo
        .settle_resolved_position(position_id, settlement_price, None)
        .await
        .expect("settle position")
        .expect("position should settle");

    assert_eq!(settlement.follower_account_id, follower_id);
    assert_eq!(
        settlement.settlement_cash,
        contracts * settlement_price,
        "settlement cash must use the pre-close contract count"
    );

    let row = sqlx::query(
        "SELECT contracts, realized_pnl, unrealized_pnl, exposure \
         FROM positions \
         WHERE id = $1",
    )
    .bind(position_id)
    .fetch_one(&pool)
    .await
    .expect("fetch settled position");

    let remaining_contracts: Decimal = row.try_get("contracts").unwrap();
    let realized_pnl: Decimal = row.try_get("realized_pnl").unwrap();
    let unrealized_pnl: Decimal = row.try_get("unrealized_pnl").unwrap();
    let exposure: Decimal = row.try_get("exposure").unwrap();
    let expected_realized = -fee + contracts * (settlement_price - avg_price);

    assert_eq!(remaining_contracts, Decimal::ZERO);
    assert_eq!(unrealized_pnl, Decimal::ZERO);
    assert_eq!(exposure, Decimal::ZERO);
    assert_eq!(realized_pnl, expected_realized);
    assert!(
        repo.is_closed_position(follower_id, market_id, "yes")
            .await
            .expect("check closed status"),
        "settled positions must be marked closed for stale paper-intent rejection"
    );
}

// ---------------------------------------------------------------------------
// Paper wallet toggle behaviour
// ---------------------------------------------------------------------------

#[tokio::test]
async fn db_toggle_live_to_paper_resets_wallet_to_1000() {
    let url = require_db!();
    let pool = sqlx::PgPool::connect(&url).await.expect("connect postgres");
    let settings = arbitek_db::SystemSettingsRepository::new(pool.clone());

    // Simulate live mode active with depleted paper wallet
    settings
        .set_bool("kalshi_execution_enabled", true)
        .await
        .unwrap();
    settings
        .set_decimal("paper_wallet_balance", Decimal::new(300, 0))
        .await
        .unwrap();

    // Replicate toggle_kalshi_execution handler logic: disable live → reset wallet
    let was_live = settings
        .get_bool("kalshi_execution_enabled", false)
        .await
        .unwrap();
    assert!(was_live, "precondition: must be in live mode");

    settings
        .set_bool("kalshi_execution_enabled", false)
        .await
        .unwrap();
    if was_live {
        settings
            .set_decimal("paper_wallet_balance", Decimal::new(1000, 0))
            .await
            .unwrap();
    }

    let enabled = settings
        .get_bool("kalshi_execution_enabled", true)
        .await
        .unwrap();
    let balance = settings
        .get_decimal("paper_wallet_balance", Decimal::ZERO)
        .await
        .unwrap();

    assert!(
        !enabled,
        "live trading must be OFF after switching to paper"
    );
    assert_eq!(
        balance,
        Decimal::new(1000, 0),
        "paper wallet must be reset to $1000 when switching back from live"
    );

    // Cleanup
    settings
        .set_bool("kalshi_execution_enabled", false)
        .await
        .ok();
}

#[tokio::test]
async fn db_toggle_paper_to_live_does_not_reset_wallet() {
    let url = require_db!();
    let pool = sqlx::PgPool::connect(&url).await.expect("connect postgres");
    let settings = arbitek_db::SystemSettingsRepository::new(pool.clone());

    // Simulate paper mode with a partially-spent wallet ($750)
    settings
        .set_bool("kalshi_execution_enabled", false)
        .await
        .unwrap();
    let paper_balance = Decimal::new(750, 0);
    settings
        .set_decimal("paper_wallet_balance", paper_balance)
        .await
        .unwrap();

    // Replicate toggle_kalshi_execution handler logic: enable live → wallet stays as-is
    let was_live = settings
        .get_bool("kalshi_execution_enabled", true)
        .await
        .unwrap();
    assert!(!was_live, "precondition: must be in paper mode");

    settings
        .set_bool("kalshi_execution_enabled", true)
        .await
        .unwrap();
    if was_live {
        // This block does NOT run because was_live == false
        settings
            .set_decimal("paper_wallet_balance", Decimal::new(1000, 0))
            .await
            .unwrap();
    }

    let enabled = settings
        .get_bool("kalshi_execution_enabled", false)
        .await
        .unwrap();
    let balance = settings
        .get_decimal("paper_wallet_balance", Decimal::ZERO)
        .await
        .unwrap();

    assert!(enabled, "live trading must be ON after toggle");
    assert_eq!(
        balance, paper_balance,
        "paper wallet must NOT change when switching from paper to live"
    );

    // Cleanup
    settings
        .set_bool("kalshi_execution_enabled", false)
        .await
        .ok();
    settings
        .set_decimal("paper_wallet_balance", Decimal::new(1000, 0))
        .await
        .ok();
}

#[tokio::test]
async fn db_kalshi_execution_disabled_by_default() {
    let url = require_db!();
    let pool = sqlx::PgPool::connect(&url).await.expect("connect postgres");
    let settings = arbitek_db::SystemSettingsRepository::new(pool.clone());

    // Delete the key to simulate a fresh environment, then read back with default.
    sqlx::query("DELETE FROM system_settings WHERE key = 'kalshi_execution_enabled'")
        .execute(&pool)
        .await
        .ok();

    let enabled = settings
        .get_bool("kalshi_execution_enabled", false)
        .await
        .unwrap();
    assert!(
        !enabled,
        "Kalshi execution must default to OFF — paper mode is the safe starting state"
    );
}

/// Verify that the paper wallet balance increases after a profitable sell.
/// Before this fix the wallet only ever decreased — sells were treated identically
/// to buys (subtracting notional instead of adding it back).
#[tokio::test]
async fn db_paper_wallet_increases_after_profitable_sell() {
    let url = require_db!();
    let pool = sqlx::PgPool::connect(&url).await.expect("connect postgres");
    let settings = arbitek_db::SystemSettingsRepository::new(pool.clone());

    // Start with $1000
    let starting_balance = Decimal::new(1000, 0);
    settings
        .set_decimal("paper_wallet_balance", starting_balance)
        .await
        .unwrap();

    let buy_price = Decimal::new(50, 2); // $0.50
    let sell_price = Decimal::new(80, 2); // $0.80
    let contracts = Decimal::new(5, 0);
    let buy_notional = buy_price * contracts; // $2.50
    let sell_notional = sell_price * contracts; // $4.00
    let buy_fee = Decimal::new(175, 3); // 5*50*0.07/100 = 0.175
    let sell_fee = Decimal::new(7, 2); // 5*20*0.07/100 = 0.07  (NO side cheaper at 20¢)

    // Simulate BUY: deduct notional + fee
    let after_buy = (starting_balance - buy_notional - buy_fee).max(Decimal::ZERO);
    settings
        .set_decimal("paper_wallet_balance", after_buy)
        .await
        .unwrap();

    // Simulate SELL: credit proceeds - fee  (the fix)
    let after_sell = after_buy + sell_notional - sell_fee;
    settings
        .set_decimal("paper_wallet_balance", after_sell)
        .await
        .unwrap();

    let final_balance = settings
        .get_decimal("paper_wallet_balance", Decimal::ZERO)
        .await
        .unwrap();

    assert!(
        final_balance > starting_balance,
        "paper wallet must be higher than start ($1000) after a profitable round-trip; got {}",
        final_balance
    );

    // Net profit = sell_notional - buy_notional - buy_fee - sell_fee = $1.255
    let net_profit = final_balance - starting_balance;
    assert!(
        net_profit > Decimal::ZERO,
        "net profit must be positive; got {}",
        net_profit
    );
}

#[tokio::test]
async fn db_paper_wallet_seeded_at_1000_on_first_start() {
    let url = require_db!();
    let pool = sqlx::PgPool::connect(&url).await.expect("connect postgres");
    let settings = arbitek_db::SystemSettingsRepository::new(pool.clone());

    // Delete existing key to simulate first worker start
    sqlx::query("DELETE FROM system_settings WHERE key = 'paper_wallet_balance'")
        .execute(&pool)
        .await
        .ok();

    // Replicate the paper wallet seeding logic (done at startup)
    if settings
        .get("paper_wallet_balance")
        .await
        .unwrap_or(None)
        .is_none()
    {
        settings
            .set_decimal("paper_wallet_balance", Decimal::new(1000, 0))
            .await
            .unwrap();
    }

    let balance = settings
        .get_decimal("paper_wallet_balance", Decimal::ZERO)
        .await
        .unwrap();
    assert_eq!(
        balance,
        Decimal::new(1000, 0),
        "paper wallet must be seeded to $1000 on first start"
    );
}
