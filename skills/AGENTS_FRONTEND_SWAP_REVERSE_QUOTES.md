# Agent playbook: Swap receive sizing across solver and mapper routes (#1256 / #1376)

Use when changing Swap's editable You Receive field, reverse quote routing, wrap fee/tax gross-ups, or reverse submit alignment.

**Issue:** [#1256](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1256). **Invariants:** [docs/frontend.md](../docs/frontend.md#swap-reverse-quotes), **R1256-1–R1256-6**. **Review checklist:** [issue-1256 QA](../docs/qa/issue-1256/README.md).

## Quote and execution protocol

1. Keep You Receive an `<input>` when a token is selected. Validate legal human amounts with the existing 24-character cap and `isPositiveDecimalAmount`. Preserve `SIM_QUOTE_DEBOUNCE_MS`, `exactField=output`, canonical URL replacement, and clear/flip behavior (**Q713-1**, **Q713-4**, **Q711-6**).
2. Dispatch mapper-only wrap/unwrap to shared fee/tax inverse math. All supported CW20 route receive sizing (including native-mapped CW20 hops and indexer-only paths) uses the explicit server `amount_out` mode and fresh wallet forward validation (#1376). Preserve legacy pair/router reverse for invoice and existing integrations. Never send native denoms into pair reverse or binary-search the public offer-in endpoint from the browser.
3. Size wrap pay with shared `wrapGrossForNetCw20` and mapper wrap fee. Invert unwrap receive through post-fee then post-burn-tax, including LCD tax caps. Use integer amounts, preserve **W8**/**W9**, and keep `routerMinReceiveBase` post-fee pre-tax (**R3**). Invoice reuses the same wrap helper.
4. Community-tax receive is net. `grossBuyTaxForTargetNet` covers the requested net receive; `executeAmountOut` and `min_return` remain pre-tax. Forward verification and hop-tax accounting remain authoritative; extra-debit counts once.
5. Submit the reverse offer raw through existing forward simulation, min-receive, hop floors, and `assertSubmitQuotePayRawAligned` / hybrid checks. Never submit the typed ask as offer. Max/fractions switch to input. Keep quote-only, balance, impact/Expert, pause, freeze, and blacklist checks.

These steps implement **R1256-1–R1256-6**; the complete invariant table lives in [docs/frontend.md](../docs/frontend.md#swap-reverse-quotes).

## Code and tests

| Path | Purpose |
|------|---------|
| `frontend-dapp/src/pages/SwapPage.tsx` | Editable receive, route-class reverse query, forward execution alignment |
| `frontend-dapp/src/services/terraclassic/router.ts` | `reverseSimulateNativeSwap`, `reverseSimulateMultiHopSwap`; unwrap min-receive boundaries |
| `frontend-dapp/src/services/terraclassic/wrapMapper.ts` | Shared `wrapGrossForNetCw20`, mapper split-fee compatibility |
| `frontend-dapp/src/utils/nativeTransferTax.ts` | `grossUlunaForTargetNet`; burn-tax rate/cap inverse |
| `frontend-dapp/src/utils/communityTaxNetOut.ts` | `grossBuyTaxForTargetNet` |
| `frontend-dapp/src/utils/payInvoice.ts` | Shared wrap gross-up compatibility re-export |
| `frontend-dapp/src/pages/SwapPage.queryParams.test.tsx` | Receive input, URL/submit/guard regressions |
| `frontend-dapp/src/services/terraclassic/router.reverse.test.ts` | Native/direct wrap/client reverse service fixtures |
| `frontend-dapp/e2e/swap-receive-1256.spec.ts` | Browser receive input and URL behavior with mocked LCD; no chain needed |

## Agent verification

Follow [the acceptance and review checklist](../docs/qa/issue-1256/README.md); retain test output and exact commit under review. Run:

```bash
make verify-issue-1256
make verify-issue-713
make verify-issue-678
make verify-issue-512
python3 scripts/check_chrome_nesting.py
```

If a check cannot be completed, record **SKIP**, the precise missing capability/prerequisite, and the remaining verification scope. Per the task's verification policy, the overall review may be **PASS with explicit skips** when completed checks pass and no executed check fails. A skipped check is never a passed check. Do not override application safety guards or manufacture chain/browser evidence. Rerun after any corrective code changes.

## Related

- [Swap URL sync](./AGENTS_FRONTEND_SWAP_URL_SYNC.md) — **Q713-4**, debounce, clear/flip, URL replacement
- [Swap query params](./AGENTS_FRONTEND_SWAP_QUERY_PARAMS.md) — **Q711-6**, legal/hostile amounts, safety settings
- [Wrap/unwrap burn tax](./AGENTS_WRAP_UNWRAP_BURN_TAX.md) — **W8**/**W9**, fee then tax, caps
- [Mapper split fees](./AGENTS_WRAP_MAPPER_SPLIT_FEES.md) — live fee config, transitional fallback
- [Router minimum receive](./AGENTS_ROUTER_MINIMUM_RECEIVE.md) — **R3**, forward hop floors
- [Swap acquire guidance](./AGENTS_FRONTEND_SWAP_ACQUIRE_GUIDANCE.md) — #678 quote-only, balance, impact guards

Current receive-sizing extension: [#1376 agent protocol](AGENTS_SWAP_EXACT_OUTPUT.md), with explicit bounded solver sizing and finite disconnected errors. This supersedes earlier no-indexer-exact-out restrictions.
