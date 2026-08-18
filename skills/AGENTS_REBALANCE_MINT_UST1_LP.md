# Agent playbook: rebalance UST1/cUSTC and mint $1k LP to CMM

Use when deepening **UST1/cUSTC** and **UST1/USTR** with oracle-sized LP sent to the **ustr-cmm CMM treasury**, or when changing that mint/swap/provide flow.

## Canonical references

| Doc / script | Purpose |
|--------------|---------|
| [`docs/runbooks/rebalance-mint-ust1-lp.md`](../docs/runbooks/rebalance-mint-ust1-lp.md) | Operator runbook |
| [`scripts/rebalance-mint-ust1-lp.sh`](../scripts/rebalance-mint-ust1-lp.sh) | Host terrad: 2-of-3 mint → admin swap → LP to CMM |
| [`scripts/lib/ust1-lp-rebalance-defaults.sh`](../scripts/lib/ust1-lp-rebalance-defaults.sh) | Columbus-5 pair / token / treasury anchors |
| [`scripts/lib/ust1-lp-rebalance-math.py`](../scripts/lib/ust1-lp-rebalance-math.py) | Peg + LP sizing (`--self-test`) |

## Rules of thumb

1. **Unlock once** — prompt or `TERRAD_HOST_KEYRING_PASS`. Never commit it. File-keyring keys share one passphrase.
2. **Mint via DEX 2-of-3 extra minter** (`terra1zlmv2…`) to the admin hot wallet (`cl8ydeploy` by default). Do not use wrap-stack governance for this path.
3. **Swap UST1/cUSTC only** (pool-only, no book). Do **not** swap UST1/USTR. Re-query reserves after mint before each swap — stale offers overshoot.
4. **Gate provide** on UST1/cUSTC within **0.1%** of `1 / USTC_USD` (indexer `/api/v1/oracle/price/ustc`). Minted inventory stays on admin if the gate fails; re-run.
5. **LP receiver is CMM** `terra16j5u6…`. Script must re-query treasury LP balances and fail if they did not increase.
6. **USTR = 2.5 × USTC** for sizing only (indexer `USTR_PER_USTC`).
7. Prefer `DRY_RUN=1` before a live broadcast. Live needs `UST1_LP_YES=1` when stdin is not a TTY.

## Quick commands

```bash
python3 scripts/lib/ust1-lp-rebalance-math.py --self-test
DRY_RUN=1 ./scripts/rebalance-mint-ust1-lp.sh
UST1_LP_YES=1 ./scripts/rebalance-mint-ust1-lp.sh
```

## Related

- [`AGENTS_UST1_SECONDARY_AMM.md`](./AGENTS_UST1_SECONDARY_AMM.md) — pair create/seed (#508)
- [`AGENTS_INDEXER_EXTERNAL_ORACLE.md`](./AGENTS_INDEXER_EXTERNAL_ORACLE.md) — USTC ticker
- [`AGENTS_INDEXER_PAIR_PRICE_USD.md`](./AGENTS_INDEXER_PAIR_PRICE_USD.md) — USTR = 2.5 × USTC
- [`AGENTS_ROTATE_FEE_TREASURY.md`](./AGENTS_ROTATE_FEE_TREASURY.md) — CMM fee sink (not LP)
- [`AGENTS_KEY_CUSTODY.md`](./AGENTS_KEY_CUSTODY.md) — 2-of-3 roster
