# Release operations #1421 — 2026-10-06

This receipt separates source/build checks, dedicated synthetic-chain execution,
actual hosted policy deployment, and the remaining encrypted-keyring broadcasts.
The integrated contract release remains factory 1.12.0, pair 1.21.0, router 1.5.0,
community-tax token 1.4.0 and AutoLP 1.1.0. Bridge-Debt issuer/receipt targets 1.1.0.

The signed Woodpecker policy was deployed from exact `fab88dbbccb514657d786e3034e12c99423026eb`.
Its `/health` matched the independently built policy bundle SHA256
`02a9b1282c318bd9dae2ac6a003e2eb7528998a33ad8a8d866332fc0cfde706c`;
unsigned configuration requests returned 401. Repo 7 retained exclusive signed
configuration, `netrc=false`, fork approval, timeout 120 minutes and no trusted
volumes/security. The existing exact case-insensitive public Venus allowlist
passes; no scanner exclusion was broadened.

Genuine hosted pipeline [336](https://ci.cl8y.com/repos/7/pipeline/336/1) passed
scanner, contracts, frontend and indexer library, then failed the PostgreSQL
`poller_advisory_lock_is_exclusive` test. Dropping the connection raced PostgreSQL
backend teardown. The test now closes gracefully and waits at most 5 seconds for
the second session to acquire the lock, retaining the assertion that an active
holder excludes it. The entire 12-check ingestion suite passed twice, and the
lock test passed 20 independent processes using separate disposable databases.
The 22 verified inactive owned child databases were removed; runtime/control
schemas and unrelated clusters were preserved. The initial hosted failure
remains an actual failure; post-repair hosted acceptance needs a genuine result
on the final reviewed commit.

Native cold builds exposed two sources of nondeterminism: random target paths
in generated Swagger code and extracted asset creation/modified timestamps.
The release helper now atomically creates a stable empty target, refuses any
existing target, remaps that path, and fixes `SOURCE_DATE_EPOCH=0`. The zero epoch
means normalized asset metadata, not a release date. The Docker builder uses the
same epoch. Existing executable output cannot satisfy either cold build. Nine
source/cache/provenance tests pass, including preservation of another build's
existing directory and rejection of an inherited timestamp. Two independent
host Rust 1.99.0 builds have identical native bytes. Two cold production builds
from the pinned Rust 1.96 image also produced identical stripped binaries, using
Rust/Cargo 1.96.1 and the same 264 reviewed inputs. Their final formatted-source artifact and
package versions are recorded in [native-production.json](native-production.json).
Both builds used the final formatted backend source, and every captured input
was checked unchanged afterwards. These builds precede the final Git commit;
the receipt records exact source bytes rather than claiming a later commit was
already built.
This is native-binary reproduction, not whole-image reproducibility.

A dedicated checkout, loopback-only LocalTerra/PostgreSQL project and separated
runtime/migrator/QA roles were provisioned at
`~/.local/share/cl8y-dex-functional1400`. Locked USTR native-wrap fixtures were
cold-built and deployed. Fresh pinned optimizer builds match all 13 reviewed
owned hashes. The daily user timer `cl8y-dex-functional1400.timer` is enabled for
03:00 UTC with linger enabled. Two uninterrupted critical policy executions
passed all funded-route/invoice/native-wrap/unwrap rollback scenarios and the
four selected simulated-wallet signing/hybrid/limit browser cases, with zero
unexpected, flaky or skipped cases. A further full run passed after aligning the fixture to the final pinned Rust
1.96 native bytes; its four browser cases had zero unexpected, flaky or skipped
results. The running indexer process matches that final independently reproduced
artifact. This run records current dirty source bytes at the earlier fixture Git
HEAD, rather than claiming final-commit deployment. Earlier controlled interruptions are retained
as partial failures, never passed receipts. Raw browser reports stay private.

Concrete maximum-eight-hook gas was measured twice on independent fixtures:
baseline swap 632,671; eight 1 bps burn hooks 3,494,375; AutoLP skim with those same
hooks 4,844,182. Quotes delivered exactly 996,206 raw units and the skim minted
498,107 LP units without refreshing policy. Missing signed floors rolled back
all recorded economic state. These are concrete optimized-wasm transactions,
not a gas promise for arbitrary callbacks or a production provider query envelope.
`make verify-issue-1398` also passed 7 checks with zero failures/skips.

Bridge-Debt read-only discovery found issuer 11709 and receipt 11708, both cw2
1.0.0, with the actual issuer owner and both wasm admins equal to the available
2-of-3 multisig. Receipt supply, debtor balance and outstanding each equal
10,000 USDT in 18-decimal raw units. The prepared
[Bridge-Debt adapter](../../../scripts/ops/2026-10-06/BRIDGE-DEBT-README.md) uses
explicit reviewed market pins, finite caps, preserved accounting and paused new
issuance. It journals exact signed bytes before a single broadcast, reconciles
unknown inclusion, skips unchanged settings, and limits total fees to 2,000 LUNC.
Two resume/accounting regressions pass; read-only plan/replan passed. Actual
migration, one-raw-unit repayment and original-payer recovery remain unexecuted
because the encrypted file keyring has no unattended unlock. This does not
establish collateral backing or waive the observed multisig custody.

To rerun the installed critical policy and inspect its actual result:

```bash
systemctl --user start cl8y-dex-functional1400.service
systemctl --user status cl8y-dex-functional1400.service --no-pager
journalctl --user -u cl8y-dex-functional1400.service -n 30 --no-pager
```

Production rollout acceptance and issue closure remain separate from these
receipts and require the pending custody operations and final live readbacks.
