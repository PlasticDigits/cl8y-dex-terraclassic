# Agent playbook: expired-limit park `reason` (GitLab #504)

Use when integrating bots, indexers, or dApp copy against **`EXPIRED_LIMIT_CLAIMS`** / LCD **`expired_limit_refund`** / wasm **`limit_order_expired_parked`**. A parked refund row is **not** “order expired unfilled.”

## Canonical docs / code

| Area | Location |
|------|----------|
| Integrator table + invariants | [`docs/limit-orders.md` § Park reason discriminator](../docs/limit-orders.md#expired-limit-park-reason-gitlab-504) |
| Integrator short form | [`docs/integrators.md` § Parked refund reason](../docs/integrators.md#expired-limit-park-reason-gitlab-504) |
| Security invariant **L22** | [`docs/contracts-security-audit.md`](../docs/contracts-security-audit.md) |
| Shared enum | [`dex-common::pair::ExpiredLimitParkReason`](../smartcontracts/packages/dex-common/src/pair.rs) |
| Storage + park write | [`pair/src/state.rs`](../smartcontracts/contracts/pair/src/state.rs), [`orderbook.rs`](../smartcontracts/contracts/pair/src/orderbook.rs) `park_limit_order_for_clean` |
| Clean keeper | [`limit_book_clean.rs`](../smartcontracts/contracts/pair/src/limit_book_clean.rs) |
| Maker claim UX (lifecycle) | [`AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md`](./AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md) |
| Swarm / bot claim polling | [`AGENTS_LOCALNET_TRADING_SWARM.md`](./AGENTS_LOCALNET_TRADING_SWARM.md) |

## Rules of thumb

1. **Read `reason` first** — query field (PascalCase JSON) or wasm attr (snake_case). Do not classify parks from `expires_at` or `force_expired` alone.
2. **`DustFilled` ⇒ near-complete fill** — remaining is sub-`LIMIT_ORDER_DUST_FLUSH_THRESHOLD` (10). Book a fill; do not treat as zero-fill expiry.
3. **`force_expired=true` means “not a TTL expiry”** — historical / inverted naming. Kept for back-compat; prefer `reason`.
4. **Claim path is reason-agnostic** — still owner-only, pause-gated (**L6**), refunds `remaining` only. No funds-path change in #504.
5. **Legacy rows** — omitted `reason` → unknown; use poll-diff workaround only if you accept races.
6. **Do not** store lifetime `filled_amount` on the pair for this issue; use indexer fill history when volume is needed.
7. **Do not** rename `ClaimExpiredLimitOrder` / storage key in opportunistic cleanups — naming is historical; `reason` closes the footgun.

## Verification

```bash
# Unit + integration coverage for all four reasons + legacy decode
cd smartcontracts && cargo test -p dex-common expired_limit_park_reason
cd smartcontracts && cargo test -p cl8y-pair --lib match_bid_dust_remainder
cd smartcontracts && cargo test -p cl8y-tests --test limit_order_tests match_dust_flush
cd smartcontracts && cargo test -p cl8y-tests --test blacklist_tests blacklisted_maker_resting
# Or full suite:
make test-contracts
```

## Follow-ups (not required to close on-chain #504)

- Indexer: parse wasm `reason` / split lifecycle beyond single `parked_expired`.
- dApp: replace `remaining_escrow < 10` “Claim dust” heuristic with indexer/LCD `reason` when available.
