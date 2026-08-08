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

1. **Read `reason` first** — query JSON and wasm attr both use **snake_case** (`dust_filled`, …). Rust variant names stay PascalCase (`DustFilled`). Do not classify parks from `expires_at` or `force_expired` alone.
2. **`dust_filled` / `DustFilled` ⇒ near-complete fill** — remaining is sub-`LIMIT_ORDER_DUST_FLUSH_THRESHOLD` (10). Book a fill; do not treat as zero-fill expiry.
3. **`force_expired=true` means “not a TTL expiry”** — historical / inverted naming. Kept for back-compat; prefer `reason`.
4. **Claim path is reason-agnostic** — still owner-only, pause-gated (**L6**), refunds `remaining` only. No funds-path change in #504.
5. **Legacy rows** — omitted `reason` → unknown; use poll-diff workaround only if you accept races.
6. **Do not** store lifetime `filled_amount` on the pair for this issue; use indexer fill history when volume is needed.
7. **Do not** rename `ClaimExpiredLimitOrder` / storage key in opportunistic cleanups — naming is historical; `reason` closes the footgun.

## Verification

```bash
# Preferred one-shot (crate names: dex-common / cl8y-dex-pair / cl8y-dex-tests)
make verify-issue-504
# Optional after make deploy-local (LCD schema smoke):
VERIFY504_LCD=1 make verify-issue-504

# Or targeted:
cd smartcontracts && cargo test -p dex-common expired_limit_park_reason
cd smartcontracts && cargo test -p cl8y-dex-pair --lib match_bid_dust_remainder
cd smartcontracts && cargo test -p cl8y-dex-tests --lib match_dust_flush
cd smartcontracts && cargo test -p cl8y-dex-tests --lib blacklisted_maker_resting
make test-contracts
```

Crosslinks: [`docs/limit-orders.md` § Park reason](../docs/limit-orders.md#expired-limit-park-reason-gitlab-504), [`docs/integrators.md`](../docs/integrators.md#expired-limit-park-reason-gitlab-504), invariant **L22**, [`AGENTS_ORDER_STATUS_QUERY.md`](./AGENTS_ORDER_STATUS_QUERY.md) (`ParkedRefund` ≠ why), [`AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md`](./AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md), [`AGENTS_LOCALNET_TRADING_SWARM.md`](./AGENTS_LOCALNET_TRADING_SWARM.md).

## Follow-ups (not required to close on-chain #504)

- Indexer: parse wasm `reason` / split lifecycle beyond single `parked_expired`.
- dApp: replace `remaining_escrow < 10` “Claim dust” heuristic with indexer/LCD `reason` when available (post-#504 consumer work).
