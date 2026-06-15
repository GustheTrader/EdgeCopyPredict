-- Repair paper wallet cash after historical settlement credits were recorded as
-- zero and stale paper buys could arrive after a position had already closed.
--
-- The current cash source of truth is reconstructed from the paper position book:
--   last reset balance + realized P&L - open cost basis
--
-- This keeps the current cash balance aligned with /pnl while preserving old
-- movement rows as audit history. The repair movement is idempotent.

WITH constants AS (
    SELECT 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa'::uuid AS paper_follower_id
),
last_reset AS (
    SELECT
        c.paper_follower_id,
        COALESCE(
            (
                SELECT balance_after
                FROM paper_wallet_movements
                WHERE follower_account_id = c.paper_follower_id
                  AND movement_type = 'reset'
                ORDER BY created_at DESC, id DESC
                LIMIT 1
            ),
            1000::numeric
        ) AS reset_balance,
        (
            SELECT created_at
            FROM paper_wallet_movements
            WHERE follower_account_id = c.paper_follower_id
              AND movement_type = 'reset'
            ORDER BY created_at DESC, id DESC
            LIMIT 1
        ) AS reset_at
    FROM constants c
),
position_book AS (
    SELECT
        c.paper_follower_id,
        COALESCE(SUM(p.realized_pnl), 0::numeric) AS realized_pnl,
        COALESCE(
            SUM(
                CASE
                    WHEN p.contracts <> 0 THEN p.avg_price * p.contracts
                    ELSE 0::numeric
                END
            ),
            0::numeric
        ) AS open_cost_basis
    FROM constants c
    LEFT JOIN positions p
      ON p.follower_account_id = c.paper_follower_id
    GROUP BY c.paper_follower_id
),
current_balance AS (
    SELECT
        lr.paper_follower_id,
        lr.reset_balance,
        lr.reset_at,
        pb.realized_pnl,
        pb.open_cost_basis,
        COALESCE(
            (
                SELECT NULLIF(value, '')::numeric
                FROM system_settings
                WHERE key = 'paper_wallet_balance'
            ),
            lr.reset_balance
        ) AS current_balance,
        lr.reset_balance + pb.realized_pnl - pb.open_cost_basis AS target_balance
    FROM last_reset lr
    JOIN position_book pb
      ON pb.paper_follower_id = lr.paper_follower_id
),
repair AS (
    INSERT INTO paper_wallet_movements (
        id,
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
        uuid_generate_v4(),
        paper_follower_id,
        'paper_wallet_repair',
        target_balance - current_balance,
        target_balance,
        'system',
        paper_follower_id,
        'Repair paper wallet cash to match the current paper position book.',
        jsonb_build_object(
            'repair_key', 'paper_wallet_position_book_20260610',
            'previous_balance', current_balance,
            'target_balance', target_balance,
            'last_reset_balance', reset_balance,
            'last_reset_at', reset_at,
            'realized_pnl', realized_pnl,
            'open_cost_basis', open_cost_basis
        ),
        NOW()
    FROM current_balance
    WHERE ABS(target_balance - current_balance) > 0.000001
      AND NOT EXISTS (
          SELECT 1
          FROM paper_wallet_movements
          WHERE follower_account_id = current_balance.paper_follower_id
            AND movement_type = 'paper_wallet_repair'
            AND source_type = 'system'
            AND source_id = current_balance.paper_follower_id
            AND payload->>'repair_key' = 'paper_wallet_position_book_20260610'
      )
    ON CONFLICT (source_type, source_id, movement_type)
        WHERE source_type IS NOT NULL AND source_id IS NOT NULL
    DO NOTHING
    RETURNING balance_after
)
INSERT INTO system_settings (key, value, updated_at)
SELECT 'paper_wallet_balance', balance_after::text, NOW()
FROM repair
ON CONFLICT (key) DO UPDATE SET
    value = EXCLUDED.value,
    updated_at = NOW();
