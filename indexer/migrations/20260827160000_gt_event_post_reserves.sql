-- GitLab #684: persist post-event AMM RESERVES on the swap / liquidity tape so
-- GET /gt/events can serve historical pool state. pair_reserves stays the
-- current LCD solver snapshot (one row per pair) and is not a history table.
--
-- NULL = ingested before this change (or catch-up without a seed). Backfill
-- reverse-applies from the current snapshot; GET emits "0" for NULL (never the
-- live pair_reserves row).

ALTER TABLE swap_events
    ADD COLUMN IF NOT EXISTS reserve_0 NUMERIC(38, 0),
    ADD COLUMN IF NOT EXISTS reserve_1 NUMERIC(38, 0);

ALTER TABLE liquidity_events
    ADD COLUMN IF NOT EXISTS reserve_0 NUMERIC(38, 0),
    ADD COLUMN IF NOT EXISTS reserve_1 NUMERIC(38, 0);

COMMENT ON COLUMN swap_events.reserve_0 IS
    'AMM RESERVES.asset_0 after this swap (raw). NULL = unknown / pre-#684.';
COMMENT ON COLUMN swap_events.reserve_1 IS
    'AMM RESERVES.asset_1 after this swap (raw). NULL = unknown / pre-#684.';
COMMENT ON COLUMN liquidity_events.reserve_0 IS
    'AMM RESERVES.asset_0 after this join/exit (raw). NULL = unknown / pre-#684.';
COMMENT ON COLUMN liquidity_events.reserve_1 IS
    'AMM RESERVES.asset_1 after this join/exit (raw). NULL = unknown / pre-#684.';
