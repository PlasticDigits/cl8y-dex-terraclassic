# Agent playbook: hourly UST1 mint → best-solver cLUNC burn

Use when running or changing the **50 UST1 / hour** mint → indexer **best solver** swap to **cLUNC** → holder-burn (or CMM transfer) until **$2500** of LUNC.

## Canonical references

| Doc / script | Purpose |
|--------------|---------|
| [`docs/runbooks/mint-swap-burn-ust1-clunc.md`](../docs/runbooks/mint-swap-burn-ust1-clunc.md) | Operator runbook |
| [`scripts/mint-swap-burn-ust1-clunc.sh`](../scripts/mint-swap-burn-ust1-clunc.sh) | Host terrad: 2-of-3 mint → `/route/solve/best` → burn or CMM |
| [`scripts/lib/ust1-clunc-buyback-defaults.sh`](../scripts/lib/ust1-clunc-buyback-defaults.sh) | Columbus-5 router / token / treasury anchors |
| [`scripts/lib/ust1-clunc-buyback-math.py`](../scripts/lib/ust1-clunc-buyback-math.py) | Mint raw, min-receive, USD, send-hook (`self-test`) |

## Rules of thumb

1. **Unlock once** — prompt or `TERRAD_HOST_KEYRING_PASS`. Never commit it. File-keyring keys share one passphrase.
2. **Mint via DEX 2-of-3 extra minter** (`terra1zlmv2…`) to the admin hot wallet (`cl8ydeploy` by default). Do not mint cLUNC (wrap-mapper is its extra minter).
3. **Execute indexer `GET /api/v1/route/solve/best`** ops as-is (hybrid hops included). Do **not** attach greedy (`G4`). `minimum_receive` uses **pre-tax** `estimated_amount_out`, not `estimated_amount_out_net`.
4. **Never unwrap.** CMM is wrap custody; InstantWithdraw would drain CMM `uluna` and pay burn tax. Default dest is CW20 **burn** of the tick’s cLUNC delta. `UST1_CLUNC_DEST=cmm` transfers that delta to CMM instead.
5. **Stop on campaign USD**, not CMM wrap-backing `uluna` (that stock is already the wrap float). Burn dest uses `$HOME/.cl8y-dex/ust1-clunc-buyback-state.json`. CMM dest uses CMM’s **cLUNC CW20** × LUNC oracle.
6. Prefer `DRY_RUN=1` before a live broadcast. Live needs `UST1_CLUNC_YES=1` when stdin is not a TTY. `UST1_CLUNC_LOOP=1` sleeps 1h between ticks and retries a failed tick rather than exiting.

## Quick commands

```bash
python3 scripts/lib/ust1-clunc-buyback-math.py self-test
DRY_RUN=1 ./scripts/mint-swap-burn-ust1-clunc.sh
UST1_CLUNC_YES=1 ./scripts/mint-swap-burn-ust1-clunc.sh
UST1_CLUNC_YES=1 UST1_CLUNC_LOOP=1 ./scripts/mint-swap-burn-ust1-clunc.sh
UST1_CLUNC_DEST=cmm UST1_CLUNC_YES=1 DRY_RUN=1 ./scripts/mint-swap-burn-ust1-clunc.sh
```

## Related

- [`AGENTS_REBALANCE_MINT_UST1_LP.md`](./AGENTS_REBALANCE_MINT_UST1_LP.md) — UST1 extra-minter mint + leftover burn
- [`AGENTS_HYBRID_QUOTING.md`](./AGENTS_HYBRID_QUOTING.md) — quote = execute via `/route/solve`
- [`AGENTS_WRAP_UNWRAP_BURN_TAX.md`](./AGENTS_WRAP_UNWRAP_BURN_TAX.md) — why unwrap is forbidden here
- [`AGENTS_KEY_CUSTODY.md`](./AGENTS_KEY_CUSTODY.md) — 2-of-3 roster
