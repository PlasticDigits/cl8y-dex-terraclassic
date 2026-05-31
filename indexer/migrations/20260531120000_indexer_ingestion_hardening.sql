-- Indexer ingestion hardening (GitLab #236): failed-block visibility for operators.

CREATE TABLE IF NOT EXISTS indexer_failed_blocks (
    height BIGINT PRIMARY KEY,
    error_message TEXT NOT NULL,
    retry_count INTEGER NOT NULL DEFAULT 1,
    first_failed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_failed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
