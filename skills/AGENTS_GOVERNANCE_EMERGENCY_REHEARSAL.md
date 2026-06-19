# Agent playbook: governance emergency rehearsal (SEC-B09)

Use when implementing or verifying **pre-launch multisig rehearsal** for factory emergency controls ([#397](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/397)).

## Canonical references

| Doc / script | Purpose |
|--------------|---------|
| [`docs/runbooks/governance-emergency-rehearsal.md`](../docs/runbooks/governance-emergency-rehearsal.md) | Operator runbook — scope, signing flow, evidence |
| [`docs/templates/governance-emergency-rehearsal-evidence.md`](../docs/templates/governance-emergency-rehearsal-evidence.md) | Evidence table for GitLab launch issue |
| [`scripts/rehearse-governance-emergency-controls.sh`](../scripts/rehearse-governance-emergency-controls.sh) | LocalTerra 2-of-3 multisig dry-run |
| [`docs/runbooks/launch-checklist.md`](../docs/runbooks/launch-checklist.md) Phase 5 PAUSE | Gate criterion — unrehearsed multisig delays launch |
| [`skills/AGENTS_LAUNCH_GO_NO_GO.md`](./AGENTS_LAUNCH_GO_NO_GO.md) | Phase 5 go/no-go decisions |

## Minimum operations

Governance must rehearse **pause → blacklist → unpause → unblacklist** via the **production signing flow** on testnet/staging:

1. `SetPairPaused { paused: true }`
2. `BlacklistWallet { address }`
3. `SetPairPaused { paused: false }`
4. `UnblacklistWallet { address }`

Contract unit tests (`cw-multi-test`, local keys) do **not** satisfy SEC-B09.

## LocalTerra automation

```bash
make setup-cloud-localterra   # first-time VM (~10–15 min)
make rehearse-governance-emergency
./scripts/rehearse-governance-emergency-controls.sh --output /tmp/sec-b09.md
```

The script rotates factory governance to a rehearsal 2-of-3 multisig, then uses `terrad tx multisign` for each admin message.

## Verification

```bash
make check-governance-emergency-rehearsal-docs
make verify-issue-397
```

Doc check: no LocalTerra. Full verify optionally runs the rehearsal when chain + deploy env exist.

## Evidence workflow

1. Operators run rehearsal on **testnet/staging** with **production multisig**.
2. Post filled evidence template on launch issue [#391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391).
3. Link evidence in Phase 5 sign-off comment.

## Related

- User-facing impact copy: [`AGENTS_USER_INCIDENT_FAQ.md`](./AGENTS_USER_INCIDENT_FAQ.md)
- Launch gate: [`AGENTS_LAUNCH_GO_NO_GO.md`](./AGENTS_LAUNCH_GO_NO_GO.md)
