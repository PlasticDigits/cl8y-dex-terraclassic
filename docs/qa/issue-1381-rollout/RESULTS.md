# Rollout automation verification

Implementation branch: `feat/1381-production-rollout`, based on main `748579ab`.
Checks ran on 2026-10-04 UTC. This adds the production contract rollout requested
after [#1381](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1381) /
[PR #1383](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1383).
[Agent checklist](README.md), [runbook](../../runbooks/route-local-tax-1381.md),
[rollout invariants](../../../skills/AGENTS_ROUTE_TAX_ROLLOUT.md).

- **59 behavioral tests passed and retested.** Coverage includes inventory,
  artifact/version/admin binding, missing/spoofed receipts, zero-rate tokens,
  partial failure containment, original pause restoration, primary receipt and
  saved transaction reconciliation, finite fees, exclusive state locking,
  verified-only application handoff, key membership, signed body/fees/modes,
  preview conservation/maximal rounding and credential handling.
- **884 contract tests passed**, all workspace targets, after the factory CW2
  metadata bump to **1.11.0**. Factory contract behavior is unchanged relative to
  repository source; the version bump permits an actual upgrade from production
  1.10.0 instead of attempting a rejected same-version migration.
- Python compilation, shell syntax, CLI help, document crosslinks and diff checks
  passed. The standard contract workspace Clippy check passed.
- Optimized wasm was rebuilt, with the reviewed factory 1.11.0 checksum
  `2a7efd5c74f774a623406b5fc510b285d52de23a5f0fd3d2277a82aa65e634da`.

## Actual SDK / LocalTerra evidence

Fresh isolated rehearsals and retests passed; the final release-artifact rehearsal
included **18 actual rollout transactions**, **18 durable prebroadcast checkpoints**
and a successful token creation through the rotated launcher, with **zero skips**.
[Public synthetic receipts](sdk-evidence.json).

Verified actual SDK behavior:

- Synthetic multisig membership order **1/3/2**, signing with members **1/2**;
  public keys, offline account/sequence, Amino modes and signer bitmap validated.
- Factory **1.10.0→1.11.0**, pair **1.15.0→1.19.0**, router and community tokens
  **1.1.0→1.2.0**; actual empty migration messages and version guards.
- Isolated CMM treasury **0.2.1→0.2.2**, then real `migrate_owned_contract` for
  its owned token. Zero-rate unpaired community-token discovery and migration.
- Preserved configuration/features/supply/reserves, retained whitelist entries,
  new pair/template references, correct live asset pins and original pause states.
- Successful buy/sell previews, maximal integer sell budget and tax conservation.
- Single-key and 2-of-3 signing/encoding, exact signed intent/fees, saved bytes
  re-encoded and matched to primary included receipts on resume.
- Repeated execution after completion was rejected without broadcasts;
  repeated finalization/verification remained idempotent. New free-token creation
  used the upgraded code and advertised funding version 1.

Only new synthetic test-keyring accounts/contracts and LocalTerra funds were used.
Shared production keys, deployed application contracts and chain volumes were
not modified. The real SDK fixture explicitly verifies its external ordinary CW20
artifact and legacy code hashes before constructing the isolated fleet.

## Read-only production preflight

The default script completed a public Columbus-5 inventory with **22 pairs** and
**two managed community-tax instances**, including ALPHA and KENA. It did not
unlock a keyring or broadcast. The immutable plan/journal remains outside git.

Observed existing deployments:

| Component | Code | CW2 |
|---|---:|---|
| Factory | 11629 | 1.10.0 |
| Router | 11640 | 1.1.0 |
| Pair sample UST1/USTR | 11676 | 1.18.0 |
| Launcher | 11632 | 1.0.0 |
| CMM treasury | 11667 | 0.2.2 |

Production factory code 11629 hash
`64abbccbd801bd399ec9f246dc7bc350531b5eea66a6ea3d53a4fac3ff22982c`
differs from the earlier repository 1.10.0 artifact. The genuine 1.11.0 release
therefore provides an increasing-version migration path. The existing launcher
supports token-template rotation without a forced same-version wasm migration;
CMM already exposes the required owned-contract dispatcher.

## Deployment boundary

**No production deployment, signing, migration or live swap was performed.** This
request creates a reviewed script and PR. An operator can later run `--execute`
with the supplied local keys; the encrypted keyring passphrase stays in memory
and is never an argument, runtime record or committed secret. Contracts are kept
paused until the matching indexer/frontend configuration is deployed and explicit
`--finalize` restores the original pauses.

Production AC9 remains unverified until actual production receipts and bounded
execution evidence are recorded. Unavailable future operator/device checks may
be explicit skips under the agent checklist; executed failures remain failures.
