# Issue #1316: layout after scroll

Issue: [#1316](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1316).
Contract: [frontend scroll invariants](../../frontend.md#scroll-invariants-after-scroll-1316).
Agent review instructions: [S1316-1–S1316-5 playbook](../../../skills/AGENTS_FRONTEND_SCROLL_INVARIANTS.md).

## Investigation and fix

The existing capture-phase listener correctly tracks document and inner ancestor
scroll. The failure was geometry: after desktop document scroll, the sticky
header ended at **62px**, the trigger ended at **51.5px**, and the menu began at
**59.5px**, covering 2.5px of header. After the fix, the menu begins at **70px**, maintaining an 8px header gap. Compare [before](portal-header-before-1280.png) and [after](portal-header-after-1280.png). The original pure geometry also allowed
an anchor above the viewport to place a menu at 16px with a 64px top inset, and
an anchor below it to produce a negative bottom offset.

Reserve the live sticky-header bottom in the top inset and clamp the menu's
attachment edge to the usable band. Keep visualViewport/IME and mobile/in-app
bottom reserves, synchronous positioning and capture scroll listener. The
existing sticky shell, flex CTA dock, and chart incremental refresh passed;
their production implementations need no change.

## Acceptance checklist for reviewing agents

Run `make verify-issue-1316` in the reviewed checkout. It runs existing/new unit
tests plus Chromium with `PLAYWRIGHT_SKIP_CHAIN=1` and five workers. Transport is
mocked; React, layout, canvas and the chart's 30s polling timer are real. Override
`PLAYWRIGHT_WEB_PORT` if port 5136 is busy. Frontend dependencies and Playwright
Chromium must be installed; use `bash scripts/with-node.sh --cwd frontend-dapp -- npm ci`
and the same wrapper for `./node_modules/.bin/playwright install chromium` when
needed.

- [x] **Picker document/inner scroll (S1316-1):** desktop 1280×720 and phone
  375×667 menu coordinates track the moved trigger, remain in the visible band,
  clear the sticky header/mobile tab bar, and permit selection. The pair panel
  gets a controlled overflow ancestor to exercise capture scroll, because its
  normal route layout is document-scrolled. Menu-inner scrolling must not move
  the portal. Check `portal-scroll-invariants.spec.ts` and moved-anchor unit cases.
- [x] **Ticket (S1316-3 / T527-1/T527-5):** desktop Limit CTA docks within 8px
  at actual body-scroll top/middle/bottom, with no overlap of visible controls.
  Confirm the footer remains a flex sibling and its CTA is neither sticky nor
  fixed. A disconnected wallet uses Connect Wallet in the same CTA slot.
- [x] **Chart (S1316-4):** both viewports preserve a real, mouse-panned logical
  range across the actual same-interval 30s request. No new historical `setData`
  calls; latest candle updates; no browser JS errors.
- [x] **Shell (S1316-2):** both viewports and dark/light themes preserve opaque
  sticky-header background and reachable brand navigation after document scroll.
  Trade heading clearance is ~16px (compact shell 14px, within 2px tolerance).
- [x] **Unit compatibility:** portal position/viewport and chart series/canvas
  tests remain green; `make verify-issue-632` preserves coarse browse, keyboard,
  in-app inset, token-logo and documentation contracts.
- [x] **Scope (S1316-5):** no contract, indexer, router, quote, tax, wallet-session
  or chart-production changes. Related closed issues remain closed.
- [x] **Build/lint:** `VITE_WC_PROJECT_ID=00000000000000000000000000000000 make build-frontend`
  and `make lint-frontend` pass. The synthetic project ID only satisfies the local
  build environment check; no deployment or WalletConnect session was attempted.
- [x] **Retest and independent review:** repeat the bundled verifier on the final
  patch, inspect `git diff --check`, and review production geometry and fixture
  assertions independently.

## Evidence and limits

Initial unchanged-code tests: 43 unit checks passed. New moved-anchor unit cases
then failed against the original geometry, proving both off-band edges. Shell
and ticket browser suite: 5 passed. Chart browser suite: 2 passed, with real 30s
refreshes at both viewport sizes. Lint: zero errors, five existing hook warnings.
Production TypeScript/Vite build passed. Final bundled verifier: **46 unit tests and 13 Chromium tests passed** (35.6s browser run, five workers). `make verify-issue-632`: **10 checks passed**, 32 unit tests passed across its two groups. Independent agent review and re-review passed. The bundled verifier was repeated unchanged: **46 unit tests and 13 Chromium tests passed again** (41.8s browser run). Review covered the corrected native-timer observation and found no blockers.

Per the implementation request, a reviewing agent may skip a check it cannot
execute and mark overall **PASS (available checks; skips listed)** if executed
checks pass. Record the exact unavailable item and reason; leave its checkbox
unchecked. Never turn a measured failure or skipped action into factual PASS.
The optional older #632 chain-backed browser suite was skipped (its separate deployed-chain fixture was not provisioned for this frontend-only acceptance run). Device wallet/in-app chrome, live-chain checks and production deployment were
not run: this issue's acceptance is frontend-only Chromium with no broadcast.
No operator handoff is needed for this patch.
