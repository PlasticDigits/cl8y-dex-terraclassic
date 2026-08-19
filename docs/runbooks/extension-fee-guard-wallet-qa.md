# Runbook: extension wallet fee guard — scope and wallet QA (SEC-E08)

Post-sign **fee/gas sanity check** for extension wallets is implemented in [`extensionSignedFeeGuard.ts`](../../frontend-dapp/src/utils/extensionSignedFeeGuard.ts) and mirrored in the cosmes **`KeplrExtension`** patch ([GitLab **#127**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/127), [**#134**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/134)). This runbook documents **why the guard is LocalTerra-only**, **why mainnet is intentionally out of scope**, and **manual wallet QA** for launch sign-off ([#429](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/429)).

**Related:** [Security model § Extension fee guard](../security-model.md#extension-wallet-fee-guard-sec-e08), [launch checklist Phase 4](./launch-checklist.md#phase-4--off-chain-stack-if-applicable), agent playbook [`skills/AGENTS_EXTENSION_FEE_GUARD.md`](../../skills/AGENTS_EXTENSION_FEE_GUARD.md).

---

## Scope (intentional)

| Network | Chain ID | Post-sign fee guard | Rationale |
|---------|----------|---------------------|-----------|
| **LocalTerra** | `localterra` | **Active** — compares wallet `signed.fee` to dApp `stdDoc.fee` (≥ **95%** of uluna + gas) | Station’s Keplr shim on LocalTerra can rewrite fees to ~**3,000 uluna** while the dApp submits ~**5.6M uluna** ([#127](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/127)); partial rewrites (~23 vs ~36 LUNC) also observed ([#134](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/134)). |
| **Mainnet / testnet** | `columbus-5`, `rebel-2`, … | **Inactive** — `extensionSignedFeeUndershootMessage` returns `null` | **Keplr on mainnet does not exhibit this stale-fee rewrite** (maintainer confirmation on [#429](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/429)). Keplr uses the standard cosmes gas flow with `preferNoSetFee` and respects the dApp fee envelope on production networks. No separate post-sign guard is required. |

**Code invariant:** `isLocalTerraChainId(chainId)` gates the guard — see [`extensionSignedFeeGuard.ts`](../../frontend-dapp/src/utils/extensionSignedFeeGuard.ts) and Vitest `skips validation on mainnet chain id` in [`extensionSignedFeeGuard.test.ts`](../../frontend-dapp/src/utils/__tests__/extensionSignedFeeGuard.test.ts).

**Risk acceptance:** Absence of the guard on mainnet is **by design**, not an oversight. Launch sign-off must record that mainnet Keplr fee behavior was reviewed per this runbook (automated unit test + optional manual smoke below).

---

## Automated verification (no chain required)

```bash
# Doc drift + unit tests for LocalTerra-only gate
make verify-issue-429
```

This runs:

- `make check-extension-fee-guard-docs` — launch checklist, security model, and agent skill markers
- `extensionSignedFeeGuard.test.ts` — asserts `columbus-5` returns `null` even when signed fee is far below expected

---

## Manual wallet QA — LocalTerra (required for LocalTerra releases)

**Wallets:** **Keplr (extension)** and/or **Simulated Wallet** only — **not Terra Station** on LocalTerra ([#235](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/235)).

**Prereqs:** `make setup-cloud-localterra` or equivalent deploy; `make dev`; Chrome + Keplr per [`AGENTS.md`](../../AGENTS.md).

| Step | Action | Expected |
|------|--------|----------|
| 1 | Connect **Keplr** on LocalTerra; open Swap | Wallet connects; no fee-guard error on load |
| 2 | Submit a **small pool swap** (e.g. LUNC → CW20) | Tx succeeds; no `Transaction fee mismatch` UI copy |
| 3 | (Optional) Disconnect/reconnect Keplr if Station shim was used earlier | Reconnect clears stale `gasPrices`; swap still succeeds |

**Failure triage:** If UI shows **`Transaction fee mismatch`**, check `console.warn` for `[extensionSignedFeeGuard]` diagnostics, run `cd frontend-dapp && npm ci` (cosmes patch), disconnect/reconnect wallet. See [`AGENTS_FRONTEND_STATION_SIGNING.md`](../../skills/AGENTS_FRONTEND_STATION_SIGNING.md).

---

## Manual wallet QA — mainnet / testnet (required for production launch)

**Wallet:** **Keplr (extension)** on **columbus-5** (or staging testnet). **Keplr + Ledger Nano** signing stalls are a separate path ([#567](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/567), [`AGENTS_FRONTEND_KEPLR_LEDGER.md`](../../skills/AGENTS_FRONTEND_KEPLR_LEDGER.md)) — this runbook does not cover hardware-wallet HID recovery.

| Step | Action | Expected |
|------|--------|----------|
| 1 | Connect Keplr on mainnet; open Swap against production/staging build | No fee-guard-related errors (guard is inactive on `columbus-5`) |
| 2 | Submit a **small swap** with normal dApp fee display | Tx broadcasts with fees matching the dApp envelope; **no** `Transaction fee mismatch` from post-sign guard |
| 3 | Confirm on explorer / LCD | `fee.amount` is reasonable for the gas limit (not ~3000 uluna undershoot) |

**Sign-off text (paste on launch tracking issue):** *"Extension post-sign fee guard is LocalTerra-only per SEC-E08. Keplr on `columbus-5` does not require the guard; manual swap QA on `<date-utc>` with Keplr confirmed correct fees without guard intervention. Automated: `make verify-issue-429` PASS at commit `<git-sha>`."*

---

## When to extend the guard

Extend to mainnet **only** if Keplr or another extension is observed rewriting signed fees below the dApp envelope on production networks. If that happens:

1. File a security issue with repro (chain ID, wallet, signed vs expected fee).
2. Update `isLocalTerraChainId` / guard gating with explicit rationale.
3. Re-run this runbook and update SEC-E08 docs before launch.
