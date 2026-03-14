CREATE TABLE IF NOT EXISTS hook_events (
    id BIGSERIAL PRIMARY KEY,
    tx_hash TEXT NOT NULL,
    hook_address TEXT NOT NULL,
    action TEXT NOT NULL,
    amount NUMERIC,
    token TEXT,
    skipped TEXT,
    warning TEXT,
    block_height BIGINT NOT NULL,
    block_time TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_hook_events_hook_address ON hook_events(hook_address);
CREATE INDEX idx_hook_events_block_time ON hook_events(block_time);
