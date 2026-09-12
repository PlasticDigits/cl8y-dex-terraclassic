-- GitLab #1227: persist LCD DLL walk order so Postgres hybrid sim matches
-- execute FIFO after UpdateLimitOrderPrice (keep order_id, join equal-price tail).
-- Snapshot replace still writes rows head→tail; queries ORDER BY walk_index, not order_id.
ALTER TABLE resting_limit_orders
    ADD COLUMN IF NOT EXISTS walk_index INTEGER NOT NULL DEFAULT 0;

DROP INDEX IF EXISTS idx_resting_orders_book;
CREATE INDEX IF NOT EXISTS idx_resting_orders_book
    ON resting_limit_orders (pair_id, side, walk_index);
