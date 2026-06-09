-- GitLab #344: GREATEST(0, negative) pinned best_trade_pnl at 0 for all-loss traders.
ALTER TABLE traders ALTER COLUMN best_trade_pnl DROP DEFAULT;
ALTER TABLE traders ALTER COLUMN worst_trade_pnl DROP DEFAULT;
ALTER TABLE traders ALTER COLUMN best_trade_pnl DROP NOT NULL;
ALTER TABLE traders ALTER COLUMN worst_trade_pnl DROP NOT NULL;

UPDATE traders SET best_trade_pnl = NULL WHERE total_trades = 0;
UPDATE traders SET worst_trade_pnl = NULL WHERE total_trades = 0;
UPDATE traders SET best_trade_pnl = NULL
WHERE total_trades > 0 AND best_trade_pnl = 0 AND total_realized_pnl <> 0;
UPDATE traders SET worst_trade_pnl = NULL
WHERE total_trades > 0 AND worst_trade_pnl = 0 AND total_realized_pnl <> 0;
