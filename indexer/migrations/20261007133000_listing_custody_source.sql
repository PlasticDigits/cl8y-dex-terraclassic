-- #1437: native wrap custody belongs to the treasury named by mapper Config.
-- Keep applied migration history immutable. Legacy rows have no proven source
-- and remain NULL: never infer an address from an operator setting or balance.
ALTER TABLE listing_mapper_custody ADD COLUMN custody_address TEXT;
ALTER TABLE listing_wrap_principal ADD COLUMN custody_address TEXT;
COMMENT ON COLUMN listing_mapper_custody.custody_address IS
    'Historical attested mapper Config.treasury. NULL means legacy or unavailable proof and must not publish a market.';
COMMENT ON COLUMN listing_wrap_principal.custody_address IS
    'Authoritative treasury verified at the event block and predecessor; NULL legacy principal must not become a projected trade.';
