-- GitLab #344: GREATEST(0, negative) pinned best_trade_pnl at 0 for all-loss traders.
ALTER TABLE traders ALTER COLUMN best_trade_pnl DROP DEFAULT;
ALTER TABLE traders ALTER COLUMN worst_trade_pnl DROP DEFAULT;
ALTER TABLE traders ALTER COLUMN best_trade_pnl DROP NOT NULL;
ALTER TABLE traders ALTER COLUMN worst_trade_pnl DROP NOT NULL;

UPDATE traders SET best_trade_pnl = NULL WHERE total_trades = 0;
UPDATE traders SET worst_trade_pnl = NULL WHERE total_trades = 0;

-- Single all-loss / all-win trades: worst/best already holds the true extrema.
UPDATE traders SET best_trade_pnl = worst_trade_pnl
WHERE total_trades = 1 AND best_trade_pnl = 0 AND worst_trade_pnl < 0;
UPDATE traders SET worst_trade_pnl = best_trade_pnl
WHERE total_trades = 1 AND worst_trade_pnl = 0 AND best_trade_pnl > 0;

-- Multi-trade rows stuck at 0: null only when aggregates show no break-even extrema.
-- (total = worst preserves e.g. break-even best with one loss; total > best preserves break-even worst with one win.)
UPDATE traders SET best_trade_pnl = NULL
WHERE total_trades > 1
  AND best_trade_pnl = 0
  AND total_realized_pnl < 0
  AND total_realized_pnl < worst_trade_pnl;
UPDATE traders SET worst_trade_pnl = NULL
WHERE total_trades > 1
  AND worst_trade_pnl = 0
  AND best_trade_pnl > 0
  AND total_realized_pnl > best_trade_pnl;
