# Agent playbook: transaction broadcast / poll timeout (frontend)

Use when trade or other wallet submits **hang silently** with no error after the wallet prompt, especially under **offline** DevTools or blocked RPC ([GitLab **#173**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/173) — W11-C3).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Transaction broadcast / confirmation timeout](../docs/frontend.md#terra-tx-broadcast-timeout) | Invariant table |
| [`frontend-dapp/src/utils/terraTxTimeout.ts`](../frontend-dapp/src/utils/terraTxTimeout.ts) | Defaults, env overrides, retail messages |
| [`frontend-dapp/src/utils/withPromiseTimeout.ts`](../frontend-dapp/src/utils/withPromiseTimeout.ts) | Shared timeout wrapper |
| [`frontend-dapp/src/services/terraclassic/transactions.ts`](../frontend-dapp/src/services/terraclassic/transactions.ts) | **`executeTerraContract`** / **`executeTerraContractMulti`** |
| [`frontend-dapp/src/services/terraclassic/__tests__/transactions.test.ts`](../frontend-dapp/src/services/terraclassic/__tests__/transactions.test.ts) | Fake-timer broadcast / poll timeout regressions |
| [`frontend-dapp/src/services/terraclassic/terraWalletSignTxRaw.ts`](../frontend-dapp/src/services/terraclassic/terraWalletSignTxRaw.ts) | Split sign + hash before RPC broadcast (extension / dev mnemonic) |
| [`frontend-dapp/src/services/terraclassic/terraTxRecoveryPoll.ts`](../frontend-dapp/src/services/terraclassic/terraTxRecoveryPoll.ts) | Post-sign LCD poll through msg `deadline` ([#359](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/359)) |
| [`frontend-dapp/src/services/terraclassic/__tests__/terraBroadcastRecovery.test.ts`](../frontend-dapp/src/services/terraclassic/__tests__/terraBroadcastRecovery.test.ts) | Recovery regressions ([#359](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/359)) |

## Rules of thumb

1. **Do not add per-page timeouts** — keep caps in **`transactions.ts`** so limit place (two txs), cancel, swap, and pool paths behave consistently.
2. **Mutations recover via `isError`** — **`TxResultAlert`** on trade/limits tickets already shows **`placeMutation.error`**; no extra spinner timeout UI is required when the service layer rejects.
3. **Post-sign hung RPC ≠ “could not broadcast”** — after the wallet signs, compute the tx hash from signed bytes and poll LCD through the swap **`deadline`** before re-enabling submit ([#359](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/359)). Pre-sign failures keep **`TERRA_TX_BROADCAST_TIMEOUT_MESSAGE`** (safe to retry).
4. **Tune poll before broadcast** — mainnet inclusion can exceed 30s; raise **`VITE_TERRA_TX_POLL_TIMEOUT_MS`** before shortening broadcast (traders need to know a hash was sent vs never left the wallet).
5. **LCD outage is separate** — frozen **queries** use [`AGENTS_FRONTEND_LCD_CONNECTIVITY.md`](./AGENTS_FRONTEND_LCD_CONNECTIVITY.md); this playbook is **wallet broadcast / tx poll** only.
6. **Lazy route chunks are separate** — offline navigation to uncached pages uses [`AGENTS_FRONTEND_LAZY_CHUNK_LOAD.md`](./AGENTS_FRONTEND_LAZY_CHUNK_LOAD.md) ([GitLab **#172**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/172)).

## Post-sign broadcast recovery (GitLab #359 / #368)

When the wallet has **already signed** but `broadcast_tx_sync` hangs or fails, the dApp must **not** invite an immediate retry (double-execution risk inside the swap msg deadline).

| Code | Role |
|------|------|
| [`terraWalletSignTxRaw.ts`](../frontend-dapp/src/services/terraclassic/terraWalletSignTxRaw.ts) | Split sign + hash before RPC broadcast (extension / dev mnemonic) |
| [`terraTxRecoveryPoll.ts`](../frontend-dapp/src/services/terraclassic/terraTxRecoveryPoll.ts) | Poll LCD `getTx` through msg deadline — no re-broadcast |
| [`terraMsgDeadline.ts`](../frontend-dapp/src/services/terraclassic/terraMsgDeadline.ts) | Recovery window from execute `deadline` fields |
| [`terraBroadcastRecovery.test.ts`](../frontend-dapp/src/services/terraclassic/__tests__/terraBroadcastRecovery.test.ts) | Unit regressions |

**UX:** `recovering` phase → button **Checking broadcast…** (disabled) + `terra-broadcast-recovery-status` copy (**Broadcast status unknown…**). Pre-sign failures still use **`Could not broadcast the transaction…`**.

**E2E (Simulated Wallet):** [`terra-broadcast-recovery.spec.ts`](../frontend-dapp/e2e/terra-broadcast-recovery.spec.ts) in `e2e-tx` (`make test-e2e-tx`).

**Keplr parity (manual / optional):** same recovery path applies to extension wallets (hash captured in cosmes broadcast). Manual QA: freeze LocalTerra RPC after Keplr approve → expect recovery copy until tx lands; do not click Swap again while button shows **Broadcast status unknown…**. Optional flag: `E2E_KEPLR_BROADCAST_RECOVERY=1` with Keplr extension (not in default CI).

## Manual QA (local)

1. `make start` + `make deploy-local` + `VITE_NETWORK=local npm run dev`, connect **Simulated Wallet**.
2. `/trade` → valid limit → DevTools **Offline** → **Place limit** → approve wallet if prompted.
3. **Pre-sign offline:** within ~30s expect **`Could not broadcast the transaction…`** and **Place limit** enabled again.
4. **Post-sign hung RPC (swap with Simulated Wallet / Keplr extension):** after wallet approval, block RPC (not wallet) → expect **`Broadcast status unknown…`**, phase **`recovering`**, submit disabled until deadline poll resolves ([#359](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/359), [#368](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/368)).

## Related

- Retail error funnel: [`AGENTS_FRONTEND_USER_ERRORS.md`](./AGENTS_FRONTEND_USER_ERRORS.md)
- LCD / query spinners: [`AGENTS_FRONTEND_LCD_CONNECTIVITY.md`](./AGENTS_FRONTEND_LCD_CONNECTIVITY.md)
- Lazy route chunks (offline `import()`): [`AGENTS_FRONTEND_LAZY_CHUNK_LOAD.md`](./AGENTS_FRONTEND_LAZY_CHUNK_LOAD.md) ([GitLab **#172**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/172))
- Limit place two-tx gas preflight: [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md) ([GitLab **#132**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/132))
