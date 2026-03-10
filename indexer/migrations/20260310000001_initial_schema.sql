-- CL8Y DEX Indexer — Initial Schema

CREATE TABLE IF NOT EXISTS assets (
    id SERIAL PRIMARY KEY,
    contract_address VARCHAR(64) UNIQUE,
    denom VARCHAR(64),
    is_cw20 BOOLEAN NOT NULL DEFAULT TRUE,
    name VARCHAR(128),
    symbol VARCHAR(20) NOT NULL,
    decimals SMALLINT NOT NULL DEFAULT 6,
    logo_url TEXT,
    coingecko_id VARCHAR(64),
    cmc_id INTEGER,
    first_seen_block BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_assets_denom ON assets(denom) WHERE denom IS NOT NULL;

CREATE TABLE IF NOT EXISTS pairs (
    id SERIAL PRIMARY KEY,
    contract_address VARCHAR(64) UNIQUE NOT NULL,
    asset_0_id INTEGER NOT NULL REFERENCES assets(id),
    asset_1_id INTEGER NOT NULL REFERENCES assets(id),
    lp_token VARCHAR(64),
    fee_bps SMALLINT,
    hooks TEXT[] NOT NULL DEFAULT '{}',
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at_block BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_pairs_assets ON pairs(asset_0_id, asset_1_id);

CREATE TABLE IF NOT EXISTS swap_events (
    id BIGSERIAL PRIMARY KEY,
    pair_id INTEGER NOT NULL REFERENCES pairs(id),
    block_height BIGINT NOT NULL,
    block_timestamp TIMESTAMPTZ NOT NULL,
    tx_hash VARCHAR(64) NOT NULL,
    sender VARCHAR(64) NOT NULL,
    receiver VARCHAR(64),
    offer_asset_id INTEGER NOT NULL REFERENCES assets(id),
    ask_asset_id INTEGER NOT NULL REFERENCES assets(id),
    offer_amount NUMERIC(38, 0) NOT NULL,
    return_amount NUMERIC(38, 0) NOT NULL,
    spread_amount NUMERIC(38, 0),
    commission_amount NUMERIC(38, 0),
    effective_fee_bps SMALLINT,
    price NUMERIC(38, 18) NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_swaps_pair_time ON swap_events(pair_id, block_timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_swaps_sender ON swap_events(sender, block_timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_swaps_tx ON swap_events(tx_hash);
CREATE INDEX IF NOT EXISTS idx_swaps_offer_asset ON swap_events(offer_asset_id, block_timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_swaps_ask_asset ON swap_events(ask_asset_id, block_timestamp DESC);

CREATE TABLE IF NOT EXISTS ohlcv_candles (
    pair_id INTEGER NOT NULL REFERENCES pairs(id),
    interval VARCHAR(4) NOT NULL,
    open_time TIMESTAMPTZ NOT NULL,
    open_price NUMERIC(38, 18),
    high_price NUMERIC(38, 18),
    low_price NUMERIC(38, 18),
    close_price NUMERIC(38, 18),
    volume_base NUMERIC(38, 0),
    volume_quote NUMERIC(38, 0),
    trade_count INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY(pair_id, interval, open_time)
);

CREATE TABLE IF NOT EXISTS traders (
    id SERIAL PRIMARY KEY,
    address VARCHAR(64) UNIQUE NOT NULL,
    total_trades INTEGER NOT NULL DEFAULT 0,
    total_volume_usd NUMERIC(38, 18) NOT NULL DEFAULT 0,
    volume_24h_usd NUMERIC(38, 18) NOT NULL DEFAULT 0,
    volume_7d_usd NUMERIC(38, 18) NOT NULL DEFAULT 0,
    volume_30d_usd NUMERIC(38, 18) NOT NULL DEFAULT 0,
    tier_id SMALLINT NOT NULL DEFAULT 0,
    tier_name VARCHAR(20) NOT NULL DEFAULT 'Default',
    fee_discount_registered BOOLEAN NOT NULL DEFAULT FALSE,
    first_trade_at TIMESTAMPTZ,
    last_trade_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_traders_volume ON traders(total_volume_usd DESC);
CREATE INDEX IF NOT EXISTS idx_traders_tier ON traders(tier_id);

CREATE TABLE IF NOT EXISTS token_volume_stats (
    asset_id INTEGER NOT NULL REFERENCES assets(id),
    period VARCHAR(4) NOT NULL,
    volume NUMERIC(38, 0) NOT NULL DEFAULT 0,
    trade_count INTEGER NOT NULL DEFAULT 0,
    unique_traders INTEGER NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY(asset_id, period)
);

CREATE TABLE IF NOT EXISTS liquidity_events (
    id BIGSERIAL PRIMARY KEY,
    pair_id INTEGER NOT NULL REFERENCES pairs(id),
    block_height BIGINT NOT NULL,
    block_timestamp TIMESTAMPTZ NOT NULL,
    tx_hash VARCHAR(64) NOT NULL,
    provider VARCHAR(64) NOT NULL,
    event_type VARCHAR(10) NOT NULL CHECK (event_type IN ('add', 'remove')),
    asset_0_amount NUMERIC(38, 0),
    asset_1_amount NUMERIC(38, 0),
    lp_amount NUMERIC(38, 0)
);
CREATE INDEX IF NOT EXISTS idx_liq_pair_time ON liquidity_events(pair_id, block_timestamp DESC);

CREATE TABLE IF NOT EXISTS indexer_state (
    key VARCHAR(100) PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
INSERT INTO indexer_state (key, value) VALUES
    ('last_indexed_height', '0'),
    ('indexer_version', '1.0.0')
ON CONFLICT (key) DO NOTHING;
