# Issue #1497: Trade entry verification

Issue: [#1497](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1497).
Read [TE1497-1–TE1497-10](../../../skills/AGENTS_TRADE_ENTRY.md) and
[frontend default contract](../../frontend.md#trading-quote-outcomes-1473).

Run `make verify-issue-1497`, `make verify-issue-713` and
`make verify-issue-1473`. Keep historical quote-matrix failures visible.
Focused entry verification requires neither a wallet nor a funded transaction.
Unavailable acceptance is explicitly unchecked and does not block a reviewable
fix. Executed failures remain failures. Never claim full acceptance from source
tests alone; close the issue only when its body and comments all pass.

| Done | Criterion | Evidence to collect |
| --- | --- | --- |
| [x] | AC1 direct/bookmark/reload/click | Final `/trade` or genuine `/trade/<pair>` plus advanced chart/book/Market/Limit controls; no automatic Swap redirect. |
| [ ] | AC2 all navigation surfaces | Desktop primary, compact More and phone bottom Trade navigation, truthful empty discovery selector. |
| [ ] | AC3 identity/prefill | Both factory directions, bounded amount/side, explicit native compatibility, pending/invalid/unknown/ambiguous/disconnected routes. |
| [ ] | AC4 stable selection/history | Late/shuffled/refreshed discovery, user pair changes, rapid navigation, Back/Forward, repeated Trade clicks; no history spam. |
| [ ] | AC5 Swap default | Native `uusd` → configured CL8Y remains exact; validated native queries preserve intent; navigation causes no signing/mutation. |
| [ ] | AC6 gates/recovery | First-visit risk acknowledgement; disconnected/connected wallet; API outage/recovery; offline/stale lazy chunk one-shot recovery. |
| [x] | AC7 contract/deployment | Updated playbooks/tests/checklists; exact source revision, first/retest results, UTC and served entry/Trade asset hashes; desktop/phone deployed entry and deep links. |

Source regressions live in
[TradePage.test.tsx](../../../frontend-dapp/src/pages/TradePage.test.tsx),
[route guards](../../../frontend-dapp/src/utils/__tests__/tradePairRoute.test.ts),
[query resolution](../../../frontend-dapp/src/utils/__tests__/tradeQueryResolve.test.ts),
and the existing navigation/chunk suites. The controlled quote browser helper
now checks Trade factory controls separately from the Swap default.
The real-router shell navigation regression verifies repeated active Trade
clicks preserve the deliberate pair and Back/Forward history, while the link
keeps `/trade` for modifier/new-tab navigation.

Read-only deployed browser receipt (default public origin `https://dex.cl8y.com`):

```bash
bash scripts/with-node.sh -- node scripts/qa/browser-trade-entry-1497.cjs \
  --output /tmp/trade-entry-1497-public.json
```

The helper covers disconnected direct entry, reload, repeated Trade navigation,
Back/Forward and canonical deep links across desktop/compact/phone widths. It captures only
public served asset identities/hashes and bounded findings. It does not claim
connected-wallet, transaction, full outage/offline or rapid pair-switch acceptance.
Run the affected existing browser suites separately when owned fixtures exist.

Merged implementation: [PR #1501](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1501).
Exact deployment and subsequent live receipts: [joint release results](../trading-1497-1500/RESULTS.md#merged-release-and-deployment).

AC1/AC7 completion is backed by [deployed first/retest](../trading-1497-1500/RESULTS.md#deployed-trade-firstretest), 30/30 checks. Other rows retain incomplete browser or wallet scope even where source regressions pass.
