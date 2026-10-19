# Issue #1561: compact mobile Swap and inline status

Issue: [#1561](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1561).
Apply [SL1498](../../../skills/AGENTS_SWAP_LAYOUT.md) and the
[accessibility policy](../../design-system.md#accessibility-policy).
Coordinate [#1560 risk policy](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1560).
This supersedes #1498's six-em nested status scroller and horizontal route scroller.

Implementation: [Swap presentation](../../../frontend-dapp/src/pages/SwapPage.tsx),
[route summary/details](../../../frontend-dapp/src/components/swap/SwapQuoteSummary.tsx),
[scoped mobile/status CSS](../../../frontend-dapp/src/index.css).

The compact mode is scoped to mobile Swap. Desktop chrome/density and shared
control targets remain unchanged. Amounts retain their exact draft/quote values;
visual truncation never modifies execution. Labels remain associated with both
inputs. All picker, balance/fraction/Max, direction, Settings, Share, DETAILS and
submit controls remain keyboard operable.

The route strip keeps every selected node, including repeated symbols and native
wrapper boundaries; flexible node widths truncate symbols when necessary. Full
labels and canonical identities appear in the one DETAILS disclosure, with
ordinary wrapping for long/unbroken/RTL identities. Symbols remain display labels.
No route content is scaled, removed or horizontally scrolled.

Current quote/submission errors use the same persistent inline status region.
One aria-hidden cross shares the sentence's text flow; scoped CSS removes alert
chrome without changing other pages. Individual notices own announcements, avoiding
a second enclosing live region. Long text grows into the page and stays reachable.
The compact minimum space remains independent of execution permission.

Acceptance checklist (check only from recorded final evidence):

- [ ] AC1: At initial scrollY=0 and 375×667, full disconnected and connected CTAs fit above measured navigation/visual viewport, with balance controls and concise current error/no-warning states; center/edge hit tests detect no overlay.
- [ ] AC2: Mobile chrome/labels/gaps compact, accessible input names preserved, legible amounts and 44px controls; real safe-area header; desktop control unchanged.
- [ ] AC3: Exactly one inline decorative error cross; no dismiss/card chrome or internal status scrollbar; long errors wrap, blockers persist and actions/announcements work.
- [ ] AC4: Every bounded route node/destination/wrapper remains in order without internal horizontal overflow; full labels/identities are readable in DETAILS.
- [ ] AC5: Page/route/status never overflow horizontally; input focus/geometry survives quote transitions; one DETAILS and one network fee.
- [ ] AC6: Keyboard/screen reader, 200% text, reduced motion, portrait/landscape, both themes, overlays/software keyboard/safe-area checks keep content reachable; vertical reflow is allowed.
- [ ] AC7: Production-build short-viewport numerical CTA/nav/visual viewport bounds, hit tests and internal route/status overflow assertions; sanitized before/after screenshots.

Run relevant Swap summary/page/acquire/picker/seam/route-alignment tests,
`make verify-issue-1498`, `make verify-issue-1500`, frontend types/lint and the
critical-routes accessibility scan. Bound each verification invocation below
60 seconds; partition browser widths/states where needed. The geometry harness is
[scripts/qa/swap-layout-geometry-1498.cjs](../../../scripts/qa/swap-layout-geometry-1498.cjs).
A fixture establishes layout and focus behavior only; record live artifacts,
physical device/assistive technology and funded settlement separately. Keep
executed failures visible until repaired/retested; list remaining scope explicitly.

Final implementation, bounded verification, deployment and remaining acceptance: [joint results](../issues-1560-1561/RESULTS.md). Both issues remain open for the explicitly unperformed scopes.
