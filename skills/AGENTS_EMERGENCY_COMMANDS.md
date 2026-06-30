# Emergency pause / blacklist command cookbook

Use when implementing or verifying **SEC-B11** operator docs for factory emergency controls ([GitLab **#399**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/399)).

## Canonical doc

| Path | Purpose |
|------|---------|
| [`docs/runbooks/emergency-commands.md`](../docs/runbooks/emergency-commands.md) | Parameterized `terrad tx wasm execute` for all eight factory emergency messages |
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

Wire types: [`smartcontracts/packages/dex-common/src/factory.rs`](../smartcontracts/packages/dex-common/src/factory.rs). Integration tests: [`blacklist_tests.rs`](../smartcontracts/tests/src/blacklist_tests.rs).

## Verification commands

```bash
# Doc invariant (no chain)
make check-emergency-commands-docs

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
