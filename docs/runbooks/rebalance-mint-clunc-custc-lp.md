# Rebalance cLUNC/cUSTC and mint $10k v2 LP to the CMM treasury

Operator script for deepening the **cLUNC/cUSTC** pair to **$10,000** TVL (**$5,000** each side) in USD rungs, after bringing the pool to the indexer LUNC/USTC oracles. LP is minted to the **ustr-cmm CMM treasury**. Leftover **cLUNC / cUSTC** on the ops wallet is holder-burned. The script **never** burns LP, LP tokens, CMM holdings, or native `uluna`.

To add ~$10k at the **live** ratio (keep a cLUNC premium, no swap): [`mint-clunc-custc-lp.md`](./mint-clunc-custc-lp.md).

Oracle peg only (mint + swap, **no LP**, then burn leftover), including UST1/cUSTC at **1 UST1 = $1**: [`rebalance-oracle-mint-swap-burn.md`](./rebalance-oracle-mint-swap-burn.md).

Script: [`scripts/rebalance-mint-clunc-custc-lp.sh`](../../scripts/rebalance-mint-clunc-custc-lp.sh)  
Math: [`scripts/lib/clunc-custc-lp-math.py`](../../scripts/lib/clunc-custc-lp-math.py) (`--self-test`)

## Roles

| Role | Default |
|------|---------|
| CMM treasury (LP receiver) | `terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2` |
| Primary CW20 minter | `cl8y2_admin` → `terra1xsecn4…` (cLUNC extra minter is wrap-mapper only) |
| Admin hot wallet (swap + provide + burn) | `cl8ydeploy` → `terra1hu4zgg…` |
| cLUNC/cUSTC pair | `terra15rl8g308yzzt5kxu4skgwlahrvm8adyv0s2cupsmvte0akgs2ttsszau38` |
| LP token | `terra132uuzdnjce0c8g5dalyvdgl47ny697udesk972cg05e5y7gn485qz6tdch` |

Both wraps are **6 decimals**. Pair asset_0 is cLUNC, asset_1 is cUSTC.

## Price

- Indexer: `GET https://indexer.dex.cl8y.com/api/v1/oracle/price/lunc` and `/ustc`.
- Hub check: `GET /api/v1/hub-prices` `lunc` / `custc` must match those oracles (1:1 wrap) and the wrap CW20 addresses.
- Live CEX: CoinGecko (`terra-luna`, `terrausd`), Binance, KuCoin, MEXC. Indexer must sit within **3%** of the live median (`CLUNC_LP_CROSS_CHECK_TOLERANCE`).
- Target pool price = **cUSTC per cLUNC** = `LUNC_USD / USTC_USD`.
- After each swap and each provide, the script refuses to continue if the pool is still outside **0.1%**.

## Rungs

Default `CLUNC_LP_RUNGS=200,500,2000,5000,10000`. Each value is **total** oracle TVL (50/50 USD). After every rung the pool is rebalanced again before the next add.

## Mint modes

| `CLUNC_LP_MINT_MODE` | What it does |
|----------------------|----------------|
| `minter` (default) | `cl8y2_admin` CW20 `mint` to the admin wallet. **Unbacked** wrap supply (W1 solvency: treasury native may fall below `total_supply`). Use only when the ops wallet does not hold enough native LUNC/USTC. |
| `wrap` | Admin `treasury.WrapDeposit` with `uluna` / `uusd`. Gross-up by on-chain `fee_wrap_bps` (200 as of 2026-09-04). No Classic burn tax on wrap_deposit. |

## Resume after a mid-rung failure

Re-run is safe. Completed rungs are skipped when live TVL is already at/above that USD target (provide `add_usd=0`). Example: after `$200` and `$500` succeeded and `$2000` oracle refresh failed, the pool is ~$500 and CMM already holds LP. The same command continues at `$2000`:

```bash
read -rs TERRAD_HOST_KEYRING_PASS; export TERRAD_HOST_KEYRING_PASS
CLUNC_LP_YES=1 ./scripts/rebalance-mint-clunc-custc-lp.sh
```

Oracle refresh retries 4 times (3s/6s/9s backoff). Hub vs indexer 0.1% drift is a warning; a live CEX blip no longer aborts the run unless prices stay disagreeing after retries. `CLUNC_LP_SKIP_CROSS_CHECK=1` still bypasses CEX if needed.

## One command

```bash
python3 scripts/lib/clunc-custc-lp-math.py --self-test
DRY_RUN=1 ./scripts/rebalance-mint-clunc-custc-lp.sh
# live (prompts passphrase once, then y/N):
./scripts/rebalance-mint-clunc-custc-lp.sh
# non-interactive:
read -rs TERRAD_HOST_KEYRING_PASS; export TERRAD_HOST_KEYRING_PASS
CLUNC_LP_YES=1 ./scripts/rebalance-mint-clunc-custc-lp.sh
# leftover cLUNC / cUSTC on cl8ydeploy after a successful run:
CLUNC_LP_BURN_ONLY=1 CLUNC_LP_YES=1 ./scripts/rebalance-mint-clunc-custc-lp.sh
```

Never commit `TERRAD_HOST_KEYRING_PASS`. File-keyring keys (`cl8y2_admin`, `cl8ydeploy`) must share that passphrase.

## What success looks like

1. Indexer LUNC/USTC prices agree with live CEX (and hub wrap identity).
2. After each rung, human cUSTC-per-cLUNC is within **0.1%** of `LUNC_USD / USTC_USD`.
3. Final pool TVL ≥ **$9,800**, each side within **$250** of **$5,000**.
4. CMM treasury **cLUNC-cUSTC LP** increased; admin holds **no** LP (provide `receiver` = treasury, leftover LP is transferred).
5. Admin leftover **cLUNC / cUSTC** is holder-burned. LP tokens are never burned.

## Overrides

| Env | Purpose |
|-----|---------|
| `CLUNC_LP_RUNGS` | Comma-separated USD TVL rungs |
| `CLUNC_LP_MINT_MODE` | `minter` or `wrap` |
| `CLUNC_LP_ADMIN_KEY` / `CLUNC_LP_ADMIN_ADDR` | Hot wallet (default `cl8ydeploy`) |
| `CLUNC_LP_MINTER_KEY` / `CLUNC_LP_MINTER_ADDR` | Primary minter (default `cl8y2_admin`) |
| `CLUNC_LP_PRICE_TOLERANCE` | Rel error vs oracle (default `0.001`) |
| `CLUNC_LP_CROSS_CHECK_TOLERANCE` | Indexer vs live CEX median (default `0.03`) |
| `CLUNC_LP_SKIP_CROSS_CHECK=1` | Skip live CEX (indexer still required) |
| `CLUNC_LP_YES=1` | Skip interactive confirm |
| `CLUNC_LP_BURN_ONLY=1` | Holder-burn leftover cLUNC/cUSTC on admin |
