# CMM USDT → UST1 best-solver drip

Operator script that spends **USDT held by the CMM treasury** to buy **UST1**
through the indexer **best-execution solver**, sends that UST1 **back to CMM**,
and repeats every **20 minutes** until CMM's USDT CW20 balance is down to a
**floor** (default **500 USDT**).

Script: [`scripts/cmm-usdt-buy-ust1.sh`](../../scripts/cmm-usdt-buy-ust1.sh)  
Defaults: [`scripts/lib/cmm-usdt-buy-ust1-defaults.sh`](../../scripts/lib/cmm-usdt-buy-ust1-defaults.sh)  
Router hook / min-receive: [`scripts/lib/ust1-clunc-buyback-math.py`](../../scripts/lib/ust1-clunc-buyback-math.py) (`self-test`)

## Why CMM needs an authorized spender

CMM `terra16j5u6…` is the **ustr-cmm treasury contract** (code `11728`), not an
EOA, and its `Config.governance` is the DEX 2-of-3. CMM cannot simply sign a
swap. USDT leaves CMM one of two ways:

| Path | Contract message | Timelock | Who calls |
|------|------------------|----------|-----------|
| Instant pull | `InstantWithdrawCw20` | none | the token's **registered spender** only |
| Treasury withdraw | `ProposeWithdraw` → `ExecuteWithdraw` | **7 days** (604800s) | governance (2-of-3) |

A 7-day timelock cannot drive a 20-minute drip, so this script uses the instant
path. That requires governance to register the admin hot wallet as the **USDT
spender** on the treasury (no timelock, same posture as `SetDenomWrapper`).

```
set_cw20_spender { token: <USDT>, spender: <admin>, limit_24h: <raw 24h cap> }
```

The script only ever pulls **the USDT needed for the current tick**, and the
treasury enforces a tumbling **24h pull limit** on top of the treasury balance.

> Registering the ops hot wallet as a spender gives that key the ability to pull
> up to the configured 24h limit from CMM. The treasury comment recommends
> audited contracts for `InstantWithdrawCw20`; there is currently **no audited
> USDT→UST1 spender contract**, so this script registers the hot wallet and
> bounds it with `CMM_USDT_BUY_PULL_LIMIT_24H_USDT` (default 3600). If a
> dedicated audited spender is deployed later, point `CMM_USDT_BUY_SPENDER` at
> it. Prefer the 7-day `ExecuteWithdraw` path if you do not want a hot key to be
> able to pull from CMM at all.

## Roles

| Role | Default |
|------|---------|
| CMM treasury | `terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2` |
| DEX 2-of-3 governance | `terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7` (`multisig_2of3` + `multisig1`/`multisig2`) |
| Admin / spender hot wallet | `cl8y2_admin` → `terra1xsecn4snv94ezcez0z3vq8an9j4h4kxxcydp8l` |
| Router | `terra1e7s0h9ftxakwca5gxspyt4haeuaqxds6swr08ul3tsepq7el924sprrsrw` |
| USDT CW20 (18 dp) | `terra1z0xe7t5ymmltg4vju8tghkq0pewy4et548ta23nlu9zxtl950uyqkv8mv4` |
| UST1 CW20 (6 dp) | `terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72` |

`USDT-Bridge-Debt` is **not** the underlying USDT and cannot be spent; this
script only touches the bridged Tether USD CW20 above.

## Stop condition

Stop when the **CMM USDT CW20 balance** is `<= CMM_USDT_BUY_FLOOR_USDT`
(default **500**). CMM UST1 is **not** counted, no USD price is used, and the
state file is telemetry only. Each tick pulls
`min(CMM_USDT_BUY_USDT_PER_TICK, CMM_USDT - floor)` so the last tick lands on the
floor exactly.

The default per-tick budget is **50 USDT**. USDT and UST1 trade near $1, so a
tick buys roughly **50 UST1**; the exact output follows the solver quote and the
slippage floor.

> **Current on-chain reading (2026-10-17, columbus-5):** after the first tick CMM
> holds about **3,896.64 USDT**, above the 500 floor, so the drip will buy UST1
> until the balance reaches 500 (about 3,396.64 USDT, ~68 ticks at 50/tick, ~23h).
> `./scripts/cmm-usdt-buy-ust1.sh --status` prints the live number.

## One command

```bash
python3 scripts/lib/ust1-clunc-buyback-math.py self-test
./scripts/cmm-usdt-buy-ust1.sh --status                       # read-only
DRY_RUN=1 CMM_USDT_BUY_FLOOR_USDT=0 ./scripts/cmm-usdt-buy-ust1.sh   # one preview tick
```

