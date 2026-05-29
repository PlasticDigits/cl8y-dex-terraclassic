# Agent playbook: Station extension signing

Use when Station users see **false “transaction rejected by user”** errors, **`WalletError: User denied, extension popup was closed`**, or stack traces mentioning **`KeplrExtension`** while connected via Station ([GitLab **#208**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/208)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Station extension signing](../docs/frontend.md#station-extension-signing) | Invariants: amino-only extension path, shim defaults, suggest-chain |
| [GitLab #127](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/127) | LocalTerra fee/gas, `preferNoSetFee`, post-sign guards |
| [GitLab #207](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/207) | LocalTerra connect via `addNetwork` when Keplr shim rejects `localterra` |
| [`stationExtensionConfig.ts`](../frontend-dapp/src/services/terraclassic/stationExtensionConfig.ts) | `applyStationKeplrShimSignDefaults()` |
| [`wallet.ts`](../frontend-dapp/src/services/terraclassic/wallet.ts) | Connect-time + post-connect `experimentalSuggestChain` for Station |
| [`terraBroadcast.ts`](../frontend-dapp/src/services/terraclassic/terraBroadcast.ts) | Re-apply shim defaults before broadcast; popup-closed error copy |
| [`terraWalletSignLock.ts`](../frontend-dapp/src/services/terraclassic/terraWalletSignLock.ts) | Serializes `broadcastTx` — only one extension popup at a time ([#208](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/208)) |
| [`frontend-dapp/patches/@goblinhunt+cosmes+0.0.71-ghunt.21.patch`](../frontend-dapp/patches/@goblinhunt+cosmes+0.0.71-ghunt.21.patch) | `StationController`: `useAminoSigning = true`; **no** second `signAmino` after fee guard ([#208](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/208)) |
| [`cosmesPatch127.test.ts`](../frontend-dapp/src/services/terraclassic/__tests__/cosmesPatch127.test.ts) | Patch regression |

## Rules of thumb

1. **`StationExtension` is `KeplrExtension`** in cosmes — `KeplrExtension` in stack traces is expected; the bug is **signDirect** + Station’s shim, not wrong wallet selection.
2. **Never route Station extension through `signDirect`** — patched `StationController` sets **`useAminoSigning = true`** for all extension connects ([#208](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/208)).
3. **Always set `station.keplr.defaultOptions.sign.preferNoSetFee`** on connect and before broadcast via **`applyStationKeplrShimSignDefaults()`**.
4. **Mainnet Station** — call **`experimentalSuggestChain`** before/after connect (same `gasPriceStep` as Keplr) so fees are not rebuilt from stale steps ([#208](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/208)).
5. **LocalTerra** — keep **`addNetwork`** path from [#207](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/207); do not rely on `experimentalSuggestChain` for `localterra` on new Station builds.
6. **Do not re-prompt `signAmino` after the user approves** — a second call often returns “extension popup was closed” ([#208](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/208)); surface the #127 fee guard instead.
7. After changing the patch, run **`cd frontend-dapp && npm ci`** and **`npm run test:run`** (`cosmesPatch127.test.ts`, `terraWalletSignLock.test.ts` must pass).

## Cross-links

- Gas limits / fee envelope: [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md)
- Connect modal / wrong network: [`AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md`](./AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md), [GitLab #207](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/207)
- User-facing errors: [`AGENTS_FRONTEND_USER_ERRORS.md`](./AGENTS_FRONTEND_USER_ERRORS.md)
