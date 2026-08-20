# Agent playbook: WalletConnect same-device mobile pairing

Use when changing **WalletConnect QR / pairing UX**, **Lunc Dash / Galaxy Station / Keplr / Station / Cosmostation mobile connect**, **cosmes `QRCodeModal`**, **`wc:` deep links**, or **Connect Wallet cancel/timeout** ([GitLab #519](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/519), [#554](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/554), [#566](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/566)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § WalletConnect same-device mobile](../docs/frontend.md#walletconnect-same-device-mobile) | Invariants **WC-M1–WC-M12** |
| `frontend-dapp/src/utils/walletConnectPairing.ts` | Mobile detect, `wc:` validation, deep-link builders, Android `intent:` normalize, scheme allowlist |
| `frontend-dapp/src/utils/walletConnectSession.ts` | Bounded WC wait + cancel errors |
| `frontend-dapp/src/services/terraclassic/walletConnectPairingHook.ts` | Registers `globalThis.__CL8Y_WC_PAIRING_MODAL__` at boot; hides Connect list on open |
| `frontend-dapp/src/components/wallet/WalletConnectPairingModal.tsx` | Mobile Open + Copy + Cancel sheet (`z-[10001]`) |
| `frontend-dapp/src/components/wallet/connectWalletOptions.ts` | Mobile Keplr / Station / Cosmostation WalletConnect vs Extension ([#554](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/554) / [#566](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/566)) |
| `frontend-dapp/node_modules/@goblinhunt/cosmes/.../QRCodeModal` (via patch) | Desktop QR unchanged; mobile delegates to the hook |
| Connect list UI | [`AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md`](./AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md) |
| Clipboard | [`AGENTS_FRONTEND_COPY_BUTTON.md`](./AGENTS_FRONTEND_COPY_BUTTON.md) — pairing copy uses `CopyButton` `buttonLabel` |
| Legal after WC | [`AGENTS_FRONTEND_CLICKWRAP.md`](./AGENTS_FRONTEND_CLICKWRAP.md) — Keplr-browser hint, no ADR-036 |

## Invariants (WC-M1–WC-M12)

1. **WC-M1 — Mobile is not QR-only.** Mobile UA, iPad desktop-UA (`MacIntel` + `maxTouchPoints > 1`), coarse pointer on a tablet-width screen, or viewport ≤767px must offer **Open {wallet}** / **Open wallet** plus **Copy pairing link**. Do not leave a QR the user cannot scan on the same phone.
2. **WC-M2 — Desktop QR unchanged.** When `isWalletConnectMobileClient()` is false, the hook returns `false` and cosmes `QRCodeModal` still shows **Scan via {name}** + canvas. Do not replace the desktop path with AppKit/Web3Modal unless product asks.
3. **WC-M3 — User-gesture deep links only.** Never `window.location.href = …` from the async WalletConnect `display_uri` / `createSession` callback. iOS/Android ignore that (not a tap). Open via `<a href>` / button click.
4. **WC-M4 — Copy the raw `wc:` URI.** Paste target is the wallet’s WalletConnect input. Use [`CopyButton`](../frontend-dapp/src/components/ui/CopyButton.tsx) / `copyToClipboard` — not ad-hoc `navigator.clipboard` in React UI. Vanilla cosmes fallback may copy directly (no React).
5. **WC-M5 — Scheme allowlist.** Pairing `href`s must pass `isAllowedWalletConnectDeepLink` (`wc:`, `luncdash:`, `keplrwallet:`, `galaxystation:`, `cosmostation:`, `intent:`, Hexxagon / Terra Station hosts). Do not open arbitrary URLs from the pairing payload. Cosmostation iOS is `cosmostation://wc` ([#566](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/566)).
6. **WC-M6 — Cosmes hook + fallback.** Patched `QRCodeModal` calls `__CL8Y_WC_PAIRING_MODAL__.open` when present. If the hook is missing, vanilla mobile UI still has **Open** + **Copy pairing link** (no auto-redirect). `postinstall` / `patch-package` required ([#367](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/367)).
7. **WC-M7 — In-app browser still works.** Opening the dApp inside Keplr / Lunc Dash / Galaxy Station’s in-app browser remains a valid alternate path. Document it in the Connect modal; do not remove it as “the” fix.
8. **WC-M8 — Pairing is the foreground UI.** Connect Wallet must not sit on top of the pairing sheet. Hide the Connect list when the hook opens; pairing portal uses `z-[10001]` vs Connect `z-[9999]`. Open / Copy must be tappable ([#554](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/554)).
9. **WC-M9 — Bounded connect.** Closing Connect, dismissing pairing, header **Cancel**, or the WC timeout must abort pending `controller.connect()`, clear `isConnecting`, and ignore a late session. Header CTA must not stay a spinner-only disabled button.
10. **WC-M10 — Mobile extension WalletConnect.** When `isWalletConnectMobileClient()` and the matching extension is absent, offer **Keplr / Station / Cosmostation via WalletConnect** — not Install-only. Injected extension (in-app) stays Extension (**WC-M7**). Keplr: [#554](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/554). Station + Cosmostation: [#566](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/566). **Do not re-add Leap** ([#159](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/159)).
11. **WC-M11 — Android Galaxy intent.** Cosmes `https://…#Intent;…` templates must become `intent://` hrefs on Android. Do not leave Chrome to open the Hexxagon website instead of the app.
12. **WC-M12 — Legal next step.** After WC connect without `window.keplr`, show **Open this site in the Keplr browser to accept terms.** Do not implement ADR-036 in the DEX (**C1**). Portal signing stays in `cl8y-ecosystem-legal`.

## Lunc Dash scheme

Cosmes already used `luncdash://wallet_connect?payload=<encoded wc:>`. Keep that string in `buildLuncDashDeepLink` — do not invent a second Lunc Dash scheme. If the app is not installed, **Copy pairing link** + **Open wallet** (`wc:`) cover it.

## Rules of thumb

1. Install the hook in [`main.tsx`](../frontend-dapp/src/main.tsx) **before** `createRoot` so it exists before any `connect()`.
2. Do not add `@walletconnect/modal` / Reown AppKit just for this — the dApp already owns a QR dialog via cosmes.
3. Do not treat WalletConnect rows as missing extensions ([#139](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/139)).
4. Keep retail copy short: **Open Lunc Dash**, **Open Galaxy Station**, **Open Keplr**, **Open Station**, **Open Cosmostation**, **Open wallet**, **Copy pairing link**, **Cancel**, **Open your wallet, then return here.**
5. After editing the cosmes patch: `npx patch-package @goblinhunt/cosmes` then update [`patches/.cosmes-patch-sha256`](../frontend-dapp/patches/.cosmes-patch-sha256).
6. Known-good same-device connect on Android Chrome: ustr-cmm `frontend/` uses one Connect modal + **Cancel** (no pairing sheet under the list). DEX pairing must stay above Connect. ustr-cmm preregister already offers Station WC + Cosmostation WC; DEX must match that without Leap ([#566](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/566) / [#159](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/159)).

## Verification

```bash
make verify-issue-566
make verify-issue-554
make verify-issue-519
# or:
bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
  src/utils/__tests__/walletConnectPairing.test.ts \
  src/utils/__tests__/walletConnectSession.test.ts \
  src/services/terraclassic/__tests__/walletConnectPairingHook.test.ts \
  src/components/wallet/__tests__/WalletConnectPairingModal.test.tsx \
  src/components/wallet/__tests__/WalletModal.test.tsx \
  src/components/wallet/__tests__/connectWalletOptions.test.ts \
  src/hooks/__tests__/useWallet.test.ts \
  src/services/terraclassic/__tests__/cosmesPatch127.test.ts \
  src/components/ui/__tests__/CopyButton.test.tsx
```

Manual: Android Chrome → Connect → Station / Cosmostation / Lunc Dash / Galaxy Station / Keplr → **Open {wallet}** or **Copy pairing link** → approve → `terra1…` in the header. Injected in-app browsers stay Extension. Desktop: Station / Cosmostation stay Extension; Lunc Dash still QR-only. Confirm **Leap** is absent.

## Cross-links

- Connect modal list / logos: [`AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md`](./AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md)
- Cosmes fork / `postinstall`: [docs/frontend.md § Forked cosmes](../docs/frontend.md#cosmes-fork-patches) · [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md)
- Copy primitive: [`AGENTS_FRONTEND_COPY_BUTTON.md`](./AGENTS_FRONTEND_COPY_BUTTON.md)
- Retail copy: [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md)
- Legal clickwrap (no ADR-036): [`AGENTS_FRONTEND_CLICKWRAP.md`](./AGENTS_FRONTEND_CLICKWRAP.md)
- Post-merge Coolify cut: [`AGENTS_POST_MERGE_STACK.md`](./AGENTS_POST_MERGE_STACK.md) ([#573](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/573))
