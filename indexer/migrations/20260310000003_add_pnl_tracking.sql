-- Add P&L position tracking for traders

ALTER TABLE traders ADD COLUMN IF NOT EXISTS total_realized_pnl NUMERIC(38, 18) NOT NULL DEFAULT 0;
ALTER TABLE traders ADD COLUMN IF NOT EXISTS best_trade_pnl NUMERIC(38, 18) NOT NULL DEFAULT 0;
ALTER TABLE traders ADD COLUMN IF NOT EXISTS worst_trade_pnl NUMERIC(38, 18) NOT NULL DEFAULT 0;
ALTER TABLE traders ADD COLUMN IF NOT EXISTS total_fees_paid NUMERIC(38, 18) NOT NULL DEFAULT 0;

CREATE TABLE IF NOT EXISTS trader_positions (
    id SERIAL PRIMARY KEY,
    trader_address VARCHAR(64) NOT NULL,
    pair_id INTEGER NOT NULL REFERENCES pairs(id),
    net_position_quote NUMERIC(38, 18) NOT NULL DEFAULT 0,
    avg_entry_price NUMERIC(38, 18) NOT NULL DEFAULT 0,
    total_cost_base NUMERIC(38, 18) NOT NULL DEFAULT 0,
    realized_pnl NUMERIC(38, 18) NOT NULL DEFAULT 0,
    trade_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(trader_address, pair_id)
);
CREATE INDEX IF NOT EXISTS idx_positions_trader ON trader_positions(trader_address);
CREATE INDEX IF NOT EXISTS idx_positions_pair ON trader_positions(pair_id);
CREATE INDEX IF NOT EXISTS idx_traders_realized_pnl ON traders(total_realized_pnl DESC);
CREATE INDEX IF NOT EXISTS idx_traders_best_trade ON traders(best_trade_pnl DESC);
CREATE INDEX IF NOT EXISTS idx_traders_worst_trade ON traders(worst_trade_pnl ASC);
CREATE INDEX IF NOT EXISTS idx_traders_fees ON traders(total_fees_paid DESC);
