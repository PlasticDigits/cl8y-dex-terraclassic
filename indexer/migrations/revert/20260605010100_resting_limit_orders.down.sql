-- Rollback for 20260605010100_resting_limit_orders.sql (manual: not run by sqlx::migrate!).
DROP INDEX IF EXISTS idx_resting_orders_book;
DROP TABLE IF EXISTS resting_limit_orders;
