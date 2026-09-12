# Oracle rebalance: mint + swap UST1/cUSTC and cLUNC/cUSTC, burn leftover

Operator script that brings **UST1/cUSTC** and **cLUNC/cUSTC** to the indexer oracles with **mint + pool-only swap**, then **holder-burns leftover wrap** on the ops wallet.

**Does not provide LP.** That is the difference from [`rebalance-mint-ust1-lp.md`](./rebalance-mint-ust1-lp.md) and [`rebalance-mint-clunc-custc-lp.md`](./rebalance-mint-clunc-custc-lp.md). To add TVL at the **live** cLUNC premium (no swap), use [`mint-clunc-custc-lp.md`](./mint-clunc-custc-lp.md) instead — this script **sells** that premium.

Script: [`scripts/rebalance-oracle-mint-swap-burn.sh`](../../scripts/rebalance-oracle-mint-swap-burn.sh)  
Playbook: [`skills/AGENTS_REBALANCE_ORACLE_MINT_SWAP_BURN.md`](../../skills/AGENTS_REBALANCE_ORACLE_MINT_SWAP_BURN.md)

## Peg

| Pair | Target human price | Formula |
|------|--------------------|---------|
| UST1/cUSTC | cUSTC per UST1 | `1 / USTC_USD` (**1 UST1 = $1**) |
| cLUNC/cUSTC | cUSTC per cLUNC | `LUNC_USD / USTC_USD` |

Indexer: `GET https://indexer.dex.cl8y.com/api/v1/oracle/price/ustc` and `/lunc`, plus hub wrap identity and live CEX median (same 3% band as the cLUNC LP script). After the **single** swap tx the script refuses to continue if a selected pool is still outside **0.1%**.

This path **sells a cLUNC premium** when live cUSTC-per-cLUNC is above the oracle. That is the point once the premium has exceeded the target.

## Atomic swap tx

Mint uses different keys (2-of-3 / `cl8y2_admin`) so it **cannot** share a transaction with the swaps. Mint does **not** move AMM prices. **Both pool swaps are one admin tx** (two `MsgExecuteContract` CW20 `Send`+`Swap` messages). Searchers cannot insert a trade between UST1/cUSTC and cLUNC/cUSTC. Leftover burn is a follow-up (also does not move AMM prices).

## Roles

Same columbus-5 anchors as the LP scripts:

| Role | Default |
|------|---------|
| Admin hot wallet | `cl8ydeploy` → `terra1hu4zgg…` |
| DEX 2-of-3 (UST1 + cUSTC extra minter) | `terra1zlmv2…` (`multisig_2of3` + `multisig1`/`multisig2`) |
| Primary CW20 minter (cLUNC) | `cl8y2_admin` → `terra1xsecn4…` |
| UST1/cUSTC pair | `terra1ceprjs…` |
| cLUNC/cUSTC pair | `terra15rl8g…` |

UST1 is minted via the **2-of-3 extra minter**. cLUNC extra minter is wrap-mapper only — cLUNC is minted by **primary minter** (or `CLUNC_LP_MINT_MODE=wrap`). cUSTC uses 2-of-3 when the UST1 pair is in this run; otherwise the cLUNC mint path.

## What success looks like

1. Indexer LUNC/USTC agree with live CEX (unless `ORACLE_RB_SKIP_CROSS_CHECK=1`).
2. Selected pools are within **0.1%** of the pegs above.
3. Admin holds **no leftover UST1 / cUSTC / cLUNC**. LP tokens, USTR, native `uluna`, and CMM holdings are **not** burned.
4. No provide: CMM LP balances are unchanged by this script.

## One command

```bash
python3 scripts/lib/ust1-lp-rebalance-math.py --self-test
python3 scripts/lib/clunc-custc-lp-math.py --self-test
DRY_RUN=1 ./scripts/rebalance-oracle-mint-swap-burn.sh
# live (prompts passphrase once, then y/N):
./scripts/rebalance-oracle-mint-swap-burn.sh
# non-interactive:
read -rs TERRAD_HOST_KEYRING_PASS; export TERRAD_HOST_KEYRING_PASS
ORACLE_RB_YES=1 ./scripts/rebalance-oracle-mint-swap-burn.sh
# one pair only:
ORACLE_RB_PAIRS=ust1-custc ORACLE_RB_YES=1 ./scripts/rebalance-oracle-mint-swap-burn.sh
ORACLE_RB_PAIRS=clunc-custc ORACLE_RB_YES=1 ./scripts/rebalance-oracle-mint-swap-burn.sh
# leftover wrap on cl8ydeploy after a successful run:
ORACLE_RB_BURN_ONLY=1 ORACLE_RB_YES=1 ./scripts/rebalance-oracle-mint-swap-burn.sh
```

Never commit `TERRAD_HOST_KEYRING_PASS`. File-keyring keys (`multisig1`, `multisig2`, `multisig_2of3`, `cl8y2_admin`, `cl8ydeploy`) must share that passphrase.

## Overrides

| Env | Purpose |
|-----|---------|
| `ORACLE_RB_PAIRS` | `both` (default), `ust1-custc`, or `clunc-custc` |
| `ORACLE_RB_PRICE_TOLERANCE` | Rel error vs oracle (default `0.001`) |
| `ORACLE_RB_YES=1` | Skip interactive confirm |
| `ORACLE_RB_BURN_ONLY=1` | Holder-burn leftover UST1/cUSTC/cLUNC; skip mint/swap |
| `ORACLE_RB_SKIP_BURN=1` | Leave leftover wrap on admin after a pegged run |
| `ORACLE_RB_SKIP_CROSS_CHECK=1` | Skip live CEX (indexer still required) |
| `CLUNC_LP_MINT_MODE` | `minter` (default) or `wrap` for cLUNC (and cUSTC when UST1 pair is skipped) |
| `UST1_LP_ADMIN_KEY` / `UST1_LP_ADMIN_ADDR` | Hot wallet |
| `ORACLE_RB_SWAP_MAX_SPREAD` | Pool swap `max_spread` (default `0.20`) |
| `ORACLE_RB_SWAP_GAS` | Gas limit on the atomic multi-msg swap tx (default `3000000`) |