Register the spender once (DEX 2-of-3 signs `set_cw20_spender`):

```bash
read -rs TERRAD_HOST_KEYRING_PASS; export TERRAD_HOST_KEYRING_PASS
CMM_USDT_BUY_YES=1 ./scripts/cmm-usdt-buy-ust1.sh --setup-spender
```

Then run the live 20-minute loop until the floor is reached:

```bash
read -rs TERRAD_HOST_KEYRING_PASS; export TERRAD_HOST_KEYRING_PASS
CMM_USDT_BUY_YES=1 ./scripts/cmm-usdt-buy-ust1.sh
# single tick only:
CMM_USDT_BUY_LOOP=0 CMM_USDT_BUY_YES=1 ./scripts/cmm-usdt-buy-ust1.sh
```

Never commit `TERRAD_HOST_KEYRING_PASS`. File-keyring keys (`multisig1`,
`multisig2`, `multisig_2of3`, `cl8y2_admin`) must share that passphrase.

## What success looks like

1. Each tick reads CMM USDT and stops at/below the floor.
2. If the admin needs USDT, it calls `InstantWithdrawCw20` for exactly the tick
   amount (clamped to the remaining 24h pull limit). Raw USDT comes from CMM to
   the admin hot wallet.
3. Swap uses indexer `GET /api/v1/route/solve/best` (`trader`/`sender` = admin)
   and executes those `router_operations` with a **0.25%** slippage tolerance:
   `minimum_receive` is 99.75% of the quote, rounded down to atomic units. Book
   hops also get per-hop `min_return` from pair `hybrid_simulation` (#334).
4. The received UST1 is CW20-transferred to CMM and the script verifies CMM's
   UST1 balance increased and the admin's returned to its pre-swap value.
5. After each tick it re-reads CMM USDT and **exits 0** once the floor is met.
   Failed ticks sleep 20 minutes and retry; they do not abort the loop.

A swap failure after a pull leaves USDT on the admin hot wallet. The next tick
consumes that balance before pulling again; re-run the script (or transfer the
USDT back to CMM) to recover.

## Overrides

| Env | Purpose |
|-----|---------|
| `CMM_USDT_BUY_USDT_PER_TICK` | USDT spent per tick (default `50`) |
| `CMM_USDT_BUY_FLOOR_USDT` | Stop when CMM USDT ≤ this (default `500`) |
| `CMM_USDT_BUY_INTERVAL_SEC` | Wait after each tick (default `1200`) |
| `CMM_USDT_BUY_LOOP` | Live default `1`; dry-run default `0`; `0` for one tick / cron |
| `CMM_USDT_BUY_SOURCE` | `cmm` (pull each tick, default) or `wallet` (only admin USDT) |
| `CMM_USDT_BUY_DEST` | `cmm` (default), `hold`, or `burn` |
| `CMM_USDT_BUY_SLIPPAGE_PERCENT` | Tolerance below the quote (default `0.25`) |
| `CMM_USDT_BUY_MAX_SPREAD` | Router `max_spread` (default `0.20`) |
| `CMM_USDT_BUY_PULL_LIMIT_24H_USDT` | `--setup-spender` 24h pull cap (default `3600`) |
| `CMM_USDT_BUY_SPENDER` | Registered spender address (default = admin) |
| `CMM_USDT_BUY_YES=1` | Skip interactive confirm |
| `CMM_USDT_BUY_STATE_FILE` | Telemetry JSON (`$HOME/.cl8y-dex/cmm-usdt-buy-ust1-state.json`) |
| `CMM_USDT_BUY_QUOTE_RETRIES` | Bounded indexer quote attempts on transient 5xx/429 (default `4`) |
| `CMM_USDT_BUY_QUOTE_RETRY_DELAY_SEC` | Initial quote retry wait, +3s each attempt (default `3`) |
| `TERRAD_HOST_GAS_ADJUSTMENT` | Default **1.6** for this script (hybrid hops) |
| `TERRAD_HOST_NODE` | RPC override (default `rpc.terra-classic.hexxagon.io:443`) |
| `TERRAD_HOST_TX_RETRIES` / `TERRAD_HOST_TX_RETRY_DELAY_SEC` | Mint/msig prepare retries (defaults `6` / `4`) |

## Endpoint identity preflight (#1392)

The shared [chain-attestation policy](chain-attestation.md) runs before reads,
dry-run planning and every transaction attempt. Failed or stale LCD/RPC identity
cannot authorize a tick.

## Oracle safety

The campaign has no USD price input: the per-tick budget and the stop floor are
**raw USDT amounts**, and the swap output floor comes from the on-chain solver
quote plus the slippage cap. No manual oracle override is accepted for live
transactions.
