# Agent playbook: router trust lifecycle (#1388)

Use when changing trusted router binding, settlement authorization, migration,
rollout preflight or trading-discovery identity. Issue:
[#1388](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1388).
[Recovery runbook](../docs/runbooks/router-trust-1388.md),
[agent checklist](../docs/qa/issue-1388/README.md),
[funding invariants](AGENTS_ROUTE_LOCAL_TAX.md),
[rollout invariants](AGENTS_ROUTE_TAX_ROLLOUT.md).

## Invariants

1. **RTTRUST1388-1 — preserve addresses.** Immediate rollout migrates the existing
   router in place. Inventory every affected token's stored router, including
   unpaired/zero-rate instances; reject mismatches before any broadcast.
2. **RTTRUST1388-2 — authenticated governance.** Only the token's live wasm admin or live
   governance of its configured factory can rotate or revoke its router. Manager privileges, invoices, caller-supplied
   trader identity and launcher identity confer no rotation permission.
3. **RTTRUST1388-3 — exact candidate.** Replacement requires the expected live
   code ID and full SHA-256 checksum, the same factory, funding version 1 and
   settlement version 1. Require immutable wasm or a live admin matching governance
   queried from the configured factory. Reject attacker custody and active/malformed
   candidates before writing state.
   Capability alone cannot establish that an attacker contract is reviewed code.
4. **RTTRUST1388-4 — atomic revocation.** Remove obsolete router authority and its
   protocol exemption together. Preserve overlapping current protocol roles and
   listed-pair exemptions. Manager-directory exemptions remain separate.
5. **RTTRUST1388-5 — single-use settlement.** Bind token, recipient, amount and
   slot to a persistent monotonic route ID. Reject duplicate slots and older
   routes. Amount, recipient and block height are never nonces. State is bounded
   to the most recent route and at most five slots per token.
6. **RTTRUST1388-6 — recovery floor.** Replacement seeds the token's replay floor
   from the candidate's last route ID. A stale pre-rotation context cannot acquire
   new settlement authority. Router migration preserves the existing counter;
   pre-counter storage initializes compatibly. Counter overflow fails closed.
7. **RTTRUST1388-7 — route-owned funds.** Preserve gross-only funding, exact pair
   credit/hook parity, owned refunds and dust isolation. Failed downstream
   execution rolls back counters, consumption, transfers and tax distributions.
8. **RTTRUST1388-8 — artifact proof.** CW2 version is metadata, not code proof.
   Compare actual code IDs and complete hashes. Same-version different-hash
   artifacts cannot pass reviewed-artifact checks.
9. **RTTRUST1388-9 — current discovery.** Resolve live instance code identity
   before cache reuse, fail closed on identity errors, and reject identity changes
   during discovery. Keep Create/Manage provenance checks independent and strict.
10. **RTTRUST1388-10 — evidence scope.** Run available checks and retests first.
    Agents may report **PASS with explicit skips** for unavailable checks, with
    concrete reasons and unchecked criteria. Executed failures remain FAIL.
    Source or synthetic-chain success never proves production migration.

## Agent procedure

Work in an isolated git worktree. Run `make verify-issue-1388` and
`make verify-issue-1381`; follow the checklist and record revision, UTC time,
commands and receipts. Never change pins, waive taxes, broaden authority or
suppress failing assertions to obtain PASS. Use `bao-exec` from openbao-tools for
inventoried Forgejo credentials; never print credentials or signing material.

LocalTerra fixtures use dedicated synthetic owners/sinks and bounded CheckTx
sequence-rejection recovery. Test `scripts/qa/test-localterra-route-tax-1388.py`;
accepted or uncertain transactions must be reconciled, never automatically resent.
Use a dedicated test container when shared chain state is unavailable.
