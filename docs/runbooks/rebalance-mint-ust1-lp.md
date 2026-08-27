# Rebalance UST1/cUSTC and mint $5k LP to the CMM treasury

Operator script for deepening **UST1/cUSTC** and **UST1/USTR** with **$5000** of LP each, after bringing the **UST1/cUSTC** pool to the indexer USTC oracle (within **0.1%**). LP is minted to the **ustr-cmm CMM treasury**. The UST1/USTR pool is **not** swapped.

Script: [`scripts/rebalance-mint-ust1-lp.sh`](../../scripts/rebalance-mint-ust1-lp.sh)  
Playbook: [`skills/AGENTS_REBALANCE_MINT_UST1_LP.md`](../../skills/AGENTS_REBALANCE_MINT_UST1_LP.md)

## Roles

| Role | Default |
|------|---------|
| CMM treasury (LP receiver) | `terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2` |
| DEX 2-of-3 (extra CW20 minter) | `terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7` (`multisig_2of3` + `multisig1`/`multisig2`) |
| Admin hot wallet (swap + provide) | `cl8ydeploy` → `terra1hu4zggf3f8yw6jw3rxrjxn2drwad675gq5k2lv` |
| UST1/cUSTC pair | `terra1ceprjsxp86ggftf5e38wwt34l83e5gq7penkdnv4wsatkwcs8v6qccw55f` |
| UST1/USTR pair | `terra16vxrhpvpcucu05y0nr862vf9hnqeh274uaff4s7hz4n0ea74006qf5hgqy` |

UST1, cUSTC, and USTR list the DEX 2-of-3 as an **extra minter**. Primary minter remains wrap-stack governance (`terra1xsecn4…`).

## Price

- Oracle: `GET https://indexer.dex.cl8y.com/api/v1/oracle/price/ustc` (override `UST1_LP_USTC_USD`).
- Target UST1/cUSTC = **cUSTC per UST1** = `1 / USTC_USD`.
- USTR fair value for **LP sizing** = **2.5 × USTC** (`USTR_PER_USTC` in this script only). Not a display oracle — Charts/Protocol USD uses DEX hub marks ([#556](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/556)). Do not swap UST1/USTR.
- After the cUSTC-pair swap, the script refuses to provide if the pool is still outside **0.1%**.
- Live run **re-queries** UST1/cUSTC reserves after mint (and before each of up to **3** swaps). A plan computed before the 2-of-3 mint txs will overshoot if the pool already moved toward the peg.
- If the price gate fails, minted inventory stays on `cl8ydeploy`. Re-run; mint should skip when balances already cover LP.

## One command

```bash
DRY_RUN=1 ./scripts/rebalance-mint-ust1-lp.sh
# live (prompts passphrase once, then y/N):
./scripts/rebalance-mint-ust1-lp.sh
# non-interactive:
read -rs TERRAD_HOST_KEYRING_PASS; export TERRAD_HOST_KEYRING_PASS
UST1_LP_YES=1 ./scripts/rebalance-mint-ust1-lp.sh
```

Never commit `TERRAD_HOST_KEYRING_PASS`. All file-keyring keys (`multisig1`, `multisig2`, `multisig_2of3`, `cl8ydeploy`) must share that passphrase.

## What success looks like

1. Admin received minted UST1 / cUSTC / USTR (or already held enough).
2. UST1/cUSTC human price within **0.1%** of `1 / USTC_USD`.
3. CMM treasury **UST1-CUST-LP** and **UST1-USTR-LP** balances increased.
4. Admin holds **no** leftover LP (provide uses `receiver` = treasury).

## Overrides

| Env | Purpose |
|-----|---------|
| `UST1_LP_USD_EACH` | USD per pair (default `5000`) |
| `UST1_LP_PAIRS` | `both` (default), `ustr`, or `custc` |
| `UST1_LP_USD_CUSTC` / `UST1_LP_USD_USTR` | Per-pair USD (default from `UST1_LP_USD_EACH` / pairs) |
| `UST1_LP_SKIP_SWAP=1` | Skip UST1/cUSTC rebalance (implied by `ustr`) |
| `UST1_LP_PRICE_TOLERANCE` | Rel error vs oracle (default `0.001`) |
| `UST1_LP_ADMIN_KEY` / `UST1_LP_ADMIN_ADDR` | Hot wallet (default `cl8ydeploy`) |
| `UST1_LP_USTC_USD` | Skip indexer oracle |
| `UST1_LP_YES=1` | Skip interactive confirm |
| `UST1_LP_SWAP_MAX_SPREAD` | Rebalance swap `max_spread` (default `0.20`) |
| `UST1_LP_SWAP_MAX_ITERS` | Live rebalance swaps after mint (default `3`) |
