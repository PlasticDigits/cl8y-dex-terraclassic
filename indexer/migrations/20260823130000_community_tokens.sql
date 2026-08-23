-- Community tax token catalog (GitLab #594). Side table — do not bloat `assets`.
CREATE TABLE IF NOT EXISTS community_tokens (
    contract_address VARCHAR(64) PRIMARY KEY,
    code_id BIGINT,
    wasm_admin VARCHAR(64),
    manager VARCHAR(64),
    launcher_address VARCHAR(64),
    buy_bps SMALLINT,
    sell_bps SMALLINT,
    transfer_bps SMALLINT,
    features JSONB NOT NULL DEFAULT '{}'::jsonb,
    treasury VARCHAR(64),
    name VARCHAR(128),
    symbol VARCHAR(32),
    decimals SMALLINT,
    attested_cmm BOOLEAN NOT NULL DEFAULT FALSE,
    launcher_tx VARCHAR(64),
    instantiate_tx VARCHAR(64),
    created_at_block BIGINT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_community_tokens_manager ON community_tokens (lower(manager));
CREATE INDEX IF NOT EXISTS idx_community_tokens_attested ON community_tokens (attested_cmm);
CREATE INDEX IF NOT EXISTS idx_community_tokens_launcher_tx ON community_tokens (launcher_tx);

COMMENT ON TABLE community_tokens IS 'Launcher-attested community tax CW20 catalog (#594). LCD code_id/admin are source of truth; columns are a cache.';

CREATE TABLE IF NOT EXISTS community_token_events (
    id BIGSERIAL PRIMARY KEY,
    contract_address VARCHAR(64) NOT NULL,
    txhash VARCHAR(64) NOT NULL,
    block_height BIGINT NOT NULL,
    action VARCHAR(64) NOT NULL,
    kind VARCHAR(32) NOT NULL,
    sku VARCHAR(64),
    invoice VARCHAR(32),
    attrs JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (txhash, action, contract_address, kind)
);

CREATE INDEX IF NOT EXISTS idx_community_token_events_addr ON community_token_events (contract_address, id DESC);

COMMENT ON TABLE community_token_events IS 'Community tax lifecycle: create, sku_unlock, settings_fee, mint. settings_fee is indexed separately from SKU unlocks (#594).';
