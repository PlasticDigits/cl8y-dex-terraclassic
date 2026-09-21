-- GitLab #1277 follow-up: one-shot heal of traders lifetime totals from swap_events.
-- Widen to NUMERIC(38, 0) is in 20260921120000_traders_rolling_volume_numeric_38_0.sql.
-- Canonical SQL is duplicated in traders.rs (keep in sync; sqlx migrations cannot call Rust).

INSERT INTO traders (address, total_trades, total_volume, first_trade_at, last_trade_at)
SELECT
  se.sender,
  COUNT(*)::bigint,
  LEAST(SUM(se.offer_amount), POWER(10::numeric, 38) - 1),
  MIN(se.block_timestamp),
  MAX(se.block_timestamp)
FROM swap_events se
GROUP BY se.sender
ON CONFLICT (address) DO NOTHING;

UPDATE traders t
SET
  total_trades = sub.cnt,
  total_volume = sub.vol,
  first_trade_at = CASE
    WHEN t.total_trades IS DISTINCT FROM sub.cnt
      OR t.total_volume IS DISTINCT FROM sub.vol
      OR t.first_trade_at IS NULL
    THEN sub.first_ts
    ELSE t.first_trade_at
  END,
  last_trade_at = CASE
    WHEN t.total_trades IS DISTINCT FROM sub.cnt
      OR t.total_volume IS DISTINCT FROM sub.vol
      OR t.last_trade_at IS NULL
    THEN sub.last_ts
    ELSE t.last_trade_at
  END,
  updated_at = NOW()
FROM (
  SELECT
    sender,
    COUNT(*)::bigint AS cnt,
    LEAST(SUM(offer_amount), POWER(10::numeric, 38) - 1) AS vol,
    MIN(block_timestamp) AS first_ts,
    MAX(block_timestamp) AS last_ts
  FROM swap_events
  GROUP BY sender
) sub
WHERE t.address = sub.sender;

UPDATE traders t
SET
  total_trades = 0,
  total_volume = 0,
  first_trade_at = NULL,
  last_trade_at = NULL,
  updated_at = NOW()
WHERE NOT EXISTS (
  SELECT 1 FROM swap_events se WHERE se.sender = t.address
)
AND (
  COALESCE(t.total_trades, 0) IS DISTINCT FROM 0
  OR t.total_volume IS DISTINCT FROM 0
);

UPDATE traders t
SET total_volume_usd = sub.usd,
    updated_at = NOW()
FROM (
  SELECT
    sender,
    LEAST(SUM(volume_usd), POWER(10::numeric, 20) - POWER(10::numeric, -18)) AS usd
  FROM swap_events
  WHERE volume_usd IS NOT NULL AND volume_usd > 0
  GROUP BY sender
) sub
WHERE t.address = sub.sender;

UPDATE traders t
SET total_volume_usd = NULL,
    updated_at = NOW()
WHERE t.total_volume_usd IS NOT NULL
  AND NOT EXISTS (
    SELECT 1
    FROM swap_events se
    WHERE se.sender = t.address
      AND se.volume_usd IS NOT NULL
      AND se.volume_usd > 0
  );
