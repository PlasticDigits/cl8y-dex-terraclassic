-- GitLab #279 (Phase 1a): mirror on-chain v2 pair reserves into Postgres so the hybrid solver can
-- read pool state without an LCD call. One current-state row per pair, replaced on each snapshot.
CREATE TABLE IF NOT EXISTS pair_reserves (
    pair_id INTEGER PRIMARY KEY REFERENCES pairs(id) ON DELETE CASCADE,
    reserve_0 NUMERIC(38, 0) NOT NULL,
    reserve_1 NUMERIC(38, 0) NOT NULL,
    fee_bps SMALLINT NOT NULL,
    block_height BIGINT,
    snapshot_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
