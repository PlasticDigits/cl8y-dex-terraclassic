# Agent playbook: layout while scrolling (#1316)

Use when changing portaled pickers, the sticky app shell, the Trade ticket
scrollport/footer, or chart refresh behavior. This is one frontend regression
surface: [issue #1316](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1316).

## Invariants

- **S1316-1 Picker tracking.** Read the current anchor rect after document,
  captured ancestor, or visual-viewport scroll. Keep the existing capture listener
  and synchronous first-open calculation. Menus track visible anchors with the
  configured gap; clamp displaced anchors to the usable viewport band, clear of
  the sticky header and mobile/in-app bottom reserves. Keep logo padding stable.
- **S1316-2 Shell.** `.app-top-sticky` stays opaque after page scroll in both
  themes, with reachable header controls. Trade heading clears it by at least
  approximately 16px at scroll-top.
- **S1316-3 Money CTA.** After ticket-body scroll, the desktop money CTA remains
  within 8px of the ticket-card bottom. Measure control overlap only after clipping
  controls to the visible scrollport. The footer remains a flex sibling of the
  scrollport, never sticky, fixed, or portaled (**T527-1**, **T527-5**).
- **S1316-4 Chart.** At the same interval, a 30s candle refresh preserves a user's
  non-default visible logical range. An unchanged historical prefix is not
  rewritten. Keep live-bar `series.update()` and the existing mount/interval/
  indicator `fitContent()` boundaries.
- **S1316-5 Scope.** Do not change quote, matching, tax, contracts, indexer,
  router, or wallet-session behavior. Keep scrollbar gutter and one chrome layer.

## Agent review and verification

1. Run `make verify-issue-1316` from the reviewed checkout with Node 24 and
   installed frontend dependencies/Playwright Chromium. It starts a dedicated
   Vite server (default port 5136; override `PLAYWRIGHT_WEB_PORT` for conflicts),
   uses five workers, and mocks all relevant market/LCD transport. No LocalTerra
   or wallet signing is needed. The chart test uses the actual 30s refresh timer.
2. Review the geometry assertions in
   [portal-scroll-invariants.spec.ts](../frontend-dapp/e2e/portal-scroll-invariants.spec.ts)
   and [layout-scroll-invariants.spec.ts](../frontend-dapp/e2e/layout-scroll-invariants.spec.ts).
   Require actual anchor/scroll movement, usable menu options, both 1280×720 and
   375×667 viewports, opaque header hit targets, and visible-control clipping.
3. Review [chart-scroll-refresh.spec.ts](../frontend-dapp/e2e/chart-scroll-refresh.spec.ts):
   it instruments the test's Vite loader, uses a real chart and user drag, and
   checks refresh requests, logical range and series writes. No production hooks.
4. Run `make lint-frontend` and `make build-frontend`; repeat the verifier after
   any fix. Inspect `git diff --check` and the diff's scope. Keep related closed
   issues closed.
5. Record each check as PASS, FAIL, or SKIP with command, reason and evidence.
   Per the implementation request, an agent may skip checks it cannot execute
   and report **overall PASS (available checks; skips listed)** when every check
   actually executed passes. A skipped item is never checked off as completed;
   a measured failure must be fixed or reported as FAIL. Device/in-app and live
   deployment checks are outside this frontend-only acceptance run.

## Cross-links

- [Frontend layout documentation](../docs/frontend.md#scroll-invariants-after-scroll-1316)
- [QA record and acceptance checklist](../docs/qa/issue-1316/README.md)
- [Portal viewport playbook](AGENTS_FRONTEND_PORTAL_LISTBOX_VIEWPORT.md)
- [Portal layout stability](AGENTS_FRONTEND_PORTAL_LISTBOX_CLS.md)
- [Trade CTA dock](AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md)
- [Chart canvas](AGENTS_FRONTEND_PRICE_CHART.md)
