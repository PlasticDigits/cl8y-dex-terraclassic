# Post-swap hooks (burn, tax, LP burn)

These contracts implement `HookMsg::AfterSwap` callbacks registered on pairs via factory governance.

**Registration playbook:** [`docs/runbooks/hook-registration.md`](../../../docs/runbooks/hook-registration.md).

## Fee settlement (invariant I-02, GitLab #377)

**Tax-hook** and **burn-hook** fees are deducted from the swap **ask token** during pair settlement — not from pre-funded hook treasury balances.

1. Pair queries each registered hook's `GetConfig` at swap time ([`dex-common::hook_settlement`](../../packages/dex-common/src/hook_settlement.rs)).
2. Pair emits CW20 transfers: tax → `recipient`; burn → burn-hook contract.
3. Net return to the swap receiver is `total_return − sum(hook fees)`.
4. `AfterSwap` on tax-hook records attrs (`settled_by_pair`); burn-hook burns tokens the pair forwarded in the same transaction.

LP-burn hook is unchanged: governance/treasury **pre-funds LP tokens** on the hook; burns are proportional to swap output on the configured `target_pair` only.

## `AfterSwap.commission_amount` (invariant L7)

`AfterSwap.commission_amount` is the **total protocol commission** for the swap, denominated in the **ask asset** (same units as `return_asset`):

- **Pool leg:** constant-product fee sent to `treasury` from the AMM leg.
- **Book leg (hybrid only):** sum of **taker** fees from each `limit_order_fill` in the same transaction (maker half was charged at placement).

`spread_amount` remains **pool leg only** (TerraSwap-style). `return_asset.amount` is **book net + pool net** to the receiver (gross output before tax/burn hook deductions).

`HybridSimulation` / `HybridReverseSimulation` expose the same total in `commission_amount` so off-chain quotes match hook payloads.

**Breaking change (GitLab [#196](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/196)):** before this fix, hooks received pool-leg commission only on hybrid txs. Integrators that assumed the old semantics must update fee accounting.

Swap **wasm event** attrs expose `return_amount` (gross), `net_return_amount`, and `hook_fee_amount` when fee hooks are registered. Pool-only `commission_amount` remains for Terraport baseline compatibility; hybrid txs also emit `book_commission_amount` when the book leg runs. Per-fill book fees remain on `limit_order_fill` events.

## LP-burn caller validation (invariant H-03)

- `AfterSwap.pair` must equal `info.sender`.
- Caller must respond to pair `Pair {}` query; `liquidity_token` must match hook config.
- `UpdateAllowedPairs` rejects non-pair addresses.

Canonical references: **[`docs/integrators.md`](../../../docs/integrators.md)** (§ Hybrid swaps and post-swap hooks), **[`docs/contracts-security-audit.md`](../../../docs/contracts-security-audit.md)** (L7, H2, I2).
