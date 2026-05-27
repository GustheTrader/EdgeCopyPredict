-- Append-only paper wallet cash ledger.
--
-- system_settings.paper_wallet_balance is the current cash value. This table is
-- the audit trail that explains how that value changed over time.

INSERT INTO followers (
    id, display_name, status, base_currency, wallet_address, platform, risk_config
)
VALUES (
    'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
    'Kalshi Paper Slave',
    'active',
    'USD',
    '0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
    'kalshi',
    '{"bankroll": 1000, "copy_scale": 1.0, "max_notional": 500, "min_notional": 0, "max_trade_bankroll_pct": 50}'::jsonb
)
ON CONFLICT (id) DO NOTHING;

CREATE TABLE IF NOT EXISTS paper_wallet_movements (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    follower_account_id UUID NOT NULL REFERENCES followers(id),
    movement_type TEXT NOT NULL,
    amount NUMERIC(36, 18) NOT NULL,
    balance_after NUMERIC(36, 18) NOT NULL,
    source_type TEXT,
    source_id UUID,
    description TEXT,
    payload JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_paper_wallet_movements_follower_created
    ON paper_wallet_movements (follower_account_id, created_at DESC);

CREATE UNIQUE INDEX IF NOT EXISTS idx_paper_wallet_movements_source
    ON paper_wallet_movements (source_type, source_id, movement_type)
    WHERE source_type IS NOT NULL AND source_id IS NOT NULL;

WITH first_fill AS (
    SELECT MIN(filled_at) AS filled_at
    FROM fills
    WHERE follower_account_id = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa'
)
INSERT INTO paper_wallet_movements (
    follower_account_id,
    movement_type,
    amount,
    balance_after,
    source_type,
    source_id,
    description,
    payload,
    created_at
)
SELECT
    'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
    'initial_balance',
    1000,
    1000,
    'follower',
    'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
    'Initial paper wallet balance',
    '{}'::jsonb,
    COALESCE((SELECT filled_at FROM first_fill), NOW()) - INTERVAL '1 second'
ON CONFLICT (source_type, source_id, movement_type)
    WHERE source_type IS NOT NULL AND source_id IS NOT NULL
DO NOTHING;

WITH ordered_fills AS (
    SELECT
        f.id,
        f.order_id,
        f.platform,
        f.follower_account_id,
        f.market_id,
        f.market_title,
        f.outcome,
        f.side,
        f.price,
        f.contracts,
        f.notional,
        f.fee,
        f.filled_at,
        CASE
            WHEN lower(f.side) = 'buy' THEN -(f.notional + f.fee)
            ELSE f.notional - f.fee
        END AS amount
    FROM fills f
    WHERE f.follower_account_id = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa'
),
running_fills AS (
    SELECT
        *,
        1000 + SUM(amount) OVER (ORDER BY filled_at ASC, id ASC) AS balance_after
    FROM ordered_fills
)
INSERT INTO paper_wallet_movements (
    follower_account_id,
    movement_type,
    amount,
    balance_after,
    source_type,
    source_id,
    description,
    payload,
    created_at
)
SELECT
    follower_account_id,
    CASE WHEN lower(side) = 'buy' THEN 'paper_buy' ELSE 'paper_sell' END,
    amount,
    balance_after,
    'fill',
    id,
    concat(upper(side), ' ', contracts::text, ' ', outcome, ' @ ', price::text),
    jsonb_build_object(
        'order_id', order_id,
        'platform', platform,
        'market_id', market_id,
        'market_title', market_title,
        'outcome', outcome,
        'side', side,
        'price', price,
        'contracts', contracts,
        'notional', notional,
        'fee', fee
    ),
    filled_at
FROM running_fills
ON CONFLICT (source_type, source_id, movement_type)
    WHERE source_type IS NOT NULL AND source_id IS NOT NULL
DO NOTHING;

WITH current_balance AS (
    SELECT COALESCE(
        (
            SELECT NULLIF(value, '')::numeric
            FROM system_settings
            WHERE key = 'paper_wallet_balance'
        ),
        1000
    ) AS value
),
ledger_balance AS (
    SELECT COALESCE(
        (
            SELECT balance_after
            FROM paper_wallet_movements
            WHERE follower_account_id = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa'
            ORDER BY created_at DESC, id DESC
            LIMIT 1
        ),
        1000
    ) AS value
),
delta AS (
    SELECT current_balance.value AS current_value,
           ledger_balance.value AS ledger_value,
           current_balance.value - ledger_balance.value AS amount
    FROM current_balance, ledger_balance
)
INSERT INTO paper_wallet_movements (
    follower_account_id,
    movement_type,
    amount,
    balance_after,
    source_type,
    source_id,
    description,
    payload,
    created_at
)
SELECT
    'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
    'balance_reconciliation',
    amount,
    current_value,
    'system_settings',
    NULL,
    'Reconciles the existing paper_wallet_balance because historical wallet movements were not recorded before this ledger existed.',
    jsonb_build_object('ledger_balance_before_reconciliation', ledger_value),
    NOW()
FROM delta
WHERE amount <> 0
  AND NOT EXISTS (
      SELECT 1
      FROM paper_wallet_movements
      WHERE follower_account_id = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa'
        AND movement_type = 'balance_reconciliation'
  );
