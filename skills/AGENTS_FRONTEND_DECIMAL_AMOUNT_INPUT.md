# Agent playbook: decimal amount inputs (frontend)

Use when adding or changing **human token amount** fields (Swap pay amount, hybrid **book leg**, trade market override, slippage %, etc.) so invalid keystrokes never reach **`BigInt()`** or **`toRawAmount`** ([GitLab **#169**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/169)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Decimal amount inputs](../docs/frontend.md#decimal-amount-inputs) | Product invariants |
| [`frontend-dapp/src/utils/decimalAmountInput.ts`](../frontend-dapp/src/utils/decimalAmountInput.ts) | **`isDecimalAmountDraft`**, **`tryParseBigInt`** |
| [`frontend-dapp/src/utils/decimalAmountInput.test.ts`](../frontend-dapp/src/utils/decimalAmountInput.test.ts) | Regression strings (`^`, `,`, `\\`) |
| Swap **You Pay** (reference UX) | [`SwapPage.tsx`](../frontend-dapp/src/pages/SwapPage.tsx) — same draft regex |
| Hybrid book leg | [`SwapPage.tsx`](../frontend-dapp/src/pages/SwapPage.tsx) Settings panel; [`TradeMarketOrderPanel.tsx`](../frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx) |
| Pool/book split (defense in depth) | [`swapDisclosure.ts`](../frontend-dapp/src/utils/swapDisclosure.ts) — returns `null` on invalid draft |

## Rules of thumb

1. **Reject at `onChange`** — only call `setState` when **`isDecimalAmountDraft(v)`** is true; do not surface raw JS conversion errors for typos.
2. **`.` only** — locale commas are **not** normalized in retail fields (W10-C4); users must use `.` as the decimal separator.
3. **Downstream math** — prefer **`tryParseBigInt`** on raw integer strings; for human → micro, use **`toRawAmount`** only after the draft passes **`isDecimalAmountDraft`**.
4. **New fields** — set `type="text"` and `inputMode="decimal"` (not uncontrolled `type="number"` with browser locale quirks).

## Related

- **Humanized errors (if something still throws):** [`AGENTS_FRONTEND_USER_ERRORS.md`](./AGENTS_FRONTEND_USER_ERRORS.md) ([GitLab **#145**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/145)).
- **Hybrid disclosure / book leg semantics:** [docs/limit-orders.md § Swap page](../docs/limit-orders.md#swap-ui-hybrid-vs-pool-only-estimates), [`AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md`](./AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md).
