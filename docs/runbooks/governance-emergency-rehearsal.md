# Runbook: governance emergency controls rehearsal (SEC-B09)

Pre-launch **PAUSE** gate: production governance must rehearse emergency admin transactions from the **actual planned multisig** (or DAO proposal flow), not only from LocalTerra `test1` deploy keys.

**Related:** [Launch checklist Phase 5 — PAUSE](./launch-checklist.md#pause--delay-launch), [Security model § Governance](../security-model.md), [User incident FAQ](../user-incident-faq.md), agent playbook [`skills/AGENTS_GOVERNANCE_EMERGENCY_REHEARSAL.md`](../../skills/AGENTS_GOVERNANCE_EMERGENCY_REHEARSAL.md), GitLab [#397](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/397).

---

## Minimum rehearsal scope

On **testnet or staging** (never first-touch on mainnet), the governance multisig must successfully sign and broadcast:

| # | Factory `ExecuteMsg` | Expected on-chain effect |
|---|----------------------|---------------------------|
| 1 | `SetPairPaused { pair, paused: true }` | Pair `IsPaused` → `true`; swaps rejected |
| 2 | `BlacklistWallet { address }` | Factory `BlacklistCheck` → `wallet_blacklisted: true` |
| 3 | `SetPairPaused { pair, paused: false }` | Pair trading resumes |
| 4 | `UnblacklistWallet { address }` | Wallet access restored |

Optional follow-ups (recommended before mainnet, not a substitute for the four steps above): `SetPairFee`, `SetPairHooks`, `SetPairCreationFee`, `UpdateConfig`.

---

## Signing flow (Cosmos multisig)

Production Terra Classic governance typically uses a **threshold multisig** (`k-of-n` signers) or a DAO module. The operator flow mirrors:

1. **Generate** unsigned tx: `terrad tx wasm execute <factory> '<json>' --from <multisig> --generate-only`
2. **Sign** with each required key: `terrad tx sign unsigned.json --from <signer> --multisig <multisig> --sign-mode amino-json`
3. **Combine**: `terrad tx multisign unsigned.json <multisig> sig1.json sig2.json …`
4. **Broadcast**: `terrad tx broadcast signed.json`

Hardware-wallet and custodial multisigs use the same message JSON; only the signing step differs.

---

## LocalTerra automated dry-run

For CI and agent VMs, the repo ships a scripted rehearsal that:

- Creates a **2-of-3** rehearsal multisig in the LocalTerra `test` keyring
- Rotates factory `governance` to that multisig (one-time `test1` `UpdateConfig`)
- Executes all four emergency messages via **threshold multisign**
- Prints a transcript with **tx hashes** for GitLab evidence

```bash
make start && make deploy-local   # or make setup-cloud-localterra
make rehearse-governance-emergency
# save transcript:
./scripts/rehearse-governance-emergency-controls.sh --output /tmp/sec-b09-rehearsal.md
```

**Important:** LocalTerra rehearsal proves the **signing flow and message shapes** — it does **not** satisfy the launch gate until operators repeat the exercise from the **planned production multisig** on testnet/staging and attach evidence to the launch issue ([#391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391)).

---

## Evidence and launch gate

1. Run the rehearsal on **testnet/staging** with the **production governance multisig**.
2. Fill [`docs/templates/governance-emergency-rehearsal-evidence.md`](../templates/governance-emergency-rehearsal-evidence.md) (tx hashes + network + multisig address).
3. Post the completed table as a **pinned comment** on the launch tracking issue ([#391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391)) or GitLab [#397](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/397).
4. In Phase 5 go/no-go sign-off, link that comment and mark the **SEC-B09 / multisig rehearsal** PAUSE criterion satisfied.

---

## Verification

```bash
make check-governance-emergency-rehearsal-docs   # doc invariants only
make verify-issue-397                          # docs + optional LocalTerra rehearsal
```

No Postgres required. LocalTerra rehearsal layer is **SKIP** when the chain is down.
