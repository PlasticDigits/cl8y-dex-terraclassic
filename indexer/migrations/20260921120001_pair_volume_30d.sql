-- Trailing 30d pair USD volume stamp for GET /api/v1/protocol/top-pairs (Forgejo #1263).
-- Written by refresh_pair_volumes_30d (~5 min) — never a live SUM(swap_events) on GET.
-- NULL = activity in the window but unpriced / overflow. 0 = idle (no 30d swaps).
-- Do not COALESCE unpriced to 0 for ranking (NULLS LAST / excluded from top-5).

CREATE TABLE IF NOT EXISTS pair_volume_30d (
    pair_id INTEGER PRIMARY KEY REFERENCES pairs(id) ON DELETE CASCADE,
    volume_usd NUMERIC(38, 18),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_pair_volume_30d_volume_usd
    ON pair_volume_30d (volume_usd DESC NULLS LAST);

COMMENT ON TABLE pair_volume_30d IS
    'Trailing 30d SUM(swap_events.volume_usd) per pair for Protocol top-5. Refreshed ~5 min — not a GET-path scan.';

COMMENT ON COLUMN pair_volume_30d.volume_usd IS
    'Human USD SUM of priced swap_events.volume_usd over trailing 30d. NULL = unpriced/overflow. 0 = idle. Never invent $0 from unpriced quote volume.';
