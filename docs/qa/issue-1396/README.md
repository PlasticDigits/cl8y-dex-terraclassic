# Issue #1396 agent review and verification

Issue: [#1396](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1396) and its
2026-10-05 comment confirm that the earlier audit was evidence, not remediation.
Preserve [DB1396-1–DB1396-10](../../../skills/AGENTS_INDEXER_DATABASE_ISOLATION.md).
Implementation/transition/rollback: [runbook](../../runbooks/indexer-database-isolation.md).
Evidence: [verification receipt](verification.md).

## Agent procedure and explicit skip policy

1. Record UTC time, exact revision (dirty worktree scope if applicable), commands
   and outcomes. Read the issue/comments, invariant playbook and runbook.
2. Provision a **fresh disposable** QA cluster with distinct runtime/migrator/QA
   identities and nondefault private credentials. Run
   `scripts/setup-postgres-dev-databases.sh`; never reset or migrate shared or
   production databases as a shortcut. The ACL regression provisions its own
   isolated cluster. Integration helpers require an admin-owned marked
   `dex_indexer_test` control database and a `dex_indexer_qa` CREATEDB credential.
3. Run `make verify-issue-1396`. It builds once, retests the same Rust executables,
   repeats ACL/network/CI source checks and checks shell syntax/cross-links.
   To exercise the optional authenticated-runtime fixture too, privately inject
   TEST_DATABASE_RUNTIME_PASSWORD matching the dedicated fixture role. Never
   print passwords or DSNs. Tests retain child DBs for inspection; dispose only
   of your dedicated cluster or exact verified inactive QA children afterward.
4. Review each criterion below against actual receipts. Request another agent's
   independent review when available. Hosted trusted-policy deployment is a
   separate action from local CI source checks.

Complete all runnable checks first. An agent may skip any unavailable
infrastructure, deployment, signing, device or other check by recording **SKIP**,
the reason, missing evidence and bounded followup. The overall result may still
be **PASS WITH SKIPS / PASS (available checks; skips recorded)**. Corresponding
criteria stay unchecked; a skip is never proof of production success. Use
VERIFY_ISSUE_1396_SKIP_RUST_REASON, _SKIP_DB_REASON, _SKIP_ACL_REASON or
_SKIP_NETWORK_REASON or _SKIP_BRIDGE_HOST_REASON (only the default bridge
published-loopback positive query; other real socket probes still run) only for genuinely unavailable capabilities. An executed
assertion failure remains **FAIL** until fixed and successfully rerun; never
convert it into a skip. Docker startup contention can be resolved with tmpfs
**test-only** data, without changing deployment durability settings.

## Acceptance checklist

- [ ] Runtime role: authenticated attributes/memberships/ownership are safe;
  DML and sequences work; CREATE ROLE/DATABASE/SCHEMA/TABLE, ALTER, TRUNCATE,
  migration ledger/schema-marker mutations, extension/file/program access and
  unrelated schema/database access are denied by real ACLs.
- [ ] Migrator/QA roles: migration runs in a separate process; protected version
  marker and runtime grant reconciliation work; QA can create disposable DBs
  and cannot reach the runtime application DB. Unsafe/legacy roles/owners fail
  closed; grants are verified and rollback is documented.
- [ ] Networking: render base and host opt-in config; real loopback queries work;
  another namespace cannot reach a non-loopback host interface; opt-in listens
  on IPv4/IPv6 loopback only; missing/default bootstrap passwords fail without
  printing values. No automatic ignored host override in repo commands.
- [ ] Seed/clean: actual executable's prod, unknown/missing modes, spoofed target,
  help/invalid/missing args and privileged server env cases do not contact the
  TCP listener. Authenticated isolated dev/QA seed and clean succeed; missing
  marker rejects before mutation. Seed never migrates.
- [ ] Worktree isolation: concurrent child processes have distinct fresh DBs;
  parent sentinel and control migration history stay unchanged; strict migration
  checks and cleanup reject the shared control DB. Rerun representative API suites.
- [ ] Documentation and maintainability: review modular guard/security/helper
  code, secret-free env handling, invariant cross-links, staged transition and
  rollback instructions. Check diff whitespace and CI failure tests.
- [ ] Production acceptance (separate): record actual deployed roles/grants,
  ownership/credential rotation, migration job/server isolation and live network
  reachability. Local tests do not check this box.
- [ ] Hosted CI acceptance (separate): rebuild/review/deploy the immutable signed
  Woodpecker policy containing the updated QA bootstrap; verify its policy hash
  and actual hosted PR logs. PR-owned workflow changes alone do not update the
  trusted configuration extension.

## 2026-10-07 close reconfirmation

Issue #1396 stays closed on the source receipt in
[verification.md](verification.md): 28 PASS, 0 FAIL, 2 explicit host-network
skips. This pass did not rebuild that Rust suite.

The production role transition is the existing
[production-maintenance.json](../issue-1385/production-maintenance.json) receipt:
61 migrations, schema marker `20261006133000`, history preserved, and four
runtime permission denials. Release coordination is
[REL1421-4](../../../skills/AGENTS_RELEASE_OPS_1421.md). Local
`make verify-issue-1396` still does not replace that production receipt.
