# Agent skill: Observe query extreme-ratio skip ([#1231](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1231))

## When to use

You touch pair **`oracle_observe_single`**, **`QueryMsg::Observe`**, **`oracle_update`**, TWAP docs, or `Decimal::from_ratio` / `checked_from_ratio` on pair reserves.

Closed [#465](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/465) already skips execute-path observation when `reserve_b/reserve_a` (or the reciprocal) cannot be a CosmWasm `Decimal`. This skill covers the **query** leftover: forward extrapolation for `seconds_ago == 0` (or any target after the last stored observation) used to call panicking `Decimal::from_ratio`.

Open [#1224](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1224) is **`price_times_dt` overflow on execute**. Do **not** fold that into this ticket. Observe already maps `price_times_dt` / `checked_add` to `ContractError::Oracle`.

## Invariants (O1231-1–O1231-6)

| Id | Rule |
|----|------|
| **O1231-1** | Observe forward-extrapolation (`seconds_ago == 0` or target after `latest_obs.timestamp`) uses `Decimal::checked_from_ratio` with the same operand order as `oracle_update`: `price_a = reserve_b/reserve_a`, `price_b = reserve_a/reserve_b`. Either `Err` **skips extrapolate** and returns last stored cumulatives (same as the zero-reserve branch). **No VM panic.** Do not clamp to `Decimal::MAX`. |
| **O1231-2** | `oracle_update` stays `#465`: `checked_from_ratio` + `return Ok(())` skip. Observe hardening must not reintroduce execute panics. |
| **O1231-3** | Representable balanced reserves with `dt > 0` still advance cumulatives. Do not freeze every Observe at `latest_obs`. |
| **O1231-4** | Historical interpolation (two stored observations, no spot ratio) is unchanged. Extreme live `RESERVES` must not change in-window TWAP points. |
| **O1231-5** | Existing `#465` tests `extreme_ratio_degrades_gracefully_instead_of_panicking` and `normal_ratio_still_records_observation` stay green. |
| **O1231-6** | Success-path Observe JSON is still `{ price_a_cumulatives, price_b_cumulatives }`. Skip is a **missed sample**, not a new error field. Query is read-only: no `RESERVES` / `OBSERVATIONS` writes. |

## Why skip (not error, not clamp)

A pair that hit the `#465` skip still has live, lopsided `RESERVES`. Indexers poll `Observe { seconds_ago: [0] }` every block. A panic looks like an LCD outage while swap/LP execute may still succeed. Returning last cumulatives matches execute “missed sample” and the zero-reserve sister branch. Integrators should treat a **flat cumulative at `seconds_ago = 0` while wall-clock advanced** as “no new sample,” not as last-price held through the gap (the integral for the gap is zero).

Typed `ContractError::Oracle` is allowed by the issue if docs say so; this implementation uses skip for query liveness.

## Forbidden

- Reopen or retarget `#465` ACs.
- Implement `#1224` (`price_times_dt` execute brick) “while here.”
- Reintroduce `Decimal::from_ratio` on execute “for consistency.”
- Change observation cardinality, ring index, `seconds_ago` semantics, or JSON field names for representable ratios.
- Gate on `#464` k-widening / `MAX_PAIR_ASSET_DECIMALS`.
- Floats. `unwrap` / `expect` on ratio construction.
- Wasm migrate / code-id bump (ops after merge).

## Tests

```bash
cd smartcontracts && cargo test -p cl8y-dex-pair oracle_overflow -- --nocapture
cd smartcontracts && cargo test -p cl8y-dex-pair oracle_observe -- --nocapture
cd smartcontracts && cargo test -p cl8y-dex-tests oracle -- --test-threads=1
make verify-issue-1231
make test-contracts
```

## Canonical docs

- Issue: [git.cl8y.com #1231](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1231)
- Product: [`docs/twap-oracle.md`](../docs/twap-oracle.md)
- Audit matrix: [`docs/contracts-security-audit.md`](../docs/contracts-security-audit.md) **O1231-1–O1231-6**
- Math: [`smartcontracts/packages/dex-common/src/oracle.rs`](../smartcontracts/packages/dex-common/src/oracle.rs)
- Pair: [`smartcontracts/contracts/pair/src/contract.rs`](../smartcontracts/contracts/pair/src/contract.rs) `oracle_update` / `oracle_observe_single`
