-- Remove the fixed-reference divergence containment (#1391). Hub marks now publish the
-- latest fresh reserve spot; genuine price movement never suppresses a mark. The
-- reference-ratchet table has no remaining reader or writer.
DROP TABLE IF EXISTS hub_price_references;
