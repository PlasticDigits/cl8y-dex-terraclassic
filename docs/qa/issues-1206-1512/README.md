# Coordinated issue investigation and release

Bodies and all comments for #1206, #1211, #1229, #1330, #1505 and #1512 were
read through `fj` with credentials injected by openbao-tools `bao-exec`.
Implementation uses an isolated Git worktree from main
`0a3c251755a88ee9cbbbbc222fc747811c9da08c`.

Source/checklist evidence:
[anonymous digest](../issue-1206/README.md),
[fee cohorts](../issue-1211/README.md),
[launcher origin](../issue-1229/README.md),
[Swap campaigns/routes](../issue-1330/README.md),
[native recovery](../issue-1505/README.md),
[USD display](../issue-1512/README.md).
The [QA signing-window correction](../issue-1505/RESULTS.md#observer-signing-window-correction-2026-10-18)
retains the original failed baseline and a separately recorded corrected bounded
probe against the unchanged deployment; it does not replace sustained acceptance.

The owner explained that temporary HTTPS/SSH outages during this work were
server upgrades. No hosting/server recovery or shared infrastructure reset was
attempted. Local QA failures reset only process-owned disposable databases.
All locally invoked focused test processes are below 60 seconds; compilation and
provisioning are separate. Repository-mandated hosted full matrix execution is
separate. Long #1505 four-by-190s acceptance windows remain explicitly
unavailable under that test-duration constraint; shortened probes cannot replace
them or erase the retained historical failed RETEST.

[Monitoring before release](monitoring-before.json) is a bounded 15-minute
Grafana/Loki read at 2026-10-18T04:18Z, before these fixes. It records 82 wrapper
completions, three unavailability events and existing provider/deadline pressure.
These are text-pattern counts, not proof of causality or successful acceptance.
No Prometheus datasource is provisioned, so its check is explicitly SKIP.
No raw log lines, actor identifiers, provider URLs or credentials are exported.

Third-party route design is [documented](../../architecture/third-party-route-adapters.md).
Adapters and actual flat 2.9% settlement remain unimplemented; unsupported routes
cannot sign. A source PR or database/API deployment does not establish on-chain
#1229 migration or third-party execution. Issue closure requires every applicable
body/comment criterion, with all remaining failures/skips listed in its comment.

The requested custody handoff was submitted to cl8y-pm as desktop chain-script
card `98602aa7-5803-477e-97bd-8551291a6919` ([submission](operator-card.json)).
Its single command consumes the encrypted keyring passphrase through the existing
concealed prompt; no passphrase is in the command, card or evidence. The PM app's
documented `CL8Y_PM_FORGEJO_LINK_REPOS=code/*` link-only setting was enabled and
redeployed because the API initially rejected the DEX issue link. Timeline and
close-comment repository permissions were unchanged. A 201 response confirms
creation; it does not establish completion or on-chain migration.

Commit hooks caught an unnecessary reference in the token origin comparison and
public checksum map entries classified as generic API keys. The comparison was
corrected and source tests/clippy repeated; optimized artifacts were rebuilt from
that source. The committed public build receipt exports filename/digest maps as
explicit `path`/`sha256` rows. The canonical generated receipt remains in the
owned build output; no scanner exemption or hook bypass was added.

The final cohort HTTP check reproduced a new-field formatting defect: BigDecimal
Display emits scientific notation for tiny values. Only the new cohort USD/share
mappers now use the existing plain-decimal formatter. The exact 1e-18 HTTP fixture
failed before and passes afterward across all three windows. The focused backend
count is now 71, including nine cohort tests; existing fee-source/token shapes
remain unchanged. Public deployment checks use the [bounded API smoke](../../../scripts/qa/live-evidence-1206-1211.py)
and [separate desktop/phone browser cases](../../../scripts/qa/browser-price-campaign-1512.cjs).

Final pre-deployment follow-ups add the real mapper `depositor` binding, initialize
all cohort windows from the prior published fee snapshot (retaining its clock),
and fix a campaign-test URL-settlement race found by genuine hosted CI. Focused
backend verification is now 72 checks plus the actual owned-chain ingestion/API
case. The full query-parameter file passes 57 tests. The standalone migration
adds 19 control checks and a real 11.957s SQLx 67→68 rehearsal with runtime login,
four permission denials, exact history preservation, idempotent replay and full
snapshot restore. [Rehearsal provenance](../issue-1211/local-migration-rehearsal.json)
explicitly identifies a dirty worktree based on the checkout, rather than claiming
a deployed release image. The production job must compile the exact merged main.
