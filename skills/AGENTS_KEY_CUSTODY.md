# Agent playbook: admin-key custody (SEC-B10)

Use when implementing or verifying the **admin-key custody policy** — multisig type/threshold, named signer roster, backup signer, and key-rotation policy for production governance ([#398](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/398)).

## Canonical references

| Doc / script | Purpose |
|--------------|---------|
| [`docs/reference/governance-multisig.md`](../docs/reference/governance-multisig.md) | Canonical on-chain governance / admin / upgrade address |
| [`docs/runbooks/key-custody.md`](../docs/runbooks/key-custody.md) | Custody framework — multisig type/threshold, signer roster, backup + escalation, rotation triggers and process |
| [`docs/runbooks/governance-emergency-rehearsal.md`](../docs/runbooks/governance-emergency-rehearsal.md) | The multisig signing flow this roster signs through (SEC-B09) |
| [`docs/runbooks/wasm-admin-migration.md`](../docs/runbooks/wasm-admin-migration.md) § Admin rotation | `update_admin` mechanics for the wasm contract admin |
| [`docs/security-model.md`](../docs/security-model.md) § Governance Keys | Governance auth surface; no-single-EOA policy |
| [`skills/AGENTS_LAUNCH_GO_NO_GO.md`](./AGENTS_LAUNCH_GO_NO_GO.md) | Phase 0 / Phase 5 go/no-go decisions |

## Custody requirements

The runbook must document all six SEC-B10 items:

1. **Multisig type and threshold** — `k-of-n` Cosmos multisig (or DAO), with a TVL-scaled minimum threshold.
2. **Named signer roster (roles)** — by role, not necessarily personal identity; filled copy kept private.
3. **Backup signer + escalation path** — a key in `n` held out of routine signing; what to do when signers are unavailable or compromised.
4. **Key-rotation triggers and process** — when to rotate and the high-level steps (exact command cookbook + rehearsal are SEC-D10 / [#408](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/408)).
5. **No single EOA** controls mainnet governance / treasury / contract `admin`.
6. Phase 0 launch-checklist gate completed with a link to the runbook.

**Production multisig:** `terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7` ([`docs/reference/governance-multisig.md`](../docs/reference/governance-multisig.md)). Signer identities and threshold are **key-ceremony fill-ins** kept privately and linked from the launch issue ([#391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391)).

## No on-chain automation here

Custody is a documentation + ceremony gate; there is no LocalTerra rehearsal in this issue. The **rotation rehearsal** (LocalTerra/testnet transcript with tx hashes) belongs to **SEC-D10** ([#408](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/408)); the **emergency-controls rehearsal** belongs to **SEC-B09** ([#397](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/397)).

## Verification

```bash
make check-key-custody-docs   # doc invariants only (markers + cross-links)
make verify-issue-398         # docs check + artifact presence
```

No chain or Postgres required.

## Evidence workflow

1. At the **key ceremony**, generate the multisig and fill the roster (threshold, members, addresses) in a **private** copy.
2. Set every contract `admin` to the multisig at instantiate; confirm on chain.
3. Link the private custody record from the launch tracking issue ([#391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391)) and mark the Phase 0 custody gate satisfied in the go/no-go sign-off.

## Related

- Emergency rehearsal: [`AGENTS_GOVERNANCE_EMERGENCY_REHEARSAL.md`](./AGENTS_GOVERNANCE_EMERGENCY_REHEARSAL.md)
- Launch gate: [`AGENTS_LAUNCH_GO_NO_GO.md`](./AGENTS_LAUNCH_GO_NO_GO.md)
