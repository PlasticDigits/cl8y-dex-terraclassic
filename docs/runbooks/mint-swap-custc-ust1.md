# Hourly $200 cUSTC mint → best-solver UST1 swap

Operator script that **mints $200 of cUSTC every hour** (sized at the indexer **USTC/USD** oracle), swaps it to **UST1** via the indexer **best-execution solver**, then **sends that UST1 to CMM** until CMM’s **vFDUSD CW20 × Venus × FDUSD/USD** is **$500**. Live default **loops** until that on-chain figure is met.

Script: [`scripts/mint-swap-custc-ust1.sh`](../../scripts/mint-swap-custc-ust1.sh)  
Math: [`scripts/lib/custc-ust1-buyback-math.py`](../../scripts/lib/custc-ust1-buyback-math.py) (`self-test`); router send-hook in [`ust1-clunc-buyback-math.py`](../../scripts/lib/ust1-clunc-buyback-math.py)  
Playbook: [`skills/AGENTS_MINT_SWAP_CUSTC_UST1.md`](../../skills/AGENTS_MINT_SWAP_CUSTC_UST1.md)

## Why buy UST1 (and not redeem the window)

CMM `terra16j5u6…` **is** the ust1-window treasury. Window redeem of UST1 **sends vFDUSD out of CMM**. This script **never** calls the window.

Each tick **does not** send vFDUSD to CMM. Buying UST1 on the DEX is buy pressure + (default) treasury inventory. Third-party arb is expected to mint UST1 through the window (deposit vFDUSD into CMM) over time. CMM’s UST1 CW20 balance is **not** part of the stop.

Optional `CUSTC_UST1_DEST=hold` leaves UST1 on the admin hot wallet; `burn` holder-burns the tick’s UST1. The stop is still vFDUSD only.

Do **not** unwrap the minted cUSTC. Extra-mint is **unbacked wrap supply** (same class as `CLUNC_LP_MINT_MODE=minter`). The script warns when `cUSTC total_supply + this mint > CMM bank uusd`.

## Roles

| Role | Default |
|------|---------|
| CMM treasury | `terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2` |
| DEX 2-of-3 (cUSTC extra minter) | `terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7` (`multisig_2of3` + `multisig1`/`multisig2`) |
| Admin hot wallet | `cl8ydeploy` → `terra1hu4zggf3f8yw6jw3rxrjxn2drwad675gq5k2lv` |
| Router | `terra1e7s0h9ftxakwca5gxspyt4haeuaqxds6swr08ul3tsepq7el924sprrsrw` |
| cUSTC | `terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch` |
| UST1 | `terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72` |
| vFDUSD | `terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3` |

cUSTC lists the DEX 2-of-3 as an extra minter (same as [`rebalance-mint-ust1-lp.md`](./rebalance-mint-ust1-lp.md)). Do not mint UST1 on this path — **buy** it.

## Stop condition

Indexer FDUSD oracle: `GET https://indexer.dex.cl8y.com/api/v1/oracle/price/vfdusd` (`price_usd` is **CEX FDUSD/USD**, not 1 vFDUSD = $1). Venus redeem: additive JSON `venus.fdusd_per_vfdusd` (or `/oracle/price/vfdusd/venus`).

Stop when

`USD = CMM vFDUSD CW20 × Venus fdusd_per_vfdusd × FDUSD/USD >= 500`

Do **not** add CMM’s UST1. Do **not** count LP. Do **not** stop on cumulative swapped USD (that file is telemetry only). Do **not** assume 1 vFDUSD = 1 FDUSD.

How long that takes depends on third-party window deposits, not on ~$200 of UST1 bought per tick.

Pin `CUSTC_UST1_VFDUSD_USD` to skip Venus × CEX (USD per 1 human vFDUSD). Pin `CUSTC_UST1_USTC_USD` to skip the USTC oracle used for mint sizing.

Live default sleeps **3600s** between ticks (`CUSTC_UST1_LOOP=1`). Dry-run is one tick unless `CUSTC_UST1_LOOP=1`.

State file (swap telemetry): `$HOME/.cl8y-dex/custc-ust1-buyback-state.json`.

## One command

```bash
python3 scripts/lib/custc-ust1-buyback-math.py self-test
DRY_RUN=1 ./scripts/mint-swap-custc-ust1.sh
# live hourly loop until CMM vFDUSD USD >= 500:
read -rs TERRAD_HOST_KEYRING_PASS; export TERRAD_HOST_KEYRING_PASS
CUSTC_UST1_YES=1 ./scripts/mint-swap-custc-ust1.sh
# single tick only:
CUSTC_UST1_LOOP=0 CUSTC_UST1_YES=1 ./scripts/mint-swap-custc-ust1.sh
```

Cron (one tick per hour; set `CUSTC_UST1_LOOP=0`):

```bash
0 * * * * cd /home/answorld/repos/cl8y-dex-terraclassic && \
  TERRAD_HOST_KEYRING_PASS=… CUSTC_UST1_YES=1 CUSTC_UST1_LOOP=0 \
  ./scripts/mint-swap-custc-ust1.sh \
  >>/var/log/custc-ust1-buyback.log 2>&1
```

Never commit `TERRAD_HOST_KEYRING_PASS`. File-keyring keys (`multisig1`, `multisig2`, `multisig_2of3`, `cl8ydeploy`) must share that passphrase.

## What success looks like

1. Each tick mints at most **$200 of cUSTC** at the live USTC oracle (skips when admin already holds that much).
2. Swap uses indexer `GET /api/v1/route/solve/best` (`trader`/`sender` = admin) and executes those `router_operations` with a **5%** `minimum_receive` floor. Book hops also get per-hop `min_return` from pair `hybrid_simulation` (#334; indexer quotes omit it).
3. Default dest **CW20-transfers only the UST1 received this tick to CMM**. Does not unwrap, does not window-redeem, does not burn cUSTC, LP, or vFDUSD already on CMM.
4. After each tick the script re-queries CMM vFDUSD and **exits 0** once that USD ≥ **500**. The swap itself does not move that number; window-mint arb does. Failed ticks sleep and retry; they do not abort the loop.

## Overrides

| Env | Purpose |
|-----|---------|
| `CUSTC_UST1_MINT_USD` | USD of cUSTC per tick (default `200`) |
| `CUSTC_UST1_TARGET_USD` | Stop USD (default `500`) |
| `CUSTC_UST1_INTERVAL_SEC` | Loop sleep (default `3600`) |
| `CUSTC_UST1_LOOP` | Live default `1`; dry-run default `0`; set `0` for a single tick / cron |
| `CUSTC_UST1_DEST` | `cmm` (default), `hold`, or `burn` |
| `CUSTC_UST1_SLIPPAGE_PERCENT` | `minimum_receive` floor (default `5`) |
| `CUSTC_UST1_MAX_SPREAD` | Router `max_spread` (default `0.20`) |
| `CUSTC_UST1_USTC_USD` | Skip indexer USTC oracle |
| `CUSTC_UST1_VFDUSD_USD` | Skip Venus × FDUSD; USD per 1 human vFDUSD |
| `CUSTC_UST1_FDUSD_USD` | Pin CEX FDUSD/USD |
| `CUSTC_UST1_FDUSD_PER_VFDUSD` | Pin Venus redeem |
| `CUSTC_UST1_YES=1` | Skip interactive confirm |
| `CUSTC_UST1_STATE_FILE` | Swap telemetry JSON |
| `TERRAD_HOST_GAS_ADJUSTMENT` | Default **1.6** for this script (hybrid hops) |
