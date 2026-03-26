-- Pattern C swap breakdown + per-maker limit fill indexing (pair wasm `limit_order_fill` events)

ALTER TABLE swap_events ADD COLUMN IF NOT EXISTS pool_return_amount NUMERIC(38, 0);
ALTER TABLE swap_events ADD COLUMN IF NOT EXISTS book_return_amount NUMERIC(38, 0);
ALTER TABLE swap_events ADD COLUMN IF NOT EXISTS limit_book_offer_consumed NUMERIC(38, 0);

CREATE TABLE IF NOT EXISTS limit_order_fills (
    id BIGSERIAL PRIMARY KEY,
    pair_id INTEGER NOT NULL REFERENCES pairs(id),
    swap_event_id BIGINT REFERENCES swap_events(id) ON DELETE SET NULL,
    block_height BIGINT NOT NULL,
    block_timestamp TIMESTAMPTZ NOT NULL,
    tx_hash VARCHAR(64) NOT NULL,
    order_id BIGINT NOT NULL,
    side VARCHAR(3) NOT NULL CHECK (side IN ('bid', 'ask')),
    maker VARCHAR(64) NOT NULL,
    price NUMERIC(38, 18) NOT NULL,
    token0_amount NUMERIC(38, 0) NOT NULL,
    token1_amount NUMERIC(38, 0) NOT NULL,
    commission_amount NUMERIC(38, 0) NOT NULL,
    UNIQUE (tx_hash, pair_id, order_id)
);
CREATE INDEX IF NOT EXISTS idx_lo_fills_pair_time ON limit_order_fills(pair_id, block_timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_lo_fills_order ON limit_order_fills(pair_id, order_id);
CREATE INDEX IF NOT EXISTS idx_lo_fills_maker ON limit_order_fills(maker, block_timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_lo_fills_tx ON limit_order_fills(tx_hash);
