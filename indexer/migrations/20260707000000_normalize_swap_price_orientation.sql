-- GitLab #466: normalize swap_events.price to quote-per-base (asset_1 per asset_0).
-- Bidirectional trades previously stored P and 1/P in the same column, corrupting OHLC and CG/CMC feeds.

UPDATE swap_events se
SET price = CASE
  WHEN se.offer_asset_id = p.asset_0_id AND se.offer_amount > 0
    THEN se.return_amount / se.offer_amount
  WHEN se.offer_asset_id = p.asset_1_id AND se.return_amount > 0
    THEN se.offer_amount / se.return_amount
  ELSE se.price
END
FROM pairs p
WHERE se.pair_id = p.id;

-- Candles were built from direction-blind offer/return volumes; truncate for rebuild.
-- Operators: `cargo run -- seed-qa --clean` on QA, or per-pair rebuild via seed_qa / indexer tooling.
TRUNCATE candles;
