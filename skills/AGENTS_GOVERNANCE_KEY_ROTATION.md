# Agent playbook: governance key rotation (SEC-D10)

Use when implementing or verifying **admin-key rotation** for the CL8Y DEX — rotating the wasm contract-admin and the factory `governance` pointer ([#408](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/408)). Admin-key compromise is the most realistic high-impact attack at small TVL.

## Canonical references

| Doc / script | Purpose |
|--------------|---------|
| [`docs/runbooks/governance-key-rotation.md`](../docs/runbooks/governance-key-rotation.md) | Cookbook — copy-paste rotation commands, verification queries, rehearsal |
| [`scripts/rehearse-governance-key-rotation.sh`](../scripts/rehearse-governance-key-rotation.sh) | LocalTerra wasm-admin round-trip rehearsal (multisig, self-restoring) |
| [`docs/runbooks/key-custody.md`](../docs/runbooks/key-custody.md) | Signer roster + threshold (SEC-B10) — who signs the rotation |
| [`docs/runbooks/governance-emergency-rehearsal.md`](../docs/runbooks/governance-emergency-rehearsal.md) | The multisig signing flow (SEC-B09) |
| [`docs/runbooks/wasm-admin-migration.md`](../docs/runbooks/wasm-admin-migration.md) | Migrate authority context for the wasm admin |

## Minimum operations

Rotate **both** admin keys and verify on chain before retiring the old keys:

1. Wasm contract-admin: `terrad tx wasm set-contract-admin <factory> <new_admin>` (per contract).
2. Factory governance: `terrad tx wasm execute <factory> '{"update_config":{"governance":"<new>"}}'`.
3. From a multisig: `--generate-only` → `tx sign --multisig` (× threshold) → `tx multisign` → `tx broadcast`.
4. Verify: `contract_info.admin` and factory `config.governance` == new key, **then** retire old keys.

terrad v4.x uses **`set-contract-admin`**, not `update-admin`. `clear-contract-admin` is irreversible.

## LocalTerra automation

```bash
make rehearse-governance-key-rotation
./scripts/rehearse-governance-key-rotation.sh --output /tmp/sec-d10-rotation.md
```

The script rotates the factory wasm admin to a rehearsal 2-of-3 multisig and back (multisig-signed return), verifying the admin on chain at each step and **restoring** the original admin — safe on a shared deploy. SKIPs when the chain is down or the factory admin is not in the local keyring.

## Verification

```bash
make check-governance-key-rotation-docs   # doc invariants only
make verify-issue-408                       # docs + optional LocalTerra rehearsal
```

Doc check: no chain. Full verify optionally runs the rotation round-trip when chain + deploy admin exist.

## Evidence workflow

1. Run the rehearsal on **testnet/staging** with the **production multisig** before mainnet.
2. Fill the evidence table in [`governance-key-rotation.md` § Evidence template](../docs/runbooks/governance-key-rotation.md#evidence-template) (network, addresses, tx hashes, signer, UTC).
3. Post it on the launch tracking issue [#391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391) or [#408](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/408); link from Phase 5 sign-off.

## Related

- Custody / signer roster: [`AGENTS_KEY_CUSTODY.md`](./AGENTS_KEY_CUSTODY.md)
- Emergency rehearsal: [`AGENTS_GOVERNANCE_EMERGENCY_REHEARSAL.md`](./AGENTS_GOVERNANCE_EMERGENCY_REHEARSAL.md)
