# Stable Swap layout (#1498)

Apply when changing Swap quote presentation, disclosure, status or minimum-return
rows. Issue [#1498](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1498).
Run `make verify-issue-1498` and the [checklist](../docs/qa/issue-1498/README.md).
This supersedes older separate route-card, Trade details/Signing details and
above-input pricing banner instructions. Preserve all independent signing gates.

| ID | Invariant |
| --- | --- |
| SL1498-1 | The card order is header, Pay/Receive, one Slippage/Fee/actual-token row, one reserved status area, one DETAILS disclosure, CTA. Automatic quote states cannot insert anything above inputs. Tax guidance stays in DETAILS so changing route-funding wording cannot resize Pay/Receive. Explicit Settings changes may move them. |
| SL1498-2 | Initial/new-intent unresolved pricing uses an accessible indeterminate replacement with the same summary height. Same-intent unexpired refresh retains the exact economic snapshot; an advisory unavailable metric never becomes 0%. |
| SL1498-3 | The bounded status envelope uses the same height for None, pricing waits and current errors. It supports scrolling/zoom without clipping text or overlapping controls. None is never an execution permission. |
| SL1498-4 | DETAILS stays mounted through quote transitions and preserves native keyboard/focus/open state. There is exactly one network fee estimate, in LUNC, separate from trading output and principal. |
| SL1498-5 | Show the checked positive quoted minimum for disconnected as well as connected supported current quotes. Missing/expired/new-intent/unsupported quotes retain an unavailable minimum row. No fabricated minimum or anonymous signing permission. |
| SL1498-6 | Route tokens/icons follow actual reviewed operations and funding, including native wrapper boundaries. Registry symbols are display labels, never identities. Long labels and full bounded paths remain accessible within the summary's horizontal scroll region. |
| SL1498-7 | Single details rows align labels/values and disclose costs, protection, deadline, action, exact pay/receive, genuine contracts and chain from the same original quote/submit plan. Never sum fee bps or include gas in order slippage. |
| SL1498-8 | Remove Quote only and routine wrapper implementation/wallet fee override prose. Preserve required exchange-deposit/trust/risk acknowledgements and truthful actionable fee coverage failures. |
| SL1498-9 | UI simplification never relaxes expiry/actor/amount/generation/operations binding, 5/30/99 reference controls, gross taxes, positive post-hook/final floors, launch/freeze/blacklist gates, offline fee/Max parity or no-resend. |
| SL1498-10 | Verify actual production-build geometry and keyboard behavior on desktop/phone, loading/errors/recovery/expiry and 200% text zoom. Repeat stable source and reset only owned affected fixtures. Record failures and skipped scope; source/DOM evidence does not attest funded/device settlement. |

Implementations: [summary](../frontend-dapp/src/components/swap/SwapQuoteSummary.tsx),
[Swap](../frontend-dapp/src/pages/SwapPage.tsx),
[summary details](../frontend-dapp/src/components/swap/SwapPreSubmitSummary.tsx).
Coordinate [TE1497](AGENTS_TRADE_ENTRY.md), [QL1499](AGENTS_QUOTE_LIFECYCLE.md)
and [order metrics](AGENTS_ORDER_SLIPPAGE.md). Preserve [OB1480](AGENTS_OFFLINE_SWAP_BUDGET.md),
[RC1483](AGENTS_ROUTE_COST_ATTRIBUTION.md), [RD1489](AGENTS_ROUTE_DISCOVERY.md)
and [TQ1473](AGENTS_TRADING_QUOTE_OUTCOMES.md).
