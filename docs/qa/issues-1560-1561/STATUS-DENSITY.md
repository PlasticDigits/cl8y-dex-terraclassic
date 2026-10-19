# Compact status and direction follow-up

Follow-up to [#1561](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1561)
and the shared [#1560](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1560)
risk notice. Error/status text now uses 1.35 line height and a 2.7em minimum
status area on every device. Expert/Cancel actions flow with the sentence rather
than adding a separate paragraph or stretching every wrapped line to 44px.

Each inline action retains an actual 44px hit area through its pseudo-element,
with a visible focus ring around that area. Status bottom clearance keeps this
target clear of DETAILS. Long errors still wrap and grow in normal flow; there
is no clipping, dismiss control or nested scrollbar. Blockers, announcements,+risk thresholds and original execution safeguards are unchanged.

Implementation: [styles](../../../frontend-dapp/src/index.css),
[shared notice](../../../frontend-dapp/src/components/swap/SwapImpactAside.tsx),
[Swap](../../../frontend-dapp/src/pages/SwapPage.tsx).
Agent invariants: [SL1498](../../../skills/AGENTS_SWAP_LAYOUT.md),
[TR1560](../../../skills/AGENTS_TRADING_RISK.md).
The simultaneous [direction fix](DIRECTION-AMOUNT.md) transfers the exact current
output to a fresh reversed input and clears old denomination-bound consent.

## Verification checklist

- [x] Production-build Chrome checks at 375×667, 768×1000 and 1440×1000:
  12px text has 16.2px leading; status height is 32.4–46.3px for the tested
  one/two-line notices. Buttons have 44px actual hit targets and pass top,
  middle and bottom hit tests without obstructing DETAILS.
- [x] Expert action opens the modal by keyboard, Escape returns focus, without
  enabling Expert Mode or signing. Zero serious/critical axe findings.
- [x] The valid synthetic blocked quote is tested first. A second layout-only
  DOM text replacement reproduces the longer reported two-metric sentence;
  it supplies no fake execution evidence and never signs.
- [x] Normal compact layout passes at 375×667, 390×844 and 1440×1000,
  including ordinary/200% text, normal scrolling, footer/centering and route
  spacing. Separate connected light-theme 375×667 fixture checks preserve CTA
  reachability. All fixtures own their state and request zero providers.
- [ ] Frontend-only rolling deployment and actual live desktop/phone-width
  direction-switch checks; results will be recorded after release.
- [ ] Physical-device, funded settlement and full repository CI acceptance.

The initial issue-1498 run found one old test expecting the previous pay amount
to survive a direction switch while no receive quote existed. The test now
requires empty fields and verifies a fresh edit obtains the reverse quote.
Retest passed: issue-1498 has 186 tests on each of two runs; issue-1500 has 171 tests on each of two runs; Trade issue-1497 has 161 tests. Types, focused lint and formatting pass. Verifiers report PASS WITH SKIPS for separate browser/unchanged Rust/funded/device scopes, with actual bounded browser receipts linked below.
Each browser/test process is capped at 55 seconds. Production compilation is
separate. This follow-up resets only its local synthetic fixtures, with no
database, chain, API or logs lifecycle changes.

Local receipts: [status geometry](status-density-local.json),
[compact geometry](status-layout-local.json),
[connected geometry](status-connected-local.json).
They bind runtime source hash
`a59e1c8e9b88b66572e211df0379cf082532a77a055acbbc172dfec0cb006272`.

[Test summary](status-direction-verification.json); [375px screenshot](screenshots/status-density-375.png), [desktop screenshot](screenshots/status-density-1440.png).
