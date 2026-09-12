# Agent playbook: mint $5k+$5k cLUNC/cUSTC LP to CMM (no rebalance)

Use when deepening **cLUNC/cUSTC** with ~**$10k** oracle TVL sent to the **ustr-cmm CMM treasury** **without** swapping the pool onto the LUNC/USTC oracle. Goal: keep a **cLUNC premium** so LUNC inflows stay attractive.

## Canonical references

| Doc / script | Purpose |
|--------------|---------|
| [`docs/runbooks/mint-clunc-custc-lp.md`](../docs/runbooks/mint-clunc-custc-lp.md) | Operator runbook |
| [`scripts/mint-clunc-custc-lp.sh`](../scripts/mint-clunc-custc-lp.sh) | Wrapper: `CLUNC_LP_SKIP_SWAP=1` + `CLUNC_LP_ADD_USD=10000` |
| [`scripts/rebalance-mint-clunc-custc-lp.sh`](../scripts/rebalance-mint-clunc-custc-lp.sh) | Shared host terrad flow (mint / provide / CMM / burn) |
| [`scripts/lib/clunc-custc-lp-defaults.sh`](../scripts/lib/clunc-custc-lp-defaults.sh) | Columbus-5 pair / token / treasury anchors |
| [`scripts/lib/clunc-custc-lp-math.py`](../scripts/lib/clunc-custc-lp-math.py) | Live-ratio LP sizing (`--self-test`) |

## Rules of thumb

1. **Unlock once** — prompt or `TERRAD_HOST_KEYRING_PASS`. Never commit it. File-keyring keys share one passphrase.
2. **Do not swap.** Provide is pro-rata at live reserves. Equal-USD legs would move the price toward the oracle.
3. **"$5k+$5k"** means add **~$10k** oracle TVL. While cLUNC is premia, oracle USD per side is **uneven**.
4. **Refuse an empty pool.** Seed/on-peg first via the rebalance script if reserves are zero.
5. **LP receiver is CMM** `terra16j5u6…`. Fail if treasury LP did not increase. Holder-burn leftover wrap on admin; never burn LP.
6. Prefer `DRY_RUN=1` before a live broadcast. Live needs `CLUNC_LP_YES=1` when stdin is not a TTY.

## Quick commands

```bash
python3 scripts/lib/clunc-custc-lp-math.py --self-test
DRY_RUN=1 ./scripts/mint-clunc-custc-lp.sh
CLUNC_LP_YES=1 ./scripts/mint-clunc-custc-lp.sh
CLUNC_LP_ADD_USD=10000 CLUNC_LP_YES=1 ./scripts/mint-clunc-custc-lp.sh
CLUNC_LP_BURN_ONLY=1 CLUNC_LP_YES=1 ./scripts/mint-clunc-custc-lp.sh
```

## Related

- [`docs/runbooks/rebalance-mint-clunc-custc-lp.md`](../docs/runbooks/rebalance-mint-clunc-custc-lp.md) — swap to oracle, then $5k/$5k rungs
- [`AGENTS_REBALANCE_ORACLE_MINT_SWAP_BURN.md`](./AGENTS_REBALANCE_ORACLE_MINT_SWAP_BURN.md) — oracle peg only (no LP), then leftover burn
- [`AGENTS_REBALANCE_MINT_UST1_LP.md`](./AGENTS_REBALANCE_MINT_UST1_LP.md) — UST1 extra-minter mint + leftover burn
- [`AGENTS_KEY_CUSTODY.md`](./AGENTS_KEY_CUSTODY.md) — 2-of-3 roster
