# ADR 0012: Scroll layout invariants

## Status

Proposed ([#1316](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1316))

Frontend layout only. This ADR does **not** change swap math, limit matching, tax, signing, indexer, factory, or router. It does **not** deploy, spend, expand custody or policy, or self-approve the design. Keywords on the issue are not approval. Deploy, spend, custody, and policy expansion stay under [agent-control #297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297). A third `design_author` claim on this source parks `needs_human` ([agent-control #432](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/432)). Do not edit `autonomy.rs` or HMAC to accept this design.

Overview: [`architecture.md`](../architecture.md#scroll-layout-invariants). Invariants: [`qa-invariants.md`](../qa-invariants.md) **Q25** / **S1316**. Product rules stay in [`frontend.md`](../frontend.md) (portal listbox, sticky header, **T527**, chart time scale). Do not open a new playbook that restates **V632**, **T527**, or **#705**.

**Numbering.** On current `main` (`0d6eaeee`): live **Q21** is Lunc Dash `payload` ([#1308](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1308)); live **Q22** / ADR **0009** is hub-wrap leftover [#1306](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1306). ADR **0008** is unused (closed [#1300](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1300)). Unpublished [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) reserved ADR **0010** / **Q23** on `origin/cac-design-issue-1305`. Unpublished [#1311](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1311) reserved ADR **0011** / **Q24** on `origin/cac-design-issue-1311`. This ticket is **0012** / **Q25** / `{#scroll-layout-1316}`. Do not take those slots. Do not merge those design branches as design-only PRs. Do not open a design-only PR from `cac-design-issue-1316`. Implement keeps Status **Proposed ([#1316])** until a reviewer accepts the design.

## Outcome

One product surface: the four scroll rules already documented in `docs/frontend.md` still hold together after the user scrolls. One MR. Do not split header, ticket, portal, and chart into sibling issues. Do not reopen closed [#181](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/181), [#336](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/336), [#482](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/482), [#498](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/498), [#500](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/500), [#527](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/527), [#632](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/632), or [#705](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/705).

1. **Measure, then fix only what fails.** Add the regression in Tests first. If a rule already passes, leave its production code alone.
2. **Portaled pickers.** An open `MenuSelect` / `TokenSelect` / `TokenSearchSelect` / pair search listbox keeps fixed `top` / `left` aligned with the trigger after `window` scroll or scroll of the nearest overflow ancestor that contains the trigger. The menu box does not intersect `.app-top-sticky` or the mobile tab bar.
3. **Sticky shell.** After `scrollY > 0` on a long route, page copy is not readable through `.app-top-sticky`, and header controls remain hit-testable. Trade H1 still clears the header by about 16px at `scrollY = 0` (existing `navigation.spec.ts` check; do not invent a second gap constant).
4. **Trade ticket.** After `trade-order-ticket-scroll` moves, the money CTA bottom stays within 8px of `trade-order-ticket-card` bottom and does not overlap controls that are actually visible inside the scrollport (**T527-1**). The footer stays a flex `shrink-0` sibling (**T527-5**).
5. **Chart time scale.** A user zoom or scroll away from `fitContent()` survives one same-interval candle refresh. `timeScale().fitContent()` stays limited to mount, indicator toggle, and interval switch.

`make verify-issue-1316` is the completion check (Vitest + Playwright at 5 workers, `PLAYWRIGHT_SKIP_CHAIN=1`, no broadcast).

## Context

Scroll-linked behavior already shipped:

| Surface | Where it lives today | What #1316 adds |
|---------|----------------------|-----------------|
| Sticky header | `.app-top-sticky` in `Layout.tsx` / `index.css` uses opaque `--bg-0`, `z-index: 40`. `e2e/navigation.spec.ts` already checks Trade H1 gap at `scrollY = 0` and `elementFromPoint` on the sticky stack. | Same opacity/hit-test after `scrollY > 0` on a long route, inside the new bundle spec. |
| Portal listbox | `usePortalListbox` reads `getBoundingClientRect()` during render and bumps a reducer on `window` `scroll` (**capture**), `resize`, and `visualViewport` `scroll` / `resize`. `computePortalListboxStyle` is pure fixed positioning. Insets come from `readPortalListboxViewport` (**#632**): `visualViewport` plus tab bar, in-app reserve, finger gap. `.token-select-dropdown` is `z-index: 200` (above the header on purpose). | A scroll regression. The capture listener is not the bug to rewrite. `topInset` is IME / pinch offset, not sticky-header height. |
| Trade ticket | Money CTA is `.trade-ticket-submit-footer`, sibling of `.trade-order-ticket-scroll`, not `position: sticky`. Overlap checks clip to the visible scrollport (**#702**). | Re-assert **T527-1** after the ticket body scrolls. |
| Chart | `priceChartLightweightSeriesSync.ts` uses `series.update()` for the live bar. `PriceChartLightweightCanvas.tsx` calls `fitContent()` on mount (line ~168), indicator toggle (~265), and interval switch when the first bar time changes (~317). Same-interval newest-N shifts do not fit (**#336** / **#705**). | A Vitest that a non-default visible logical range survives a last-bar refresh. Touch the sync file only if that test shows a history rewrite or a refresh `fitContent()`. |

Pair search on `/trade` sits in the page shell, not inside `trade-order-ticket-scroll`. Swap pay/receive uses `TokenSearchSelect`. Charts uses `MenuSelect`. There is no portaled picker inside the ticket body today. Do not add one so the test has a subject.

The capture-phase `window` listener already sees scroll events from inner overflow elements (they do not bubble, but they pass window capture). Ticket-body scroll while the pair menu is open must not move a menu whose trigger did not move.

## Non-goals

- Rewriting the capture-phase scroll listener, or attaching a listener per overflow parent, when the existing listener already tracks.
- `position: sticky`, `position: fixed`, or a document portal on the ticket money CTA (**T527-5**).
- `fitContent()` on a 30s refetch or a sliding newest-N window at the same interval (**#705**).
- Folding sticky-header height into `readPortalListboxViewport` `topInset` (that inset is `visualViewport.offsetTop` for **#632**).
- Sizing menus from `innerHeight` alone. Focusing a text field on coarse/narrow browse (**V632-5**).
- Toggling leading-logo padding on `open` (**#498**). Removing `html { scrollbar-gutter: stable }`.
- Raising `.app-top-sticky` above portal `z-index: 200` (**#181** / **T527-8**). Lowering the menu under the header so it is covered by scrolled copy.
- A new nested `shell-panel` / `card-glass` (**#653**).
- Swap math, route/solve, limit matching, tax, wallet session, contracts, or indexer.
- Reopening the closed tickets listed in Outcome.
- A new `AGENTS_FRONTEND_SCROLL_*.md` playbook.
- Coolify clicks, founder cards, or #297 deploy. This is ordinary frontend design.
- Opening a design-only PR. Self-marking this ADR **Accepted**.

## Decision

**Keep the four mechanisms. Add one regression bundle. Change production code only where that bundle fails.**

### Anchor tracking

“Follow the trigger” means the **delta** of the menu box matches the **delta** of the trigger box, within **4px**, after the scroll settles. It does not mean `menu.top === trigger.top`. Below-placement uses `top: anchor.bottom + gap` (`gap` default 8) and a clamped `left`. Compare `menu.top - trigger.bottom` and `menu.left - trigger.left` before and after scroll (each difference within 4px). Use `expect.poll` so the assertion waits for the reducer re-render. Do not assert in the same synchronous turn as `scrollBy`.

Scroll distance in the follow case must leave the trigger fully below `.app-top-sticky` and fully above the mobile tab bar, so follow and chrome clearance can both be true.

If a larger scroll moves the trigger under the sticky header or out of its scrollport, the allowed behaviors are: close the listbox, or keep it open with **no** intersection between the menu box and `.app-top-sticky` / the tab bar. Do not leave the menu painted over the header. Do not satisfy that by changing header `z-index`.

Header clearance, when a fix is required, is a clamp measured from `.app-top-sticky` `getBoundingClientRect().bottom`, passed beside viewport insets. Do not store that height in `viewport.topInset`.

### Ticket and chart

Ticket: scroll `trade-order-ticket-scroll`, then the same **T527-1** measurement as `e2e/trade-page-responsive.spec.ts` (CTA bottom vs card bottom ≤ 8px; intersection clipped to the visible scrollport). Do not restyle the footer to silence an overflow layout-box that is not visible.

Chart: drive `PriceChartLightweightCanvas` in Vitest. After `setVisibleLogicalRange` to a non-default range, apply a same-interval payload that only changes the last bar (the `series.update` path). `getVisibleLogicalRange()` `from` / `to` stay within **0.01** of the pre-refresh range, and `fitContent` call count does not increase. If that passes, do not edit `priceChartLightweightSeriesSync.ts` or the canvas `fitContent` sites.

### Component / state / interface

| Piece | Change |
|-------|--------|
| `usePortalListbox` | No change if scroll tests pass. If the menu misses the trigger, fix positioning only. Keep the capture listener and the render-time rect read. |
| `computePortalListboxStyle` / `readPortalListboxViewport` | Signature stays. No new required argument. Optional header clamp stays outside `topInset`. Add a unit case in `portalListboxPosition.test.ts` only when the pure geometry needs one (anchor moved, insets unchanged). Do not assert DOM listeners there. |
| `Layout.tsx` / `.app-top-sticky` | No change if opacity and hit-testing pass. Do not add a second sticky bar. |
| Ticket footer CSS | No change if **T527-1** passes after scroll. |
| Chart sync / canvas | No change if the range Vitest passes. |
| Public HTTP / wallet / contract interfaces | None. |

### Affected invariants

| ID | Rule |
|----|------|
| **S1316-1** | Open listbox on `/trade` (pair search) and `/` (pay token) at ~1280×720 and ~375×667. After document scroll that keeps the trigger in the visible band, menu deltas track the trigger within 4px. Menu box does not intersect `.app-top-sticky` or the mobile tab bar. |
| **S1316-2** | With the `/trade` pair listbox open, scrolling `trade-order-ticket-scroll` does not move the menu (trigger is outside that scrollport). If a trigger ever sits inside an overflow ancestor, scroll that ancestor and apply **S1316-1**. Do not mount a new picker inside the ticket. |
| **S1316-3** | Long route (`/pool`) with `scrollY > 0`: `elementFromPoint` on `.app-top-sticky` hits the sticky stack or a header control, and the sticky element’s own background alpha is 1. Trade H1 gap at `scrollY = 0` stays the existing navigation spec. |
| **S1316-4** | `/trade` Limit ticket, ~1280×720, ticket at scroll-top, then scroll the ticket body: CTA bottom within 8px of the card bottom; no overlap of **visible** Pay / Receive / Expiry / Advanced. Footer is not `sticky` / `fixed` and is not portaled. |
| **S1316-5** | Non-default chart logical range survives one same-interval last-bar refresh. No new `fitContent()` on that path. |
| **S1316-6** | **#632** viewport math, **#498** logo padding, `scrollbar-gutter: stable`, and one chrome layer (**#653**) stay. Coarse/narrow open does not focus a text field. |
| **S1316-7** | One MR. Closed issues in Outcome stay closed. Diff has no contract, indexer, router, or wallet-session files. |
| **S1316-8** | `make verify-issue-1316` is the gate. Playwright uses 5 workers and `PLAYWRIGHT_SKIP_CHAIN=1`. No LocalTerra and no chain broadcast for this ticket. |

## Alternatives

| Alternative | Why not |
|-------------|---------|
| Rewrite portal positioning onto `position: absolute` inside the trigger | Reopens **#181** CLS. Fixed + portal stays. |
| Raise header `z-index` above 200 | Menus would vanish under the header. Contradicts **#181** / **T527-8**. |
| Put sticky-header height into `topInset` | Mixes IME offset with DEX chrome and breaks **#632** unit tests. |
| `position: sticky` footer inside the ticket scrollport | Forbidden by **T527-5**. The float bug returns. |
| Call `fitContent()` after every candle refetch | Wipes user zoom. Forbidden by **#336** / **#705**. |
| Split four sibling issues | The failure mode is the rules drifting apart. One bundle. |
| Playwright-only chart proof against a live indexer | Needs chain or a flaky empty canvas. Vitest on the canvas effect is the proof. Skip an e2e chart case when the skip-chain trade page has no series; do not start LocalTerra to get one. |
| New skill file that copies **T527** / **V632** / **C705** | Those playbooks stay canonical. Crosslink one line each at implement time. |

## Complexity

**Added:** one Playwright spec, one chart-range Vitest, `scripts/qa/verify-issue-1316.sh`, a Makefile target, a Fixed-menu sentence in `docs/frontend.md`, and the **Q25** pointer. A header clamp or ticket/chart edit only if a test fails.

**Removed:** nothing. Do not delete the capture listener or the `fitContent` sites that mount, toggle indicators, or switch interval.

## Migration

None. No schema, env, contract address, or stored user setting. Existing zoom state is in the chart instance only; a reload already resets it.

## Observability

No new metric, log line, or health field. Failure is a red Vitest or Playwright assertion. Do not add `console` traces on scroll. Production attest is the frontend Coolify cut that already follows `main`; this ticket does not probe `dex.cl8y.com` and does not read indexer `/health`.

## Failure modes

| Mode | What to do |
|------|------------|
| Assert runs before the position reducer commits | `expect.poll` the menu box. A single `boundingBox()` immediately after `scrollBy` is a flake, not a product bug. |
| Follow test scrolls the trigger under the header | Shrink the delta. Cover the under-header case separately (close, or no intersection). |
| Phone menu intersects the tab bar | Keep **#632** bottom inset. Do not drop the finger gap to make the follow test pass. |
| Header clamp uses `topInset` | Revert that. **#632** tests must stay green without a header-height fixture. |
| Ticket overlap fails on off-screen Expiry | Clip to the visible scrollport (**#702**). Do not restyle the footer. |
| Refresh calls `setData` on unchanged history and the library resets the scale | Then, and only then, edit the sync helper so last-bar refreshes stay on `update`. Do not “fix” it with `fitContent()`. |
| Newest-N first-bar change at the same interval calls `fitContent` | That is a **#705** regression. The interval gate must stay `intervalChanged && prevFirst !== nextFirst`. |
| Coarse/narrow Swap focuses the search input on open | **V632-5** failure. Do not “fix” scroll by focusing the field. |

## Implementation slices

Ordered. Later slices depend on the earlier ones. No open-issue dependencies (closed tickets are constraints).

| Slice | Owner | Work | Depends on |
|-------|--------|------|------------|
| **0 — this design** | design_author | ADR **0012**, architecture `#scroll-layout-invariants`, **Q25**, README index. Branch `cac-design-issue-1316` only. | — |
| **1 — measure** | implement | Add `e2e/scroll-layout-invariants.spec.ts` and the chart-range Vitest. Run them. Record which **S1316** rows fail. | Slice 0 accepted |
| **2 — fix** | implement | Edit only the failing surface, inside the Decision bounds. Re-run slice 1. | Slice 1 |
| **3 — bundle** | implement | `verify-issue-1316` script + Makefile + help lines; Fixed-menu sentence; one-line crosslinks in the portal, ticket-dock, and chart sections of `docs/frontend.md` and the matching skills; `docs/testing.md` row; `AGENTS.md` verify line. One MR. | Slice 1 (spec path exists). Lands with slice 2. |

**Slice 0 apply — insert; do not whole-file checkout.** `docs/architecture.md`, `docs/qa-invariants.md`, and `docs/README.md` also move on unpublished **#1305** / **#1311** designs. Copy the new ADR file. Insert the architecture section, the **Q25** block, and the README line. Do not `git checkout <design-sha> --` those three files.

## Tests

Playwright project stays at **5 workers** for `e2e-smoke`. This spec is smoke, not `e2e-tx`.

| Check | File / command |
|-------|----------------|
| Portal geometry unit | Existing `portalListboxPosition.test.ts`. Extend only for a new pure case. |
| Chart range | New Vitest next to the canvas tests. Also keep `priceChartLightweightSeriesSync.test.ts` green. |
| Scroll bundle | `frontend-dapp/e2e/scroll-layout-invariants.spec.ts` at 1280×720 and 375×667. `PLAYWRIGHT_SKIP_CHAIN=1`. |
| Sticky gap at `scrollY = 0` | Existing `e2e/navigation.spec.ts`. Do not fork the 16px constant. |
| Ticket dock at scroll-top | Existing `e2e/trade-page-responsive.spec.ts`. The new spec adds the **after inner scroll** pass. |

`scripts/qa/verify-issue-1316.sh` order (FAIL fails the stack):

1. Vitest: `portalListboxPosition.test.ts`, `priceChartLightweightSeriesSync.test.ts`, and the new chart-range test (`bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run …`).
2. Playwright `e2e-smoke`, **5 workers**, on `e2e/scroll-layout-invariants.spec.ts`. Copy `free_tcp_port` from `scripts/qa/verify-issue-703.sh` (function only — not its `node_modules` symlink). `free_tcp_port 30116`, then run in a **subshell** with `PLAYWRIGHT_SKIP_CHAIN=1 PLAYWRIGHT_WEB_PORT=30116 PLAYWRIGHT_BASE_URL=http://127.0.0.1:30116`. Do not export those into the parent shell.
3. Grep **S1316** / `verify-issue-1316` / this ADR in `docs/frontend.md`, `docs/testing.md`, `docs/architecture.md` (`scroll-layout-invariants`), `docs/qa-invariants.md` (**Q25**), and `AGENTS.md`.

Makefile target `verify-issue-1316` next to other frontend verify targets. Add it to the Cloud Agent `help` echo and the Frontend `help` echo that lists `verify-issue-*` layout checks.

`VERIFY_ISSUE_1316_SKIP_E2E=1` may skip step 2 only when the Playwright package is missing. An installed browser that crashes is FAIL. Skip-E2E is not completion.

## Rollout

Frontend MR to `main`. No flag, no migration, no indexer deploy. Vite auto-deploy on the existing frontend Coolify app is an operator flag (ADR **0006**); this ticket does not flip it and does not treat a production glance as the gate. Local completion is `make verify-issue-1316` with Playwright actually run.

## Rollback

Revert the MR. No data to undo. Chart zoom, menu coords, and sticky paint return to the previous build on the next frontend cut. Do not roll back contracts or the indexer for a layout revert.

## Integration completion

All of the following:

- Status of this ADR is still **Proposed** in the implement MR until a reviewer accepts it. Implement does not write **Accepted**.
- `make verify-issue-1316` exits 0 with the Playwright step run (not `VERIFY_ISSUE_1316_SKIP_E2E=1`).
- **S1316-1–S1316-8** hold on Chromium at ~1280×720 and ~375×667.
- The diff does not include contract, indexer, router, or wallet-session files.
- Closed issues listed in Outcome are not reopened.
- `cac-design-issue-1316` is not merged as a design-only PR. The product MR cites **#1316**.

Not completion: a green unit run that skipped Playwright; a production screenshot; a Coolify toggle; a founder card.
