# Hourly UST1 mint → best-solver cLUNC burn

Operator script that **mints 50 UST1 every hour**, swaps it to **cLUNC** via the indexer **best-execution solver**, then **holder-burns** that cLUNC until CMM’s **bank `uluna` × LUNC oracle** is **$2500**. Live default **loops** until that on-chain figure is met.

Script: [`scripts/mint-swap-burn-ust1-clunc.sh`](../../scripts/mint-swap-burn-ust1-clunc.sh)  
Math: [`scripts/lib/ust1-clunc-buyback-math.py`](../../scripts/lib/ust1-clunc-buyback-math.py) (`self-test`)  
Playbook: [`skills/AGENTS_MINT_SWAP_BURN_UST1_CLUNC.md`](../../skills/AGENTS_MINT_SWAP_BURN_UST1_CLUNC.md)

## Why burn (and not unwrap)

CMM `terra16j5u6…` **is** wrap custody. Mapper unwrap → treasury `InstantWithdraw` **sends native `uluna` out of CMM** (plus Classic burn tax). This script **never** unwraps.

Each tick **does not** send native LUNC to CMM. Burning cLUNC after buying it on the DEX is buy pressure + supply shrink. Third-party arb is expected to wrap (deposit native `uluna` into CMM) over time. CMM’s cLUNC CW20 balance is **not** part of the stop.

Optional `UST1_CLUNC_DEST=cmm` CW20-transfers instead of burning; the stop is still bank `uluna` only.

## Roles

| Role | Default |
|------|---------|
| CMM treasury | `terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2` |
| DEX 2-of-3 (UST1 extra minter) | `terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7` (`multisig_2of3` + `multisig1`/`multisig2`) |
| Admin hot wallet | `cl8ydeploy` → `terra1hu4zggf3f8yw6jw3rxrjxn2drwad675gq5k2lv` |
| Router | `terra1e7s0h9ftxakwca5gxspyt4haeuaqxds6swr08ul3tsepq7el924sprrsrw` |
| UST1 | `terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72` |
| cLUNC | `terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg` |

UST1 lists the DEX 2-of-3 as an extra minter (same as [`rebalance-mint-ust1-lp.md`](./rebalance-mint-ust1-lp.md)). cLUNC extra minter is wrap-mapper only — this path **buys** cLUNC, it does not mint it.

## Stop condition

Indexer LUNC oracle: `GET https://indexer.dex.cl8y.com/api/v1/oracle/price/lunc` (`UST1_CLUNC_LUNC_USD` to pin).

Stop when

`USD = CMM bank uluna × LUNC oracle >= 2500`

Do **not** add CMM’s cLUNC CW20. Do **not** count LP. Do **not** stop on cumulative burned USD (that file is telemetry only).

Columbus-5 snapshot 2026-09-09: wrap-custody `uluna` ≈ **$243**, so remain ≈ **$2257**. How long that takes depends on third-party wrap deposits, not on ~$45 of cLUNC burned per tick.

Live default sleeps **3600s** between ticks (`UST1_CLUNC_LOOP=1`). Dry-run is one tick unless `UST1_CLUNC_LOOP=1`.

State file (burn telemetry): `$HOME/.cl8y-dex/ust1-clunc-buyback-state.json`.

## One command

```bash
python3 scripts/lib/ust1-clunc-buyback-math.py self-test
DRY_RUN=1 ./scripts/mint-swap-burn-ust1-clunc.sh
# live hourly loop until CMM bank uluna USD >= 2500:
read -rs TERRAD_HOST_KEYRING_PASS; export TERRAD_HOST_KEYRING_PASS
UST1_CLUNC_YES=1 ./scripts/mint-swap-burn-ust1-clunc.sh
# single tick only:
UST1_CLUNC_LOOP=0 UST1_CLUNC_YES=1 ./scripts/mint-swap-burn-ust1-clunc.sh
```

Cron (one tick per hour; set `UST1_CLUNC_LOOP=0`):

```bash
0 * * * * cd /home/answorld/repos/cl8y-dex-terraclassic && \
  TERRAD_HOST_KEYRING_PASS=… UST1_CLUNC_YES=1 UST1_CLUNC_LOOP=0 \
  ./scripts/mint-swap-burn-ust1-clunc.sh \
  >>/var/log/ust1-clunc-buyback.log 2>&1
```

Never commit `TERRAD_HOST_KEYRING_PASS`. File-keyring keys (`multisig1`, `multisig2`, `multisig_2of3`, `cl8ydeploy`) must share that passphrase.

## What success looks like

1. Each tick mints at most **50 UST1** (skips when admin already holds that much).
2. Swap uses indexer `GET /api/v1/route/solve/best` (`trader`/`sender` = admin) and executes those `router_operations` with a **5%** `minimum_receive` floor. Book hops also get per-hop `min_return` from pair `hybrid_simulation` (#334; indexer quotes omit it).
3. Default dest **holder-burns only the cLUNC received this tick**. Does not unwrap, does not burn UST1, LP, native `uluna`, or anything already on CMM.
4. After each tick the script re-queries CMM bank `uluna` and **exits 0** once that USD ≥ **2500**. The burn itself does not move that number; wrap arb does. Failed ticks sleep and retry; they do not abort the loop.

## Overrides

| Env | Purpose |
|-----|---------|
| `UST1_CLUNC_MINT_HUMAN` | UST1 per tick (default `50`) |
| `UST1_CLUNC_TARGET_USD` | Stop USD (default `2500`) |
| `UST1_CLUNC_INTERVAL_SEC` | Loop sleep (default `3600`) |
| `UST1_CLUNC_LOOP` | Live default `1`; dry-run default `0`; set `0` for a single tick / cron |
| `UST1_CLUNC_DEST` | `burn` (default) or `cmm` |
| `UST1_CLUNC_SLIPPAGE_PERCENT` | `minimum_receive` floor (default `5`) |
| `UST1_CLUNC_MAX_SPREAD` | Router `max_spread` (default `0.20`) |
| `UST1_CLUNC_LUNC_USD` | Skip indexer LUNC oracle |
| `UST1_CLUNC_YES=1` | Skip interactive confirm |
| `UST1_CLUNC_STATE_FILE` | Burn telemetry JSON |
| `TERRAD_HOST_GAS_ADJUSTMENT` | Default **1.6** for this script (hybrid hops) |
