# Stable Swap layout (#1498)

Apply when changing Swap quote presentation, disclosure, status or minimum-return
rows. Issue [#1498](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1498).
Run `make verify-issue-1498` and the [checklist](../docs/qa/issue-1498/README.md).
The mobile density, inline status and route-fitting follow-up is [#1561](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1561); use its [short-viewport checklist](../docs/qa/issue-1561/README.md).
This supersedes older separate route-card, Trade details/Signing details and
above-input pricing banner instructions. Preserve all independent signing gates.

| ID | Invariant |
| --- | --- |
| SL1498-1 | The card order is header, Pay/Receive, one Slippage/Fee/actual-token row, one reserved status area, one DETAILS disclosure, CTA. Automatic quote states cannot insert anything above inputs. Tax guidance stays in DETAILS so changing route-funding wording cannot resize Pay/Receive. Explicit Settings changes may move them. |
| SL1498-2 | Initial/new-intent unresolved pricing uses an accessible indeterminate replacement with the same summary height. Same-intent unexpired refresh retains the exact economic snapshot; an advisory unavailable metric never becomes 0%. |
| SL1498-3 | A compact minimum status space replaces the former six-em scrolling envelope (#1561). None, pricing waits and concise current errors preserve input geometry. Long/zoomed text grows into ordinary page scrolling without nested scrollbars, clipping or overlap. Current blocking errors persist as one inline decorative cross plus accessible subdued red text, with no dismiss control/card chrome. None is never an execution permission. |
| SL1498-4 | DETAILS stays mounted through quote transitions and preserves native keyboard/focus/open state. There is exactly one network fee estimate, in LUNC, separate from trading output and principal. |
| SL1498-5 | Show the checked positive quoted minimum for disconnected as well as connected supported current quotes. Missing/expired/new-intent/unsupported quotes retain an unavailable minimum row. No fabricated minimum or anonymous signing permission. |
| SL1498-6 | Route tokens/icons follow actual reviewed operations and funding, including native wrapper boundaries. Registry symbols are display labels, never identities. The full bounded route fits the summary without horizontal scrolling (#1561); mobile uses the full second summary line. Each node remains present with flexible symbol truncation. The one DETAILS disclosure provides full readable labels and canonical identities, including repeated symbols and wrapper boundaries. |
| SL1498-7 | Single details rows align labels/values and disclose costs, protection, deadline, action, exact pay/receive, genuine contracts and chain from the same original quote/submit plan. Never sum fee bps or include gas in order slippage. |
| SL1498-8 | Remove Quote only and routine wrapper implementation/wallet fee override prose. Preserve required exchange-deposit/trust/risk acknowledgements and truthful actionable fee coverage failures. |
| SL1498-9 | UI simplification never relaxes expiry/actor/amount/generation/operations binding, order >5 OR total >10 ordinary consent and independent total >30/>=99 controls ([TR1560](AGENTS_TRADING_RISK.md)), gross taxes, positive post-hook/final floors, launch/freeze/blacklist gates, offline fee/Max parity or no-resend. |
| SL1498-10 | Verify actual production-build geometry and keyboard behavior on desktop/phone, loading/errors/recovery/expiry and 200% text zoom. At 375×667, normal text and collapsed disclosures, both connected and disconnected CTAs must start entirely above the measured nav/visual viewport edge at scrollY=0, including balance controls and a concise error. Mobile removes outer card/visible heading and redundant visible labels while preserving accessible names and 44px controls. Desktop chrome/density remains unchanged. Long/zoomed/open-details states may scroll normally above the measured nav clearance. Repeat stable source and reset only owned affected fixtures. Record failures and skipped scope; source/DOM evidence does not attest funded/device settlement. |

Implementations: [summary](../frontend-dapp/src/components/swap/SwapQuoteSummary.tsx),
[Swap](../frontend-dapp/src/pages/SwapPage.tsx),
[summary details](../frontend-dapp/src/components/swap/SwapPreSubmitSummary.tsx).
Coordinate [TE1497](AGENTS_TRADE_ENTRY.md), [QL1499](AGENTS_QUOTE_LIFECYCLE.md)
and [order metrics](AGENTS_ORDER_SLIPPAGE.md). Preserve [OB1480](AGENTS_OFFLINE_SWAP_BUDGET.md),
[RC1483](AGENTS_ROUTE_COST_ATTRIBUTION.md), [RD1489](AGENTS_ROUTE_DISCOVERY.md)
and [TQ1473](AGENTS_TRADING_QUOTE_OUTCOMES.md).

Mobile shell follow-up: hide the content footer below 768px and center Swap in the spare space between the sticky header and measured bottom navigation. Use normal-flow auto margins, never a fixed-height clipping/absolute positioning scheme; long content and zoom still scroll. Desktop footer/geometry remain unchanged. Environment/product/legal information stays accessible in mobile More. See [#1561 shell follow-up](../docs/qa/issues-1560-1561/MOBILE-CENTERING.md).
