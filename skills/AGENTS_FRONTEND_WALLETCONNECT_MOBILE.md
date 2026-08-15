# Agent playbook: WalletConnect same-device mobile pairing

Use when changing **WalletConnect QR / pairing UX**, **Lunc Dash / Galaxy Station mobile connect**, **cosmes `QRCodeModal`**, or **`wc:` deep links** ([GitLab #519](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/519)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § WalletConnect same-device mobile](../docs/frontend.md#walletconnect-same-device-mobile) | Invariants **WC-M1–WC-M7** |
| `frontend-dapp/src/utils/walletConnectPairing.ts` | Mobile detect, `wc:` validation, deep-link builders, scheme allowlist |
| `frontend-dapp/src/services/terraclassic/walletConnectPairingHook.ts` | Registers `globalThis.__CL8Y_WC_PAIRING_MODAL__` at boot |
| `frontend-dapp/src/components/wallet/WalletConnectPairingModal.tsx` | Mobile Open + Copy sheet |
| `frontend-dapp/node_modules/@goblinhunt/cosmes/.../QRCodeModal` (via patch) | Desktop QR unchanged; mobile delegates to the hook |
| Connect list UI | [`AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md`](./AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md) |
| Clipboard | [`AGENTS_FRONTEND_COPY_BUTTON.md`](./AGENTS_FRONTEND_COPY_BUTTON.md) — pairing copy uses `CopyButton` `buttonLabel` |

## Invariants (WC-M1–WC-M7)

1. **WC-M1 — Mobile is not QR-only.** Mobile UA, iPad desktop-UA (`MacIntel` + `maxTouchPoints > 1`), coarse pointer on a tablet-width screen, or viewport ≤767px must offer **Open {wallet}** / **Open wallet** plus **Copy pairing link**. Do not leave a QR the user cannot scan on the same phone.
2. **WC-M2 — Desktop QR unchanged.** When `isWalletConnectMobileClient()` is false, the hook returns `false` and cosmes `QRCodeModal` still shows **Scan via {name}** + canvas. Do not replace the desktop path with AppKit/Web3Modal unless product asks.
3. **WC-M3 — User-gesture deep links only.** Never `window.location.href = …` from the async WalletConnect `display_uri` / `createSession` callback. iOS/Android ignore that (not a tap). Open via `<a href>` / button click.
4. **WC-M4 — Copy the raw `wc:` URI.** Paste target is the wallet’s WalletConnect input. Use [`CopyButton`](../frontend-dapp/src/components/ui/CopyButton.tsx) / `copyToClipboard` — not ad-hoc `navigator.clipboard` in React UI. Vanilla cosmes fallback may copy directly (no React).
5. **WC-M5 — Scheme allowlist.** Pairing `href`s must pass `isAllowedWalletConnectDeepLink` (`wc:`, `luncdash:`, `keplrwallet:`, `galaxystation:`, `intent:`, Hexxagon / Terra Station hosts). Do not open arbitrary URLs from the pairing payload.
6. **WC-M6 — Cosmes hook + fallback.** Patched `QRCodeModal` calls `__CL8Y_WC_PAIRING_MODAL__.open` when present. If the hook is missing, vanilla mobile UI still has **Open** + **Copy pairing link** (no auto-redirect). `postinstall` / `patch-package` required ([#367](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/367)).
7. **WC-M7 — In-app browser still works.** Opening the dApp inside Keplr / Lunc Dash / Galaxy Station’s in-app browser remains a valid alternate path. Document it; do not remove it as “the” fix.

## Lunc Dash scheme

Cosmes already used `luncdash://wallet_connect?payload=<encoded wc:>`. Keep that string in `buildLuncDashDeepLink` — do not invent a second Lunc Dash scheme. If the app is not installed, **Copy pairing link** + **Open wallet** (`wc:`) cover it.

## Rules of thumb

1. Install the hook in [`main.tsx`](../frontend-dapp/src/main.tsx) **before** `createRoot` so it exists before any `connect()`.
2. Do not add `@walletconnect/modal` / Reown AppKit just for this — the dApp already owns a QR dialog via cosmes.
3. Do not treat WalletConnect rows as missing extensions ([#139](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/139)).
4. Keep retail copy short: **Open Lunc Dash**, **Open wallet**, **Copy pairing link**, **Open your wallet, then return here.**
5. After editing the cosmes patch: `npx patch-package @goblinhunt/cosmes` then update [`patches/.cosmes-patch-sha256`](../frontend-dapp/patches/.cosmes-patch-sha256).

## Verification

```bash
make verify-issue-519
# or:
bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
  src/utils/__tests__/walletConnectPairing.test.ts \
  src/services/terraclassic/__tests__/walletConnectPairingHook.test.ts \
  src/components/wallet/__tests__/WalletConnectPairingModal.test.tsx \
  src/services/terraclassic/__tests__/cosmesPatch127.test.ts \
  src/components/ui/__tests__/CopyButton.test.tsx
```

Manual: phone Safari/Chrome → Connect Wallet → Lunc Dash → **Open Lunc Dash** (or copy into the wallet) → approve → address in the header. Desktop: Lunc Dash still shows QR only.

## Cross-links

- Connect modal list / logos: [`AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md`](./AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md)
- Cosmes fork / `postinstall`: [docs/frontend.md § Forked cosmes](../docs/frontend.md#cosmes-fork-patches) · [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md)
- Copy primitive: [`AGENTS_FRONTEND_COPY_BUTTON.md`](./AGENTS_FRONTEND_COPY_BUTTON.md)
- Retail copy: [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md)
