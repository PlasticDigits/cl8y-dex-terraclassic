# Runbook: admin-key custody and signer roster (SEC-B10)

Custody policy for the production **governance** key that controls the factory, router, pair, and fee-discount admin paths: the **multisig type and threshold**, a **named signer roster** (by role), a **designated backup signer** with an **escalation path**, and the **key-rotation triggers and process**. The controlling rule is **no single EOA** holds mainnet governance. Parent remediation: GitLab [#398](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/398) (**SEC-B10**).

**Related:** [Security model § Governance](../security-model.md#governance-keys), [launch checklist Phase 0](./launch-checklist.md#phase-0--preconditions), [governance emergency rehearsal](./governance-emergency-rehearsal.md) (**SEC-B09**, [#397](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/397)) — the signing flow this roster signs through, [wasm admin migration § Admin rotation](./wasm-admin-migration.md#admin-rotation) — `terrad tx wasm set-contract-admin` / `update_admin` mechanics, [operator secrets § Chain signing keys](../operator-secrets.md#chain-signing-keys), [security posture § TVL bands](../security-posture.md#security-requirements-scale-with-tvl). Full rotation command cookbook + LocalTerra rehearsal record is tracked as **SEC-D10** ([#408](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/408)). Agent playbook: [`skills/AGENTS_KEY_CUSTODY.md`](../../skills/AGENTS_KEY_CUSTODY.md).

---

## Policy

- **No single EOA** controls mainnet governance, treasury, or any contract `admin`. Governance is a **threshold multisig** (`k-of-n`) or a **DAO** module. Reaffirms [Security model § Governance](../security-model.md#governance-keys) ("Never use a single EOA for mainnet governance") and [operator secrets § Chain signing keys](../operator-secrets.md#chain-signing-keys) ("Multisig governance for factory/router/pair admin is required for production").
- Every contract `admin` (factory, router, pair, fee-discount) **must be set** to the **governance multisig** at instantiate — the deploy guide passes `--admin <governance_addr>` on every instantiate ([deployment guide](../deployment-guide.md)). No mainnet contract may be left with an EOA admin or a cleared admin.
- This runbook is the **framework**. The **filled roster** — real signer identities, key fingerprints, the on-chain multisig address, and the final threshold — is maintained **privately** (operator key store / password manager) and is **never committed** to git ([operator secrets](../operator-secrets.md)). Only role structure and placeholders are committed here.

> **Status — pending multisig setup.** The production multisig has not yet been generated (see [#398](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/398)). Sections 1–4 define the required structure and process; the concrete addresses, signer identities, and final threshold are filled at the **key ceremony** before mainnet, and the filled copy is linked **privately** from the launch tracking issue ([#391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391)). Do **not** treat the Phase 0 custody gate as satisfied until the roster is populated and the multisig address is live on chain.

---

## 1. Multisig type and threshold

Production governance is a Cosmos **threshold multisig** (`k-of-n`) — created with `terrad keys add <name> --multisig <m1,m2,...> --multisig-threshold <k>` — or a DAO module (cw3 / DAODAO). The on-chain governance address is this multisig account, and the same address is set as each contract `admin`.

| TVL band (see [security posture](../security-posture.md#security-requirements-scale-with-tvl)) | Minimum threshold | Notes |
|---|---|---|
| **Bootstrap** ($0–$1M) | **2-of-3** | No EOA admin; signer keys on separate hardware where possible |
| **Growth** ($1M–$25M) | **3-of-5** | Signer diversity — distinct people, distinct devices / locations |
| **Mature** ($25M+) | **≥3-of-5 + timelock** | Formal key ceremony; timelock on sensitive governance actions where feasible |

- The **signing flow** (generate-only → per-signer `terrad tx sign --multisig` → `terrad tx multisign` → broadcast) is documented and rehearsed in [governance emergency rehearsal § Signing flow](./governance-emergency-rehearsal.md#signing-flow-cosmos-multisig) (**SEC-B09**, [#397](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/397)).
- Choose `n` so that `n ≥ k + 1` — losing one key must not drop the multisig below threshold (see §3).
- Final threshold (`k`) and member set (`n`): **`<fill at key ceremony>`**.

## 2. Signer roster (roles)

Signers are named by **role**, not necessarily personal identity (keep identities private where required). Fill the signer / key-location columns at the key ceremony; keep the filled copy private per the Policy section.

| Role | Responsibility | Signer (private) | Key location |
|---|---|---|---|
| **Primary governance signer** | Routine governance txs — fees, hooks, CW20 whitelist | `<fill>` | `<hardware / HSM>` |
| **Treasury signer** | Treasury config via `UpdateConfig` (governance only) | `<fill>` | `<hardware / HSM>` |
| **Security signer** | Co-signs emergency `SetPairPaused` / blacklist actions | `<fill>` | `<hardware / HSM>` |
| **Backup signer** (see §3) | Stands in to reach threshold when a primary is unavailable | `<fill>` | `<offline / custody>` |

- Hot wallets used for `terrad tx` should be **hardware-wallet or HSM-backed** ([operator secrets § Chain signing keys](../operator-secrets.md#chain-signing-keys)).
- The roster covers governance for **factory, router, pair, and fee-discount**; the governance-only action surface is enumerated in [Security model § Governance](../security-model.md#governance-keys) (auth tables) and [security posture § admin controls](../security-posture.md#admin-controls-users-should-know).

## 3. Backup signer and escalation

- A **designated backup signer** holds a key counted in `n` but kept out of routine signing. It supplies the `k`-th signature when a primary signer is unavailable, so a single absence never blocks an emergency `SetPairPaused` or blacklist.
- **Escalation path** when signers are unavailable, or a key is suspected compromised:
  1. Reach the primary signers (roster roles) on the operator comms channel and assemble `k` available keys.
  2. If `k` keys cannot be reached, bring in the **backup signer** to make threshold.
  3. If a key is **suspected compromised**, do not wait for the routine flow — go to [§4 rotation](#4-key-rotation--triggers-and-process) and rotate governance to a new multisig that excludes the compromised key.
  4. If threshold is **unrecoverable** (≥ `n − k + 1` keys lost), governance is effectively frozen — the contract `admin` cannot be re-authorized without a valid signer set. Treat per [wasm admin migration § Irrecoverable cases](./wasm-admin-migration.md#irrecoverable-cases-on-chain-migration-cannot-be-rolled-back).

## 4. Key rotation — triggers and process

**Triggers** — rotate governance and/or contract `admin` when any of these holds:

- A signer key is **suspected or known compromised** (rotate immediately).
- A signer **departs** or can no longer be trusted with the role.
- A **scheduled** rotation interval is reached (operator policy).
- A **threshold change** is needed (e.g. 2-of-3 → 3-of-5 as TVL grows — see §1).

**Process** (high-level — the exact copy-paste `terrad` commands and a LocalTerra/testnet rehearsal record are owned by **SEC-D10**, [#408](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/408); current mechanics live in [wasm admin migration § Admin rotation](./wasm-admin-migration.md#admin-rotation)):

1. **Generate** the new multisig (`terrad keys add` with the new member set / threshold) and record its address in the private roster.
2. **Prepare** the rotation: the on-chain wasm `admin` for each contract via `terrad tx wasm set-contract-admin` (the native wasm `update_admin` operation), **and** the factory governance pointer via `ExecuteMsg::UpdateConfig { governance: <new_multisig> }`.
3. **Threshold-approve** every rotation tx through the **current** multisig signing flow (generate-only → `multisign` → broadcast).
4. **Verify on chain before revoking old keys:** `terrad query wasm contract <addr> | jq -r .contract_info.admin` equals the new multisig for each contract, and factory `config.governance` equals the new multisig.
5. **Retire old keys** only after step 4 confirms the new `admin` everywhere.
6. **Update records:** the private roster, the [deploy trace](../templates/deploy-trace.md) ([#410](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/410)), and the launch tracking issue ([#391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391)).
7. **Rehearse first:** run the rotation on LocalTerra/testnet before mainnet — evidence per [#408](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/408).

> Rotation is **governance-gated**: if the multisig can no longer reach threshold, rotation is impossible (see §3 step 4). Never rotate the `admin` or governance to a single EOA, even temporarily.

---

## Doc invariant (SEC-B10)

```bash
make check-key-custody-docs   # doc invariants only (markers + cross-links)
make verify-issue-398         # docs check + artifact presence
```

No chain or Postgres required. The **filled** roster (identities, addresses, final `k`) lives privately and is **not** verified here — this gate confirms the custody framework is present, internally consistent, and cross-linked from the launch checklist and security model.
