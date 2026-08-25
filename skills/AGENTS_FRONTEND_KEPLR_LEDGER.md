# Agent playbook: Keplr + Ledger Nano signing

Use when Keplr **Ledger** users stall on the Keplr–Ledger UI, see **Signing…** forever, or report that a Terra Classic swap only worked after **refreshing chains in Keplr** and switching the device from **Cosmos** to the **Terra Classic (LUNA)** app ([GitLab **#567**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/567)).

This is a **wallet transport** issue, not a pair/token bug (first report mentioned USTR). The same `broadcastTerraExecuteContracts` path serves Swap, Trade market, limits, pool, wrap, and `/ust1`.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Keplr + Ledger signing](../docs/frontend.md#keplr-ledger-signing) | Invariants **K567-1–K567-8** |
| [docs/qa-onboarding.md § Wallet Matrix](../docs/qa-onboarding.md#wallet-matrix) | Keplr+Ledger Nano **P1** on columbus-5 |
| [docs/user-incident-faq.md § Keplr + Ledger signing stall](../docs/user-incident-faq.md#keplr-ledger-signing-stall) | Retail recovery steps |
| [`keplrExtensionConfig.ts`](../frontend-dapp/src/services/terraclassic/keplrExtensionConfig.ts) | `isNanoLedger` session flag; pre-sign `experimentalSuggestChain` |
| [`terraWalletSignTxRaw.ts`](../frontend-dapp/src/services/terraclassic/terraWalletSignTxRaw.ts) | Exported `walletUsesAmino` — Ledger never `signDirect` |
| [`terraBroadcast.ts`](../frontend-dapp/src/services/terraclassic/terraBroadcast.ts) | Keplr prepare + sign-stall timeout (not #173 30s) |
| [`terraTxTimeout.ts`](../frontend-dapp/src/utils/terraTxTimeout.ts) | Sign-stall copy ≠ `TERRA_TX_BROADCAST_TIMEOUT_MESSAGE` |
| [`TerraBroadcastPendingLink.tsx`](../frontend-dapp/src/components/ui/TerraBroadcastPendingLink.tsx) | Signing-phase Ledger / delayed Keplr hint |

## Invariants (K567-1–K567-8)

1. **K567-1 Software Keplr** — stay on **`signDirect`** unless `useAmino` / `isNanoLedger`. Pre-sign suggest is best-effort; do not spam a new wallet brand or Leap.
2. **K567-2 Ledger amino** — Keplr `getKey().isNanoLedger` or `useAmino` → split path **`signAmino`** + `preferNoSetFee`. **Never** `signDirect` for Ledger.
3. **K567-3 Pre-sign suggest** — `prepareKeplrExtensionForTerraClassicSign` uses only **`getTerraChainSuggestion()`** (coin type 330 lives in metadata, **not** in UI). Failures `console.warn` and continue.
4. **K567-4 Signing hint** — Ledger sees LUNA-app copy immediately; software Keplr has **no** Ledger-only text at t=0; after ~12s a generic Keplr refresh hint is OK. Do not mention coin types 330/118. Do not ask for seed or PIN.
5. **K567-5 Sign-stall timeout** — Keplr extension sign wait is **`TERRA_TX_SIGN_TIMEOUT_MS`** (default **4 min**, `VITE_TERRA_TX_SIGN_TIMEOUT_MS`). **Do not** apply **`TERRA_TX_BROADCAST_TIMEOUT_MS` (30s)** to signing. Stall copy must not say “check your connection”. Retry is allowed only when **no signed bytes** were returned.
6. **K567-6 Post-sign #359** — once a signature exists, recover / do **not** invite immediate retry. Late `signAmino` after UI timeout must **not** broadcast.
7. **K567-7 Fee guard + no second amino** — mainnet post-sign fee guard stays **off** ([#429](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/429)). Do **not** re-prompt `signAmino` after approval ([#208](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/208)).
8. **K567-8 Docs + verify** — frontend subsection, QA matrix, FAQ, this playbook, `make verify-issue-567`.

## Rules of thumb

1. **Not a USTR bug** — do not add pair-specific Ledger branches.
2. **Do not force amino for all Keplr** — software Keplr `signDirect` is the working columbus-5 path.
3. **LocalTerra cannot drive a physical Ledger** ([#235](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/235)). Automated tests are mocks + copy. Acceptance is **manual columbus-5 Keplr + Nano**.
4. **Station+Ledger / Cosmostation+Ledger** are out of scope as separate products; `walletIsNanoLedger` may still show shared hint if the flag is true.
5. **Pre-sign swap summary (#409)** stays phishing-resistant field labels — Ledger help is the signing-phase hint, not the summary.

## Verification

```bash
make verify-issue-567
make test-frontend
make verify-issue-429
```

Manual columbus-5: Terra Classic (LUNA) app open → swap confirms; Cosmos app open → hint visible → switch to LUNA + refresh Keplr chain → retry succeeds; device reject → `Transaction rejected by user`; software Keplr → no Ledger copy, no extra click.

## Cross-links

- Station amino / no second `signAmino`: [`AGENTS_FRONTEND_STATION_SIGNING.md`](./AGENTS_FRONTEND_STATION_SIGNING.md) ([#208](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/208))
- Broadcast / poll timeout vs sign stall: [`AGENTS_FRONTEND_TX_BROADCAST_TIMEOUT.md`](./AGENTS_FRONTEND_TX_BROADCAST_TIMEOUT.md) ([#173](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/173), [#359](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/359))
- Extension fee guard LocalTerra-only: [`AGENTS_EXTENSION_FEE_GUARD.md`](./AGENTS_EXTENSION_FEE_GUARD.md) ([#429](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/429))
- Pre-sign swap summary: [`AGENTS_FRONTEND_SWAP_SIGNING_CONFIRMATION.md`](./AGENTS_FRONTEND_SWAP_SIGNING_CONFIRMATION.md) ([#409](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/409))
- Retail copy: [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) ([#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/489))
- Post-merge Coolify cut: [`AGENTS_POST_MERGE_STACK.md`](./AGENTS_POST_MERGE_STACK.md) ([#573](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/573))
- Keplr Add Token name/logo (not signing): [`AGENTS_KEPLR_CW20_REGISTRY.md`](./AGENTS_KEPLR_CW20_REGISTRY.md) ([#629](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/629))
