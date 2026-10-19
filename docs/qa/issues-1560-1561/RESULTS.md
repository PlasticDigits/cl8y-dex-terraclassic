# Results — trading risk and mobile Swap

Issues: [#1560](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1560),
[#1561](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1561).
Date: 2026-10-19 UTC. Work started from `b7b08736b3529af344a693ae6d16236a4f324c13`
in an isolated Git worktree. Product evidence is synthetic unless explicitly
identified as live below; no funded transaction was signed or broadcast.

## Implemented and verified locally

- [x] #1560 AC1–AC4: shared original-integer risk evaluator drives Swap/Trade
  notices, funded guidance, ordinary review and final mutation checks. Strict
  signed order >5 OR symmetric total >10; exact equality does not trigger.
  >30 Expert and >=99 nonwaivable symmetric refusal remain. Missing/forged/stale
  evidence blocks permission; no legacy worst-hop fallback. Supported direct
  wrapper paths revalidate original capabilities/floors/mapping rather than
  fabricating a pool diagnostic.
- [x] #1560 AC2: 3.34/1.71/6.91 arithmetic control is quiet; separate order,
  total and combined triggers produce one accurate reason. Tests include
  one-raw-unit boundaries at large Uint128 amounts, favorable order improvement,
  favorable/unfavorable total deviation and malformed/forged evidence.
- [x] #1560 AC5–AC7, automated scope: adjacent active/inactive noise audit,
  persistent inline blockers, polite ordinary risk notices, original observation
  review binding and final fresh-evidence consent assertion; docs and agent
  guidance explicitly supersede old ordinary aggregate >5 consent. Existing
  admission/floor/fee/tax/account/expiry/no-resend safeguards remain.
- [x] #1561 AC1–AC3/AC5/AC7, production-build synthetic scope: measured initial
  CTA versus nav/visual viewport and five pointer-hit probes, compact mobile
  controls with accessible field names and >=44px action targets, exactly one
  inline decorative cross, no error card chrome or route/status scroller.
  Routine loading/error/recovery/expiry preserves input positions and DETAILS.
- [x] #1561 AC4, bounded component/synthetic scope: every supplied route token
  remains ordered in the summary; full labels/canonical identities are readable
  in the one DETAILS disclosure. Actual native live-route observation is recorded
  separately after deployment; complete funded execution-family coverage is absent.
- [ ] #1561 AC6 complete physical-device/screen-reader/software-keyboard and
  real safe-area coverage. Automated keyboard, axe, themes, zoom, simulated
  visual viewport/nav insets and portrait/landscape checks pass, but do not prove
  physical behavior.
- [ ] Complete body/comment verification matrix for both issues: funded actual
  execution families/decimal variants/manual routes, physical signing and
  assistive-device trials have no receipts. Both issues must remain open. Automatic quote arithmetic and deferred promotion
  pause during active ordinary review, preserving the original unexpired snapshot
  while independent restrictions continue. Final 92-test Swap/Trade regression
  covers 25-second review retention, edit revocation, original expiry and resumed
  refresh; an unsigned production-browser control checks 12 seconds of retention
  with zero quote requests despite a changing independent producer.

## Geometry and screenshots

| 375×667, scrollY=0 | Original CTA top–bottom | Fixed CTA top–bottom | Nav top |
| --- | --- | --- | --- |
| Disconnected | 688–742px | 399–453px | 594px |
| Connected, synthetic unsigned wallet | 740–794px | 450–504px | 594px |

The connected fixture intercepts balance and legal-status reads; it cannot sign.
Its screenshots mask the wallet chip. Original route content was 215px wide in
103px disconnected / 152px connected space; the fixed mobile route is 350/350px
scrollWidth/clientWidth. Status is 36/36px scrollHeight/clientHeight on mobile.
Connected desktop 1440×1000 Pay/Receive/summary/status rectangles and CTA
714–772px match the original exactly; desktop status retains its 72px reservation.

[Browser methodology and limits](BROWSER.md), [before/after screenshots](screenshots/),
[disconnected](browser-after-disconnected.json),
[connected and desktop](browser-after-connected.json),
[375 lifecycle](browser-lifecycle-375.json),
[phone/landscape/breakpoint matrix](browser-matrix.json).
All recorded fixture provider-request counts are zero. Production Vite artifacts
were compiled separately from browser test processes; reused artifacts reject
source-tree-hash or controlled-endpoint mismatches. Receipts retain the dirty-tree
build identity honestly; their product source hash binds the later committed code.

## Verification commands and limits

All actual test processes were bounded to <=55s. A final keyboard check found
that Escape from Connect Wallet lost the invoking focus; Modal now restores
focus when closing without stealing deliberate external focus. Stacked dialog
titles now have unique IDs, fixing the independently reproduced wrong accessible
name. Focused unit and
rebuilt production browser controls cover this additional correction. Secondary frontend regressions
were deduplicated across the requested 1483/1489/1473/1480/1452/1497 suites in
22/22/17-file batches. Failures were repaired and only affected files rerun.
Unchanged DB/chain/full-backend suites were not rerun for this frontend change.

| Check | Result and scope |
| --- | --- |
| `make verify-issue-1500` | PASS WITH SKIPS, 10 passed / 0 failed / 2 declared external skips; final first/retest each 171 frontend tests plus six order-economics and one manual-split Rust test; types, fmt and crosslinks pass. |
| `make verify-issue-1498` | Final first/retest each 160 frontend tests pass; 6 passed checks, 0 failed, 4 declared duplicate/external skips. Types run in 1500 and production browser is split into independently bounded batches. An earlier 146-test run passed both attempts before its outer 55s orchestration cap during subsequent checks; no individual test timed out. |
| Secondary frontend | Initial 61-file batches: 897 passed,23 stale-fixture assertions failed. Relevant fixtures repaired without relaxing validators; affected Swap files67/67 and Trade/metric files81/81 then pass. |
| `make verify-issue-1483` | PASS WITH SKIPS, 16 passed / 0 failed / 7 explicit unchanged/duplicate/external skips. |
| `make verify-issue-1489` | PASS WITH SKIPS; exact discovery Rust 30 tests and probe/browser-source/privacy controls pass; DB/duplicate/external scopes skipped. |
| `make verify-issue-1473`, `1480`, `1452` | PASS WITH SKIPS in relevant source scopes; original DB/chain/funded checks remain unchecked, overlapping frontend/browser coverage deduplicated. |
| `make verify-issue-1497` | PASS, frontend Trade entry, types and syntax. |
| Modal/Expert/wallet | 32 focused tests pass, including stacked naming and keyboard return. |
| Types / full frontend lint | PASS; ESLint0 errors,5 existing warnings in unrelated chart/LazyRoute/PortalListbox/LimitOrders files. |
| `make verify-issue-1544` | PASS focused source/config/monitor privacy/target checks, isolated config parsers; no production monitoring lifecycle mutation. |
| Frontend-only release helper | Seven synthetic identity/revision/status/log-privacy controls pass. PASS requires exact finished revision and zero unexpected error-pattern entries; queued/in-progress is PENDING. |

The first1500 wrapper failed only its stable-source check because final CSS edits
landed during verification; source was frozen and the full target rerun successfully.
The original Rust formatting failure described in both issue bodies was reproduced
and repaired with three formatting-only changes in two files; no backend behavior
or schema changed. Initial1483/1489 verifiers also exposed outdated source bundling
and a missing producer-age field in QA fixtures; their actual-source dependency
loader and fixtures were repaired, focused tests passed, and the targets rerun.
No executed failure was relabeled PASS. Failed loops reset owned mocks/browser
processes/HTTP fixtures, never the production chain/API/database/logs stack.

## Live release

[PR #1562](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1562) is merged.
Source commit `4b1c803d0ba647613515183c30a7118b733064c9`; deployed merged main
`6a5118ffc2a3bf71b4ad638226d48228bd2eb64a`. Both local main and the isolated
worktree were updated from origin. Pre-commit Rust fmt/Clippy, frontend checks,
message policy and gitleaks passed. The post-hook product source hash remains
`e984d27f76c9f91e838b86438edf91eb9752adf695be79fdce371cdfdbc79d7b`, matching
all final isolated browser receipts. Modal tests also passed after hook formatting.

- [x] [Coolify frontend deployment](deployment-status.json)
  `qvplhwapjhqfxd2mm8lfnzua` finished at the exact reviewed revision. Its 94 log
  entries were inspected privately: 0 unexpected error-level pattern entries,
  2 exact expected missing-cleanup-container entries. No raw logs/configuration
  were exported. The initial observer rejected an omitted public internal-ID
  field, then counted an Error substring in a public bundle filename. Both
  [failed observer receipts](deployment-status-initial.json) and the
  [pattern receipt](deployment-log-pattern-initial.json) remain. The helper now
  binds the deployment's embedded application UUID/numeric ID and matches whole
  error-level words; seven focused controls pass. These were observer defects,
  not failed application deployments.
- [x] [Live browser](live-browser.json), observed 2026-10-19 at 04:22 UTC:
  actual anonymous cached native USTC→CL8Y Swap and wrapped Trade market previews
  pass the independently calculated strict risk rule. This live sample was below
  both ordinary thresholds and showed zero risk notices. At 375×667, CTA 399–453px
  lies above nav 594px, is hit-testable, and the actual three-token native/wrapper
  path fits 350/350px with no status/page overflow. Desktop also passes. Both
  Swap viewports and the Trade market page have 0 serious/critical axe findings
  and 0 runtime errors. This does not reproduce the historical 3.34% quote or
  establish connected funded settlement.
- [x] [Artifact attestation](artifacts-after.json): completed frontend source
  matches 6a5118f; every root external script, including both bootstrap scripts,
  is hashed. Backend deployment/public health remain at unchanged
  `b7b08736b3529af344a693ae6d16236a4f324c13`. No backend, database or logs
  lifecycle operation was performed. [Live screenshots](screenshots/) are public,
  disconnected product evidence without wallet/session identifiers.
- [x] [During-deployment samples](availability-during.json) and
  [after](availability-after.json): frontend, API liveness and serving all HTTP200;
  backend revision unchanged. No outage was observed. Sparse samples do not prove
  uninterrupted service between observations; no Stop/reset operation was used.
- [x] [Fresh Grafana/Loki/Prometheus observation](monitoring-after.json),
 2026-10-19 at 04:23 UTC: PASS in 11 reads. Both exact DEX jobs scraped successfully,
  original observations aged 28.961s (90s bound), datasource healthy, liveness,
  serving and useful-quote readiness 1. Indexer readiness was separately 0; it is
  retained as an independent observation, not hidden or called an API outage.
  Loki aggregate queries pass. These are not traffic SLOs or proof of native
  recovery/complete indexing.
- [ ] Repository-wide CI is not claimed PASS. The normal merge returned 405 while
  its required Woodpecker context was pending. The user explicitly authorized
  overriding workflow gates; the merge API override was applied after bounded
  local verification. [CI status](ci-status.json) records this separately.
- [ ] Physical-device check assigned through the [cl8y-pm inbox](https://pm.cl8y.com/inbox),
  card `3fff415d-9cc2-42b1-b635-37e0be4fd0f7` ([receipt](operator-card.json)).
  It requests one physical-phone reachability check and no transaction signing.
  The complete funded/assistive-device matrices remain unperformed, so #1560
  and #1561 remain open. No chain signing or privileged sudo operation was needed
  for the implemented frontend release.

The follow-up evidence/observer-helper commit changes no deployed frontend files;
the runtime remains the independently attested 6a5118f release. See the
[original baseline](availability-before.json), [prior frontend pin](deployment-before.json)
and [pre-release monitoring](monitoring-before.json) for comparison.

## Mobile shell correction

The prior release still rendered the checked None status despite the audit intent above. The [mobile centering/compact route follow-up](MOBILE-CENTERING.md) corrects it and supersedes the older mobile footer placement. Historical screenshots and receipts above remain unchanged.
