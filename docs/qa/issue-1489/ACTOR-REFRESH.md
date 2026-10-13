# Stable actor guard and fee observations

On the released `8500370c` source, a repeated false blacklist result cleared a
fresh 8% discount before the optional fee lookup. A controlled actual admission
refresh changed an anonymous executable quote's generation despite the same final
fee and original expiry. Another actual overlapping refresh allowed an older 8%
completion to replace a newer 4% result. A false query started before a known
block also cleared that newer block on response. These executed RED receipts are
retained privately; selected live-path timestamps alone do not prove equivalent
full production cohorts or identify the production cause.

The correction retains only fresh verified cached fees during unchanged positive
guard refreshes, preserving original deadlines. Actual unblocks and expired fees
use conservative zero discount. The bounded actor record owns one latest-started
fee nonce and authority version. New accepted fee changes invalidate cached
economics immediately; unchanged completions keep the economic identity. A
superseded admission is discarded without treating it as capacity exhaustion.
Current optional-read failure or cancellation revokes fees conservatively; older
cancellation cannot disturb a newer verified result. Guard false responses validate
the authority captured before their query starts under the same store lock.

Controls exercise cached positive preservation, actual anonymous quote continuity,
8%/4% reversed completion, stale prepared publication with extended deadlines,
failed/current/older cancellation, expired cached fees, known/inventory blocks
while a false response waits, and fresh conservative unblocking afterward.
Existing namespace/actor caps, query boundaries, reference quality, native modes,
original expiry, funding and signing controls remain independent.

Run all fourteen named applicable verifiers in [the issue checklist](README.md).
Final source tests and deployed verification are separate; original six `8500370c`
API cohort windows remain UNVERIFIED and original browser budget failures remain
FAIL. [Browser tool scope](BROWSER-LIFECYCLE.md) preserves the original measured
adoption window and read budgets. [RD1489](../../../skills/AGENTS_ROUTE_DISCOVERY.md)
and [indexer invariants](../../indexer-invariants.md) define the combined contract.

Implementation: [fee observations](../../../indexer/src/api/local_quote/actor_fees.rs),
[bounded authority](../../../indexer/src/api/local_quote/actor_versions.rs),
[scheduled admission](../../../indexer/src/api/local_quote/admission.rs).


## Final combined source evidence

[Current source receipt](actor-refresh-final-summary.json) binds product SHA256
`e8382e1294fd51fede902d1a6c8e7912b82ad9fbb4cdf43f208f37e030935506`
and the verified QA lifecycle changes to the modified `8500370c` worktree.
Thirty meaningful focused controls and default-parallel 824 tests pass twice,
with one existing ignored benchmark separately exercised by applicable verifiers.
All fourteen named commands pass **208 groups, zero FAIL, 21 explicit SKIPs**.
The owned chain was reset to the same pinned image and original 1GiB tmpfs
before the repeat; no production infrastructure was reset or signed.

[Fresh complete controlled matrix](actor-refresh-controlled-browser-summary.json)
records 4,250 cases, zero failures/provider requests and 3,948 useful quotes.
p50/p95/p99/maximum useful latency is 94.9/156.1/172.5/264.2ms at the original
750ms clock. This source/owned-fixture evidence does not reclassify prior
production UNVERIFIED or browser FAIL receipts. Final independent deployment
and sustained live acceptance remain separate.
