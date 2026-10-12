# Route attribution results (#1483)

Issue: [#1483](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1483).
Implementation PR and final revision will be linked in the issue completion
comment. Follow [the checklist](README.md) and
[RC1483](../../../skills/AGENTS_ROUTE_COST_ATTRIBUTION.md).

## Supported source scope

Generic signed raw-output attribution retains independent fair-reference quality,
the exact aggregate warning, authorized native gross and original expiry. One
same-cohort synthetic fixture independently calculates 6/18-decimal constant
product/fee outputs for direct and eligible UST1/CL8Y alternatives. It warms a
small quote before large inputs and repeats the small input, testing changing
amount-specific winners rather than scaling a cached route. Native 2% wrapping
is compared with matching wrapped credit and equal wrapped gross separately.
These are owned synthetic inputs, not a historical report reconstruction.

Exact verification counts, UTC/source fingerprints, deployment/tool/artifact
provenance, timing distributions and failures will be appended after final
stable first/retest and deployment. Test logs stay private when they may contain
synthetic actor or local endpoint fields; public summaries retain bounded counts.

## Retained failures and unavailable acceptance

SKIP: original reported amount/protection/cohort/frontend/wallet is unavailable;
no similar percentage establishes historical reproduction. Original atomic
production SQL and physical-wallet/device signing are separate missing evidence.
Complete native/unwrap/tax/hook/book/attack acceptance and deployment are not
established merely by the focused attribution fixture. Gas calibration remains
[#1480](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1480), independent
small-market/oracle integrity remains
[#1479](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1479), and external
DEX/splitting design remains
[#1330](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1330).

## First verification and retained failures

At 2026-10-12 UTC, the first independent amount-cache fixture passed on the
reviewed working-tree patch based on `d65228c2970a1ec7b11d81000d3f83ceb58132c4`.
Its initialized useful socket sample (500ms budget) was n=14, p50=1.420ms,
p95/p99=1.666ms, maximum=11.772ms. Follow-up assertions independently check every
raw attribution component from exact original reserves/marks and actual fee math.

Retained failures: concurrent unfinished source caused source-fingerprint FAILs;
an old frontend copy assertion and two unused imports caused the first #1480
frontend/type FAIL and were corrected. A first broad #1483 run also failed the
existing pending-fee provider fixture's three-second gate on its last case during
concurrent testing. The failure remains recorded; successful fresh #1479 runs of
the same original test are tracked separately and no production TTL was relaxed.
Final stable source first/retest verdicts follow below.

A further independent regression reproduced an actual ranking defect: the
bounded deeper worker evaluated a strictly higher-net three-hop route but did
not publish it when its improvement was below the undocumented 0.1% threshold.
The exact fixture's initial direct output is 997,000 raw, with a three-hop output
of 997,498 raw on the same original cohort. The red run retained
the initial inferior output through 100 bounded cache requests and failed. The
fix compares strict positive raw output without changing work caps or reviewed
transaction binding; green/retest receipts follow below. The user's separate
small-amount-cache hypothesis was tested with changing direct/two-hop winners:
exact amount/generation keys already prevent linear scaling or stale winners
from deciding the larger amount in the supported bounded cohort.

## Owned funded pool subset

The [first funded receipt summary](funded-offline-first-summary.json) records
24 supported CW20 pool-only, hook-free, empty-book settlements and 48 intentionally
rejected hop/final floors. Legacy and route-local-v1 each cover 1–4 hops and raw
inputs 10,000/1,000,000/1,000,000,000 in six-decimal assets. Independently
calculated delivered net/floors, original wallet principal, reserves and LP rollback
pass. Requested/signed gas agrees; gas used remains below the unchanged budget
with the fixture's required 20% headroom. Network fees are separate evidence.
Tracked artifacts and complete current contract/package/lock inputs match the
provenance preflight; original build recipe/production artifacts are not re-attested.
This subset does not establish native composition, tax/hooks/books or device
acceptance. Only the explicitly owned isolated chain was created; no shared or
production state was reset.
