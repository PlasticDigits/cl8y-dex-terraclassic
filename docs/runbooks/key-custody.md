# Runbook: admin-key custody and signer roster (SEC-B10)

Custody policy for production DEX core governance and contract-admin paths: factory, router, pairs, fee-discount, CMM treasury, and wrap-mapper. Product controls have documented EOA residuals in the [columbus-5 registry](../../deployments/mainnet-ust1-wrap/REGISTRY.md#governance-split-ops-critical); [#526](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/526) retains the accept queue and [#697](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/697) tracks the wider security attestation. Parent remediation: GitLab [#398](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/398) (SEC-B10).

**Related:** [Security model § Governance](../security-model.md#governance-keys), [launch checklist Phase 0](./launch-checklist.md#phase-0--preconditions), [governance emergency rehearsal](./governance-emergency-rehearsal.md) (**SEC-B09**, [#397](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/397)) — the signing flow this roster signs through, [wasm admin migration § Admin rotation](./wasm-admin-migration.md#admin-rotation) — `terrad tx wasm set-contract-admin` / `update_admin` mechanics, [operator secrets § Chain signing keys](../operator-secrets.md#chain-signing-keys), [security posture § TVL bands](../security-posture.md#security-requirements-scale-with-tvl). Full rotation command cookbook + LocalTerra rehearsal record is tracked as **SEC-D10** ([#408](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/408)). Agent playbook: [`skills/AGENTS_KEY_CUSTODY.md`](../../skills/AGENTS_KEY_CUSTODY.md).

---

## Policy

- **Policy target:** production DEX governance and new contract-admin paths use a threshold multisig or DAO. Record each current EOA residual in the registry with its tracking issue; do not mark the custody gate complete while #526 authority transfers remain.
- New production DEX contract admins must be the governance multisig at instantiate. Verify each live address; existing product-admin residuals are listed in the registry and remain tracked through [#526](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/526) / [#697](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/697).
- This runbook is the **framework**. The **filled roster** — real signer identities, key fingerprints, and the final threshold — is maintained **privately** (operator key store / password manager) and is **never committed** to git ([operator secrets](../operator-secrets.md)). The on-chain multisig address is public: [`docs/reference/governance-multisig.md`](../reference/governance-multisig.md).

> **Status — multisig address live.** Core DEX governance / admin / upgrade address: **`terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7`** ([governance multisig reference](../reference/governance-multisig.md)). Signer identities and threshold are filled at the **key ceremony** and linked **privately** from the launch tracking issue ([#391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391)). Do **not** treat the Phase 0 custody gate as satisfied until the roster is populated and emergency rehearsals are complete from this multisig on testnet/staging.

---

## 1. Multisig type and threshold

Production DEX governance uses a Cosmos threshold multisig (k-of-n) or a DAO module. New core DEX contracts use the DEX multisig as wasm admin. Current product-control exceptions are listed in the [REGISTRY governance split](../../deployments/mainnet-ust1-wrap/REGISTRY.md#governance-split-ops-critical).

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
