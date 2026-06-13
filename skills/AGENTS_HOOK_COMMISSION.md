# Agent skill: post-swap hook commission (L7)

**Audience:** third-party agents integrating burn/tax/LP-burn hooks or parsing `AfterSwap` payloads.

## Invariant L7 (GitLab #196)

| Field | Semantics |
|-------|-----------|
| `AfterSwap.commission_amount` | **Total** protocol commission in the **ask asset**: pool treasury fee + sum of book **taker** fees for the tx. |
| `AfterSwap.spread_amount` | **Pool leg only** (unchanged). |
| `AfterSwap.return_asset.amount` | Book net + pool net to receiver. |

`HybridSimulation` / `HybridReverseSimulation` `commission_amount` uses the **same total** so quotes match hook callbacks.

## Breaking change

Before GitLab #196, hybrid swaps passed **pool-leg-only** `commission_amount` to hooks. Fee hooks that accrued on that field under-counted on hybrid txs. Update any logic that assumed the old semantics.

## Swap wasm events vs hooks

- **Hooks:** total `commission_amount` (see above).
- **Swap tx attrs (Terraport baseline):** `commission_amount` = pool leg only; `book_commission_amount` present when book leg > 0.
- **Per fill:** `limit_order_fill` events still carry per-maker `commission_amount`.

## Tests to run

```bash
cd smartcontracts
cargo test -p cl8y-dex-tests hybrid_hook_commission pool_only_hook_commission -- --nocapture
```

## Tax / burn output fees (I-02, GitLab #377)

Tax and burn hooks charge from **swap output** during pair settlement (pair queries `OutputFee`, forwards ask tokens, then calls `AfterSwap`). Receiver net = `return_asset.amount − sum(OutputFee)`. Do not rely on pre-funded hook balances for normal fee collection.

## Canonical docs

- [docs/integrators.md](../docs/integrators.md) — § Hybrid swaps and post-swap hooks
- [docs/contracts-security-audit.md](../docs/contracts-security-audit.md) — row L7, H2, I-02
- [docs/runbooks/hook-registration.md](../docs/runbooks/hook-registration.md)
- [smartcontracts/contracts/hooks/README.md](../smartcontracts/contracts/hooks/README.md)
- [docs/limit-orders.md](../docs/limit-orders.md) — § Post-swap hooks and hybrid

## Related skills

- [AGENTS_HYBRID_QUOTING.md](./AGENTS_HYBRID_QUOTING.md) — L8 simulation parity
- [AGENTS_TESTING_MULTIHOP_HYBRID.md](./AGENTS_TESTING_MULTIHOP_HYBRID.md) — multihop hybrid regression
