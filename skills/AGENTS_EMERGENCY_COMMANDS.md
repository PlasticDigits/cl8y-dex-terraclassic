# Emergency pause / blacklist command cookbook

Use when implementing or verifying **SEC-B11** operator docs for factory emergency controls ([GitLab **#399**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/399)) and **SEC-G07** unpause prerequisite gates ([#440](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/440)).

## Canonical doc

| Path | Purpose |
|------|---------|
| [`docs/runbooks/emergency-commands.md`](../docs/runbooks/emergency-commands.md) | Parameterized `terrad tx wasm execute` for all eight factory emergency messages |
| [`docs/runbooks/emergency-commands.md` § Quick pool triage](../docs/runbooks/emergency-commands.md#quick-pool-triage-sec-g03) | Rank pools by liquidity before choosing `$PAIR_ADDR` (SEC-G03, [#436](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/436)) |
| [`docs/runbooks/blacklist-decision.md`](../docs/runbooks/blacklist-decision.md) | Symmetric restore gate for `Unblacklist*` rollback checklist |
| [`docs/templates/incident-dex-indexer.md`](../docs/templates/incident-dex-indexer.md) | Mitigation step links to the cookbook; **Incident timeline** table for tx audit trail |
| [`docs/adr/0003-governance-trading-blacklist.md`](../docs/adr/0003-governance-trading-blacklist.md) | Blacklist design rationale |

## Eight operations (factory `ExecuteMsg`)

| Action | JSON variant | Confirm query |
|--------|--------------|---------------|
| Pause pair | `set_pair_paused` + `paused: true` | Pair `{"is_paused":{}}` → `paused: true` |
| Unpause pair | `set_pair_paused` + `paused: false` | Pair `is_paused` → `paused: false` |
| Blacklist wallet | `blacklist_wallet` | Factory `blacklist_check` → `wallet_blacklisted: true` |
| Unblacklist wallet | `unblacklist_wallet` | `blacklist_check` → `wallet_blacklisted: false` |
| Blacklist token | `blacklist_token` | `blacklist_check` with `tokens` → token in `blacklisted_tokens` |
| Unblacklist token | `unblacklist_token` | `blacklist_check` → token cleared |
| Blacklist pair | `blacklist_pair` | `blacklist_check` with `pair` → `pair_blacklisted: true` |
| Unblacklist pair | `unblacklist_pair` | `blacklist_check` → `pair_blacklisted: false` |

**Unpause prerequisite (SEC-G07):** before broadcasting `set_pair_paused` with `paused: false`, complete the mandatory **Before you unpause** checklist in section 2 of the cookbook (incident reference, resolution evidence, funds-at-risk check, timeline entry with approver). Symmetric with the [blacklist rollback checklist](../docs/runbooks/blacklist-decision.md#false-positive-rollback-unblacklist).

Wire types: [`smartcontracts/packages/dex-common/src/factory.rs`](../smartcontracts/packages/dex-common/src/factory.rs). Integration tests: [`blacklist_tests.rs`](../smartcontracts/tests/src/blacklist_tests.rs).

## Verification commands

```bash
# Doc invariant (no chain)
make check-emergency-commands-docs

# SEC-G07 unpause prerequisite checklist (#440)
make verify-issue-440

# LocalTerra rehearsal (chain + deploy required)
make has-localterra && make verify-issue-399
```

## Do not duplicate

- **User-facing** impact copy lives in [`docs/user-incident-faq.md`](../docs/user-incident-faq.md) — link, do not fork ([`AGENTS_USER_INCIDENT_FAQ.md`](./AGENTS_USER_INCIDENT_FAQ.md)).
- **Tier 255** on fee-discount is not factory blacklist — see [security model § Trading blacklist](../docs/security-model.md).

## LocalTerra notes

- Governance on LocalTerra deploy is **`test1`** (`scripts/deploy-dex-local.sh`).
- Rehearsal script: [`scripts/qa/verify-issue-399.sh`](../scripts/qa/verify-issue-399.sh).
- Use `make setup-cloud-localterra` on Cloud Agent VMs before claiming SKIP on chain rehearsal.
