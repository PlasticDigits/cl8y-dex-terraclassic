# Production browser evidence for #1561

The isolated Chrome 151 fixture serves a real production Vite build and scheduled
synthetic indexer observations. It cannot sign or broadcast. Connected scenarios
use an unsigned synthetic Keplr extension, intercepted positive balances and an
intercepted legal-acceptance status; these are explicitly fixture evidence.
Screenshots mask the connected wallet control. No real wallet or session is used.

The regression harness is [the lifecycle runner](../../../scripts/qa/swap-layout-geometry-1498.cjs)
and [the mobile geometry module](../../../scripts/qa/swap-mobile-geometry-1561.cjs).
The [1498 verifier](../../../scripts/verify-issue-1498.sh) builds once, then runs
browser batches with a 55-second hard limit. Every reused build checks the current
runtime source hash and controlled endpoint configuration before launching tests.
Tests, `__tests__` and `src/test` fixtures are excluded from the runtime hash.
After intentional 200% root-font resizing, the baseline waits for two animation
frames and fonts so rem-sized scrollbar/nav reflow has settled. The
[earlier immediate-baseline failure](browser-zoom-failure.json) remains recorded;
unchanged quote transitions retain strict 1px geometry checks.

At scrollY=0 with Settings/DETAILS collapsed, the actual CTA is measured against
both visualViewport and the actual bottom-nav top edge. Its center and four inset
edge midpoints must hit the button. Every route token must retain nonzero visible
width inside its route region; page, route and status overflow are checked
independently. A DOM visibility assertion alone is insufficient.

| Isolated 375×667 path | Before CTA top–bottom | After CTA top–bottom | Nav top | After route width/scrollWidth | After status height/scrollHeight |
| --- | --- | --- | --- | --- | --- |
| Disconnected, dark | 688–742 | 399–453 | 594 | 350/350 | 36/36 |
| Connected, light | 740–794 | 450–504 | 594 | 350/350 | 36/36 |

Each final path also verifies a concise persistent error: one inline decorative
cross, no dismiss control, zero card padding/border/background, no nested scroller
and no CTA occlusion. Axe reports zero serious/critical WCAG A/AA violations on
these Swap states. Keyboard probes cover input/visible focus, DETAILS and an enabled
primary action; disconnected Enter opens Connect Wallet and Escape returns focus
to the original CTA.

The initial keyboard probe failed because the shared Modal discarded the invoking
focus on close. The fix restores a still-connected invoking element unless focus
has deliberately moved to another surface. A stacked-modal unit test also exposed
shared `modal-title` IDs that named the newer dialog incorrectly; each modal now
uses a unique React ID. All 13 Modal tests pass, including returning focus,
preserving deliberate external focus and preserving focus in a newer dialog.
The sanitized [failed observation](browser-focus-failure.json) remains recorded.

The separate [ordinary-review browser probe](browser-review.json) uses matched
original integer CP economics with zero order slippage and ~11% signed total
reference divergence. First-click Confirm Swap remains active for 12 seconds while
the independent producer advances, with zero automatic quote HTTP requests and
unchanged original generation, expiry and minimum. An explicit amount edit fetches
a new quote and resets consent. No second confirmation click, signing or broadcast
occurs. This regression checks review stability; the longer fake-clock tests and
original-expiry checks remain separate.

The matrix covers 320×568, 390×844, 414×896, 430×932, 667×375 landscape and 767/768
breakpoint neighbors. It verifies current errors, a resized measured navigation
stack, a shortened viewport and 200% root text. The nav test sets bottom padding
to 20px; it simulates an enlarged nav stack and does not attest a real device's
safe-area environment. The shortened viewport similarly exercises ordinary page
scrolling and does not attest a physical software keyboard. At enlarged text or
open DETAILS, the action may require normal scrolling; it must remain reachable
above navigation and pass all five hit tests after scrolling.

Lifecycle tests at 375×667 and 1440×1000 retain the same input geometry through
unchanged refresh, rejection, missing/stale observations, recovery, original
expiry and rapid changed intents. One mounted DETAILS, anonymous positive minimum
and one LUNC network fee remain. The 200% text row-alignment and keyboard checks
also pass. An earlier attempt combined both lifecycle widths under one 55-second
cap: the 375 context passed, while the desktop expiry wait failed near the cap and
later steps lost the terminated browser. That [failed batch](browser-batch-failure.json)
remains FAIL. A fresh isolated desktop process then passed all 10 checks; the verifier
runs lifecycle widths separately. Connected desktop 1440×1000 before/after control rectangles are exactly
equal: Pay `(511.5,298,408,32)`, Receive `(511.5,465,408,32)`, summary
`(490.5,534,450,48)`, status `(490.5,582,450,72)` and CTA top/bottom `714/772`.

These fixtures use a direct synthetic pair. Wrapper boundaries and longer routes
are additionally exercised by component/route tests; this browser receipt does not
establish live native-route settlement. Physical screen readers, devices, real
safe areas, funded transactions and the complete abuse matrix remain separate
acceptance scopes. See [the joint results](RESULTS.md),
[layout policy](../../../skills/AGENTS_SWAP_LAYOUT.md) and
[focus policy](../../../skills/AGENTS_FRONTEND_A11Y_FOCUS.md).
