# Agent playbook: route-tax production rollout (#1381)

Use when preparing, reviewing, executing or resuming the contract rollout for
[issue #1381](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1381).
Protocol funding rules remain **RT1381-1–RT1381-10** in
[AGENTS_ROUTE_LOCAL_TAX.md](AGENTS_ROUTE_LOCAL_TAX.md).

Canonical references: [operator runbook](../docs/runbooks/route-local-tax-1381.md),
[rollout review checklist](../docs/qa/issue-1381-rollout/README.md),
[entrypoint](../scripts/upgrade-1381-route-tax.sh),
[rollout engine](../scripts/lib/route_tax_rollout.py),
[CLI/signing transport](../scripts/lib/route_tax_rollout_cli.py),
[rollback limitations](../docs/runbooks/wasm-admin-migration.md#rollback-and-limitations-sec-h05).

## Rollout invariants

1. **RTROLL1381-1 — deliberate execution.** Planning and probing are read-only.
   Production broadcasts require an operator's explicit execution invocation.
   Adding a deployment script to a PR does not authorize running it on production.
2. **RTROLL1381-2 — exact reviewed identity.** The manifest binds chain, existing
   contract addresses, original code/admin/config/pauses and artifact hashes.
   Do not replace official contract addresses with freshly instantiated contracts
   or trust a token symbol, catalog pin or CW2 version as proof of reviewed wasm.
3. **RTROLL1381-3 — complete affected inventory.** Enumerate factory pair assets,
   launcher template instances and explicit tokens, including ALPHA. A zero-rate
   official community-tax token such as KENA is still an affected instance; its
   current rates cannot establish that its code is compatible. Reject unexpected
   factory/router bindings or administrators rather than guessing a dispatcher.
4. **RTROLL1381-4 — version-aware upgrades.** Funding targets are
   router 1.5.0 and token 1.4.0; this rollout targets factory 1.12.0 and pair 1.21.0. The factory CW2
   release includes bounded hook admission (#1398); live 1.10.0 also has a
   different hash from the historical repository 1.10.0 artifact.
   Upgrade prerequisite factory/pair code when behind the reviewed target.
   Do not force a same-version migration with a different hash; contracts reject
   non-increasing versions. Report the mismatch for a properly versioned release.
5. **RTROLL1381-5 — real custody.** Store code with `cl8ydeploy`; use the actual
   live administrator for migrations. `multisig_2of3` requires two valid component
   signatures, normally `multisig1` and `multisig2`. `cl8y2_admin` applies only when
   the inventory proves its EOA address controls the target. ALPHA's CMM admin
   requires CMM `migrate_owned_contract`; prove dispatcher support (CMM >=0.2.2)
   or migrate CMM to reviewed compatible code before dispatching token migrations.
   A compatible deployed CMM needs no prerequisite migration. A compatible
   launcher 1.0.0 rotates its token template through configuration only.
   For the integrated AutoLP 1.1.0 release, optional reviewed
   `--autolp-wasm`/`--autolp-sha256` inputs also rotate its sister creation
   template. This neither migrates existing sisters nor approves LP reserves.
6. **RTROLL1381-6 — preserved configuration.** Rotate launcher `token_code_id`
   and factory template references while preserving unrelated settings, AutoLP
   configuration and required old whitelist entries. Refresh asset code pins
   after token migrations. Do not waive tax, broaden unrelated token admissions
   or exempt the router to make a smoke test pass.
7. **RTROLL1381-7 — resumable evidence.** Keep the immutable plan and append-only
   execution receipts outside git. Record a submitted transaction hash before
   polling for inclusion. On resume, reconcile pending hashes and observed chain
   state; never blindly resend a transaction with an uncertain outcome. A failed
   receipt is a failure, not a completed action.
8. **RTROLL1381-8 — secret boundaries.** Prompt for the local keyring passphrase
   once and retain it only in process memory. Never write it to the manifest,
   journal, environment snippets, logs or PR. Use openbao-tools `bao-exec` only
   for separately required API credentials; public deployment output contains
   code IDs/hashes and application pins, never signing material.
9. **RTROLL1381-9 — contained transition.** Record original pair pauses, pause
   affected markets before changing the fleet, and restore only pauses introduced
   by this rollout through explicit finalization. Deploy matching indexer/frontend
   configuration, clear route caches and verify live capability/preview/pins first.
   A partial failure leaves markets contained and receipts available for recovery.
10. **RTROLL1381-10 — honest verification.** Tests, read-only production probes
    and production execution prove different things. Agents may report **PASS
    with explicit skips** when unavailable checks remain listed and unverified.
    Executed failures remain FAIL. Skipping custody/device checks cannot establish
    live rollout or close AC9 as deployed.

## Agent procedure

Read the runbook and entrypoint `--help`. Work in an isolated git worktree; review
the generated inventory before execution and reproduce available automated tests
and retests using the [checklist](../docs/qa/issue-1381-rollout/README.md).
Read-only inspection is useful without signing custody. Stop dependent production
actions on an identity or version mismatch; this does not block finishing source
implementation, documentation, tests or the PR.

When execution is explicitly requested, use the operator's documented keys and
an external receipt directory. Review migration payloads and dispatcher authority;
do not substitute an EOA `wasm migrate` for a contract administrator. After a
partial rollout resume the same plan and journal rather than generating an
unrelated plan. Confirm live hashes and receipts before application configuration
or finalization.

For an explicitly requested operator execution, unavailable keyring TTY,
multisig signing or physical-wallet work belongs in a
cl8y-pm inbox card under repository AGENTS.md: one action, at most five numbered
taps, and the DEX issue URL. Record the concrete skip in the review report without
inventing production receipts or making unavailable work a PR hold.

Router replacement, revocation and single-use settlement follow
[RTTRUST1388-1–RTTRUST1388-10](AGENTS_ROUTER_TRUST_LIFECYCLE.md)
and the [recovery runbook](../docs/runbooks/router-trust-1388.md).

Funded reverse compatibility follows [FRQ1422](AGENTS_FUNDED_REVERSE_QUERY_BUDGET.md)
and the [3M evidence checklist](../docs/qa/issue-1422/README.md). The current
pair 1.21.0/router 1.5.0 targets supersede old pair 1.19.0/router 1.3.0 rollout
instructions in #1384/#1386. Before acceptance record actual provider limits and
successful read-only funded forward/reverse quotes with matching reviewed hashes.
Source work budgets and larger-limit test nodes cannot establish that envelope.
A deterministic provider gas error leaves that provider/route unverified; do not
waive tax/signing guards, fan out or raise limits to conceal it.
