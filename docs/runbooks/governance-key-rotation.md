# Runbook: governance key rotation (SEC-D10)

Copy-pastable procedure to **rotate** the two admin keys that control the CL8Y DEX — the **wasm contract-admin** (migrate authority) and the factory **`governance` pointer** — when a key is compromised, a signer changes, or on a scheduled cadence. Admin-key compromise is the most realistic high-impact attack at small TVL, so this is the least-optional emergency procedure. Parent remediation: GitLab [#408](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/408) (**SEC-D10**); implements backlog item **DEX-P2-026**.

**Related:** [key custody — signer roster + thresholds](./key-custody.md) (**SEC-B10**, [#398](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/398)), [wasm admin migration § Admin rotation](./wasm-admin-migration.md#admin-rotation), [governance emergency rehearsal — multisig signing flow](./governance-emergency-rehearsal.md#signing-flow-cosmos-multisig) (**SEC-B09**, [#397](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/397)), [Security model § Governance](../security-model.md#governance-keys), [operator secrets § Chain signing keys](../operator-secrets.md#chain-signing-keys). Agent playbook: [`skills/AGENTS_GOVERNANCE_KEY_ROTATION.md`](../../skills/AGENTS_GOVERNANCE_KEY_ROTATION.md).

---

## 1. What rotates

| Key | Controls | Rotate with |
|-----|----------|-------------|
| **Wasm contract-admin** | `migrate` of factory / router / pair / fee-discount | `terrad tx wasm set-contract-admin` (chain-level) |
| **Factory `governance` pointer** | fees, hooks, CW20 whitelist, pause, blacklist, treasury, trusted routers | factory `ExecuteMsg::UpdateConfig { governance }` |

Both should point at the **same governance multisig** ([key custody](./key-custody.md)). Rotating one does **not** rotate the other — a full rotation does both. Triggers (compromise / departure / scheduled / threshold change) and the higher-level process are in [key custody § 4](./key-custody.md#4-key-rotation--triggers-and-process).

> Terra Classic `terrad` (v4.x) uses **`set-contract-admin`** — not `update-admin`. `clear-contract-admin` removes the admin **permanently** (no further migrate); never use it on a contract you may need to upgrade.

## 2. Rotate the wasm contract-admin

Signed by the **current** admin. Replace `<factory>` (repeat per contract), `<new_admin>`, node, and fees.

```bash
# Forward (current admin is a single key):
terrad tx wasm set-contract-admin <factory> <new_admin> \
  --from <current_admin> --chain-id <chain-id> --node <rpc-url> \
  --gas auto --gas-adjustment 1.4 --fees 500000uluna -y

# Current admin is a multisig — generate-only, threshold-sign, combine, broadcast:
terrad tx wasm set-contract-admin <factory> <new_admin> --from <multisig> \
  --chain-id <chain-id> --node <rpc-url> --gas auto --gas-adjustment 1.4 \
  --fees 500000uluna --generate-only > unsigned.json
terrad tx sign unsigned.json --from <signer1> --multisig <multisig> --sign-mode amino-json \
  --chain-id <chain-id> --node <rpc-url> > sig1.json
terrad tx sign unsigned.json --from <signer2> --multisig <multisig> --sign-mode amino-json \
  --chain-id <chain-id> --node <rpc-url> > sig2.json
terrad tx multisign unsigned.json <multisig> sig1.json sig2.json \
  --chain-id <chain-id> --node <rpc-url> > signed.json
terrad tx broadcast signed.json --chain-id <chain-id> --node <rpc-url>
```

> Add `--keyring-backend <os|test|file>` to the `keys` / `tx` commands to match your operator setup (examples omit it and default to `os`; the LocalTerra rehearsal uses `test`).

## 3. Rotate the factory governance pointer

Signed by the **current** `governance` (single key or multisig, same flow as §2 — `wasm execute` instead of `set-contract-admin`):

```bash
terrad tx wasm execute <factory> '{"update_config":{"governance":"<new_governance>"}}' \
  --from <current_governance> --chain-id <chain-id> --node <rpc-url> \
  --gas auto --gas-adjustment 1.4 --fees 500000uluna -y
# multisig governance: --generate-only -> tx sign --multisig (x threshold) -> tx multisign -> tx broadcast (as in §2)
```

`UpdateConfig` also updates `treasury` / default fee — pass only the field(s) you intend to change.

## 4. Post-rotation verification

**Verify the new key everywhere before retiring the old one.**

```bash
# Wasm admin per contract:
terrad query wasm contract <factory> --node <rpc-url> --output json | jq -r .contract_info.admin   # == <new_admin>

# Factory governance pointer:
terrad query wasm contract-state smart <factory> '{"config":{}}' --node <rpc-url> --output json | jq -r .data.governance
```

- [ ] Every contract `contract_info.admin` == new admin
- [ ] Factory `config.governance` == new governance
- [ ] A governance-only action (e.g. a no-op `SetPairFee` on a test pair, or the [emergency rehearsal](./governance-emergency-rehearsal.md)) succeeds from the new key
- [ ] Only **after** the above: retire / revoke the old keys; update the [deploy trace](../templates/deploy-trace.md) ([#410](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/410)) and the private [key custody roster](./key-custody.md#2-signer-roster-roles)

## 5. Signers and threshold

The multisig type, threshold (`k-of-n`), named signer roles, and backup signer are defined in **[key custody](./key-custody.md)** (**SEC-B10**). The filled roster (identities, addresses, final threshold) stays **private** and is not committed here. The multisig signing flow used above is rehearsed in [governance emergency rehearsal](./governance-emergency-rehearsal.md#signing-flow-cosmos-multisig).

## 6. Automated LocalTerra rehearsal

The repo ships a scripted rehearsal that rotates the factory **wasm contract-admin** round-trip — current admin → a rehearsal **2-of-3 multisig** → back to the original — including the **multisig-signed** return rotation (the production "rotate away from the old multisig" path). It verifies the admin on chain at each step and **restores** the original admin, so it is safe to run against a shared LocalTerra deploy.

```bash
make rehearse-governance-key-rotation
# or save a transcript for GitLab evidence:
./scripts/rehearse-governance-key-rotation.sh --output /tmp/sec-d10-rotation.md
```

**Important:** the LocalTerra rehearsal proves the **signing flow and message shapes**. It does **not** satisfy the launch gate until operators repeat a rotation from the **planned production multisig** on testnet/staging and attach evidence.

### Evidence template

| Field | Value |
|-------|-------|
| Checklist | SEC-D10 ([#408](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/408)) |
| Network | `<localterra / testnet / staging>` |
| Contract(s) | `<factory / router / ...>` |
| Original admin (restored) | `<addr>` |
| Rotation multisig | `<addr>` (`<k>`-of-`<n>`) |
| Rehearsed by / UTC | `<name>` / `<YYYY-MM-DDThh:mm:ssZ>` |
| Tx hashes | forward `<hash>`, return `<hash>` |

Post the filled table on the launch tracking issue ([#391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391)) or [#408](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/408).

## Limitations

- **`clear-contract-admin` is irreversible** — a cleared admin can never migrate again; treat as an [irrecoverable case](./wasm-admin-migration.md#irrecoverable-cases-on-chain-migration-cannot-be-rolled-back).
- Rotation is **governance/admin-gated**: you can only rotate from the key you currently hold. If the current multisig can no longer reach threshold, rotation is impossible — see [key custody § 3 escalation](./key-custody.md#3-backup-signer-and-escalation).
- **Never** rotate to a single EOA or `clear` the admin as a shortcut, even temporarily.
- Verify the new admin on chain (§4) **before** retiring old keys — a rotation that points at an uncontrolled address strands the contract.

## Doc invariant (SEC-D10)

```bash
make check-governance-key-rotation-docs   # doc invariants only (markers + cross-links)
make verify-issue-408                      # docs + optional LocalTerra rotation rehearsal
```

No Postgres required. The LocalTerra rehearsal layer is **SKIP** when the chain is down or the deploy admin is not in the local keyring.
