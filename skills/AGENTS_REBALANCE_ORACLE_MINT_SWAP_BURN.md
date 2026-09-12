# Agent playbook: oracle rebalance via mint + swap, then burn leftover

Use when bringing **UST1/cUSTC** and/or **cLUNC/cUSTC** onto the indexer oracles **without** adding LP. Peg is **1 UST1 = $1** and LUNC/USTC oracles. Leftover wrap on the ops wallet is holder-burned.

Do **not** use this to keep a cLUNC premium — swapping cLUNC/cUSTC onto the oracle sells it. For live-ratio LP, see [`AGENTS_MINT_CLUNC_CUSTC_LP.md`](./AGENTS_MINT_CLUNC_CUSTC_LP.md).

## Canonical references

| Doc / script | Purpose |
|--------------|---------|
| [`docs/runbooks/rebalance-oracle-mint-swap-burn.md`](../docs/runbooks/rebalance-oracle-mint-swap-burn.md) | Operator runbook |
| [`scripts/rebalance-oracle-mint-swap-burn.sh`](../scripts/rebalance-oracle-mint-swap-burn.sh) | Host terrad: mint → **one** admin swap tx → burn |
| [`scripts/lib/ust1-lp-rebalance-math.py`](../scripts/lib/ust1-lp-rebalance-math.py) | UST1/cUSTC swap-only plan (`usd_*=0`) |
| [`scripts/lib/clunc-custc-lp-math.py`](../scripts/lib/clunc-custc-lp-math.py) | cLUNC/cUSTC swap-only plan (`force_add_usd=0`) |

## Rules of thumb

1. **Unlock once** — prompt or `TERRAD_HOST_KEYRING_PASS`. Never commit it.
2. **No LP.** Provide is not part of this path. Fail if the planner returns LP legs.
3. **UST1 = $1.** Target cUSTC-per-UST1 = `1 / USTC_USD`. cLUNC/cUSTC target = `LUNC_USD / USTC_USD`. **Sell** a cLUNC premium when it is above oracle (intended once the premium has exceeded the target).
4. **Mint paths:** UST1 (and cUSTC when that pair is in the run) via DEX 2-of-3. cLUNC via primary minter (or wrap). Mint is a **prior** tx — it does not move AMM prices.
5. **One swap tx.** Both pool-only CW20 Send+Swap messages go in a **single** admin transaction. Do not iterate swaps across blocks. If the peg check fails after inclusion, stop (inventory stays on admin) and re-run.
6. **Burn leftover UST1 / cUSTC / cLUNC only.** Never LP, USTR, native `uluna`, or CMM.
7. Prefer `DRY_RUN=1` before a live broadcast. Live needs `ORACLE_RB_YES=1` when stdin is not a TTY.

## Quick commands

```bash
python3 scripts/lib/ust1-lp-rebalance-math.py --self-test
python3 scripts/lib/clunc-custc-lp-math.py --self-test
DRY_RUN=1 ./scripts/rebalance-oracle-mint-swap-burn.sh
ORACLE_RB_YES=1 ./scripts/rebalance-oracle-mint-swap-burn.sh
ORACLE_RB_PAIRS=ust1-custc DRY_RUN=1 ./scripts/rebalance-oracle-mint-swap-burn.sh
ORACLE_RB_BURN_ONLY=1 ORACLE_RB_YES=1 ./scripts/rebalance-oracle-mint-swap-burn.sh
```

## Related

- [`AGENTS_REBALANCE_MINT_UST1_LP.md`](./AGENTS_REBALANCE_MINT_UST1_LP.md) — same UST1 peg, then $5k LP
- [`docs/runbooks/rebalance-mint-clunc-custc-lp.md`](../docs/runbooks/rebalance-mint-clunc-custc-lp.md) — same cLUNC peg, then $10k rungs
- [`AGENTS_MINT_CLUNC_CUSTC_LP.md`](./AGENTS_MINT_CLUNC_CUSTC_LP.md) — no swap, keep premium
- [`AGENTS_KEY_CUSTODY.md`](./AGENTS_KEY_CUSTODY.md) — 2-of-3 roster
