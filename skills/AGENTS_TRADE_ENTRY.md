# Advanced Trade entry (#1497)

Apply to Trade entry/default selection, query links, navigation and history.
[#1497](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1497)
supersedes the bare Trade redirect permission in TQ1473-8/P534-5/P562-4.
Run `make verify-issue-1497` and use the
[acceptance checklist](../docs/qa/issue-1497/README.md).

| Invariant | Required behavior |
| --- | --- |
| TE1497-1 | Bare `/trade` and `/trade/` stay advanced: replace to the first catalog-ranked retail genuine factory address, or retain a usable selector when discovery is empty. Never auto-redirect bare Trade to Swap. |
| TE1497-2 | Native `uusd` → configured exact CL8Y remains the Swap default. Native and wrapped assets remain distinct; symbols cannot select execution identities. |
| TE1497-3 | Valid path identity and pending/invalid/unknown notices take precedence. Unique factory queries preserve bounded amount/side prefills. Unresolved or ambiguous queries cannot pick an unrelated default market. |
| TE1497-4 | Only validated explicit native funding intent with no genuine factory match may use existing same-origin Swap compatibility. Ambiguous factory queries remain Trade-local. No native order-book fabrication or new limit support. |
| TE1497-5 | Catalog refresh/reordering cannot override a deliberate pair. Replace automatic canonical navigation; user selection remains normal navigation. Plain active Trade nav clicks retain the selected workspace without adding history; modified/new-tab clicks retain the genuine `/trade` href. History has no automatic extra entry or loop. |
| TE1497-6 | Navigation/prefill never connects, submits, signs or broadcasts. All quote expiry, amount/actor/generation binding, funding, protection and offline signing controls remain enforced. |
| TE1497-7 | Keep production gem filtering and genuine address membership; opaque listing IDs and malformed destinations are ineligible. No external redirect destinations or quote-triggered chain polling. |
| TE1497-8 | First-visit acknowledgement, outage and lazy-load recovery preserve Trade-local intent. Existing same-origin one-shot chunk recovery remains bounded. |
| TE1497-9 | Assert final path together with chart/book/Market/Limit controls in real-router and browser tests. Heading/skeleton alone does not establish advanced readiness. |
| TE1497-10 | Record exact revision/UTC, first/retest and served asset hashes separately. Unavailable browser/wallet scopes stay unchecked; executed failures stay failures. |

Implementation: [TradePage](../frontend-dapp/src/pages/TradePage.tsx),
[catalog ranking](../frontend-dapp/src/utils/pairCatalogRank.ts),
[query identity/prefill](../frontend-dapp/src/utils/tradeQueryResolve.ts),
[route guards](../frontend-dapp/src/utils/tradePairRoute.ts),
[entry browser receipt](../scripts/qa/browser-trade-entry-1497.cjs).
Preserve [TQ1473](AGENTS_TRADING_QUOTE_OUTCOMES.md) and
[Q713](AGENTS_FRONTEND_SWAP_URL_SYNC.md).
