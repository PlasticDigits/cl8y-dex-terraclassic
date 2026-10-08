# Implementation verification (#1354)

Source implementation and live acceptance are recorded separately. Worktree: `fix/issue-1354-reconciliation`, based on investigation commit `09039309` and main `48f01fd1`; verification date 2026-10-08 UTC. [Earlier failing investigation](investigation-verification.md) remains historical evidence.

Implemented positive partial volume with exact completeness, actual pair/event fee scope with explicit legacy ambiguity, atomic historical repair, creation/SKU/settings source ingest and guarded replay, fixed UI labels, bounded drift scheduling and correction readbacks. Fee fixtures now provide #1391 oracle quorum/confirmed hub prerequisites. Swagger 404 was a shared-target embedded-asset cache pointing to a removed worktree; targeted `utoipa-swagger-ui` rebuild restored the actual asset route without weakening its test.

## Available evidence

- DeFiLlama integration: 17/17 passed and retested from the same fresh executable.
- Economic fee integration: 8/8 passed with the corrected oracle fixture.
- Security integration: 43/43 passed; Swagger additionally passed its targeted check.
- Drift receipt boundaries: 5/5 passed, including exact decimals/nulls and historical readback corrections.
- User systemd timer enabled for 01:15 UTC plus up to five minutes jitter. Its actual first run checked 52 completed days, recorded five live findings and exited 1, as designed. JSON is in the local user state directory and service failure is in the journal. External alert delivery is separate.

Available source verification: **PASS WITH SKIPS**; production accounting acceptance remains uncompleted. The public drift diagnostic separately remains **FAIL** on five actual findings from the serving older revision.

| Command/check | Result |
|---|---|
| `make verify-issue-1354` with explicit undeployed/access/upstream live reason, first/retest | 10 PASS, 0 FAIL, 1 live acceptance SKIP, repeated |
| `make verify-issue-1209` / `make verify-issue-1210` shared harness | 5 PASS, 0 FAIL (parser/ledger/API/series/catalog/UI); shared suite run in #1354 |
| `make verify-issue-687`, including #631 and scoped #683 | 7 PASS, 0 FAIL; #631 5/5, #683 6/6 |
| `make verify-issue-1396`, first/retest | 28 PASS, 0 FAIL, 3 verifier SKIP; optional runtime skip separately closed by PASS twice, leaving two host-network skips |
| `make verify-issue-1395` | PASS: full library twice (581 passed, one ignored), exact-amount integrity twice and all selected economic/replay regressions |
| Historical repair / DeFiLlama targeted first/retest after auxiliary preservation | 3/3 and 17/17, repeated |
| Creation/invoice ledger/API first/retest | 31/31 twice; actual captured paid LocalTerra receipts included |
| Frontend source/page tests | 60/60; source label tests 3/3 retested |
| TypeScript `tsc -b --pretty false` | PASS after correcting optional legacy event-count handling |
| Original failing acceptance probes | 2/2 now PASS (temporary source removed by harness) |
| Bounded read-only SQL on an owned marked migrated child | PASS; no production connection or writes |
| #614 source regressions | 6 PASS, 0 FAIL, explicit fresh-production-mint/redeem acceptance SKIP |
| #594 catalog / #1213 child-owned home map | 3/3 and 4/4 PASS |

The actual native creation receipt emits 100,000,000 uluna; the actual create-time SKU receipt emits 50,000,000 UST1. Both exact successful LocalTerra receipts pass block ingestion into disposable QA databases and replay twice. Invoice transport failure propagates for retry instead of silently consuming the block. The captured events and read-only capture instructions are in [fixtures](../../../indexer/tests/fixtures/issue-1210/README.md). No real settings-batch receipt was available; synthetic settings/provenance coverage passes.

Resolved verification failures: the historical USD test compared decimal strings with different scales, corrected to numeric equality; the optional legacy UI event count needed a default for TypeScript; stale home-map/source grep checks were updated to current child-owned implementation. The original 512 MB disposable cluster exited while retaining multiple QA databases; a fresh 4 GB cluster resolved EOF/timeout failures without weakening assertions. Prior failed logs are not substituted for the successful retest receipts.

The companion protocol browser density test previously used an unavailable localhost indexer and obsolete copy. Its scoped fixture now supplies populated valid UTC grains and keeps the density/width assertions. The complete `make verify-issue-586` bundle passed twice: 7 checks, 0 failures, including 13/13 five-worker Playwright tests each time, 62 RTL tests, 38 fee units, 31 fee integrations and two overview tests. Recursive unrelated bundles were omitted using the existing explicit flag; required companion checks were run separately. No browser check was skipped.

Known live #614 diagnostic: configured mint/redeem sources had zero events in the current trailing 24h window. Its legacy live gate requires new paid production activity, which was not generated. That acceptance is explicitly skipped; source/parser tests passed and no live paid-event claim is inferred.

## Explicit uncompleted acceptance

Production SQL authentication still fails for the actual Coolify server and all three existing noninteractive SSH identities. No production DB credentials were exported or maintenance/deployment run. Historical repair/reload and verified CW20 price onboarding therefore remain uncompleted. The serving public revision remains the earlier implementation; its diagnostic failures are preserved independently of source-only verification skips.

TVL remains on-chain factory reserves. No UST1/USTR/CL8Y peg or historical mark was invented, and no same-block TVL target or CL8Y/ALPHA USD price acceptance is inferred. Completed #631/#687 remain closed; closed docs-only #1209/#1210 were reopened to preserve accurate tracking of paid-source rollout/replay acceptance.

[Agent checklist and explicit skip policy](README.md) permits unavailable acceptance checks to be SKIP and overall PASS WITH SKIPS when available checks pass. Executed failures remain FAIL until fixed and retested; local/fixture PASS never establishes deployment, upstream reload or price acceptance.

The final reviewed #1354 bundle also passed 10/10 available checks after auxiliary history preservation; its single live-only skip retains the known public FAIL receipt. The scoped read-only SQL report was rerun successfully after selecting a fully migrated owned child (an earlier optional-security-only child lacked ledger tables). All executed source/fixture failures are resolved and retested. Independent agents reviewed accounting, provenance, infrastructure retry, historical retention, snapshots, resource bounds and fixtures.

Required production followups stay with #1354/#1209/#1210: apply the two additive migrations through the separate migrator, roll out source/frontend, reconcile/replay paid history with authorized access, reaggregate the affected completed days, obtain upstream fee reload and verified CW20 prices/pinned reserve evidence. No human approval/signing handoff or fabricated deployed PASS is introduced.
