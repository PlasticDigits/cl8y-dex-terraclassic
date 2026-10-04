# Agent playbook: Compact / Full number display

Audience: third-party agents changing numeric dApp chrome, retail Settings, or signing summaries.

Issue: [#1259](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1259). Canonical invariants: [**Q1259-1–Q1259-10**](../docs/frontend.md#number-display-preference). Acceptance / abuse evidence: [`docs/qa/issue-1259/README.md`](../docs/qa/issue-1259/README.md). Regression command: `make verify-issue-1259`.

Canonical implementation: [`numberDisplayPreference.ts`](../frontend-dapp/src/utils/numberDisplayPreference.ts) (store / subscriptions), [`formatAmount.ts`](../frontend-dapp/src/utils/formatAmount.ts) (dispatcher / exact grouping), [`SwapPage.tsx`](../frontend-dapp/src/pages/SwapPage.tsx) (retail Settings), and [`SwapPreSubmitSummary.tsx`](../frontend-dapp/src/components/swap/SwapPreSubmitSummary.tsx) (signing digits).

## Implementation boundaries

- Keep Compact the default, including first visit and unavailable / hostile storage. Only `compact` and `full` are legal values. Persist `cl8y-dex-number-display-mode` in localStorage; ignore URL format / locale / compact keys.
- Use the shared `formatAmount.ts` dispatcher after resolving token decimals and humanizing raw amounts. Full is en-US grouped human digits without suffixes. Do not fork per-page formatters or coerce large whole integers through Number.
- Keep `formatPairPrice` independent of mode. Never pass a raw ratio or unresolved token raw integer to `formatNum`.
- Use exact grouping for signing amounts: `fromRawAmount` + `formatHumanAmountGrouped` in Swap, `formatTokenAmountGrouped` in limit / pool summaries. Confirmations are always full grouped, even when Settings says Compact.
- Keep input drafts and all quote / simulate / execute raw values unchanged. Do not call a display formatter to populate a controlled amount input, wallet amount, min received, or request payload.
- Preserve raw integer length guards, decimals 0–18, unresolved `—`, and `PAIR_STATS_VOLUME_DISPLAY_CAP = 24`.
- Subscribe mounted and memoized displays to preference changes. Do not refetch React Query data or rewrite Swap search / Share URLs to refresh rendering.
- Keep the Number display group in the existing retail Settings card. Chips use pressed state, 44px hit targets, and button-press sound; fixture preview is a text node. No new card, `dangerouslySetInnerHTML`, `eval`, or locale library.
- Preserve Expert / slippage protections, hybrid Advanced behavior, blacklist / pause / freeze, and production gem hiding.

## Verify and review

1. Run `make verify-issue-1259`, `make test-frontend`, and appropriate lint / build checks; fix failures and rerun the affected checks.
2. Run `VERIFY_ISSUE_1259_BROWSER=1 make verify-issue-1259` for the actual Settings chips, Full preview, reload, plain Pay input, and no format URL keys. Mocked LCD reads prove rendering only; chain execution needs its own evidence.
3. Follow the linked AC1–AC10 and A1–A8 checklist. Record results by criterion with commands / observations and commit identifier.
4. Agents are explicitly permitted to skip items they cannot complete and still mark overall **PASS with SKIPs** when executed runnable checks pass. For each SKIP record the reason and unverified scope. Unavailable wallet, chain, screenshot, or production checks create no hold. Never mark an unexecuted check PASS or turn a reproducible failure into a SKIP.

## Related playbooks

- [`AGENTS_FRONTEND_CHROME_NESTING.md`](./AGENTS_FRONTEND_CHROME_NESTING.md): **C653**, single Settings card.
- [`AGENTS_FRONTEND_CHARTS_PAIR_STATS.md`](./AGENTS_FRONTEND_CHARTS_PAIR_STATS.md): pair decimals / hostile volume / caps.
- [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md): USD headlines use the shared dispatcher.
- [`AGENTS_FRONTEND_SHARE_LINK.md`](./AGENTS_FRONTEND_SHARE_LINK.md): canonical Swap keys only; preference is not shareable.
- [`AGENTS_INDEXER_PAIR_PRICE_USD.md`](./AGENTS_INDEXER_PAIR_PRICE_USD.md): [#522](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/522) human ratio and USD scale exceptions.
