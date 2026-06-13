# Post-swap hooks (burn, tax, LP burn)

These contracts implement `HookMsg::AfterSwap` callbacks registered on pairs via factory governance.

## `AfterSwap.commission_amount` (invariant L7)

`AfterSwap.commission_amount` is the **total protocol commission** for the swap, denominated in the **ask asset** (same units as `return_asset`):

- **Pool leg:** constant-product fee sent to `treasury` from the AMM leg.
- **Book leg (hybrid only):** sum of **taker** fees from each `limit_order_fill` in the same transaction (maker half was charged at placement).

`spread_amount` remains **pool leg only** (TerraSwap-style). `return_asset.amount` is **book net + pool net** to the receiver.

`HybridSimulation` / `HybridReverseSimulation` expose the same total in `commission_amount` so off-chain quotes match hook payloads.

**Breaking change (GitLab [#196](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/196)):** before this fix, hooks received pool-leg commission only on hybrid txs. Integrators that assumed the old semantics must update fee accounting.

Swap **wasm event** attrs still expose pool-only `commission_amount` for Terraport baseline compatibility; hybrid txs also emit `book_commission_amount` when the book leg runs. Per-fill book fees remain on `limit_order_fill` events.

Canonical references: **[`docs/integrators.md`](../../../docs/integrators.md)** (§ Hybrid swaps and post-swap hooks), **[`docs/contracts-security-audit.md`](../../../docs/contracts-security-audit.md)** (L7, H2, I-02), **[`docs/runbooks/hook-registration.md`](../../../docs/runbooks/hook-registration.md)**.

## Tax / burn fees (invariant I-02)

Tax and burn hooks expose `OutputFee` (see `dex_common::hook::HookFeeQueryMsg`). During swap settlement the **pair** forwards the fee from ask-token output to the hook (burn) or tax recipient **before** the receiver transfer, then invokes `AfterSwap`. Hooks must not subsidize fees from pre-funded treasuries on normal swaps. LP-burn hooks remain treasury-funded (LP tokens) but require `pair == info.sender` (H-03).
