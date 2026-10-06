-- #1395: parked remaining is a raw amount, like swap/fee NUMERIC(38,0).
-- Preserve the legacy fractional scale rather than rounding any existing evidence.
-- New ingestion permits only unsigned integer raw amounts, including all 38 digits.
ALTER TABLE limit_order_placements
    ALTER COLUMN remaining_escrow TYPE NUMERIC(56,18);

COMMENT ON COLUMN limit_order_placements.remaining_escrow IS
    'Raw parked escrow; new events require unsigned integer amounts up to 38 digits (#1395). Legacy fractional evidence is preserved by widening precision without changing scale.';

-- Position accounting stores raw base-unit fees and P&L, not human/USD values.
-- Preserve scale while covering raw-amount arithmetic and cumulative accounting.
-- Otherwise valid 18-decimal amounts or 38-digit commissions overflow after
-- the source swap/fee rows have already been inserted and poison block retries.
ALTER TABLE traders
    ALTER COLUMN total_realized_pnl TYPE NUMERIC(114,18),
    ALTER COLUMN best_trade_pnl TYPE NUMERIC(114,18),
    ALTER COLUMN worst_trade_pnl TYPE NUMERIC(114,18),
    ALTER COLUMN total_fees_paid TYPE NUMERIC(78,18);

-- A sell can multiply two 38-digit raw quantities. The additional precision
-- covers that product and cumulative int32 trade counts without clipping P&L.
ALTER TABLE trader_positions
    ALTER COLUMN realized_pnl TYPE NUMERIC(114,18);

-- Raw sums require more headroom than one event. 58 integer digits cover a
-- 38-digit raw amount multiplied by a bigint row count without saturation.
ALTER TABLE traders
    ALTER COLUMN total_volume TYPE NUMERIC(58,0),
    ALTER COLUMN volume_24h TYPE NUMERIC(58,0),
    ALTER COLUMN volume_7d TYPE NUMERIC(58,0),
    ALTER COLUMN volume_30d TYPE NUMERIC(58,0);
ALTER TABLE candles
    ALTER COLUMN volume_base TYPE NUMERIC(58,0),
    ALTER COLUMN volume_quote TYPE NUMERIC(58,0);
ALTER TABLE token_volume_stats ALTER COLUMN volume TYPE NUMERIC(58,0);
ALTER TABLE pair_volume_24h ALTER COLUMN volume_quote TYPE NUMERIC(58,0);
ALTER TABLE global_stats_24h ALTER COLUMN total_volume TYPE NUMERIC(58,0);

COMMENT ON COLUMN traders.total_volume IS
    'Exact lifetime raw SUM(swap_events.offer_amount), NUMERIC(58,0), without the historical 38-digit aggregate cap (#1395). Human/USD quantities retain separate storage policy.';
