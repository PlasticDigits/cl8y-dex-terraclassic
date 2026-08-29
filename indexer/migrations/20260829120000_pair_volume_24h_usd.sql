-- Trailing 24h pair USD volume stamp for GET /api/v1/pairs (GitLab #692).
-- Written by refresh_pair_volumes (~5 min) — never a live SUM(swap_events) on GET.
-- NULL = activity in the window but unpriced / overflow. 0 = idle (no 24h swaps).
-- Do not COALESCE unpriced to 0 for JSON or sort (NULLS LAST).

ALTER TABLE pair_volume_24h
    ADD COLUMN IF NOT EXISTS volume_usd NUMERIC(38, 18);

CREATE INDEX IF NOT EXISTS idx_pair_volume_24h_volume_usd
    ON pair_volume_24h (volume_usd DESC NULLS LAST);

COMMENT ON COLUMN pair_volume_24h.volume_usd IS
    'Human USD SUM(swap_events.volume_usd) over trailing 24h. NULL = unpriced/overflow. 0 = idle. Never invent $0 from unpriced quote volume.';
