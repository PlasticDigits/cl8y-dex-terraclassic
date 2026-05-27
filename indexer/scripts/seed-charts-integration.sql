-- Minimal indexer data for frontend charts integration tests (CI / local).
-- Run after `sqlx migrate run`. Safe to re-run (idempotent upserts).
-- Pair address must match CHARTS_INTEGRATION_PAIR_ADDRESS in the frontend test constants.

INSERT INTO assets (denom, is_cw20, name, symbol, decimals)
VALUES ('uluna', false, 'Luna Classic', 'LUNC', 6)
ON CONFLICT (denom) WHERE denom IS NOT NULL DO NOTHING;

INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
VALUES ('terra1ustctoken', true, 'TerraClassicUSD', 'USTC', 6)
ON CONFLICT (contract_address) DO NOTHING;

INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps, hooks)
VALUES (
  'terra1paircontractabc',
  (SELECT id FROM assets WHERE denom = 'uluna' LIMIT 1),
  (SELECT id FROM assets WHERE contract_address = 'terra1ustctoken' LIMIT 1),
  'terra1lptoken',
  30,
  '{}'
)
ON CONFLICT (contract_address) DO NOTHING;

INSERT INTO traders (
  address,
  total_trades,
  total_volume,
  volume_24h,
  volume_7d,
  volume_30d,
  registered
)
VALUES ('terra1traderxyz', 5, 5000, 500, 2000, 4000, true)
ON CONFLICT (address) DO NOTHING;

INSERT INTO swap_events (
  pair_id,
  block_height,
  block_timestamp,
  tx_hash,
  sender,
  offer_asset_id,
  ask_asset_id,
  offer_amount,
  return_amount,
  price
)
SELECT
  p.id,
  1000,
  TIMESTAMPTZ '2024-06-01 12:00:00+00',
  'charts_int_tx_1',
  'terra1traderxyz',
  a0.id,
  a1.id,
  1000,
  950,
  0.95
FROM pairs p
JOIN assets a0 ON a0.denom = 'uluna'
JOIN assets a1 ON a1.contract_address = 'terra1ustctoken'
WHERE p.contract_address = 'terra1paircontractabc'
ON CONFLICT (tx_hash, pair_id) DO NOTHING;

INSERT INTO candles (
  pair_id,
  interval,
  open_time,
  open,
  high,
  low,
  close,
  volume_base,
  volume_quote,
  trade_count
)
SELECT
  id,
  '1h',
  TIMESTAMPTZ '2024-06-01 12:00:00+00',
  0.94,
  0.96,
  0.93,
  0.95,
  5000,
  4750,
  5
FROM pairs
WHERE contract_address = 'terra1paircontractabc'
ON CONFLICT (pair_id, interval, open_time) DO NOTHING;
