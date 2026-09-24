# Pair TWAP Uint256 rollout — Columbus-5 (#1324)

This runbook covers the operational rollout of the pair TWAP widening merged
in [#1323](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1323) for
[#1322](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1322). It is
the live-chain companion to [`docs/twap-oracle.md`](../twap-oracle.md) and
[`skills/AGENTS_TWAP_CUMULATIVE_U256.md`](../../skills/AGENTS_TWAP_CUMULATIVE_U256.md).
The ALPHA chart rollout is cross-linked in [ADR 0012](../adr/0012-alpha-pair-price-candles.md)
and [#1315](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1315).

## Invariants

1. Pair `price × dt` and both cumulative values are `Uint256`. Existing
   observation decimal strings load as the same integer by zero-extension.
   Never wrap, saturate, rewrite, or reset `OBSERVATIONS`; there is no
   cumulative setter. The pair `MigrateMsg` is `{}`. The contract migration
   may backfill its other documented legacy keys, but #1232's
   `DISCOUNT_REGISTRY` / `ORACLE_STATE` backfill remains separate.
2. Migrate every factory-listed pair to pair cw2 **1.18.0**, checking the
   enumerated count against `GetPairCount`. Migrate the saturated UST1/USTR
   pair first. Check every pair's admin before a governance migrate.
3. Keep the factory on code ID **11629**, cw2 **1.10.0**. Store pair wasm,
   update only `config.pair_code_id`, then migrate pairs. Do not store or
   migrate factory wasm.
4. The script records the current raw `OBSERVATIONS[index]` bytes before and
   after each individual migrate and stops if that ring entry changes. Its
   final query requires `Observe([0, 60])` to return both offsets on both
   sides. The script never sends a swap, provide, or withdraw.
5. Columbus-5 reserve movement is a separate acceptance step: confirm one
   authorized small swap, provide, or withdraw succeeds and compare the
   post-event reserves. Do not use the probe as a substitute for that check.
6. The indexer and dApp are separate Coolify applications. Confirm both
   finished deployments descend from `c17e71d3`, verify
   `GET /api/v1/evidence/daily?day=YYYY-MM-DD`, and verify an ALPHA candle
   exposes `usd_leg` / `subject_*` for the chart bundle.

## Read-only verification

```bash
make verify-issue-1324
UPGRADE1324_PROBE_ONLY=1 ./scripts/upgrade-1324-pair-twap.sh
```

The Make target runs local shell guards only. `UPGRADE1324_PROBE_ONLY=1` is a
read-only Columbus-5 acceptance probe: it verifies the factory pointer, every
listed pair's cw2/code ID, stored observations, and UST1/USTR Observe. It does
not load a keyring or sign.

The default `scripts/upgrade-1324-pair-twap.sh` path can store wasm and request
governance migrations. A live run requires the human operator, the configured
store identity, and the 2-of-3 governance ceremony. If that work is outstanding,
assign it as a [cl8y-pm inbox card](../../AGENTS.md#operator-work); third-party
agents must not try to bypass the signer or keyring boundary.

For Coolify inspection, use the OpenBao workstation wrapper with only the two
read-only service variables needed, for example:

```bash
bao-exec --only COOLIFY_TOKEN,COOLIFY_URL -- python3 <read-only-coolify-check.py>
```

Do not print, persist, or pass credential values as command arguments. The
wrapper's `--list` prints names only; it does not reveal values. See the
[OpenBao workstation guide](https://git.cl8y.com/PlasticDigits/openbao-ops/src/branch/main/docs/workstation.md).

## Recorded acceptance for #1324 (2026-09-24)

- All **21** listed pairs reported code ID **11676** and cw2 **1.18.0**;
  factory code ID **11629** / cw2 **1.10.0** remained unchanged.
- The saturated UST1/USTR pair reported 360 stored observations and
  `Observe([0, 60])` returned two values per side, including a cumulative
  greater than `u128::MAX`.
- A successful UST1→USTR swap at block **30535152** (6.866855 UST1 in,
  986.999440591662043534 USTR out) followed an indexed swap at block
  **30535127**. `/gt/events` showed different post-event reserves at the two
  heights.
- Production indexer and frontend Coolify records were healthy/finished on
  commits descended from `c17e71d3`. The daily evidence route returned 200;
  UST1/ALPHA candles returned `usd_leg=asset_1` and subject OHLC fields.
- `make verify-issue-1324` passed all three shell guards, and the read-only
  live probe passed for all listed pairs.

## Related references

- [#1324](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1324) — this operations rollout.
- [#1224](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1224), [#1322](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1322), and merged [PR #1323](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1323) — cumulative-width fix.
- [#465](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/465) / [#1231](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1231) — ratio skip remains unchanged.
- [#1232](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1232) — missing-key backfill is separate.
- [#1315](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1315) and [ADR 0012](../adr/0012-alpha-pair-price-candles.md) — ALPHA candle and dApp behavior.

## Systems checked

- Pair migration and observation storage: [`contract.rs`](../../smartcontracts/contracts/pair/src/contract.rs), [`state.rs`](../../smartcontracts/contracts/pair/src/state.rs), and [`upgrade-1324-pair-twap.sh`](../../scripts/upgrade-1324-pair-twap.sh).
- Indexer production wire: [`evidence.rs`](../../indexer/src/api/evidence.rs), [`pairs.rs`](../../indexer/src/api/pairs.rs), and [candle migration `20260922120000`](../../indexer/migrations/20260922120000_candle_usd_leg_1315.up.sql).
- dApp chart mapping: [`AGENTS_FRONTEND_PRICE_CHART.md`](../../skills/AGENTS_FRONTEND_PRICE_CHART.md) and [ADR 0012](../adr/0012-alpha-pair-price-candles.md).
