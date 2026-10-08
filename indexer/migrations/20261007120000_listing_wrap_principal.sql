-- Listing adapter projections only (#1437). These are not economic pair/swap rows.
CREATE TABLE listing_wrap_principal (
    id BIGSERIAL PRIMARY KEY,
    mapper_address TEXT NOT NULL,
    native_denom TEXT NOT NULL CHECK (native_denom IN ('uluna', 'uusd')),
    direction TEXT NOT NULL CHECK (direction IN ('wrap', 'unwrap')),
    principal_raw NUMERIC(38, 0) NOT NULL CHECK (principal_raw > 0),
    custody_raw NUMERIC(38, 0) NOT NULL CHECK (custody_raw >= 0),
    tx_hash TEXT NOT NULL,
    trader_address TEXT NOT NULL,
    source_ordinal BIGINT NOT NULL DEFAULT 0,
    txn_index BIGINT NOT NULL DEFAULT 0,
    block_height BIGINT NOT NULL,
    block_timestamp TIMESTAMPTZ NOT NULL,
    block_hash TEXT NOT NULL DEFAULT '',
    UNIQUE (mapper_address, tx_hash, source_ordinal)
);
CREATE INDEX listing_wrap_principal_window ON listing_wrap_principal
    (mapper_address, block_height, txn_index, source_ordinal);
CREATE INDEX listing_wrap_principal_recent ON listing_wrap_principal
    (mapper_address, native_denom, block_timestamp DESC, id DESC);

CREATE TABLE listing_mapper_custody (
    mapper_address TEXT NOT NULL,
    native_denom TEXT NOT NULL CHECK (native_denom IN ('uluna', 'uusd')),
    custody_raw NUMERIC(38, 0) NOT NULL CHECK (custody_raw >= 0),
    block_height BIGINT NOT NULL,
    block_timestamp TIMESTAMPTZ NOT NULL,
    block_hash TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (mapper_address, native_denom, block_height)
);
COMMENT ON TABLE listing_wrap_principal IS 'CG/GT-only authenticated mapper principal. Never read for trader volume, protocol fees, TVL, candles or DeFiLlama.';
COMMENT ON TABLE listing_mapper_custody IS 'CG/GT-only bank backing snapshots obtained by background indexing, never by API requests.';
