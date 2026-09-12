# Mint $5k+$5k cLUNC/cUSTC v2 LP to CMM (no rebalance)

Operator script for **adding** about **$10,000** of cLUNC/cUSTC v2 LP (**~$5,000** each side in oracle USD when the pool is on peg) at the **live pool ratio**, then sending LP to the **ustr-cmm CMM treasury**. Leftover **cLUNC / cUSTC** on the ops wallet is holder-burned.

The script **does not swap**. It keeps a **cLUNC premium** (human cUSTC-per-cLUNC above the indexer LUNC/USTC oracle) so selling LUNC into the pool stays attractive. USD legs follow the live ratio and are **not** forced to $5k/$5k while the pool is off peg.

On-oracle deepen (swap to peg, then $5k/$5k): [`rebalance-mint-clunc-custc-lp.md`](./rebalance-mint-clunc-custc-lp.md).

Peg only, no LP (UST1/cUSTC + cLUNC/cUSTC, then burn leftover): [`rebalance-oracle-mint-swap-burn.md`](./rebalance-oracle-mint-swap-burn.md).

Script: [`scripts/mint-clunc-custc-lp.sh`](../../scripts/mint-clunc-custc-lp.sh)  
Math: [`scripts/lib/clunc-custc-lp-math.py`](../../scripts/lib/clunc-custc-lp-math.py) (`--self-test`)

## Roles

Same columbus-5 anchors as the rebalance runbook:

| Role | Default |
|------|---------|
| CMM treasury (LP receiver) | `terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2` |
| Primary CW20 minter | `cl8y2_admin` → `terra1xsecn4…` |
| Admin hot wallet | `cl8ydeploy` → `terra1hu4zgg…` |
| cLUNC/cUSTC pair | `terra15rl8g308yzzt5kxu4skgwlahrvm8adyv0s2cupsmvte0akgs2ttsszau38` |

Both wraps are **6 decimals**. Pair asset_0 is cLUNC, asset_1 is cUSTC.

## What it does

1. Fetch indexer LUNC/USTC oracles and cross-check live CEX (same as the rebalance script).
2. Size a **pro-rata** provide of `CLUNC_LP_ADD_USD` (default **10000**) at the current reserves. No pool swap.
3. Mint (or wrap) inventory to the admin wallet, `provide_liquidity` with `receiver` = CMM.
4. Refuse to continue if the pool price moved outside **0.1%** of the pre-provide price.
5. Transfer leftover LP to CMM; holder-burn leftover wrap CW20. Never burns LP or native `uluna`.

Refuses an **empty** pool (no live ratio). Warns if cLUNC is **not** at a premium vs oracle; it will not create a premium.

## One command

```bash
python3 scripts/lib/clunc-custc-lp-math.py --self-test
DRY_RUN=1 ./scripts/mint-clunc-custc-lp.sh
# live (prompts passphrase once, then y/N):
./scripts/mint-clunc-custc-lp.sh
# non-interactive:
read -rs TERRAD_HOST_KEYRING_PASS; export TERRAD_HOST_KEYRING_PASS
CLUNC_LP_YES=1 ./scripts/mint-clunc-custc-lp.sh
# leftover cLUNC / cUSTC on cl8ydeploy after a successful run:
CLUNC_LP_BURN_ONLY=1 CLUNC_LP_YES=1 ./scripts/mint-clunc-custc-lp.sh
```

Never commit `TERRAD_HOST_KEYRING_PASS`. File-keyring keys (`cl8y2_admin`, `cl8ydeploy`) must share that passphrase.

## What success looks like

1. Indexer LUNC/USTC prices agree with live CEX (and hub wrap identity).
2. Pool **price** after provide is within **0.1%** of the pre-provide price (premium kept).
3. Oracle TVL rose by about **$10,000** (97% floor).
4. CMM treasury **cLUNC-cUSTC LP** increased; admin holds **no** LP.
5. Admin leftover **cLUNC / cUSTC** is holder-burned.

## Overrides

| Env | Purpose |
|-----|---------|
| `CLUNC_LP_ADD_USD` | Oracle TVL to **add** (default `10000`) |
| `CLUNC_LP_SKIP_SWAP` | Forced `1` by this wrapper |
| `CLUNC_LP_MINT_MODE` | `minter` (default) or `wrap` |
| `CLUNC_LP_ADMIN_KEY` / `CLUNC_LP_ADMIN_ADDR` | Hot wallet (default `cl8ydeploy`) |
| `CLUNC_LP_MINTER_KEY` / `CLUNC_LP_MINTER_ADDR` | Primary minter (default `cl8y2_admin`) |
| `CLUNC_LP_SKIP_CROSS_CHECK=1` | Skip live CEX (indexer still required) |
| `CLUNC_LP_YES=1` | Skip interactive confirm |
| `CLUNC_LP_BURN_ONLY=1` | Holder-burn leftover cLUNC/cUSTC on admin |
