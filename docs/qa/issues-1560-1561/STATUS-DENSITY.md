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
- [x] Frontend-only rolling deployment and actual live desktop/phone-width
  direction-switch checks; results are recorded below.
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


## Live release

[PR #1566](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1566) is merged.
Coolify deployment `wezykqhxcuhuvdilkmbyogla` finished at frontend revision
`bd2b4ba508d08bf8632b164c82ae02cff1284be0`. The user-authorized CI gate override
was used with full repository CI pending; it is not claimed PASS.

[Live browser evidence](status-direction-live.json) passes at 375×667 and
1440×1000, using actual public cached native quotes. Both retain the complete
18-decimal receive amount as new pay and request the same raw quantity in the
reversed quote (HTTP200). No wallet injection, signing or transaction calls.
Status computed leading/minimum are 16.2/32.4px. Mobile form 136.3–515.7px is
centered between header 58px and nav 594px; CTA bottom 515.7px is clear of the
nav. Footer visibility, compact right-aligned routes, wrapped Trade preview,
zero runtime errors and zero serious/critical axe findings pass.
[Phone switch screenshot](screenshots/direction-live-375x667.png);
[desktop switch screenshot](screenshots/direction-live-1440x1000.png).

The [conservative deployment checker](status-direction-deployment.json) flagged
one keyword entry among 75 inspected log entries. [Exact classification](status-direction-log-review.json)
shows that row equals the public merge commit subject, whose wording contains
“error spacing”; no error tokens remain after removing that exact public subject.
No unexpected build/runtime error was found. Two exact missing cleanup-container
messages were also recognized. The original checker FAIL is retained, with this
explicit review; it is not silently rewritten as PASS or broadly allowlisted.

Fresh [Loki/Grafana/Prometheus observations](status-direction-monitor.json)
and [public availability samples](status-direction-availability.json) are linked.
DEX API serving/quote readiness remain up; indexer readiness is independently 0,
so this does not certify indexer recovery. Backend stays
`b7b08736b3529af344a693ae6d16236a4f324c13`; no API/database/chain/logs reset occurred.
Source/config monitoring verification 1544 passed; traffic SLO and physical/funded
acceptance remain separate. Existing physical-phone Inbox card
`3fff415d-9cc2-42b1-b635-37e0be4fd0f7` remains the operator follow-up.

Issues #1560/#1561 remain open because full acceptance, including physical-device,
funded and full CI scopes, is incomplete. This evidence follow-up changes no
frontend runtime files from the deployed revision.
