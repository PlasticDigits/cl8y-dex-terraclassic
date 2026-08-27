# Agent playbook: Swap / Trade unfunded pay + acquire guidance (GitLab #678)

Use when changing Swap or Trade **insufficient-balance** UX, UST1 acquire deep-links, `/ust1` query prefill, or retail high-impact (size) copy.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [`docs/swap-max-spread-ux.md`](../docs/swap-max-spread-ux.md) | 5 / 30 / 99 slippage gates stay; acquire does not weaken them |
| [`docs/runbooks/ust1-window-ui.md`](../docs/runbooks/ust1-window-ui.md) | Window caps; Swap → `/ust1` acquire (**U9**) |
| [`swapPayAcquireGuidance.ts`](../frontend-dapp/src/utils/swapPayAcquireGuidance.ts) | Discriminated helper (A678) |
| [`ust1WindowMath.ts`](../frontend-dapp/src/utils/ust1WindowMath.ts) | Inverse deposit `vfdusdInForTargetUst1` |
| [`ust1AcquirePrefill.ts`](../frontend-dapp/src/utils/ust1AcquirePrefill.ts) | Safe `?direction=&amount=` parse + clamp |
| [`SwapPayAcquireGuidanceBanner.tsx`](../frontend-dapp/src/components/swap/SwapPayAcquireGuidanceBanner.tsx) | One sentence + Guide / Reduce |
| [`useSwapPayAcquireGuidance.ts`](../frontend-dapp/src/hooks/useSwapPayAcquireGuidance.ts) | Swap + Trade consume the same helper |
| U1 copy | [`ust1SecondaryMarket.ts`](../frontend-dapp/src/utils/ust1SecondaryMarket.ts) `assertSecondaryMarketCopy` |

## Invariants (A678-1–A678-11)

1. **A678-1 Disconnected quote-only.** Positive pay + settled quote while disconnected: CTA stays **Connect Wallet**; receive is marked `swap-quote-only` / `trade-market-quote-only`; **Min received** is not shown as a promise; no confirm-again; no invented balance.
2. **A678-2 Unfunded blocks submit.** Connected + pay raw > LCD (`getTokenBalance` / escrow) balance: submit stays disabled; one-sentence shortfall; no executable Swap/Market.
3. **A678-3 UST1 under cap.** Pay identity is **`UST1_TOKEN_ADDRESS`** (not the ticker). When window is healthy and shortfall ≤ min(per-tx, rolling remaining): Guide → `/ust1?direction=deposit&amount={human vFDUSD}` from inverse `effective_swap` math.
4. **A678-4 Over window.** If shortfall exceeds remaining capacity: copy says the window cannot mint that size; Guide still opens `/ust1`; never suggest a vFDUSD amount that would fail on-chain.
5. **A678-5 Fail closed.** Window env off, query fail, paused, stale, or rate `0`: generic “You don’t have enough {symbol}.” — no invented vFDUSD number.
6. **A678-6 U1.** Copy never markets AMM Swap/Trade as mint/redeem. Guide for UST1 is **`/ust1`**, never “Swap vFDUSD → UST1”. Run `assertSecondaryMarketCopy` on new strings.
7. **A678-7 Funded high impact.** Expected slippage **> 5%** and the trade is **funded**: retail size warning + existing confirm-again. **30% / 99% / Expert Mode** gates are unchanged. Unfunded must not prefer high-impact over insufficient.
8. **A678-8 Shared helper.** Trade market uses `evaluateSwapPayAcquireGuidance` / `useSwapPayAcquireGuidance` — do not fork copy.
9. **A678-9 #489.** One sentence + Guide / Reduce control. No always-on oracle essay on the Swap card. Guide is a `Link`, not a paragraph stack.
10. **A678-10 Prefill safety.** `/ust1` reads only `direction` + `amount`. Ignore scientific / negative / non-decimal / hostile strings. Clamp legal amounts to remaining capacity **before** apply. Never auto-submit.
11. **A678-11 Same-origin Guide.** Href allowlist is `/ust1` (+ safe query) or `/wrap`. No `javascript:`, no third-party URLs, no `window.location` from token symbols. No HTML interpolation of amounts.

## Rules of thumb

- Do **not** change pair/router `max_spread`, indexer `route/solve`, or window contract limits.
- Do **not** auto-switch pay to vFDUSD or auto-wrap.
- In-app / WebView without an injected wallet stays on **Connect Wallet**.
- Optional Reduce sets pay to a conservative fraction of typed amount **capped by spendable** — never above balance.
- Wrap-native shortfall (cLUNC / cUSTC when wrap env is on) Guides to `/wrap`. Generic CW20 has no Guide.

## Quick commands

```bash
make verify-issue-678
make verify-issue-506
make lint-frontend
```

## Related

- [`AGENTS_UST1_WINDOW_UI.md`](./AGENTS_UST1_WINDOW_UI.md) — `/ust1` mint/redeem
- [`AGENTS_UST1_SECONDARY_AMM.md`](./AGENTS_UST1_SECONDARY_AMM.md) — **U1** AMM ≠ mint
- [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) — one-sentence errors
- [`AGENTS_FRONTEND_DEFAULT_SLIPPAGE.md`](./AGENTS_FRONTEND_DEFAULT_SLIPPAGE.md) — 5% protection / confirm-again
- Expert Mode: [#293](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/293)
