-- GitLab #1277: traders rolling + lifetime raw volume are SUM(swap_events.offer_amount),
-- not USD. NUMERIC(38, 18) only stores |x| < 10^20 (~100 human 18-dec CW20). Sibling
-- #548 widened global_stats_24h.total_volume / pair_volume_24h.volume_quote to
-- NUMERIC(38, 0); rolling trader columns were renamed from volume_*_usd in
-- 20260310000002 and never widened. Keep total_volume_usd as NUMERIC(38, 18)
-- (P522-Q / #553; cap 10^20 - 10^-18).
--
-- Expand-only. Do not clamp rolling raw volume to the USD 10^20 cap.

ALTER TABLE traders
    ALTER COLUMN volume_24h TYPE NUMERIC(38, 0),
    ALTER COLUMN volume_7d TYPE NUMERIC(38, 0),
    ALTER COLUMN volume_30d TYPE NUMERIC(38, 0),
    ALTER COLUMN total_volume TYPE NUMERIC(38, 0);

COMMENT ON COLUMN traders.volume_24h IS
    'Raw SUM(swap_events.offer_amount) over trailing 24h (GitLab #1277). NUMERIC(38, 0) integer, not USD.';
COMMENT ON COLUMN traders.volume_7d IS
    'Raw SUM(swap_events.offer_amount) over trailing 7d (GitLab #1277). NUMERIC(38, 0) integer, not USD.';
COMMENT ON COLUMN traders.volume_30d IS
    'Raw SUM(swap_events.offer_amount) over trailing 30d (GitLab #1277). NUMERIC(38, 0) integer, not USD.';
COMMENT ON COLUMN traders.total_volume IS
    'Lifetime raw SUM(swap_events.offer_amount) (GitLab #1277). NUMERIC(38, 0) integer, not USD. total_volume_usd stays P522-Q.';
