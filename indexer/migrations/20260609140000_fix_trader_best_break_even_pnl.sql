-- GitLab #344 follow-up: 20260609120000 backfill set best_trade_pnl = worst_trade_pnl when
-- total_realized_pnl = worst_trade_pnl for multi-trade rows. That pattern means every
-- non-worst close was break-even (0); best must remain 0, not the worst loss.

UPDATE traders SET best_trade_pnl = 0
WHERE total_trades > 1
  AND best_trade_pnl = worst_trade_pnl
  AND worst_trade_pnl < 0
  AND total_realized_pnl = worst_trade_pnl;
