# Agent skill: pay with any token (GitLab #595)

Use when adding or changing a **paid protocol feature** that invoices a canonical CW20 (community SKUs / manager settings [#592](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/592) / [#593](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/593), market-making subscription [#597](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/597)).

**Do not** fork Swap quote/execute, TokenSearchSelect+router construction, or a second “swap then pay” path into the feature page. Import [`quotePayInvoice`](../frontend-dapp/src/utils/payInvoice.ts) + [`PayWithAnyToken`](../frontend-dapp/src/components/payments/PayWithAnyToken.tsx).

Issue **#595 is implemented** (v1 multi-msg settlement). An on-chain `invoice-payer` adapter and indexer exact-out `amount_out=` are **follow-ups**, not v1 blockers. `#593` Create Token / Manage Save consume this card ([`AGENTS_FRONTEND_CREATE_TOKEN.md`](./AGENTS_FRONTEND_CREATE_TOKEN.md)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#595**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/595) | Shared DEX-routed invoice module |
| [`docs/frontend.md` § Pay with any token](../docs/frontend.md#pay-with-any-token) | Product invariants **I595-1–I595-14** |
| [`payInvoice.ts`](../frontend-dapp/src/utils/payInvoice.ts) | `quotePayInvoice` / `buildPayInvoiceMsgs` |
| [`PayWithAnyToken.tsx`](../frontend-dapp/src/components/payments/PayWithAnyToken.tsx) | Presentational card |
| [`AGENTS_HYBRID_QUOTING.md`](./AGENTS_HYBRID_QUOTING.md) | GET `/route/solve` + reverse sim (no second solver) |
| [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md) | Wrap + N-hop + invoice Send envelope |
| [`AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md`](./AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md) | Gem-bridge reject (**P562-6**) |
| [`AGENTS_FRONTEND_DEFAULT_SLIPPAGE.md`](./AGENTS_FRONTEND_DEFAULT_SLIPPAGE.md) | 5% default + 0.5/1/5 chips |

## Invariants **I595-1–I595-14**

1. **I595-1 — canonical invoice.** The on-chain price stays the invoice CW20 amount (SKU / batch = 50 UST1). This module never re-prices the SKU in LUNC.
2. **I595-2 — atomic settlement.** Swap + pay is **one** Cosmos tx (`executeTerraContractMulti`). A test that counts two broadcasts is a fail.
3. **I595-3 — payee gets exactly the invoice.** Swap `to` = user (never payee). Last msg `Send` amount = invoice. Excess invoice token stays with the user. Overpay-as-tip is not default.
4. **I595-4 — no pair math upgrade.** Use existing router + pairs. No FoT balance-delta (**H-01**).
5. **I595-5 — reuse the solver.** Path = indexer `GET /route/solve` (probe `amount_in`) + router `ReverseSimulateSwapOperations` + forward sim `out ≥ invoice`. Do not add a second best-execution engine. Exact-out indexer endpoint is optional later.
6. **I595-6 — invoice is a floor.** `minimum_receive` ≥ invoice. Do **not** set `minimum_receive = invoice × (1 − slippage)`. `max_in` = reverse-sim offer × (1 + slippage) (ceiling).
7. **I595-7 — retail route policy.** Frozen hops stay excluded by the solver. Production gem-bridge hops → disable + **No route** (**P562-6**). Unroutable → short reason, not an LCD dump.
8. **I595-8 — same-asset short path.** Pay token = invoice token → one `Send`, no router. Default the picker to the invoice token when the wallet holds enough. Insufficient invoice balance does **not** silently switch token.
9. **I595-9 — native wrap.** LUNC/USTC wraps then routes. Unwrap is not needed when the invoice is UST1 CW20. Wrap env unset → **Wrap config unavailable**.
10. **I595-10 — payee from config.** `Invoice.payee` / `hookMsg` come from the caller (launcher env). Never from URL/query. `resolveInvoicePayee` ignores search strings.
11. **I595-11 — no unlimited allowance.** `Send` amount = quoted `payRaw` / `cw20SendAmount` only. Builder rejects `increase_allowance`.
12. **I595-12 — copy / CTA.** “You pay ~X TOKEN (incl. DEX swap) → 50 UST1 fee”. Primary CTA is **Pay** / **Enable**, not Swap. One **Route** row.
13. **I595-13 — gas.** Invoice hook Send uses `PAY_INVOICE_SEND_GAS_LIMIT`. Combined wrap+≥2hop+invoice is larger than wrap+2hop swap-only (`wrap_plus_2hop_plus_invoice_send`). `make verify-issue-475` must stay green.
14. **I595-14 — launcher stays dumb.** #592 still accepts only the invoice CW20 `Send`. Routing lives here, not inside the launcher.

## Rules of thumb

- `#593` (and tests) **import** this card. Do not assemble router ops on Create Token or Manage Save.
- Pass `trader` (connected wallet) so fee-tier quotes match execute ([#245](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/245)).
- Tax-token pay: pass `taxPreviewExtraDebit` so max-in matches extra-debit. Fail closed if the wallet cannot cover it.
- Failed `max_spread` / LCD errors go through `humanizeUserFacingError` ([#134](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/134)).
- v1 fallback (no adapter wasm) is **equivalent settlement**: swap to user, then exact invoice `Send`. Document any adapter as a later crate — do not block SKU UX on it.
- Next paid feature after SKUs: [#597](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/597) prepaid MM subscription. Same module, different hook.

## Verify

```bash
make verify-issue-595
make verify-issue-475
# Optional: make test-frontend
```

Manual LocalTerra (needs #592 launcher): enable a SKU paying in cLUNC; launcher UST1 +50; user cLUNC decreased by `payRaw`; no leftover on router. Repeat settings **batch** Save (always 50 UST1) with a non-UST1 pay token.
