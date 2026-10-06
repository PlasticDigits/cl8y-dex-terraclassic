# cLUNC/USDT seed and oracle integrity

[`scripts/mint-clunc-usdt-lp.sh`](../../scripts/mint-clunc-usdt-lp.sh) seeds a new,
empty exact-contract cLUNC/USDT pair. LUNC/USD values the 1:1 cLUNC wrap; USDT is
18 decimals, and raw amount comparisons run in Python. The script never mints
USDT. Existing nonempty pools are refused.

Follow [oracle health policy](indexer-external-oracle.md) and
[agent guidance](../../skills/AGENTS_MINT_CLUNC_USDT_LP.md). Both planning and
transaction entry points require fresh independent CEX quorum, exact LUNC quote
identity, named source policy and an observation no older than 300 seconds.
Source/quality failure stops preparation. More than 1% movement from the plan
also stops preparation; recompute instead of signing the old plan.

Use `DRY_RUN=1 bash scripts/mint-clunc-usdt-lp.sh` for a plan. A manual
`CLUNC_USDT_LUNC_USD` value is allowed only for dry runs and cannot create live
source-health evidence. Live signing follows the normal script keyring and
confirmation flow.

Run `bash scripts/qa/test-clunc-usdt-lp.sh` and
`python3 scripts/qa/test-oracle-lp-consumers.py`. The latter uses synthetic HTTP
prices and mocked preparation to prove stale, degraded, spoofed, invalid and
expired quotes never reach a transaction helper, while healthy controls do.
Unavailable signing/deployment checks may be recorded as SKIP with reason under
[the #1391 review policy](../qa/issue-1391/README.md); executed failures remain FAIL.
