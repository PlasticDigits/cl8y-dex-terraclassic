# Offline-budget implementation and scoped verification

Issue: [#1480](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1480).
Implementation PR: [#1481](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1481).
Policy: [OB1480](../../../skills/AGENTS_OFFLINE_SWAP_BUDGET.md).
Checklist: [README](README.md). Source started at
`2d1627ffa33eb3e08199acfff574a5b63fcfbd39` in an isolated worktree.

Owned optimized current pair/router pool-route measurements reproduce legacy
underbudgeting: four hops use about 8.66M gas against the historical 3.81M limit.
Local execution now uses **800k+2.4M×hops legacy** and **800k+1.8M×hops funding-v1**,
plus existing applicable book/wrapper work costs. Final client gas remains bounded
at 15M. Historical non-local auto-gas/inventory uses its separate existing policy.
No asset-address exception or quote-driven calibration/simulation was added.

Swap/Trade fee display, native reserve and Max use the reviewed operation family,
including one legacy router hop and real hybrid work. Unknown/malformed/over-limit
families fail before wallet authorization. Fee arithmetic is checked and rounds up.
Local split signing rejects changed gas/uluna amount/payer/granter before broadcast,
and checks original quote expiry before signing and afterward. Serialized router
deadlines cannot exceed that original expiry. Atomic WalletConnect APIs cannot
expose signed fields or post-sign expiry before posting; that limitation remains
explicit. Included failure/unknown-inclusion flows do not auto-resend.

## Funded first and retest

[First](funded-first.json), [retest](funded-retest.json): each **24 successful
settlements+48 intentional rejected hop/final floors** after an owned chain reset.
Both rounds decode requested/signed gas, uluna fee and actual messages, calculate
net output independently from read raw reserves, check positive 5% per-hop/final
floors and exact delivered balance delta. Rejected floors preserve all fixture
CW20 balances, reserves and LP shares. Every successful gas budget has ≥20% measured
headroom. Both rounds bind identical production budget-source hash.

| Hops | Legacy gas limit | Funding-v1 gas limit |
| --- | ---: | ---: |
|1|3,200,000|2,600,000|
|2|5,600,000|4,400,000|
|3|8,000,000|6,200,000|
|4|10,400,000|8,000,000|

These fixtures are plain six-decimal CW20, empty-book, hook-free core routes,
with 0.01/1/1000 human offers. They do not certify production USTR, wrappers,
arbitrary callbacks, tax/books, physical wallets or production settlement.

All 170 contract/package/lock input hashes match the current complete source map.
Artifact bytes match tracked checksums. Original build provenance predates a pair
rebuild; recipe/build provenance remains distinct from contract-input equivalence.
The funded driver's manifest validator was hardened between first/retest: each
receipt keeps its actual driver hash; production budget source is identical and
first's complete input map was independently verified. No retroactive PASS edits.

## Source verification

`make verify-issue-1480`: **PASS WITH SKIPS**, 10 passing groups, zero failures,
three explicit standalone funded/device/broader acceptance scope skips. 348 frontend
tests pass each round, covering actual Swap/Trade builders, requested/signed fee
rewrites, original expiry, Max/native reserve, zero gas-reader calls, recovery,
sequence/concurrent signing, impact/Expert and fixed out-of-gas copy. Separate
funded receipts above complete their documented core subset. Type checking passes.

Gas regressions `1360`, `1328`, `1264` passed. `1222` initially failed its obsolete
production-file prohibition; the explicit measured follow-up now replaces that
hold with functional canonical offline/auto-gas boundary tests. Retest passed all
nine groups. The #1264 runner executes actual Swap tests and offline admission,
removing its earlier empty name filter. Historical measurements/FAILs stay scoped.

Monitoring guard tests: 3, funded receipt/provenance guards: 6, quote probe safety: 11
and browser-helper DOM/verdict: 11 passed. Independent review covered fee/expiry,
privacy, fixed read caps and documentation limits.

## Retained failures and outstanding acceptance

Early source-fingerprint failures reflect concurrent unfinished implementation;
all source functional tests passed again on stable final scope. Funded fixture
startup race and CLI inclusion acquisition timeout were fixed with bounded owned
readiness and original-hash REST reconciliation. Accepted/unknown transactions were
never rebuilt/resubmitted. Earlier failed logs remain retained. Controlled read-only
live quotes and production deployment are separate evidence below.

Original reporter wallet/version/amount/signed-transaction details remain unavailable.
Named native/USTR, tax/hook/populated-book worst cases, full 1/5/30% mixed-decimal
funded matrix, physical wallets and production contract provenance are incomplete.
None is assigned a fabricated PASS or silently waived by the calibrated core model.
The issue stays open unless every body/comment criterion passes.

## Additional owned settlement scope

[Taxed routes and atomic invoice summary](funded-tax-invoice-summary.json):six
route-local-tax scenarios and two exact invoice/rollback flows passed using current
tracked artifacts and synthetic owned accounts. Full private receipts retain
synthetic transaction linkage; only actor-free aggregate checks are published.
These run at12M fixture gas, not the calibrated retail tax/native envelope.
[Mixed 6/18/9-decimal net protection](../issue-1479/funded-mixed-net-protection.json)
also passed24 cases at1%/5%/30%; its fee budget is separate from wallet acceptance.

## Independent review and pending physical approval

Final review found that Swap disabled the button for fee coverage/native reserve
failures but did not recheck them at mutation entry. The narrow mutation-entry
repair and actual-entry regression are verified separately before publication;
Trade already checks its native gas gate. Earlier completed source results retain
their pre-review fingerprint; interrupted checks never become final acceptance.

The original report's `make verify-issue-1257` remains **FAIL**: the isolated
canonical-identity retest passed39 tests, while historical source-grep/top-five
solver documentation failures remain unresolved. This does not establish a gas
cause or a passing full1257 verification.

Physical mobile fee approval is pending in [the PM inbox](https://pm.cl8y.com/inbox),
card `f2041acd-86a0-4a86-9b90-469196638b90`, linked through
[PM tracking #143](https://git.cl8y.com/PlasticDigits/cl8y-pm/issues/143) after the
DEX repository's structured link was rejected by the configured allowlist. Its
one action compares the displayed network fee with wallet approval, then cancels
without signing. This is not funded settlement or atomic WalletConnect signed-fee
inspection; both remain incomplete.


## Final reviewed source

[Final source receipt](../issue-1479/source-final.json) records all four stable-source entrypoints after
both independent review repairs. #1479: 12 passing groups/zero failures/one explicit
acceptance skip (11 Rust,149 frontend,24 socket invocations per round).
#1480: 10 passing groups/zero failures/three scope skips (351 frontend per round).
Canonical #1452 passes all 14 groups (785 library plus benchmark,95 socket
invocations and 479 frontend per round); controlled first/retest maxima 55.9/53.6 ms
meet 500 ms. #1473 passes 17 groups with two acceptance skips (785 library,81 socket
invocations and 437 frontend per round). Its complete 4250-case controlled matrix
has 3948 useful quotes, zero provider calls and useful p 50/p 95/p 99/max
89.7/147.7/166.7/212.6 ms against 750 ms.

These final checks use owned tmpfs browser profile/build scratch and unchanged
measurement clocks/budgets. Earlier normal-host FAILs remain retained. The previous
completed 1452 run belongs to pre-review source;1473 was explicitly interrupted for
the review fixes and has no final verdict. Controlled source success does not
establish live deployed acceptance or physical wallets.
