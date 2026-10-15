# Agent playbook: LCD and treasury chain attestation (#1392)

Implementation: [PR #1413](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1413).

Use when changing indexer LCD reads, discovery caches, network health, treasury
endpoint selection or host transaction helpers. Issue [#1392](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1392)
tracks implementation; the audit report alone did not remediate it.

Read the [invariants](../docs/indexer-invariants.md#chain-attestation-1392),
[endpoint policy/runbook](../docs/runbooks/chain-attestation.md),
[agent verification checklist](../docs/qa/issue-1392/README.md) and
[recorded results](../docs/qa/issue-1392/verification.md).

## Local quote observation boundary (#1452)

[Local quotes](AGENTS_LOCAL_SWAP_QUOTES.md) attest chain identity during the
independently scheduled bounded ingestion pass. Quote, progress, references,
refresh and retail route preparation consume that qualified local generation
with original observation times and client/chain isolation. They must not call
`require_verified_chain` or financial smart queries on a cache miss/hit, and must
fail explicitly when required cached evidence is unavailable. The historical
request-time quote middleware rule is superseded for these local endpoints;
host treasury/maintenance transaction guards below retain their own trust checks.
See [scope and verification](../docs/qa/issue-1452/README.md).

## NET1392 invariants

1. **NET1392-1 — explicit expectation.** Require a configured expected chain in
   every indexer mode. Mainnet is `columbus-5`; LocalTerra is explicitly
   `localterra`. Never learn the expected network from the untrusted response.
2. **NET1392-2 — trust before use.** Attest each LCD before consuming its data;
   wrong, missing, malformed or unavailable identity cannot fill caches. Block
   payload chain IDs independently agree with the configured expectation;
   historical timestamps remain readable.
3. **NET1392-3 — failover/recovery.** Every alternative and recovered endpoint
   passes the same checks. A timeout never means identity validation succeeded.
4. **NET1392-4 — qualified metadata.** Cache identity includes the expected
   network, and health distinguishes expected identity from observed attestation.
   Historical observations do not mean current availability.
5. **NET1392-5 — fresh views.** Treasury requires fresh positive-height LCD/RPC
   views on the expected chain with bounded disagreement. Check a shared block
   identity before using financial preflight data.
6. **NET1392-6 — immutable protocol.** Check protocol addresses/code identities
   through both transports; preserve configured code/hash pins and reject changes
   to the independently corroborated in-run baseline. Do not pin ticker names.
7. **NET1392-7 — no transaction on failure.** Guard unsigned generation, sign,
   multisign and broadcast, including direct exec helpers and retries. Failed
   trust preflight cannot be bypassed by `DRY_RUN` or a loop restart.
8. **NET1392-8 — existing boundaries.** Preserve HTTPS verification, custody,
   spend limits, route constraints, address/hash pins and dry-run no-broadcast
   behavior. Do not publish keys, API tokens or endpoint credentials in receipts.
9. **NET1392-9 — residual trust.** A same-chain malicious node can lie. Matching
   LCD/RPC views are corroboration, not light-client verification or proof of
   independent ownership. A baseline is not an externally reviewed hash pin.
10. **NET1392-10 — truthful verification.** Complete runnable checks and retest
    fixes. Agents may report overall PASS with explicit unavailable-item SKIPs;
    leave those criteria unverified. Executed failures remain FAIL. Unit tests
    and read-only probes do not prove deployment or successful live signing.

## Commands and scope

Code: [LCD client](../indexer/src/lcd/mod.rs),
[identity boundary](../indexer/src/lcd/chain_identity.rs),
[treasury attestation](../scripts/lib/chain-attestation.py) and
[host transaction guards](../scripts/lib/terrad-host.sh).

Run `make verify-issue-1392` in an isolated worktree; follow the checklist for
prerequisites and evidence. The verifier runs synthetic failures and retests.
Live probes must remain read-only. Do not modify mainnet state as an attestation
test. Third-party agent players must apply these rules when using the buyback,
LP, treasury rotation and shared `terrad-host` helpers.

For transient provider lifecycle recovery preserve [PR1464](AGENTS_PROVIDER_RECOVERY.md).
Wrong-chain/integrity rejection remains fatal, never retryable availability or success.
[Recovery/readiness](../docs/runbooks/indexer-provider-recovery.md); `make verify-issue-1464`.

Historical block/checkpoint routes and `tx.height`-filtered reads validate requested
body heights before acceptance. Their verified historical height must not enter a
live-tip lag rejection and falsely open circuits after a newer head was observed.
Substituted body heights remain integrity failures; exact pinned smart-query header
validation remains mandatory. Run the #1464 historical-body recovery regression.

For [NR1505](AGENTS_NATIVE_QUOTE_RECOVERY.md), a complete zero-attempt scheduled
scan may wait for ordinary circuit admission within its existing total deadline.
Keep typed deferral separate from integrity errors, never force a probe or reset
backoff, and stop rescanning after an actual attempt. Endpoint metadata locks and
the freshly calculated identity attempt deadline share the original absolute
bound. Recovered providers still require fresh expected-chain attestation before
any data is accepted; direct/request callers retain prompt failure.

When an admitted identity attempt times out, account for the provider failure
through the already-owned provider state before waiting for endpoint metadata.
Bound metadata invalidation by the same original total deadline; cancellation
must neither lose quarantine nor count the same failure twice. Diagnostic
verified metadata is never a substitute for a fresh successful identity read.
