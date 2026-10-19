# Mobile shell and route follow-up

The follow-up to [#1561](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1561)
implements the explicit request to hide the mobile content footer, vertically
center Swap, and remove excessive route spacing with right alignment.

Below 768px, the footer is hidden; its environment, official product and legal
information is available in More → About and legal, outside ConnectedTermsGate.
The initial risk acknowledgement, wallet terms, header network badge and all link
allowlist/target/CSP controls remain. Desktop retains its footer.

Swap uses normal-flow flex sizing with min-height 100dvh and the measured bottom
navigation inset. Auto margins divide only spare space equally between the
sticky header and navigation. Expanded DETAILS, long errors, enlarged text and
shorter viewports scroll normally. No fixed-height page clipping or absolute
form positioning is used. The artificial root top padding is removed on mobile;
real safe-area padding stays. The navigation observer now watches its border box
so padding/safe-area changes update the covered height even if content height
stays unchanged.

Route nodes no longer grow into equal-width slots. The full path clusters at the
right with its declared small CSS gaps; a path can wrap as a unit when enlarged
metrics leave too little room. Token order, wrapper boundaries and canonical
identities in DETAILS remain intact. Tests measure visible label/icon bounds,
not just the wider token boxes that previously concealed the excessive gaps.

The old settled checked None line was still present in the prior implementation,
despite the earlier audit's stated intent. It is now an empty, aria-hidden
placeholder; the reserved status space remains, while real pricing and blocking
states keep their own accessible announcements.

Verification: 71 focused summary/navigation/product/Swap tests pass; final 1500
and 1498 source verifiers pass with declared unchanged/duplicate/external skips.
Product-link verifier 663 passes with its separate Playwright suite skipped in
favor of the actual production-build More/footer checks. Browser batches are
bounded to 55s and check 375×667, 390×844 and 1440×1000, connected/disconnected,
light/dark, concise errors, 200% text, expanded DETAILS, keyboard/modal behavior,
resized measured navigation and a short visual viewport. No serious/critical
axe findings, route overflow or hidden intermediate nodes are accepted.

The first geometry run exposed the root 8px offset and the content-box observer's
missed padding change; both were fixed. Enlarged desktop text also exposed a
route that needed to wrap to its own row. A subsequent assertion was corrected
to compare actual visual gaps against the declared, font-scaled CSS gap rather
than a fixed 5px ceiling at 200% text. Product source is independently hashed by
the reused-build guard; fixtures are reset between failed runs.

The requested mobile placement explicitly supersedes the old all-breakpoint
footer placement rules in the responsive-header/product/risk playbooks. It does
not change quote economics, risk thresholds, original 30-minute signing windows,
actor/amount/operations binding, offline admission, gas budgets or no-resend.
Physical-device/funded verification remains separate from these fixture checks.
Deployment and live observations are recorded below after release.

Final fixture measurements: 375×667 disconnected form 134.5–517.5px between
header 58px and navigation 594px; connected form 114.5–543.5px between 64/594px.
Both have equal spare space above and below (76.5px and 50.5px respectively).
The CTA remains fully above navigation. 375/390 mobile and 1440 desktop compact
states pass, including 200% text and real border-box padding updates, with zero
serious/critical axe findings and zero fixture provider requests. See
[the bounded local receipt](mobile-centering-local.json).
