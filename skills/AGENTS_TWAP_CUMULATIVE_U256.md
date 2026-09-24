# Agent skill: TWAP cumulative Uint256 ([#1224](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1224) / [#1322](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1322))

## When to use

You touch `price_times_dt`, `compute_twap_price`, `Observation.price_*_cumulative`, `oracle_update`, `oracle_observe_single`, Charts `computeTwapPriceDecimalString`, or TWAP docs.

[#465](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/465) and [#1231](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1231) skip an unrepresentable reserve ratio. This skill is the **width** of the integral, not that skip. Playbook for the skip: [`AGENTS_TWAP_OBSERVE_RATIO.md`](AGENTS_TWAP_OBSERVE_RATIO.md).

Founder direction: **256-bit**, **zero-extend** a stored `u128` into `u256`. Do not replace this with wrapping modulo `2^128`, saturating at `Uint128::MAX`, or skip-and-freeze.

## Invariants (O1322-1–O1322-8)

| Id | Rule |
|----|------|
| **O1322-1** | Cumulatives and `price_times_dt` products are `Uint256`. A stored JSON decimal string that fit in `u128` loads as the same integer (zero-extend). `migrate` does not rewrite or reset `OBSERVATIONS`. |
| **O1322-2** | `Decimal` atomics (`u128`) × `dt` (`u64`) is the full product (always ≤ 192 bits). Never `wrapping_mul`, never the low 128 bits, never an execute `Err` that aborts swap / provide / withdraw for that product. |
| **O1322-3** | `oracle_update` and Observe forward-extrapolation `checked_add` the full delta. A sum above `u128::MAX` is that `Uint256`. Not wrapping, not saturating, not a missed sample. |
| **O1322-4** | Observe JSON keys stay `price_a_cumulatives` / `price_b_cumulatives` (decimal strings). One wide point must not fail the rest of `seconds_ago` when the sum fits in `Uint256`. |
| **O1322-5** | `compute_twap_price` and `computeTwapPriceDecimalString` subtract the wide integers. `end < start` is corruption (error / `null`), not a wrap. A window that crosses `2^128` returns `(end − start) / elapsed`. |
| **O1322-6** | #465 / #1231 stay: unrepresentable ratio skips; no `Decimal::from_ratio` on those paths; no clamp to `Decimal::MAX`. Pre-op reserves. |
| **O1322-7** | Same-block (`block_time <= last_ts`), zero reserves, and the first zero-cumulative seed do not add a delta. Observe does not write `RESERVES` or `OBSERVATIONS`. |
| **O1322-8** | The only execute writer of `price_*_cumulative` is `oracle_update` (plus the first zero seed inside it). Limit place, cancel, claim, and reprice do not call it. Migrate cannot set a cumulative. |

Pair cw2 for this wasm is **1.18.0**. Columbus-5 deployment and live acceptance
were verified under [#1324](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1324)
on 2026-09-24. This skill remains the contract/math reference; migration
procedure and current-state checks live in the
[`#1324 runbook`](../docs/runbooks/pair-twap-uint256-columbus5.md) and
[`AGENTS_PAIR_TWAP_MIGRATION.md`](AGENTS_PAIR_TWAP_MIGRATION.md). Production
state can drift; re-run its read-only probe before making a current-state
claim. Do not infer that this skill authorizes a store, migrate, or reserve move.

Pair schema compatibility is separate from cumulative-width compatibility: if an older pair lacks `ORACLE_STATE`, pair migrate must initialize the empty-ring default while preserving any existing state and `OBSERVATIONS`. The current migration still lacks that backfill ([#1232](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1232)); see [`AGENTS_PAIR_STORAGE_MIGRATION.md`](./AGENTS_PAIR_STORAGE_MIGRATION.md). Do not treat the #1324 live migration as proof of the #1232 code invariant.

## Why zero-extend (not wrap)

A live cumulative near `u128::MAX` made `checked_add` fail on both Observe and the three reserve-mutating executes. The accumulator is monotonic, so the error does not heal. Wrapping would keep the public integer in `u128` but makes `end < start` mean “crossed the modulus” and rejects a single delta that itself does not fit in `u128` ([#1224](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1224)). Widening the product and the sum fixes both. Existing decimal strings stay the same digits.

A Charts-length window’s integral still fits in `u128` at current accrual. The stored counter may not. Clients must parse the strings as arbitrary-precision integers (`bigint` / `Uint256`), not as a language `u128`.

## Forbidden

- `wrapping_add` / `wrapping_sub` / `wrapping_mul` on this oracle.
- Saturate at `Uint128::MAX` or skip forever once the counter is high.
- Rescale by token decimals inside the pair. Human scale stays in the dApp.
- A migrate or admin message that writes `price_*_cumulative`.
- Clamping an unrepresentable spot to `Decimal::MAX`, or bringing back panicking `from_ratio`.
- Feeding TWAP into swap `belief_price`.
- Treating `cumEnd < cumStart` in Charts as a modulo wrap.

## Tests

```bash
cd smartcontracts && cargo test -p dex-common --lib oracle -- --nocapture
cd smartcontracts && cargo test -p cl8y-dex-pair oracle_overflow -- --nocapture
cd smartcontracts && cargo test -p cl8y-dex-pair oracle_observe -- --nocapture
cd smartcontracts && cargo test -p cl8y-dex-pair oracle_u256 -- --nocapture
cd smartcontracts && cargo test -p cl8y-dex-tests oracle -- --test-threads=1
make verify-issue-1224
make verify-issue-1322
make verify-issue-1231
```

## Canonical docs

- Issues: [#1224](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1224), [#1322](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1322)
- Product: [`docs/twap-oracle.md`](../docs/twap-oracle.md)
- Audit matrix: [`docs/contracts-security-audit.md`](../docs/contracts-security-audit.md) **O1322-1–O1322-8**
- Ratio skip (do not weaken): [`AGENTS_TWAP_OBSERVE_RATIO.md`](AGENTS_TWAP_OBSERVE_RATIO.md) **O1231**
- Math: [`smartcontracts/packages/dex-common/src/oracle.rs`](../smartcontracts/packages/dex-common/src/oracle.rs)
- Pair: [`smartcontracts/contracts/pair/src/contract.rs`](../smartcontracts/contracts/pair/src/contract.rs) `oracle_update` / `oracle_observe_single`
- Pair migrate storage defaults: [`AGENTS_PAIR_STORAGE_MIGRATION.md`](./AGENTS_PAIR_STORAGE_MIGRATION.md) (**#1232**, open)
- Charts: [`frontend-dapp/src/services/terraclassic/oracle.ts`](../frontend-dapp/src/services/terraclassic/oracle.ts)
