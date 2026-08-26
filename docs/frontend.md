# Frontend Guide

## Design system

Visual primitives (**QuickSwap-inspired blue + gold**, [#488](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/488)): [`design-system.md`](./design-system.md). Agent playbooks: [`skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md`](../skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md), [`skills/AGENTS_FRONTEND_CHROME_NESTING.md`](../skills/AGENTS_FRONTEND_CHROME_NESTING.md) ([#653](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/653) one chrome layer), [`skills/AGENTS_FRONTEND_THEME_TOGGLE.md`](../skills/AGENTS_FRONTEND_THEME_TOGGLE.md), [`skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](../skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) ([#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/489) docs/skills alignment), [`skills/AGENTS_FRONTEND_OPENGRAPH.md`](../skills/AGENTS_FRONTEND_OPENGRAPH.md) ([#578](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/578) social cards). Class-name migration from neo→glass: [#415](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/415).

### Retail copy & cognitive load {#retail-copy-cognitive-load}

On-card copy stays short: labels ≤ ~5 words, blocking errors ≤ 1 sentence, optional **Docs** link for depth — no instructional paragraphs on primary trade cards ([#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/489)). **Do not merge** always-on educational blurbs, cross-nav “use Swap/UST1” panels, or gas/burn-tax footers that are not live gates — see playbook invariant **9**. Shared terminology: [`design-system.md` § Terminology glossary](./design-system.md#terminology-glossary) (including **24h volume** = trailing window, [#576](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/576)). Agent playbook: [`skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](../skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md). Required risk ack, footer NFA, and trust-boundary warnings stay visible. Retail LUNC LP steps belong in the opt-in `/pool` how-to ([#531](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/531), [§ Retail LUNC liquidity how-to](#retail-lunc-liquidity-howto)) — do not paste this engineering page onto the dApp.

### One chrome layer / anti-nesting {#one-chrome-layer}

Global UI invariant ([GitLab **#653**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/653); Trade application **L561-2**). Agent playbook: [`skills/AGENTS_FRONTEND_CHROME_NESTING.md`](../skills/AGENTS_FRONTEND_CHROME_NESTING.md). Stack: page `--bg-*` → **one** `shell-panel*` per region → content. Mechanical guard: [`scripts/check_chrome_nesting.py`](../scripts/check_chrome_nesting.py) + [`scripts/chrome_nesting_allowlist.txt`](../scripts/chrome_nesting_allowlist.txt).

| ID | Rule |
|----|------|
| **C653-1** | Forbidden: `shell-panel*` wrapping another `shell-panel*`, or wrapping a **grid of** `card-glass` / default `StatBox`, for the same region. Metric tiles are content, not a second chrome region. |
| **C653-2** | Allowlist is short: Swap Pay/Receive `card-glass` (`swap-io-card-*`); a **single** inner well for a table/chart; Trade **sibling** panels (book / chart / ticket / tape). Not a panel-of-panels. |
| **C653-3** | `StatBox` default remains `card`. Call sites **inside** a panel pass `variant="flat"` (`.stat-flat`: no second radius/border/blur). Do not silently flip the default. |
| **C653-4** | Flattened metric grids: Charts pair 24h + TWAP, Trader summary + P&L chips, Protocol Global stats / fees / oracle stat chips. Protocol hub prices stay typographic `dl`. Charts has **no** DEX-census overview strip ([#666](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/666)). Inline Δ% / daily volume chart on Global stats + fees is [#652](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/652) — this ticket only applies `flat`. |
| **C653-5** | Swap IO cards stay nested. `PriceChart` stays a single `shell-panel-strong` (**L561-1**). No `PanelResizeHandle`. |
| **C653-6** | Testids, `title` / `aria-label`, and overview JSON stay. Flatten is chrome + a11y only. |
| **C653-7** | New same-file `shell-panel` + `card-glass` class hits fail the check unless allowlisted. No `eval` of page source. |
| **C653-8** | Light + dark; 375 / 1280: values use `--ink` on `--panel-bg`. No empty-ring tiles. No retail lecture banner. |

Regression: `make verify-issue-653`. Trade workspace still: `make verify-issue-561`.

## Tech Stack

| Layer          | Library                                   |
|----------------|-------------------------------------------|
| Framework      | React 18+ with TypeScript                 |
| Build          | Vite                                      |
| Routing        | React Router                              |
| State          | Zustand (local), React Query (server)     |
| Wallet         | Station wallet / WalletConnect            |
| Styling        | Tailwind CSS                              |
| Testing        | Vitest (unit), Playwright (E2E)           |

## Project Structure

```
frontend-dapp/
├── e2e/                  # Playwright E2E tests
│   └── fixtures/         # Test fixtures (dev wallet, etc.)
├── public/               # Static assets
├── src/
│   ├── components/       # Reusable UI components (incl. `components/legal` — GitLab #138)
│   ├── hooks/            # Custom React hooks (useSwap, usePool, etc.)
│   ├── pages/            # Route-level page components (Swap, Pool, Tiers)
│   ├── services/         # Chain interaction, contract queries
│   ├── stores/           # Zustand stores
│   ├── test/             # Test setup and helpers
│   ├── types/            # TypeScript type definitions
│   └── utils/            # Pure utility functions
├── patches/              # npm patch files
├── vitest.config.ts      # Unit test config
├── vitest.config.integration.ts
└── playwright.config.ts  # E2E test config
```

### Production build — Vite source maps {#vite-production-sourcemaps}

[`frontend-dapp/vite.config.ts`](../frontend-dapp/vite.config.ts) sets `build.sourcemap` from Vite **`mode`**: maps are **off** for the default production build (`vite build`, `mode === 'production'`) and **on** for non-production `vite build --mode …` so staging or custom pipelines can still emit `.js.map` when needed.

| Invariant | Meaning |
|-----------|---------|
| No public maps in prod | Default `npm run build` output must not ship separate `*.js.map` files that static hosts would serve alongside the bundle (reverses minification for attackers). Tracked in [GitLab #117](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/117). |
| Intentional opt-in for other modes | If you need maps for a non-prod build, use `vite build --mode staging` (or similar), not a change to prod defaults. |
| Ecosystem QA checklist | Row **1.20 (source map leakage)** in `cl8y-ecosystem-qa` `specs/DEX-Security-Checklist-DRAFT.md` should reflect **OK** once this invariant is deployed to the environment under review. |

Regression coverage: [`frontend-dapp/src/viteConfig.build.test.ts`](../frontend-dapp/src/viteConfig.build.test.ts) (loads `vite.config.ts` via `loadConfigFromFile`).

| Invariant | Meaning |
|-----------|---------|
| **`tsc -b` before Vite** | `npm run build` runs **`tsc -b`** then **`vite build`**. The TypeScript project includes **Vitest test files** — mocks and route helpers must satisfy strict checking (e.g. **`MediaQueryList`** event listener signatures in [`TradePage.test.tsx`](../frontend-dapp/src/pages/TradePage.test.tsx), optional mock params in [`PriceChart.test.tsx`](../frontend-dapp/src/components/charts/__tests__/PriceChart.test.tsx), type predicates such as **`isKnownFactoryTradePair`** in [`tradePairRoute.ts`](../frontend-dapp/src/utils/tradePairRoute.ts)). QA gate: [GitLab #139](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/139). |

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_PRODUCTION_BUILD.md`](../skills/AGENTS_FRONTEND_PRODUCTION_BUILD.md) · [`skills/AGENTS_FRONTEND_TRUST_BOUNDARIES.md`](../skills/AGENTS_FRONTEND_TRUST_BOUNDARIES.md) ([#378](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/378)).

## Wallet Integration

The dApp connects to Terra Classic wallets using the Station browser extension or WalletConnect for mobile. Key considerations:

- **Network detection:** the `VITE_NETWORK` env var controls which chain the dApp targets (`mainnet`, `testnet`, `local`).
- **Signing:** all transactions use the connected wallet's signer. The dApp never handles private keys in production; the Simulated Wallet (dev only) is an exception and is described below.

### Forked `@goblinhunt/cosmes` and patch-package {#cosmes-fork-patches}

Wallet signing uses the fork **`@goblinhunt/cosmes`** (`frontend-dapp/package.json`, exact version pinned in `package-lock.json`). Upstream cosmes does not ship the Terra Classic extension mitigations we need ([#127](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/127), [#208](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/208)). Local changes are applied with **`patch-package`** on every install:

| Artifact | Purpose |
|----------|---------|
| [`patches/@goblinhunt+cosmes+*.patch`](../frontend-dapp/patches/) | `KeplrExtension`: per-sign **`preferNoSetFee`**, post-sign fee guard vs **`stdDoc.fee`**; `StationController`: extension → **amino always**; `QRCodeModal`: mobile pairing hook + no auto-redirect ([#519](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/519)) |
| [`patches/.cosmes-patch-sha256`](../frontend-dapp/patches/.cosmes-patch-sha256) | Committed SHA-256 of the patch file — CI fails if the patch changes without updating this hash ([#367](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/367)) |
| [`cosmesPatch127.test.ts`](../frontend-dapp/src/services/terraclassic/__tests__/cosmesPatch127.test.ts) | Regression: hash gate + asserts patched symbols exist in **built** `node_modules/@goblinhunt/cosmes/dist/...` after `postinstall` |

| Invariant | Meaning |
|-----------|---------|
| **`postinstall` required** | `package.json` runs `patch-package` in **`postinstall`**. **`npm ci --ignore-scripts`**, broken CI caches, or copying `node_modules` without install **skips patches** — wallet fee guards silently disappear. Production and CI installs must run scripts. |
| **Lockfile pin** | Do not rely on mutable dist tags; keep `@goblinhunt/cosmes` pinned in `package-lock.json`. |
| **No casual upgrades** | Do not bump the fork major/minor without re-running Keplr E2E ([#361](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/361) H12). Track upstream `@goblinhunt/cosmes` for eventual un-fork. |
| **CI gate** | `make test-frontend` includes `cosmesPatch127.test.ts` (content hash + patched `node_modules` symbols). |

**Patch upgrade checklist**

1. Edit `node_modules/@goblinhunt/cosmes` (or bump the dependency if upstream merged fixes), then `cd frontend-dapp && npx patch-package @goblinhunt/cosmes`.
2. Update the hash: `sha256sum patches/@goblinhunt+cosmes+*.patch` → write hex to [`patches/.cosmes-patch-sha256`](../frontend-dapp/patches/.cosmes-patch-sha256).
3. Fresh install: `npm ci` (runs `postinstall` / `patch-package`).
4. Verify: `make test-frontend` — `cosmesPatch127.test.ts` must pass.
5. Re-run Keplr / Station extension signing QA on columbus-5 or staging before release.

**Third-party / agent context:** [`skills/AGENTS_TERRACLASSIC_GAS.md`](../skills/AGENTS_TERRACLASSIC_GAS.md) · [`skills/AGENTS_FRONTEND_STATION_SIGNING.md`](../skills/AGENTS_FRONTEND_STATION_SIGNING.md).

### Connect modal: extension install detection {#connect-modal-extension-install}

Browser **extension** wallets use the same `window` signals as [`getKeplrLikeExtension`](../frontend-dapp/src/services/terraclassic/keplrLikeExtension.ts) plus **`'station' in window`** for Station ([GitLab #139](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/139)). When an extension is detected, the row shows a **Ready** pill next to the **Extension** pill; when it is not, the row is visually subdued and an **Install** link appears — there is **no** separate **Not installed** pill (redundant with **Install**; frees horizontal space on narrow modals, [GitLab #160](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/160)). Long wallet names truncate with an ellipsis; the full name is available via **`title`** on the label. **WalletConnect** rows are unchanged (no extension install check). **Leap** is not listed ([GitLab #159](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/159)). Each production row shows a **circular brand logo** left of the name ([GitLab #490](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/490)) — see [§ Circular wallet logos](#connect-modal-wallet-logos). Implementation: [`walletExtensionInstall.ts`](../frontend-dapp/src/services/terraclassic/walletExtensionInstall.ts), [`WalletModal.tsx`](../frontend-dapp/src/components/wallet/WalletModal.tsx), [`useWalletExtensionInstallSnapshot.ts`](../frontend-dapp/src/hooks/useWalletExtensionInstallSnapshot.ts).

| Invariant | Meaning |
|-----------|---------|
| Align with `getKeplrLikeExtension` | **Cosmostation** detection must stay in sync with [`keplrLikeExtension.ts`](../frontend-dapp/src/services/terraclassic/keplrLikeExtension.ts); if that mapping changes, update **`isBrowserWalletExtensionDetected`** and the Vitest suite in [`walletExtensionInstall.test.ts`](../frontend-dapp/src/services/terraclassic/__tests__/walletExtensionInstall.test.ts). **Leap** is intentionally **not** offered ([GitLab #159](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/159)). |
| Station vs Station shim | **Station** uses **`'station' in window`** (extension injected), not only `station.keplr`, so the row does not depend on the Keplr-shaped shim being present. |
| WalletConnect | **`WalletType.WALLETCONNECT`** options must **not** be treated as missing extensions; they are always offered as QR (desktop) / same-device deep-link + copy (mobile) flows (detection returns “present” for install UI purposes). See [§ WalletConnect same-device mobile](#walletconnect-same-device-mobile) ([#519](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/519), [#554](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/554), [#566](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/566)). |
| No duplicate “missing” chrome | Missing extensions are communicated by the dimmed row + **Install** CTA only — do not add a second **Not installed** badge ([GitLab #160](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/160)). |
| Long labels | Wallet **name** column uses **`min-w-0`**, **`truncate`**, and **`title={name}`** so long names (e.g. **COSMOSTATION**) do not collide with the **Extension** / **Ready** badges on small viewports. |
| Re-check after install | The modal subscribes via **`useSyncExternalStore`** to **`window` `focus`** and **`visibilitychange`** so returning from a store install refreshes badges without a full page reload. |
| Regression tests | [`walletExtensionInstall.test.ts`](../frontend-dapp/src/services/terraclassic/__tests__/walletExtensionInstall.test.ts). |
| **Build gate** | QA checklist item 4: **`npm run build`** and **`npx vitest run`** in `frontend-dapp` must pass on `main` before closing [GitLab #139](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/139). See [Production build — Vite source maps § `tsc -b`](#vite-production-sourcemaps). |

**Third-party / agent context:** [`skills/AGENTS_BUNDLE_DEV_WALLET.md`](../skills/AGENTS_BUNDLE_DEV_WALLET.md) · [`skills/AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md`](../skills/AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md) (connect modal layout + install UX + logos) · [`skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md`](../skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md) (same-device WalletConnect, [#519](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/519) / [#554](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/554) / [#566](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/566)).

### WalletConnect same-device mobile pairing {#walletconnect-same-device-mobile}

On a phone, the wallet app is on the **same device** as the browser — a QR-only pairing sheet cannot be scanned ([GitLab #519](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/519), [#554](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/554)). Cosmes `QRCodeModal` used to `window.location.href` the deep link from the async WalletConnect callback (not a user gesture) and still painted a QR; that is why community reports needed a second device. Desktop → phone scanning stays the desktop path. After #519 shipped, Android Chrome still hung because the pairing sheet sat **under** Connect Wallet (`z-[9999]` vs later portal) with no Cancel — #554 fixes stacking, cancel/timeout, Keplr WC, and Galaxy `intent:`.

| Invariant | Meaning |
|-----------|---------|
| **WC-M1** Mobile sheet | Mobile UA / iPad desktop-UA / coarse+narrow / viewport ≤767px shows **Open {wallet}**, **Open wallet** (`wc:`), and **Copy pairing link** — not QR-only. |
| **WC-M2** Desktop QR | Hook returns `false` on desktop; cosmes still shows **Scan via {name}** + QR canvas. |
| **WC-M3** User gesture | Deep links open from a tap (`<a href>` / button). Do not auto-redirect from `createSession` / `display_uri`. |
| **WC-M4** Copy raw `wc:` | Clipboard writes the pairing URI via [`CopyButton`](../frontend-dapp/src/components/ui/CopyButton.tsx) `buttonLabel` (React) so the user can paste into the wallet. |
| **WC-M5** Allowlist | `isAllowedWalletConnectDeepLink` — `wc:`, `luncdash:`, `keplrwallet:`, `galaxystation:`, `cosmostation:`, `intent:`, Hexxagon / Terra Station hosts only. Cosmostation iOS uses `cosmostation://wc` ([#566](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/566)); Android already uses `intent:`. |
| **WC-M6** Hook + fallback | Boot installs `globalThis.__CL8Y_WC_PAIRING_MODAL__`. Patched cosmes delegates when the hook handles the URI; vanilla mobile fallback still has Open + Copy if the hook is missing. Requires `patch-package` `postinstall`. |
| **WC-M7** In-app browser | Opening the dApp inside a wallet’s in-app browser remains a valid alternate connect path (document; not the only fix). |
| **WC-M8** Pairing foreground | Connect list hides when the pairing hook opens; pairing portal `z-[10001]` above Connect `z-[9999]` so Open / Copy are tappable ([#554](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/554)). |
| **WC-M9** Bounded connect | Cancel / close / timeout abort pending `connect()`, clear `isConnecting`, ignore a late WC session. Header **Cancel** is always visible (not spinner-only). |
| **WC-M10** Mobile extension WC | Mobile + matching extension absent → **Keplr / Station / Cosmostation** **WalletConnect** row (not Install-only). Injected extension (in-app) stays Extension (**WC-M7**). Keplr: [#554](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/554). Station + Cosmostation: [#566](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/566). **Leap** stays absent ([#159](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/159)). |
| **WC-M11** Android Galaxy intent | `https://…#Intent` templates become `intent://` on Android Chrome. |
| **WC-M12** Legal next step | After WC without `window.keplr`, hint to open in the Keplr browser. DEX does not implement ADR-036 (**C1**). |
| Regression | [`walletConnectPairing.test.ts`](../frontend-dapp/src/utils/__tests__/walletConnectPairing.test.ts), [`walletConnectPairingHook.test.ts`](../frontend-dapp/src/services/terraclassic/__tests__/walletConnectPairingHook.test.ts), [`WalletConnectPairingModal.test.tsx`](../frontend-dapp/src/components/wallet/__tests__/WalletConnectPairingModal.test.tsx), [`connectWalletOptions.test.ts`](../frontend-dapp/src/components/wallet/__tests__/connectWalletOptions.test.ts), [`cosmesPatch127.test.ts`](../frontend-dapp/src/services/terraclassic/__tests__/cosmesPatch127.test.ts). `make verify-issue-519`. `make verify-issue-554`. `make verify-issue-566`. |

Implementation: [`walletConnectPairing.ts`](../frontend-dapp/src/utils/walletConnectPairing.ts) (`toAndroidIntentUri` for **WC-M11**), [`walletConnectSession.ts`](../frontend-dapp/src/utils/walletConnectSession.ts), [`walletConnectPairingHook.ts`](../frontend-dapp/src/services/terraclassic/walletConnectPairingHook.ts) (installed from [`main.tsx`](../frontend-dapp/src/main.tsx)), [`WalletConnectPairingModal.tsx`](../frontend-dapp/src/components/wallet/WalletConnectPairingModal.tsx) last in [`Layout.tsx`](../frontend-dapp/src/components/common/Layout.tsx), [`connectWalletOptions.ts`](../frontend-dapp/src/components/wallet/connectWalletOptions.ts). Lunc Dash deep link stays `luncdash://wallet_connect?payload=…` (same as cosmes).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md`](../skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md) (**WC-M1–WC-M12**, [#519](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/519) / [#554](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/554) / [#566](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/566)).

### Connect modal: circular wallet logos {#connect-modal-wallet-logos}

Connect Wallet rows show a fixed **32px circular logo** immediately left of the uppercase wallet name ([GitLab #490](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/490)). Logos are **local static files** under [`frontend-dapp/public/wallets/`](../frontend-dapp/public/wallets/) (mapped in [`walletIconSrc.ts`](../frontend-dapp/src/components/wallet/walletIconSrc.ts), rendered by [`WalletOptionIcon.tsx`](../frontend-dapp/src/components/wallet/WalletOptionIcon.tsx)); they are never hotlinked from CDNs. Asset provenance (official brand kit / product art vs original Simulated glyph) is listed in [`public/wallets/PROVENANCE.md`](../frontend-dapp/public/wallets/PROVENANCE.md).

| Invariant | Meaning |
|-----------|---------|
| Local assets only | `src` paths are `/wallets/*` served by Vite — no remote `img` hosts (CSP + supply-chain). |
| Official / unique marks | Station, Keplr, Cosmostation use verified official marks; LuncDash / Galaxy Station use product logos vendored locally (not generic bridge placeholders). Simulated Wallet (dev) uses an **original** glyph — never a vendor trademark. |
| Decorative a11y | Icons use empty `alt=""` + `aria-hidden` on the circle; the row **`aria-label`** and visible name remain the accessible name. |
| Layout | `.wallet-option-icon` is `flex-shrink: 0` (32×32); name column keeps **`min-w-0` / `truncate`** so **Extension** / **Ready** / **Install** still fit on ~320px widths ([#160](#connect-modal-extension-install)). |
| Behavior unchanged | Connect handlers, install URLs, Ready detection, and WalletConnect wiring are presentation-only for this feature. |
| No Leap | Leap stays absent ([#159](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/159)). |
| Regression tests | [`WalletModal.test.tsx`](../frontend-dapp/src/components/wallet/__tests__/WalletModal.test.tsx) asserts icons + local files under `public/wallets/`. |

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md`](../skills/AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md).

### Connected wallet chip — native LUNC balance {#connected-wallet-chip-lunc-balance}

When a wallet is connected, the header chip must show **bank uluna** as human **LUNC** without opening the menu ([GitLab **#140**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/140), W3-C1 visual checklist). CW20 escrow balances on trade/swap/pool forms are separate; this surface is the **native gas / buying-power** line retailers expect in the shell.

| Invariant | Meaning |
|-----------|---------|
| **Query reuse** | [`useNativeUlunaBalance`](../frontend-dapp/src/hooks/useNativeUlunaBalance.ts) — React Query key `['tokenBalance', address, 'uluna']`, same LCD path as swap/pool (`getTokenBalance` + `native_token.denom`). |
| **Chip + menu** | [`WalletLuncBalance`](../frontend-dapp/src/components/wallet/WalletLuncBalance.tsx) renders on the connected trigger (desktop and mobile widths) and in the dropdown header with the **full** bech32 address. |
| **Formatting** | Six decimals via [`formatTokenAmount`](../frontend-dapp/src/utils/formatAmount.ts); label suffix **`LUNC`**; loading spinner / **`— LUNC`** on error (no silent hide). |
| **`data-testid`** | `wallet-lunc-balance` on the balance span for Vitest and Playwright. |
| **Address row** | Connected menu header uses [`AddressRow`](../frontend-dapp/src/components/ui/AddressRow.tsx) for full bech32 + inline copy + explorer icon ([#188](#addressrow-primitive)); chip trigger network label: [#186](#connected-wallet-chip-network-mobile). |
| **Menu dismiss + Escape** | Connected dropdown uses the same semantic backdrop as shell nav: `type="button"` + `aria-label="Close wallet menu"` + class **`app-menu-dismiss`** ([`WalletButton.tsx`](../frontend-dapp/src/components/wallet/WalletButton.tsx)). **`Escape`** closes the wallet menu via a `window` listener while open; [`Layout.tsx`](../frontend-dapp/src/components/common/Layout.tsx) still owns More-menu Esc ([GitLab **#187**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/187)). |
| **Dropdown menu items** | Labeled rows **Copy address**, **View on explorer**, **Switch wallet** — [#185](#connected-wallet-dropdown). |
| **Out of scope (#140 B)** | Remaining AddressRow surfaces: `TxResultAlert` tx copy — [#188](#addressrow-primitive) (wallet header done). Pair + token-leg chips on Pool / Trade / Charts: [#541](#token-identity). Address explorer URLs: [#184](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/184) — see [Terra Classic block explorer URLs](#terra-classic-block-explorer-urls). Chip network label + mobile layout: [#186](#connected-wallet-chip-network-mobile) — done. |

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_WALLET_CHIP.md`](../skills/AGENTS_FRONTEND_WALLET_CHIP.md) · [`skills/AGENTS_FRONTEND_COPY_BUTTON.md`](../skills/AGENTS_FRONTEND_COPY_BUTTON.md) · [`skills/AGENTS_FRONTEND_ADDRESS_ROW.md`](../skills/AGENTS_FRONTEND_ADDRESS_ROW.md).

### Connected wallet chip — network label & mobile layout {#connected-wallet-chip-network-mobile}

The connected header trigger must expose **which chain** the build targets, not only a chain icon ([GitLab **#186**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/186)). Copy / explorer / switch-wallet labeled menu rows ([#185](#connected-wallet-dropdown)) live in the dropdown at all breakpoints.

| Invariant | Meaning |
|-----------|---------|
| **Single source of truth** | [`getNetworkBadgeCopy`](../frontend-dapp/src/utils/networkDisplay.ts) supplies `shortLabel` / `fullLabel` / `chainId` (same as footer [`NetworkBadge`](../frontend-dapp/src/components/wallet/NetworkBadge.tsx) and [`EnvironmentRibbon`](../frontend-dapp/src/components/legal/EnvironmentRibbon.tsx)). |
| **Desktop trigger (`sm+`)** | [`WalletChipNetworkIndicator`](../frontend-dapp/src/components/wallet/WalletChipNetworkIndicator.tsx) renders chain logo + visible **`shortLabel`** (`data-testid="wallet-network-short-label"`) beside balance/address columns in [`WalletButton`](../frontend-dapp/src/components/wallet/WalletButton.tsx). |
| **Mobile trigger (`<sm`)** | Chip keeps **`WalletLuncBalance`** + truncated `4+4` address; network **text** is hidden on the trigger (icon + `title` tooltip only) to preserve header width. Persistent environment strip ([`EnvironmentRibbon`](../frontend-dapp/src/components/legal/EnvironmentRibbon.tsx)) still shows the active network. |
| **Menu vs chip (#140 W2–C3)** | **Balance** and **address** are on the chip at all breakpoints; **copy**, **explorer**, and **switch/disconnect** are reached by opening the wallet menu (dropdown siblings [#185](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/185)). Do not assume desktop-only chip chrome for QA. |
| **Accessibility** | Connected trigger `aria-label` includes network short label (e.g. `Connected wallet on Local`). |
| **Regression tests** | Vitest: [`WalletButton.test.tsx`](../frontend-dapp/src/components/wallet/__tests__/WalletButton.test.tsx). Playwright: `e2e/navigation.spec.ts` — desktop label, mobile LUNC-without-label, tablet **More ↔ wallet** non-overlap. |

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_WALLET_CHIP.md`](../skills/AGENTS_FRONTEND_WALLET_CHIP.md) · [`skills/AGENTS_FRONTEND_RESPONSIVE_HEADER.md`](../skills/AGENTS_FRONTEND_RESPONSIVE_HEADER.md).

### Connected wallet dropdown menu items {#connected-wallet-dropdown}

Labeled menu rows for retailers who expect explicit actions in the wallet chip menu ([GitLab **#185**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/185), [#140](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/140) scope **B**). Implemented in [`WalletDropdownMenuItems.tsx`](../frontend-dapp/src/components/wallet/WalletDropdownMenuItems.tsx), inserted by [`WalletButton.tsx`](../frontend-dapp/src/components/wallet/WalletButton.tsx) after the `AddressRow` header.

| Invariant | Meaning |
|-----------|---------|
| **Menu order** | Header (`WalletLuncBalance` + `AddressRow`) → **Copy address** → **View on explorer** (if URL) → **Switch wallet** → **Trader profile** → **Disconnect**. |
| **Copy address** | [`CopyButton`](../frontend-dapp/src/components/ui/CopyButton.tsx) with `menuLabel="Copy address"` ([#183](#copy-button-primitive)); full bech32 written to clipboard. |
| **View on explorer** | [`getExplorerAddressUrl`](../frontend-dapp/src/utils/terraExplorer.ts) ([#184](#terra-classic-block-explorer-urls)); omit the row when helper returns `null`. |
| **Switch wallet** | `disconnect()` then `setWalletModalOpen(true)`; connect modal portals while connected (no full-page flow). |
| **`data-testid`** | `wallet-menu-copy-address`, `wallet-menu-view-explorer`, `wallet-menu-switch-wallet`. |
| **Regression tests** | [`WalletDropdownMenuItems.test.tsx`](../frontend-dapp/src/components/wallet/__tests__/WalletDropdownMenuItems.test.tsx), [`WalletButton.test.tsx`](../frontend-dapp/src/components/wallet/__tests__/WalletButton.test.tsx), wallet block in [`navigation.spec.ts`](../frontend-dapp/e2e/navigation.spec.ts).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_WALLET_CHIP.md`](../skills/AGENTS_FRONTEND_WALLET_CHIP.md) · [`skills/AGENTS_FRONTEND_COPY_BUTTON.md`](../skills/AGENTS_FRONTEND_COPY_BUTTON.md) · [`skills/AGENTS_FRONTEND_TERRA_EXPLORER.md`](../skills/AGENTS_FRONTEND_TERRA_EXPLORER.md).

### Copy to clipboard — `CopyButton` primitive {#copy-button-primitive}

Reusable one-click clipboard control for addresses, contract IDs, and tx hashes ([GitLab **#183**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/183)). Wallet **Copy address** menu row uses optional **`menuLabel`** ([#185](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/185) — [Connected wallet dropdown](#connected-wallet-dropdown)); do not add ad-hoc `navigator.clipboard` calls in feature code.

| Invariant | Meaning |
|-----------|---------|
| **Single API path** | [`copyToClipboard`](../frontend-dapp/src/utils/copyToClipboard.ts) wraps `writeText`; UI uses [`CopyButton`](../frontend-dapp/src/components/ui/CopyButton.tsx) only. WalletConnect pairing uses **`buttonLabel`** ([#519](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/519)). |
| **Accessible control** | Button has explicit **`aria-label`**; success/failure is announced in a **`sr-only`** region with **`aria-live="polite"`** and **`aria-atomic="true"`**. |
| **Retail copy** | Strings live in [`copyButtonCopy.ts`](../frontend-dapp/src/utils/copyButtonCopy.ts); failures use a permission-safe message, not raw `DOMException` text. |
| **Trim before write** | Whitespace-only `text` fails without calling the clipboard API. |
| **Feedback window** | Success/error live text clears after **2s** so repeat copies stay screen-reader friendly. |
| **Regression tests** | [`copyToClipboard.test.ts`](../frontend-dapp/src/utils/__tests__/copyToClipboard.test.ts), [`CopyButton.test.tsx`](../frontend-dapp/src/components/ui/__tests__/CopyButton.test.tsx) (mock `navigator.clipboard`). |

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_COPY_BUTTON.md`](../skills/AGENTS_FRONTEND_COPY_BUTTON.md) · [`skills/AGENTS_FRONTEND_ADDRESS_ROW.md`](../skills/AGENTS_FRONTEND_ADDRESS_ROW.md).

### Address display — `AddressRow` primitive {#addressrow-primitive}

Reusable **shortened or full address + copy + explorer** row for bech32 and contract IDs ([GitLab **#188**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/188)). Composes [#183](#copy-button-primitive) `CopyButton` and [#184](#terra-classic-block-explorer-urls) `getExplorerAddressUrl`.

| Invariant | Meaning |
|-----------|---------|
| **Single component** | [`AddressRow`](../frontend-dapp/src/components/ui/AddressRow.tsx) — do not duplicate shorten/copy/explorer markup in feature pages. |
| **Shorten defaults** | `shortenAddress(address, 8, 6)` when `showFull` is false; override with `startChars` / `endChars` when design needs a different chip (e.g. trader header 12/6). |
| **Full text mode** | `showFull` shows the entire string with `break-all` (wallet dropdown menu). |
| **Explorer** | Label and icon link share `getExplorerAddressUrl`; both omitted when the helper returns `null`. |
| **Aria** | Pass explicit `copyAriaLabel` / `explorerAriaLabel` per surface (wallet, LP token, trader). |
| **Regression tests** | [`AddressRow.test.tsx`](../frontend-dapp/src/components/ui/__tests__/AddressRow.test.tsx). |

**First consumers:** wallet menu (`wallet-menu-address-row`), pool LP token line (`pool-lp-token-address-row`), trader profile header (`trader-profile-address-row`). Pair contract chips on Pool / Trade / Charts use the same primitive via [`PairTokenLinks`](../frontend-dapp/src/components/ui/PairTokenLinks.tsx) (`token-identity-pair`) — [Token identity](#token-identity) ([#541](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/541)).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_ADDRESS_ROW.md`](../skills/AGENTS_FRONTEND_ADDRESS_ROW.md) · [`skills/AGENTS_FRONTEND_TOKEN_IDENTITY.md`](../skills/AGENTS_FRONTEND_TOKEN_IDENTITY.md).

### Token identity — Pool / Trade / Charts {#token-identity}

Compact copy + explorer for **both pair legs** and the **pair contract** on `/pool`, `/trade`, and `/charts` ([GitLab **#541**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/541)). One shared row — not a token page, not a Protocol address dump, not an always-on bech32 essay ([#489](#retail-copy-cognitive-load), [#378](#frontend-trust-boundaries)).

| Surface | Placement |
|---------|-----------|
| **`/pool`** | [`PairTokenLinks`](../frontend-dapp/src/components/ui/PairTokenLinks.tsx) on each table row ([#547](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/547)). LP withdraw keeps `pool-lp-token-address-row` on Manage expand. |
| **`/trade`** | Same row under `trade-pair-select-panel`, **outside** the `PairSearchSelect` listbox. Hidden on [#176](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/176) / [#175](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/175) notices. |
| **`/charts`** | Same row under the pair `MenuSelect`. Hidden when “No pairs yet”. |

| Invariant | Meaning |
|-----------|---------|
| **T541-1** | Pool, Trade, and Charts each expose copy + (when applicable) explorer for **both pair legs** and the **pair contract** without opening a wallet menu. |
| **T541-2** | CW20 / pair `href` is only [`getExplorerAddressUrl`](../frontend-dapp/src/utils/terraExplorer.ts); native denoms (`uluna`, `uusd`) are copy-only — never passed to the helper. |
| **T541-3** | Symbol / logo never wrap an `<a>`; explorer is a sibling control (`TokenIdentity` composes `TokenDisplay` + `CopyButton` + icon). |
| **T541-4** | Copy payload is the checksummed contract or the denom, never the display symbol (look-alike tickers stay visible as text). |
| **T541-5** | [#524](#trade-pair-display-invert) invert may reorder chips / labels; `token-identity-base` stays factory `asset_0` and `token-identity-quote` stays `asset_1`. |
| **T541-6** | Invalid / missing pair address: omit the row (no `terra1` placeholders, no links on #176 / #175). |
| **T541-7** | No `/token/:id`, no CoinGecko/CMC/website hosts, no identity icons inside picker options, no factory/router `AddressRow` on these pages. |
| **T541-8** | No always-on address essay or cross-nav banner. Full bech32 is `title` / `aria-label` only. |

**Helper:** [`tokenIdentityTarget`](../frontend-dapp/src/utils/tokenIdentity.ts) → `{ kind: 'cw20', address, explorerUrl } \| { kind: 'native', denom } \| null`.

**Regression:** `make verify-issue-541` — unit + scoped page tests + Playwright smoke `e2e/token-identity-541.spec.ts` (5 workers, no e2e-tx).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_TOKEN_IDENTITY.md`](../skills/AGENTS_FRONTEND_TOKEN_IDENTITY.md). Visible native tickers are **LUNC** / **USTC** ([#630](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/630), [`AGENTS_FRONTEND_NATIVE_TICKERS.md`](../skills/AGENTS_FRONTEND_NATIVE_TICKERS.md)); copy payload stays the denom (**T541-4** / **N630-7**).

### Charts pair-scoped page {#charts-pair-scoped}

[`/charts`](../frontend-dapp/src/pages/ChartsPage.tsx) and `/charts/:pairAddr` are **pair-contextual** ([GitLab **#666**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/666)). Agent playbook: [`skills/AGENTS_FRONTEND_CHARTS_PAIR_SCOPED.md`](../skills/AGENTS_FRONTEND_CHARTS_PAIR_SCOPED.md). DEX-wide census (24h vol / trades / pairs / tokens / USTC) lives on [`/protocol`](#protocol-page) via `GET /api/v1/overview` — Charts must **not** render `charts-overview-*` tiles or call `getOverview()`.

**Layout:** Find pair → pair **24h Stats** (`charts-pair-24h-stats`, immediately **above** `PriceChart`) → chart → TWAP → Recent Trades → Leaderboard. Hide **Best Trade**. Pair 24h content: [Charts pair 24h stats](#charts-pair-stats). Leaderboard: [Charts trader leaderboard](#charts-trader-leaderboard) with `pair=` on `GET /api/v1/traders/leaderboard`.

Invariants **CS-1–CS-15** are in the playbook. Regression: `make verify-issue-666`.

### DEX census overview (Protocol, not Charts) {#charts-overview}

`GET /api/v1/overview` remains the integrator + `/protocol` Global stats payload ([GitLab **#548**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/548)). Agent playbook: [`skills/AGENTS_FRONTEND_CHARTS_OVERVIEW.md`](../skills/AGENTS_FRONTEND_CHARTS_OVERVIEW.md). Format 24h volume with [`formatChartsOverviewVolumeUsd`](../frontend-dapp/src/utils/chartsOverviewStats.ts) on Protocol (`protocol-global-stats` / `protocol-stat-volume-24h`). The window is **trailing** `now − 24h`, not a UTC midnight reset ([#576](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/576); [`trailingWindowCopy.ts`](../frontend-dapp/src/utils/trailingWindowCopy.ts)). `$0` is an idle window, not a daily close.

Do **not** restore Charts testids `charts-overview-volume-usd`, `charts-overview-ustc-usd`, `charts-overview-trades`, `charts-overview-pairs`, or `charts-overview-tokens`. JSON still returns **`total_volume_24h`** (raw `SUM(offer_amount)`) for integrators — Protocol and Charts **must not** render it. Indexer ingest for `volume_usd` is P522-Q ([`volume_usd_for_swap`](../indexer/src/indexer/pair_price_usd.rs)), shared with [#544](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/544). Pair-level 24h stats + TWAP are [#565](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/565) / [#564](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/564) (below). Do **not** change the cutoff to calendar days to make the figure hit `$0`.

Regression: `make verify-issue-548`, `make verify-issue-576`. Trailing-window decay / stale rollup: `make verify-issue-577`, [`skills/AGENTS_INDEXER_VOLUME_WINDOW_DECAY.md`](../skills/AGENTS_INDEXER_VOLUME_WINDOW_DECAY.md).

### Charts pair 24h stats {#charts-pair-stats}

[`/charts`](../frontend-dapp/src/pages/ChartsPage.tsx) pair **24h Stats** primary volume is **USD** ([GitLab **#565**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/565); leftover from #540 / #544 **AC4**). Secondary **Vol ({symbol})** rows and **TWAP Oracle** use human token / quote-per-base scale ([#564](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/564)). Indexer `GET /api/v1/pairs/{addr}/stats` keeps **raw** `volume_base` / `volume_quote` integers. Agent playbook: [`skills/AGENTS_FRONTEND_CHARTS_PAIR_STATS.md`](../skills/AGENTS_FRONTEND_CHARTS_PAIR_STATS.md). On-chain oracle: [`docs/twap-oracle.md`](./twap-oracle.md).

**Layout (#565 / #666 CS-3):** sits **immediately below Find pair** and **above** `PriceChart`. Row 1 — Vol (USD), Trades, Price Change, High/Low/Open/Close (USD); row 2 — Vol (base symbol), Vol (quote symbol). Guard: stats render only when `activePair.pair_address === activePairAddr` (`charts-pair-24h-stats`). Tiles are flat (**C653-4**) — no `card-glass` chips inside the section panel. TWAP stays **below** the chart.

| Box | Source | Display |
|-----|--------|---------|
| **Last 24h Vol (USD)** | `volume_usd` | `$` + compact human ([`formatIndexedVolumeUsd`](../frontend-dapp/src/utils/chartsOverviewStats.ts)). Unpriced (`null` / `"0"` / invalid with trades) → `—`. Idle (`trade_count === 0` and USD `0`) → `$0`. Trailing-window `title` / `aria-label` (`TRAILING_24H_VOLUME_TITLE`, [#576](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/576)). `charts-pair-volume-usd`. |
| **Vol ({symbol})** (secondary) | raw `volume_base` / `volume_quote` × **that pair’s** `asset_0` / `asset_1` decimals | [`formatChartsPairTokenVolume`](../frontend-dapp/src/utils/chartsPairStats.ts). Missing / out-of-range decimals → `—`. Never `formatNum(raw)`. Factory order — [#524](#trade-pair-display-invert) invert does not swap legs. `charts-pair-volume-base` / `charts-pair-volume-quote`. |
| **High/Low/Open/Close (USD)** | factory `*_usd` | [`formatPairStatsUsdOhlc`](../frontend-dapp/src/utils/chartsPairStats.ts). `charts-pair-*-usd`. |

`volume_base` / `volume_quote` stay **raw** in integrator JSON (**P565-6** / **S564-9**). Display `stats.volume_usd` as-is (one notional, **L10**). Do not divide human USD by 1e6. Do not match decimals by symbol.

| ID | Rule |
|----|------|
| **P565-1** | Pair 24h stats **primary** volume is **Vol (USD)** (`formatIndexedVolumeUsd`). |
| **P565-2** | Never `formatNum(stats.volume_base)` or `formatNum(stats.volume_quote)`. |
| **P565-3** | Secondary token vols use `formatChartsPairTokenVolume` with that pair’s leg decimals. |
| **P565-4** | Unpriced / invalid `volume_usd` with trades → `—`; idle → `$0`. |
| **P565-5** | [#524](#trade-pair-display-invert) invert does not change USD or swap leg decimals on stats. |
| **P565-6** | Integrator JSON keeps raw token volumes; `volume_usd` stays human USD. |
| **P565-7** | Token vols render only for the selected pair that fetched stats. |
| **S564-1** | Token vols scale raw sums with **that pair row's** `asset_*.decimals`. UST1 6-dec raw is hundreds, not `385.8M`. |
| **S564-2** | 18-dec quote volume is tens of thousands (`K` OK). Compact `T` only if **human** ≥ 1e12. |
| **S564-3** | Equal-decimal pairs (UST1/cUSTC 6/6) are not extra-scaled by 1e6 or 1e12. |
| **S564-4** | **Vol (USD)** is indexer `volume_usd` via `formatIndexedVolumeUsd`. Do not invent USD in the client. |
| **S564-5** | TWAP 5m/1h/24h is human factory token1-per-token0: `raw × 10^(d0 − d1)` then `formatPairPrice`. Not USD. |
| **S564-6** | Same-decimal TWAP is identity in magnitude. |
| **S564-7** | High/Low/Open/Close (USD) use factory `*_usd` + `formatPairStatsUsdOhlc` (never compact `T`). |
| **S564-8** | Candle histogram scales quote volume by quote decimals (else base by base decimals). Invert does not flip volume (**C543-8**). |
| **S564-9** | Indexer JSON units unchanged. No human-volume field. CG/CMC raw unchanged. |
| **S564-10** | Missing / out-of-range decimals (`0…18`) or non-numeric volume/TWAP → `—`. |
| **S564-11** | Display only — not settlement, limit price, or zap floors. Tape amounts stay [#557](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/557). |

`data-testid`s: `charts-pair-volume-usd`, `charts-pair-volume-base`, `charts-pair-volume-quote`, `charts-pair-*-usd`, `charts-twap-5m` / `1h` / `24h`.

Regression: `make verify-issue-565` · `make verify-issue-564`.

### Charts trader leaderboard {#charts-trader-leaderboard}

[`/charts`](../frontend-dapp/src/pages/ChartsPage.tsx) **Trader leaderboard** is **pair-scoped** ([GitLab **#666**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/666)): `GET /api/v1/traders/leaderboard?pair={addr}&sort=…`. Volume tab is **USD-only** ([GitLab **#553**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/553)). Hide **Best Trade**. Empty copy: “No traders on this pair yet”. Unscoped `GET /leaderboard` (no `pair`) stays DEX-wide for ops / [#657](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/657). Agent playbook: [`skills/AGENTS_FRONTEND_TRADER_VOLUME_USD.md`](../skills/AGENTS_FRONTEND_TRADER_VOLUME_USD.md), [`skills/AGENTS_FRONTEND_CHARTS_PAIR_SCOPED.md`](../skills/AGENTS_FRONTEND_CHARTS_PAIR_SCOPED.md).

| Control | Source | Display |
|---------|--------|---------|
| **Volume (USD)** | `total_volume_usd` | `$` + compact human ([`formatIndexedVolumeUsd`](../frontend-dapp/src/utils/chartsOverviewStats.ts)). Unpriced → `—`. Idle (`total_trades === 0`) → `$0`. `data-testid="charts-leaderboard-volume"`. |
| Rank | `GET /api/v1/traders/leaderboard?sort=total_volume_usd` | `DESC NULLS LAST` so unpriced rows do not rank first. Matches the displayed column (**T553-5**). |
| **Total Volume (USD)** (`/trader`, `/portfolio`) | same field | Same formatter. `data-testid="trader-total-volume-usd"`. |

`GET /api/v1/traders/{addr}` and leaderboard still return **`total_volume`** (raw `SUM(offer_amount)`) for integrators — the dApp **must not** render it as Volume. API default sort remains `total_volume`; Charts explicitly requests `total_volume_usd`. Rolling `volume_24h` / `7d` / `30d` stay raw and are not shown on this table.

Regression: `make verify-issue-553` · `make verify-issue-666`.

### Terra Classic block explorer URLs {#terra-classic-block-explorer-urls}

Network-aware explorer links for transactions and accounts live in [`terraExplorer.ts`](../frontend-dapp/src/utils/terraExplorer.ts). Both helpers read **`VITE_NETWORK`** / [`DEFAULT_NETWORK`](../frontend-dapp/src/utils/constants.ts) and resolve public Finder bases from [`chainlist.json`](../frontend-dapp/public/chains/chainlist.json) (`explorerUrl` per `chainId`). Implemented for [GitLab **#184**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/184); Galaxy Finder mainnet path corrected in [**#478**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/478). Wallet **View on explorer** menu row consumes `getExplorerAddressUrl` ([#185](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/185) — [Connected wallet dropdown](#connected-wallet-dropdown)).

| Helper | Use | `local` | `mainnet` (`columbus-5`) | `testnet` (`rebel-2`) |
|--------|-----|---------|---------------------------|------------------------|
| [`getExplorerTxUrl`](../frontend-dapp/src/utils/terraExplorer.ts) | Tx hashes in alerts, trade history, swaps table | `{lcd}/cosmos/tx/v1beta1/txs/{hash}` | `https://finder.terraclassic.community/columbus-5/tx/{hash}` | `https://finder.terra-classic.hexxagon.io/testnet/tx/{hash}` |
| [`getExplorerAddressUrl`](../frontend-dapp/src/utils/terraExplorer.ts) | Wallet / contract bech32 “View on explorer” | `{lcd}/cosmos/auth/v1beta1/accounts/{addr}` | `…/columbus-5/address/{addr}` | `…/testnet/address/{addr}` |

| Invariant | Meaning |
|-----------|---------|
| **Single source** | Do not hardcode Finder hosts in components; extend `chainlist.json` + `explorerPathBaseForChainId` if explorers change. |
| **Galaxy Finder chain-id path (#478)** | Mainnet (`columbus-5`) `explorerUrl` must be `https://finder.terraclassic.community/columbus-5` — Galaxy Finder routes by **chain id**, not the product label `/mainnet`. Do not revert to `/mainnet`. |
| **Null when unknown** | Missing `explorerUrl` for a chain returns `null` (UI hides the link). |
| **Null when unsafe** | Tx hash must match 64 hex digits; address must pass Terra bech32 validation — otherwise `null` (no injectable `href`; [#430](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/430)). |
| **Local dev LCD** | `local` uses REST paths on [`NETWORKS.local.terra.lcd`](../frontend-dapp/src/utils/constants.ts) (default `http://localhost:1317`), mirroring tx vs account resources. |
| **Regression tests** | [`terraExplorer.test.ts`](../frontend-dapp/src/utils/__tests__/terraExplorer.test.ts) — one case per network for **tx** and **address**, plus adversarial-input cases ([#430](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/430)). |

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_TERRA_EXPLORER.md`](../skills/AGENTS_FRONTEND_TERRA_EXPLORER.md) · [`skills/AGENTS_FRONTEND_ADDRESS_ROW.md`](../skills/AGENTS_FRONTEND_ADDRESS_ROW.md) · [`skills/AGENTS_FRONTEND_ORDER_HISTORY.md`](../skills/AGENTS_FRONTEND_ORDER_HISTORY.md) (tx links only).

### Risk surfacing, NFA copy, and first-visit acknowledgement {#legal-risk-surfacing}

Pre-launch legal / UX requirements are tracked in [GitLab #138](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/138). Implementation overview:

| Surface | Location |
|---------|----------|
| Environment strip | [`EnvironmentRibbon`](../frontend-dapp/src/components/legal/EnvironmentRibbon.tsx) lives in the **footer** on all breakpoints — **local**, **testnet**, and **mainnet** builds all show chain context. Desktop/tablet omit duplicate header [`NetworkBadge`](../frontend-dapp/src/components/wallet/NetworkBadge.tsx) for density ([GitLab **#483**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/483)); wallet chip + footer strip carry network context. Sticky header no longer hosts the ribbon (supersedes under-header seam stacking from [#482](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/482) / [#486](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/486)). |
| NFA + risk summary | [`legalCopy.ts`](../frontend-dapp/src/components/legal/legalCopy.ts) — reused by the modal and [`LegalFooterNotice`](../frontend-dapp/src/components/legal/LegalFooterNotice.tsx) in the **footer on all breakpoints** (footer shell stays visible on mobile above the bottom tab bar, with `EnvironmentRibbon`). Footer includes **Report suspicious activity** → GitLab security template ([#392](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/392), [`SECURITY.md`](../SECURITY.md)). |
| First-visit gate | [`RiskAcknowledgementModal`](../frontend-dapp/src/components/legal/RiskAcknowledgementModal.tsx) — blocking `Modal` (`dismissible={false}`) until the user checks the confirmation and clicks **Continue**; persisted in `localStorage` via [`riskAcknowledgement.ts`](../frontend-dapp/src/utils/riskAcknowledgement.ts). |
| Playwright | `VITE_PLAYWRIGHT_E2E=true` on the Playwright `webServer` env ([`playwright.config.ts`](../frontend-dapp/playwright.config.ts)) skips the gate so E2E is not blocked; do not set this on user-facing production builds. |

| Invariant | Meaning |
|-----------|---------|
| Ack version gate | `RISK_ACK_VERSION` in [`riskAcknowledgement.ts`](../frontend-dapp/src/utils/riskAcknowledgement.ts) must be bumped when risk or NFA copy changes materially so users see the updated gate. |
| Blocking modal | First-visit modal must remain **non-dismissible** by backdrop, Escape, or header close (only the explicit CTA after checkbox). Regression: [`Modal.test.tsx`](../frontend-dapp/src/components/ui/__tests__/Modal.test.tsx), [`RiskAcknowledgementModal.test.tsx`](../frontend-dapp/src/components/legal/__tests__/RiskAcknowledgementModal.test.tsx). |
| E2E vs prod | **`VITE_PLAYWRIGHT_E2E`** is **only** for automated browser tests; production and manual QA should leave it unset so the modal and copy behave as users will see them. |
| Storage key | `cl8y-dex-risk-ack` — changing the key resets acknowledgement for all users; avoid unless migrating storage intentionally. |
| NFA footer on navigation | [`RouteContentReadyProvider`](../frontend-dapp/src/contexts/RouteContentReadyContext.tsx) exposes ready only when `readyForPath === pathname` (stale paths never satisfy a new route), then [`RouteContentReadyMarker`](../frontend-dapp/src/components/common/RouteContentReadyMarker.tsx) sets it via context (not `window` events — child `useEffect` runs before parent listeners and previously dropped the signal; [GitLab #138](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/138)). **Do not** use render-phase `setState` in this provider — it can break React Router tab clicks ([GitLab #182](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/182)). Shell tab clicks must also remount lazy routes via **`Outlet key={pathname}`** in [`Layout.tsx`](../frontend-dapp/src/components/common/Layout.tsx) so Pool/Trade content replaces Swap when the URL changes. Regression: [`RouteContentReadyContext.test.tsx`](../frontend-dapp/src/contexts/__tests__/RouteContentReadyContext.test.tsx), E2E navigation NFA-after-route-change + “navigates to Pool page”. |
| E2E / unit verification | See [`docs/testing.md` § E2E Tests — GitLab #138 verification](./testing.md#e2e-tests) for the checklist (`npm run test:unit`, hybrid seed re-run, Playwright NFA + Pool nav). Commits **`bd763be`** (cosmes patch test + hybrid seed idempotency), **`f58cce5`** (Outlet remount on tab nav). |

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_RISK_DISCLAIMERS.md`](../skills/AGENTS_FRONTEND_RISK_DISCLAIMERS.md).

### CL8Y Legal clickwrap (connected TermsGate) {#legal-clickwrap}

Wallet-bound, versioned Terms & Conditions for the DEX property are tracked in [GitLab #517](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/517). This is **in addition to** the anonymous first-visit risk acknowledgement above — not a replacement.

| Surface | Location |
|---------|----------|
| SDK | [`@plasticdigits/cl8y-clickwrap`](https://gitlab.com/PlasticDigits/cl8y-ecosystem-legal/-/tree/main/packages/cl8y-clickwrap) via [`frontend-dapp/.npmrc`](../frontend-dapp/.npmrc) (GitLab npm registry) |
| Client / property / redirect helpers | [`legalClickwrap.ts`](../frontend-dapp/src/utils/legalClickwrap.ts) |
| Shell gate | [`ConnectedTermsGate`](../frontend-dapp/src/components/legal/ConnectedTermsGate.tsx) wraps `<Outlet>` in [`Layout.tsx`](../frontend-dapp/src/components/common/Layout.tsx) (header/wallet stay mounted so users can disconnect) |
| Signing | Full navigation to Legal portal `sign_urls.terra_classic` — **no** in-dapp ADR-036 verify |
| CSP | Production `connect-src` includes `https://api.terms.cl8y.com` + `https://terms.cl8y.com` ([`viteCsp.ts`](../frontend-dapp/viteCsp.ts)) |
| Playwright | Same `VITE_PLAYWRIGHT_E2E=true` escape hatch as #138 — never on production builds |

| Invariant | Meaning |
|-----------|---------|
| **C1** SDK only | Use `@plasticdigits/cl8y-clickwrap`; do not fork Terra Classic verify in the DEX. After WalletConnect without `window.keplr`, show the Keplr-browser hint ([#554](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/554) **WC-M12**) — portal signing stays in Legal. |
| **C2** Property | Status + portal use **`dex.cl8y.com`** (`VITE_LEGAL_PROPERTY` override for staging only). |
| **C3** Network | `TerraClassic` → API `TERRA_CLASSIC` only. |
| **C4** Sequence | Risk ack (#138) for anonymous browse; clickwrap after wallet connect. |
| **C5** Fail closed | Connected + unknown/error status must not render transactional route children. |
| **C6** Redirect safety | Client `sanitizeRedirectUri` preflight; portal allowlist is source of truth. |
| **C7** CSP | Legal hosts in `connect-src` without blanket `https:`. |
| **C8** No admin secrets | Public Legal endpoints only. |
| **C9** E2E hatch | `VITE_PLAYWRIGHT_E2E` skips gate for Playwright `webServer` only. |
| **C10** NFA retained | Footer NFA / environment ribbon unchanged. |

**Ops (Legal Coolify / admin — cross-repo):** register property `dex.cl8y.com`; add `https://dex.cl8y.com` to Legal API `CORS_ORIGINS` and portal `VITE_REDIRECT_URI_ALLOWLIST`.

**Regression:** `make verify-issue-517` · Vitest `legalClickwrap` / `ConnectedTermsGate` / `viteCsp` · Playwright `e2e/legal-clickwrap-517.spec.ts`.

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_CLICKWRAP.md`](../skills/AGENTS_FRONTEND_CLICKWRAP.md).

### Simulated (dev) wallet and `VITE_DEV_MNEMONIC` {#simulated-dev-wallet-and-vite_dev_mnemonic}

When `VITE_DEV_MODE=true`, the UI can offer a **Simulated Wallet** (no browser extension) implemented in [`devWallet.ts`](../frontend-dapp/src/services/terraclassic/devWallet.ts). Invariants:

| Invariant | Meaning |
|-----------|---------|
| No seed in app source | There is **no** default mnemonic in TypeScript. `VITE_DEV_MNEMONIC` must be supplied at dev time (e.g. `.env.development`, which Vite loads for `vite` / `npm run dev` but not for the default production `vite build`). |
| Same test vector as chain | For LocalTerra, use the same phrase as `TEST_MNEMONIC` in [`docker/init-chain.sh`](../docker/init-chain.sh). `scripts/deploy-dex-local.sh` writes it to `frontend-dapp/.env.development` after deploy. |
| Production build guard | `vite.config.ts` throws if `VITE_DEV_MNEMONIC` is present in the merged env for any `vite build` unless `mode === 'development'` or `VITE_ALLOW_DEV_MNEMONIC=local-only` (staging/production bundles). Tracked in [GitLab #118](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/118), [#378](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/378). |
| WalletConnect project ID | `vite build --mode production` **requires** `VITE_WC_PROJECT_ID`; `wallet.ts` has no shared default ID in the bundle ([#378](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/378)). |
| Production CSP | `vite build --mode production` replaces `index.html` CSP with env-scoped `connect-src` (LCD/RPC/indexer + WalletConnect relay + Legal API/portal — [#517](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/517)) via [`viteCsp.ts`](../frontend-dapp/viteCsp.ts). Dev `vite` keeps broad `https:` in `index.html` for local endpoints. |
| Protocol audit addresses | Factory and router addresses render on [`/protocol`](../frontend-dapp/src/pages/ProtocolPage.tsx) only (`protocol-contract-addresses`) — not on swap confirmation ([#378](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/378)). |
| Protocol global stats + oracle | `/protocol` leads with **Global stats** (`protocol-global-stats`, **Total liquidity** USD + inline 24h/30d snapshot Δ%, volume USD + prior-window Δ%, UTC-day chart — [#569](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/569) / [#652](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/652)) then **Protocol fees** (`protocol-fee-stats`: trailing 24h/7d/30d treasury USD + inline Δ% + source/token mix, [#586](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/586) / [#652](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/652)) then **DEX hub prices** (`protocol-dex-hub-prices`: cUSTC / LUNC / UST1 / USTR from `GET /api/v1/hub-prices`, `formatPairPrice`; cUSTC and LUNC show wrap CW20 `AddressRow`s — `protocol-dex-hub-custc-token` / `protocol-dex-hub-lunc-token`) then **one** CEX oracle card (`protocol-oracle`) with USTC / LUNC / vFDUSD tabs. `getOraclePrice(ticker)` is CEX-only — never `ustr`. `?ticker=` is allowlisted (`ustc` \| `lunc` \| `vfdusd`). Mixed-unit `total_volume_24h` is not the headline. Protocol never reads CG `liquidity_in_usd`. vFDUSD is CEX FDUSD reference, not `$1` and not the UST1 window ([#550](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/550) / [#556](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/556) / [#569](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/569) / [#570](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/570); skills [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](../skills/AGENTS_FRONTEND_PROTOCOL_STATS.md), [`AGENTS_INDEXER_HUB_USD.md`](../skills/AGENTS_INDEXER_HUB_USD.md), [`AGENTS_FRONTEND_PROTOCOL_HUB.md`](../skills/AGENTS_FRONTEND_PROTOCOL_HUB.md)). |
| Protocol external oracle | Bare `/api/v1/oracle/price` is a ticker catalog only — snapshots use `/price/{ticker}` (`ustc`, `lunc`, `vfdusd`) ([#515](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/515) / [#550](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/550); runbook [`runbooks/indexer-external-oracle.md`](./runbooks/indexer-external-oracle.md); skill [`AGENTS_INDEXER_EXTERNAL_ORACLE.md`](../skills/AGENTS_INDEXER_EXTERNAL_ORACLE.md)). |
| Token logo allowlist | [`TokenLogo`](../frontend-dapp/src/components/ui/TokenLogo.tsx) accepts `https:` logos only from [`tokenLogoAllowlist.ts`](../frontend-dapp/src/utils/tokenLogoAllowlist.ts); other hosts fall back to blockies. |
| Expert mode friction | [`ExpertModeModal`](../frontend-dapp/src/components/swap/ExpertModeModal.tsx) requires typing `ENABLE EXPERT MODE` before enable; 30% block / 50% settings cap unchanged ([#378](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/378)). |
| Address in UI | The connected address comes from the `MnemonicWallet` instance (`devWallet.address`), not a hardcoded constant, so a custom dev mnemonic is reflected correctly. |
| Secret scanning | [`.gitleaks.toml`](../.gitleaks.toml) adds a custom rule for BIP39-like quoted phrases under `frontend-dapp/src` (default gitleaks rules do not cover this pattern). |

**Third-party / agent context:** [`skills/AGENTS_BUNDLE_DEV_WALLET.md`](../skills/AGENTS_BUNDLE_DEV_WALLET.md).

### Frontend trust boundaries (GitLab #378) {#frontend-trust-boundaries}

Off-chain hardening for [#376](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/376) remediation ([#378](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/378)). Canonical security narrative: [`security-model.md` § Off-chain trust](./security-model.md#off-chain-trust-boundaries-frontend--indexer).

| Area | Invariant | Code / doc |
|------|-----------|------------|
| Indexer quotes | HTTPS `VITE_INDEXER_URL` in prod; hop summary at swap confirm (`swap-route-summary`); labeled pre-sign panel (`swap-pre-submit-summary`, SEC-D11 / #409) | [`client.ts`](../frontend-dapp/src/services/indexer/client.ts), [`SwapPage.tsx`](../frontend-dapp/src/pages/SwapPage.tsx), [`SwapPreSubmitSummary.tsx`](../frontend-dapp/src/components/swap/SwapPreSubmitSummary.tsx) |
| Build guards | No `VITE_DEV_MNEMONIC` in non-dev builds; `VITE_WC_PROJECT_ID` required for production `vite build` | [`vite.config.ts`](../frontend-dapp/vite.config.ts), [`viteConfig.build.test.ts`](../frontend-dapp/src/viteConfig.build.test.ts) |
| CSP | Production: `script-src 'self'`; `connect-src` = LCD + RPC + indexer + WalletConnect + Legal API/portal (no `https:` wildcard). Dev: broader policy for Vite HMR | [`index.html`](../frontend-dapp/index.html), [`viteCsp.ts`](../frontend-dapp/viteCsp.ts), [`render.yaml`](../render.yaml) |
| Deploy addresses | Factory/router on `/protocol` only; optional LCD sanity check | [`ProtocolPage.tsx`](../frontend-dapp/src/pages/ProtocolPage.tsx), [`deployAddressVerification.ts`](../frontend-dapp/src/utils/deployAddressVerification.ts) |
| Token logos | Host allowlist; evil URLs → blockie | [`tokenLogoAllowlist.ts`](../frontend-dapp/src/utils/tokenLogoAllowlist.ts), [`TokenLogo.tsx`](../frontend-dapp/src/components/ui/TokenLogo.tsx) |
| Expert mode | Type `ENABLE EXPERT MODE` to enable; 30% block / 50% settings cap unchanged | [`ExpertModeModal.tsx`](../frontend-dapp/src/components/swap/ExpertModeModal.tsx), [`swapRouteSlippage.ts`](../frontend-dapp/src/utils/swapRouteSlippage.ts) |
| Footer security link | Public posture doc linked from every page footer (SEC-A01, [#387](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/387)) | [`LegalFooterNotice.tsx`](../frontend-dapp/src/components/legal/LegalFooterNotice.tsx), [`security-posture.md`](./security-posture.md) |

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_TRUST_BOUNDARIES.md`](../skills/AGENTS_FRONTEND_TRUST_BOUNDARIES.md).

### Open Graph / social cards (GitLab #578) {#open-graph-social-cards}

Every public dApp path is rewritten to the same [`index.html`](../frontend-dapp/index.html) shell ([`docker/frontend/nginx.conf`](../docker/frontend/nginx.conf)). Crawlers do not execute React, so Open Graph and Twitter tags are **static** and identical on `/`, `/trade`, `/pool`, `/charts/:pairAddr`, `/trader/:address`, and the other SPA routes. [#488](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/488) shipped the first product card; **#578** replaces that product-copy artwork with the community medallion and bakes **absolute** `https://` image URLs so X/Twitter can render `summary_large_image`.

| Invariant | Meaning |
|-----------|---------|
| **OG-1** Absolute image | Production `og:image` / `twitter:image` are `https://dex.cl8y.com/og-image.png` (or another origin from [`PUBLIC_ORIGIN_ALLOWLIST`](../frontend-dapp/viteOg.ts)). Relative `/og-image.png` is local Vite only. |
| **OG-2** Large card | `twitter:card` is `summary_large_image`. Do not invent `twitter:site`. Title/description stay swaps / limits / Terra Classic. |
| **OG-3** Community composition | Shipped [`public/og-image.png`](../frontend-dapp/public/og-image.png) is **1200×630**, community-medallion art (not a stretched square). Source: [`brand/community-opengraph-concept.png`](../frontend-dapp/brand/community-opengraph-concept.png). Regenerate: `python3 scripts/compose-og-image.py`. |
| **OG-4** File budget | PNG or JPEG, under 5 MB (target under 1 MB). No SVG. Do not serve the square concept as `og:image`. |
| **OG-5** One shell | No `react-helmet`, prerender, or per-route crawler titles. nginx `/og-image.png` is a real image (`try_files $uri =404`). |
| **OG-6** No request origin | OG URLs are never taken from the request host header, `X-Forwarded-Host`, the browser location object, query, hash, pair, or wallet. `VITE_PUBLIC_ORIGIN` must be https and allowlisted or the production build fails. |
| **OG-7** Dimensions + alt | `og:image:width` / `height` match the file; alt text describes the medallion (no user data). |
| **OG-8** Docs + verify | [`design-system.md`](./design-system.md), this subsection, [`AGENTS_FRONTEND_OPENGRAPH.md`](../skills/AGENTS_FRONTEND_OPENGRAPH.md), [#488 QA note](./qa/issue-488/README.md). Verify: `make verify-issue-578`. |

Production bake lives in [`viteOg.ts`](../frontend-dapp/viteOg.ts) (`og-absolute-meta` in [`vite.config.ts`](../frontend-dapp/vite.config.ts)). Coolify passes `VITE_PUBLIC_ORIGIN` (default `https://dex.cl8y.com`) as a Docker build-arg. After deploy, reset the X/Twitter card cache if the previous relative URL was cached.

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_OPENGRAPH.md`](../skills/AGENTS_FRONTEND_OPENGRAPH.md).

- **CW20 allowances:** before `ProvideLiquidity`, the dApp must ensure both CW20 tokens have sufficient allowance for the Pair contract.

### LCD / RPC connectivity (W11-C2) {#lcd-rpc-connectivity}

When the Terra **LCD** endpoint is down, halted, or unreachable, trading routes must not show an **infinite** loading skeleton. Traders see explicit outage copy, a **Retry** control, and **automatic recovery** when the node returns — without requiring a tab switch or full reload ([GitLab **#171**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/171)).

| Invariant | Meaning |
|-----------|---------|
| Classify transport failures | **`isLcdConnectivityError`** in [`lcdConnectivity.ts`](../frontend-dapp/src/utils/lcdConnectivity.ts) recognizes LCD timeouts, `Failed to fetch`, HTTP 5xx query failures, and related transport strings — not business logic (e.g. insufficient funds). |
| Global banner | [`Layout.tsx`](../frontend-dapp/src/components/common/Layout.tsx) renders [`LcdConnectivityBanner`](../frontend-dapp/src/components/common/LcdConnectivityBanner.tsx) (`data-testid="lcd-connectivity-banner"`) when the LCD health probe fails. Copy: **`LCD_CONNECTIVITY_OUTAGE_MESSAGE`**. |
| Recovery polling | [`useLcdConnectivityRecovery`](../frontend-dapp/src/hooks/useLcdConnectivityRecovery.ts) probes `GET …/node_info` every **5s** (`LCD_CONNECTIVITY_RECOVERY_POLL_MS`). On **unreachable → reachable**, it **`invalidateQueries()`** so all React Query caches refetch. |
| Blocking queries | Factory pair lists on **Swap**, **Trade**, and **Limits** use [`LcdQueryGate`](../frontend-dapp/src/components/common/LcdQueryGate.tsx): loading fallback → **`RetryError`** on LCD failure (never a silent empty form). **Pool** cards surface **`RetryError`** on per-pair `pool` query failure. |
| Query defaults | [`App.tsx`](../frontend-dapp/src/App.tsx) `QueryClient`: LCD errors retry up to **3** times with exponential backoff; **`refetchOnReconnect: true`**. |
| Humanization funnel | Banner and gate use **`LCD_CONNECTIVITY_OUTAGE_MESSAGE`** or **`RetryError`** (which humanizes via [`humanizeUserFacingError`](../frontend-dapp/src/utils/humanizeUserFacingError.ts)); do not pre-humanize raw throws. |
| **Funds safety (SEC-E05)** | **`LCD_CONNECTIVITY_OUTAGE_MESSAGE`** must reassure users that **on-chain funds are safe** ([GitLab **#427**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/427)). Current copy: *Could not connect to the network. Your on-chain funds are safe.* Vitest: [`lcdConnectivity.test.ts`](../frontend-dapp/src/utils/__tests__/lcdConnectivity.test.ts), [`LcdQueryGate.test.tsx`](../frontend-dapp/src/components/common/__tests__/LcdQueryGate.test.tsx). |

Regression: [`lcdConnectivity.test.ts`](../frontend-dapp/src/utils/__tests__/lcdConnectivity.test.ts).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_LCD_CONNECTIVITY.md`](../skills/AGENTS_FRONTEND_LCD_CONNECTIVITY.md); error copy funnel: [`skills/AGENTS_FRONTEND_USER_ERRORS.md`](../skills/AGENTS_FRONTEND_USER_ERRORS.md).

### Lazy route chunks (offline navigation) {#lazy-route-chunks}

When the app is already loaded and the trader navigates to a route whose JS chunk is not cached (e.g. **Charts**, **Pool**), going offline must not strand them behind a full-screen crash with a broken **Try Again** ([GitLab **#172**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/172), W11-C3).

| Invariant | Meaning |
|-----------|---------|
| Route-scoped shell | Every lazy page uses [`LazyRoute`](../frontend-dapp/src/components/common/LazyRoute.tsx): route-level [`ErrorBoundary`](../frontend-dapp/src/components/common/ErrorBoundary.tsx) (`isRoute`, `data-testid="route-error-boundary"`) inside [`Layout`](../frontend-dapp/src/components/common/Layout.tsx) — header/nav stay visible. |
| Retry re-imports | **Try Again** calls `onRetry`, which bumps `loadAttempt` so `React.lazy(loader)` and `<Page key={loadAttempt} />` run a fresh `import()` (not just `setState` on the boundary). |
| Chunk classifier | [`isChunkLoadError`](../frontend-dapp/src/utils/chunkLoadError.ts) / [`humanizeOffChainError`](../frontend-dapp/src/utils/humanizeOffChainError.ts) recognize `Failed to fetch dynamically imported module`, `Loading chunk N failed`, etc. |
| Retail copy | Chunk failures show headline **`Page unavailable`** and humanized body copy (offline / stale deploy); technical details scrub dev-server URLs via [`sanitizeChunkLoadTechnicalDetail`](../frontend-dapp/src/utils/chunkLoadError.ts). |
| App-level fallback | Root [`ErrorBoundary`](../frontend-dapp/src/App.tsx) still wraps the tree; chunk errors at app depth offer **Reload App** (`location.reload()`). |
| Trader `resetKeys` | `/trader/:address` passes `resetKeys` so navigation clears a prior route error ([GitLab **#126**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/126)). |
| Trade route fallback | `/trade` and `/trade/:pairAddr` pass `fallback={<TradePageRouteFallback />}` so the Suspense boundary paints a workspace skeleton, not only the generic spinner — see [§ Trade page — initial load / LCP](#trade-page-initial-load). |

Regression: [`chunkLoadError.test.ts`](../frontend-dapp/src/utils/__tests__/chunkLoadError.test.ts), [`LazyRoute.test.tsx`](../frontend-dapp/src/components/common/__tests__/LazyRoute.test.tsx).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_LAZY_CHUNK_LOAD.md`](../skills/AGENTS_FRONTEND_LAZY_CHUNK_LOAD.md).

### Trade page — initial load / LCP (W13-C1) {#trade-page-initial-load}

Hard reload on `/trade` must not show a blank white viewport until all assets finish. A **trade workspace skeleton** should paint within ~200ms; **LCP** should be main trading chrome (skeleton or chart/book), not [`LegalFooterNotice`](../frontend-dapp/src/components/legal/LegalFooterNotice.tsx) ([GitLab **#179**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/179)).

| Invariant | Meaning |
|-----------|---------|
| HTML bootstrap | [`index.html`](../frontend-dapp/index.html) injects `#trade-bootstrap-shell` when `location.pathname` matches `/trade` **before** the Vite bundle executes — themed via the same `data-theme` bootstrap as the app. [`trade-bootstrap.css`](../frontend-dapp/public/bootstrap/trade-bootstrap.css) mirrors a minimal `--bg-0` / `--ink` / `--line` token subset from theme files ([#416](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/416)). |
| Route Suspense fallback | [`TradePageRouteFallback`](../frontend-dapp/src/components/trade/TradePageRouteFallback.tsx) wraps [`TradePageWorkspaceSkeleton`](../frontend-dapp/src/components/trade/TradePageWorkspaceSkeleton.tsx) (`data-testid="trade-workspace-skeleton"`, `min-height` ≥ ~72vh) for lazy chunk load on `/trade`. |
| Factory pair gate | [`TradePage`](../frontend-dapp/src/pages/TradePage.tsx) shows the same workspace skeleton while `allPairs` is loading (`pairsQuery.isLoading`) instead of an empty book/chart grid. |
| Legal footer deferral | [`Layout`](../frontend-dapp/src/components/common/Layout.tsx) renders [`LegalFooterNotice`](../frontend-dapp/src/components/legal/LegalFooterNotice.tsx) only after [`RouteContentReadyProvider`](../frontend-dapp/src/contexts/RouteContentReadyContext.tsx) reports ready (signaled from [`RouteContentReadyMarker`](../frontend-dapp/src/components/common/RouteContentReadyMarker.tsx) when the lazy page mounts), with a 12s failsafe ([`ROUTE_CONTENT_READY_FAILSAFE_MS`](../frontend-dapp/src/contexts/routeContentReadyConstants.ts)). |
| Per-panel loading | After pairs resolve, chart/tape/book panels keep their existing **`Skeleton`** / **`Spinner`** states — do not remove them when extending this work. |
| Desktop skeleton matches live layout | `trade-workspace-skeleton-desktop` is book \| chart \| ticket plus an independent tape row (`trade-workspace-skeleton-desktop-tape`), not a nested chart/tape stack. HTML bootstrap (`trade-bootstrap-grid` + `trade-bootstrap-block-tape`) uses the same tracks. Chart placeholder is `shell-panel-strong` ([#561](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/561) **L561-3**). |

Regression: [`TradePageWorkspaceSkeleton.test.tsx`](../frontend-dapp/src/components/trade/__tests__/TradePageWorkspaceSkeleton.test.tsx), [`RouteContentReadyContext.test.tsx`](../frontend-dapp/src/contexts/__tests__/RouteContentReadyContext.test.tsx), E2E [`trade-page-initial-load.spec.ts`](../frontend-dapp/e2e/trade-page-initial-load.spec.ts).

**Verify (manual):** hard reload `/trade/:pairAddr` → skeleton visible immediately; Lighthouse **LCP** &lt; 4s and LCP element is not `.app-legal-footer-notice`.

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_TRADE_INITIAL_LOAD.md`](../skills/AGENTS_FRONTEND_TRADE_INITIAL_LOAD.md).

### Transaction broadcast / confirmation timeout (W11-C3) {#terra-tx-broadcast-timeout}

Wallet **`broadcastTx`** and LCD **`pollTx`** must not hang indefinitely when the browser or wallet RPC is offline or stalled. After bounded waits, mutations fail with retail copy and the submit control re-enables via React Query **`isPending`** clearing ([GitLab **#173**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/173)).

| Invariant | Meaning |
|-----------|---------|
| Broadcast cap | **`executeTerraContract`** / **`executeTerraContractMulti`** wrap **`wallet.broadcastTx`** with **`withPromiseTimeout`** and **`TERRA_TX_BROADCAST_TIMEOUT_MS`** (default **30s**, override **`VITE_TERRA_TX_BROADCAST_TIMEOUT_MS`**). |
| Poll cap | **`wallet.pollTx`** uses **`TERRA_TX_POLL_TIMEOUT_MS`** (default **90s**, override **`VITE_TERRA_TX_POLL_TIMEOUT_MS`**) so slow LocalTerra blocks can still confirm while offline hangs still surface. |
| Broadcast copy (pre-sign) | **`TERRA_TX_BROADCAST_TIMEOUT_MESSAGE`** — *"Could not broadcast the transaction. Check your connection and try again."* — only when signing did **not** complete (atomic WC `post` path or transport failure before a signed tx exists). Safe to retry immediately. |
| Poll copy | Timeout message: **`TERRA_TX_POLL_TIMEOUT_MESSAGE`** — *"Transaction confirmation timed out. Check your connection and try again."* — when a hash exists, split-path wallets enter recovery instead of surfacing this as the final error ([GitLab **#359**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/359)). |
| Post-sign broadcast unknown | After signing, hung RPC may still deliver the tx. Split-path wallets compute the hash from signed bytes, show **`TERRA_TX_POST_SIGN_BROADCAST_UNKNOWN_MESSAGE`** during phase **`recovering`**, and poll LCD until the swap **`deadline`** (or default **300s**). Submit stays disabled until recovery resolves. |
| Post-sign not found | **`TERRA_TX_POST_SIGN_NOT_FOUND_MESSAGE`** — only after the deadline poll finds no tx; **then** invite retry (avoids double-execution inside the deadline window, [#359](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/359)). |
| Pass-through errors | **`handleBroadcastError`** returns timeout / recovery messages unchanged (no `Transaction failed:` prefix); **`TxResultAlert`** still humanizes via the standard funnel. |
| All on-chain submits | Applies to limit place/cancel, swaps, pool add/withdraw, Mint faucet drip, and any path using **`broadcastTerraExecuteContracts`** — not only `/trade`. |
| Account sequence at sign ([GitLab #499](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/499)) | Split-path **`signTerraTxRaw`** and atomic pre-broadcast refresh call **`getAuthInfo(false)`** so sequence is not reused from wallet connect / prior txs. |
| Code-32 auto-retry once (#499) | On CheckTx **`account sequence mismatch`**, apply the chain-expected sequence (or clear cache), **re-sign + re-broadcast exactly once**. Still a definite rejection — **no** #359 recovery poll (tx never entered the mempool). |
| Code-32 retail copy (#499 / [#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/489)) | After a failed retry, UI shows **`Wallet out of sync. Try again.`** via **`tryHumanizeTerraTxMessage`**; raw chain text stays in **`console.error`**. |

Implementation: [`terraTxTimeout.ts`](../frontend-dapp/src/utils/terraTxTimeout.ts), [`withPromiseTimeout.ts`](../frontend-dapp/src/utils/withPromiseTimeout.ts), [`terraBroadcast.ts`](../frontend-dapp/src/services/terraclassic/terraBroadcast.ts) (canonical sign/broadcast/poll + post-sign recovery + #499 sequence retry), [`terraWalletSignTxRaw.ts`](../frontend-dapp/src/services/terraclassic/terraWalletSignTxRaw.ts), [`terraAccountSequence.ts`](../frontend-dapp/src/utils/terraAccountSequence.ts), [`terraTxRecoveryPoll.ts`](../frontend-dapp/src/services/terraclassic/terraTxRecoveryPoll.ts), [`terraGas.ts`](../frontend-dapp/src/services/terraclassic/terraGas.ts) (gas + `Fee` build), [`transactions.ts`](../frontend-dapp/src/services/terraclassic/transactions.ts) (public `executeTerraContract*` wrappers).

Regression: [`withPromiseTimeout.test.ts`](../frontend-dapp/src/utils/__tests__/withPromiseTimeout.test.ts), [`transactions.test.ts`](../frontend-dapp/src/services/terraclassic/__tests__/transactions.test.ts) (broadcast / poll timeout cases), [`terraBroadcastRecovery.test.ts`](../frontend-dapp/src/services/terraclassic/__tests__/terraBroadcastRecovery.test.ts) ([#359](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/359), [#499](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/499)), [`terraAccountSequence.test.ts`](../frontend-dapp/src/utils/__tests__/terraAccountSequence.test.ts). **Keplr + Ledger sign wait is a separate, longer bound** — see [§ Keplr + Ledger signing](#keplr-ledger-signing) ([#567](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/567)); do not apply **`TERRA_TX_BROADCAST_TIMEOUT_MS`** to `signTerraTxRaw`.

### Broadcast phase UI (signing → confirming) {#broadcast-phase-ui}

Retail submit buttons distinguish wallet signing from on-chain confirmation ([GitLab **#305**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/305), umbrella [#304](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/304)):

| Phase | When | Typical button copy |
|-------|------|---------------------|
| `signing` | Before `wallet.broadcastTx` enters the sign lock | Signing… |
| `broadcasting` | Inside `broadcastTx` (sign + submit) | Broadcasting… |
| `confirming` | After tx hash, during `pollTx` | Confirming… (+ explorer link) |
| `recovering` | Post-sign broadcast/poll timeout — LCD deadline poll ([#359](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/359)) | Checking broadcast… (+ unknown-status copy + explorer link) |

**Invariants:** `broadcastTerraExecuteContracts` accepts optional `onPhaseChange`; failed **pre-sign** broadcast never enters `confirming`; post-sign hung RPC enters **`recovering`** before retry is offered. Failed poll does not re-fire `signing`. React mutations use [`useTerraBroadcastMutation`](../frontend-dapp/src/hooks/useTerraBroadcastMutation.ts) + [`terraBroadcastScope`](../frontend-dapp/src/services/terraclassic/terraBroadcastScope.ts) so service layers stay unchanged. **`isPending`** remains the disable guard.

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_TX_BROADCAST_TIMEOUT.md`](../skills/AGENTS_FRONTEND_TX_BROADCAST_TIMEOUT.md).

### Keplr + Ledger signing {#keplr-ledger-signing}

Keplr connected to a **Ledger Nano** can stall on the Keplr–Ledger UI until the user opens the **Terra Classic (LUNA)** app (not Cosmos) and refreshes Terra Classic in Keplr ([GitLab **#567**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/567)). This is **wallet transport**, not a pair/token bug. The same stack signs Swap, Trade market, limits, pool, wrap, and `/ust1`.

| Invariant | Meaning |
|-----------|---------|
| **K567-1** Software Keplr | Split path uses **`signDirect`** unless `useAmino` / `isNanoLedger`. Pre-sign `experimentalSuggestChain` is best-effort (warn, don’t fail). No new wallet brand / Leap. |
| **K567-2** Ledger amino | `getKey().isNanoLedger` or Keplr `useAmino` → **`signAmino`** + `preferNoSetFee`. **Never** `signDirect` for Ledger. |
| **K567-3** Pre-sign suggest | [`prepareKeplrExtensionForTerraClassicSign`](../frontend-dapp/src/services/terraclassic/keplrExtensionConfig.ts) before Keplr extension sign — only [`getTerraChainSuggestion()`](../frontend-dapp/src/services/terraclassic/terraChainSuggestion.ts) (coin type 330 in metadata, **not** in UI). |
| **K567-4** Signing hint | Ledger: immediate LUNA-app copy during `signing`. Software Keplr: no Ledger-only text at t=0; delayed generic Keplr hint after **`TERRA_TX_SIGNING_HINT_DELAY_MS`** (~12s). No seed/PIN. Button may stay **Signing…**. |
| **K567-5** Sign-stall timeout | **`TERRA_TX_SIGN_TIMEOUT_MS`** (default **4 min**, `VITE_TERRA_TX_SIGN_TIMEOUT_MS`) on Keplr extension sign only. **Not** the 30s **`TERRA_TX_BROADCAST_TIMEOUT_MS`**. Stall copy must not say “check your connection”. Retry allowed only when **no signed bytes** exist. |
| **K567-6** Post-sign #359 | After a signature exists, recover — no immediate retry. Late `signAmino` after UI timeout must **not** broadcast ([`withPromiseTimeout`](../frontend-dapp/src/utils/withPromiseTimeout.ts) ignores late settle). |
| **K567-7** Guardrails | Mainnet fee guard stays **off** ([#429](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/429)). No second `signAmino` after approval ([#208](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/208)). |
| **K567-8** Docs + verify | This subsection, QA matrix Keplr+Ledger Nano columbus-5, FAQ recovery, [`AGENTS_FRONTEND_KEPLR_LEDGER.md`](../skills/AGENTS_FRONTEND_KEPLR_LEDGER.md). Verify: `make verify-issue-567`. |

**LocalTerra / Playwright** cannot drive a physical Ledger ([#235](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/235)). Automated coverage is Vitest mocks + copy. Manual acceptance: **columbus-5 Keplr + Nano**.

Implementation: [`keplrExtensionConfig.ts`](../frontend-dapp/src/services/terraclassic/keplrExtensionConfig.ts), [`terraWalletSignTxRaw.ts`](../frontend-dapp/src/services/terraclassic/terraWalletSignTxRaw.ts) (`walletUsesAmino`), [`terraBroadcast.ts`](../frontend-dapp/src/services/terraclassic/terraBroadcast.ts), [`terraTxTimeout.ts`](../frontend-dapp/src/utils/terraTxTimeout.ts), [`TerraBroadcastPendingLink.tsx`](../frontend-dapp/src/components/ui/TerraBroadcastPendingLink.tsx).


### User-facing errors (wallet, fetch, indexer, tx) {#user-facing-errors-humanization}

Friendly failure copy should flow through **`humanizeUserFacingError`** ([`frontend-dapp/src/utils/humanizeUserFacingError.ts`](../frontend-dapp/src/utils/humanizeUserFacingError.ts)): it applies **`tryHumanizeTerraTxMessage`** first (on-chain / LCD patterns from [GitLab #134](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/134)), then wallet and transport classifiers in [`humanizeOffChainError.ts`](../frontend-dapp/src/utils/humanizeOffChainError.ts) ([GitLab #145](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/145)).

| Invariant | Meaning |
|-----------|---------|
| Single funnel | Call **`humanizeUserFacingError`** / **`humanizeUserFacingErrorFromUnknown`** at leaf call sites, or rely on components that already apply it: **`RetryError`**, **`TxResultAlert`** (`type === 'error'` only), and the **`useWalletStore.connect`** catch (wallet modal). |
| Diagnostics elsewhere | Full throws remain in **`console.error`** / devtools; **ErrorBoundary** adds a collapsed **Technical details** block (chunk failures scrub dev URLs — see [§ Lazy route chunks](#lazy-route-chunks)). Post-sign fee/gas guard failures ([`extensionSignedFeeGuard.ts`](../frontend-dapp/src/utils/extensionSignedFeeGuard.ts), [GitLab #127](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/127)) keep developer diagnostics in logs; UI shows **`EXTENSION_SIGNED_FEE_UNDERSHOOT_USER_MESSAGE`** via **`tryHumanizeTerraTxMessage`** ([GitLab #371](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/371)). Account sequence mismatch after the shared one-shot retry ([GitLab #499](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/499)) maps to **`Wallet out of sync. Try again.`** (raw log in console only — no always-on expandable). |
| Success strings | **`TxResultAlert`** must not rewrite **`type === 'success'`** messages. |
| Regression tests | [`frontend-dapp/src/utils/__tests__/humanizeUserFacingError.test.ts`](../frontend-dapp/src/utils/__tests__/humanizeUserFacingError.test.ts). |
| Retail copy (#414) | User-visible alerts must not embed GitLab issue refs, `Pattern C`, `hybrid_simulation`, or raw doc filenames — use plain language (“limit book + pool”, “estimated output”) and **Learn more** footers ([`directHybridQuote.ts`](../frontend-dapp/src/utils/directHybridQuote.ts), [`swapDisclosure.ts`](../frontend-dapp/src/utils/swapDisclosure.ts), [`LimitOrderPreSubmitSummary.tsx`](../frontend-dapp/src/components/trade/LimitOrderPreSubmitSummary.tsx), [`TradeMarketOrderPanel.tsx`](../frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx)). Market quote errors use **`humanizeUserFacingErrorFromUnknown`**. |

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_USER_ERRORS.md`](../skills/AGENTS_FRONTEND_USER_ERRORS.md).

### Transaction toast notifications {#tx-toast-notifications}

Floating success/error toasts supplement inline **`TxResultAlert`** banners on tx surfaces ([GitLab **#351**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/351)). **`ToastProvider`** ([`ToastContext.tsx`](../frontend-dapp/src/contexts/ToastContext.tsx), hooks in [`toastContextState.ts`](../frontend-dapp/src/contexts/toastContextState.ts)) portals a top-right viewport (`aria-live`, ~6s auto-dismiss, dismiss button). **`useTerraBroadcastMutation`** accepts optional **`toastSuccess`** and defaults **`toastOnError`** to `true`; plain limit mutations use **`useOptionalToast()`** so tests without the provider still pass.

| Invariant | Meaning |
|-----------|---------|
| Dual feedback | Do not remove inline **`TxResultAlert`** on forms when adding toasts. |
| Error copy | Toast errors use **`toastErrorMessage`** / **`humanizeUserFacingError`**. |
| Agent playbook | [`skills/AGENTS_FRONTEND_TX_TOASTS.md`](../skills/AGENTS_FRONTEND_TX_TOASTS.md). |

### Decimal amount inputs {#decimal-amount-inputs}

Retail **human amount** fields (Swap **You Pay**, Settings **book leg amount**, Trade market **book leg override**, etc.) must not pass invalid keystrokes into **`toRawAmount`** / **`BigInt()`** — users must never see raw JS errors such as `Cannot convert … to a BigInt` ([GitLab **#169**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/169), W10-C4 locale / number format).

| Invariant | Meaning |
|-----------|---------|
| Draft regex | Only **`''`**, digits, and at most one **`.`** are accepted in controlled inputs — use **`isDecimalAmountDraft`** from [`decimalAmountInput.ts`](../frontend-dapp/src/utils/decimalAmountInput.ts) (same rule as Swap **You Pay**). |
| Reject at `onChange` | Invalid characters are **not** stored; no toast, modal, or query error for typos. |
| Locale commas | **`,`** is not auto-normalized to **`.`** in these fields; European decimal commas are rejected at the field (product uses `.` only). |
| Downstream safety | **`getDirectHybridBookSplit`** / trade hybrid helpers return **`null`** (no throw) when the book draft is invalid; raw chain integers use **`tryParseBigInt`**. |
| Raw integer protection | Slippage **`min_received`**, pool withdraw minimums, and LP sufficiency checks operate on **raw uint strings** via **`rawAmountMath.ts`** — never **`parseFloat`** / **`Number`** on chain amounts ([GitLab **#237**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/237)). |
| Regression tests | [`decimalAmountInput.test.ts`](../frontend-dapp/src/utils/decimalAmountInput.test.ts), [`rawAmountMath.test.ts`](../frontend-dapp/src/utils/__tests__/rawAmountMath.test.ts), [`swapDisclosure.test.ts`](../frontend-dapp/src/utils/swapDisclosure.test.ts), [`SwapPage.test.tsx`](../frontend-dapp/src/pages/SwapPage.test.tsx). |

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_DECIMAL_AMOUNT_INPUT.md`](../skills/AGENTS_FRONTEND_DECIMAL_AMOUNT_INPUT.md).

### Terra Classic gas limits (router `execute_swap_operations`) {#terra-classic-gas-limits}

The dApp does **not** LCD-simulate every swap before broadcast. All contract executes flow through **`broadcastTerraExecuteContracts`** ([`terraBroadcast.ts`](../frontend-dapp/src/services/terraclassic/terraBroadcast.ts)), which builds **`MsgExecuteContract`** messages, sets **Cosmos `Fee.gas`** via [`terraGas.ts`](../frontend-dapp/src/services/terraclassic/terraGas.ts) + [`constants.ts`](../frontend-dapp/src/utils/constants.ts), then signs/broadcasts/polls. **`executeTerraContract`** / **`executeTerraContractMulti`** are thin wrappers; CW20 allowance-first retail paths use **`executeCw20AllowanceThen`** / **`placeLimitOrderWithAllowance`** ([GitLab #127](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/127)). **Underestimating gas causes on-chain `out of gas` after the wallet signs** (users still pay fees for failed txs).

**Formula (pool-only `execute_swap_operations`, no hybrid hop):** for `hops = max(operations.length, 1)`,

`gasWanted = max( round(SWAP_GAS_PER_HOP × hops × SWAP_GAS_BUFFER) + hops × SWAP_MULTIHOP_GAS_PADDING_PER_HOP, hops × EXECUTE_SWAP_OPS_MIN_GAS_PER_HOP )`

Hybrid hops use quote-driven limits in [`hybridSwapGas.ts`](../frontend-dapp/src/services/terraclassic/hybridSwapGas.ts) (sum per hop for router paths; flat **15M** only when `max_maker_fills` is unknown — [GitLab **#249**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/249)). When `book_input > 0`, **`bookWalkScanOverheadGas`** budgets worst-case **`MAX_SCAN_STEPS`** (500) and **`MAX_EXPIRED_PARKS_PER_SWAP`** (15) offline ([#260](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/260), [#254](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/254), [#262](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/262)); see [`hybridBookWalkLimits.ts`](../frontend-dapp/src/services/terraclassic/hybridBookWalkLimits.ts). Single-hop with a known pair must not use the router ([`swapRouting.ts`](../frontend-dapp/src/services/terraclassic/swapRouting.ts)).

| Invariant | Meaning |
|-----------|---------|
| Buffer tracks chain variance | `SWAP_GAS_BUFFER` must cover wasm execution variance on columbus-5 / LocalTerra; raising it increases **LUNC fee** (`effectiveGasPriceUluna() × gas`) proportionally — trade off vs reliability. |
| Floor guards multi-hop | `EXECUTE_SWAP_OPS_MIN_GAS_PER_HOP` prevents totals from collapsing when buffer × base is still too small for some hop shapes. |
| Padding absorbs rounding | Per-hop padding exists so totals do not sit exactly on prior “just barely enough” values (historical 2-hop near-miss: ~1,320,097 used vs 1,320,000 wanted). |
| Pool-only regression | Single-hop **direct pair** `swap` `gasWanted` must exceed **753,321** gas used in repro [GitLab #115](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/115). With **`SWAP_GAS_BUFFER = 1.3`** plus **`SWAP_GAS_SAFETY_MARGIN` (10k)**, one-hop direct `swap` is **840,000**. |
| Router `execute_swap_operations` ([#353](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/353)) | Native wrap + CW20 router paths use **`gasLimitForRouterExecuteSwapOperations`**: single-hop **1.4M** (`ROUTER_SINGLE_HOP_GAS_LIMIT`, measured ~1.28M); multi-hop floor **`ROUTER_SWAP_OPS_MIN_GAS_PER_HOP` (950k)** per hop (raised from 900k after 2-hop sat on `gasUsed` 1,810,064 vs wanted 1,810,000). **`WRAP_GAS_LIMIT` = 400k** (measured ~301k). Direct pair `swap` stays **840k**. |
| Wrap + ≥2hop combo ([#587](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/587)) | Combined `wrap_deposit` + CW20 `send`→`execute_swap_operations` (N≥2) adds **`WRAP_ROUTER_COMBO_OVERHEAD_GAS` (400k)** in `totalGasLimitForExecuteMsgs`. Wrap+2hop = **2,710,000** (~76.76 LUNC at 28.325) — above the gem-calibrated 2.31M sum. Wrap+1hop stays **1.8M** (#353). Malformed `send.msg` **throws** (`SendHookGasDecodeError`) instead of silent 600k. Native path stays **pool-only** (no hybrid / `book_input`). |
| Unwrap + ≥2hop combo ([#599](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/599)) | Same-msg router `unwrap_output` after N≥2 hops adds **`UNWRAP_ROUTER_COMBO_OVERHEAD_GAS` (400k)** in `gasLimitForSwapOperationsMsg` — analog of the wrap combo so InstantWithdraw after two hub hops (taxed `uusd` on USTR→USTC) is not charged to every unwrap. USTR→USTC / USTR→LUNC = **3,110,000** (~88.09 LUNC at 28.325), above the 2.71M columbus-5 OOG sum. Direct mapper unwrap (cUSTC→USTC / cLUNC→LUNC) stays **800k**. Router 1-hop + unwrap stays **2.2M**. Wrap+2hop+unwrap = **3,910,000**. Native path stays **pool-only** (**H596-7**). Inventory: `send_2hop_unwrap_ustc` / `send_2hop_unwrap`. Verify: `make verify-issue-599`. Post-merge LocalTerra E9 + columbus-5: `make verify-issue-600` ([#600](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/600)). |
| Invoice pay combo ([#595](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/595)) | CW20 `send` → launcher hook (`enable_feature` / `create_token` / `update_settings` / `apply_settings` / `subscribe`) uses **`PAY_INVOICE_SEND_GAS_LIMIT` (600k)**. Combined wrap+2hop+invoice Send (`wrap_plus_2hop_plus_invoice_send`) is **larger** than wrap+2hop swap-only. Playbook: [`AGENTS_FRONTEND_PAY_INVOICE.md`](../skills/AGENTS_FRONTEND_PAY_INVOICE.md). |
| Swap Network fee row ([#587](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/587)) | When a quote is ready, Swap shows **Network fee (est.) ~X LUNC** (`swap-network-fee`) from the same envelope as broadcast. Fee denom is **LUNC / `uluna` only** — 0 USTC does not block a LUNC-funded swap. AMM **Fee** row stays pool `fee_bps`. Optional `gas × price` internals only inside **Trade details**. Native LUNC **Max** uses the same wrap+N-hop hints (`hopCount` defaults to **2** until the route is known). Submit preflight: bank LUNC ≥ pay+fee (native) or fee (CW20). |
| **Min uluna gas price (fee amount)** | `effectiveGasPriceUluna()` in [`constants.ts`](../frontend-dapp/src/utils/constants.ts) floors a low `VITE_GAS_PRICE_ULUNA` at **`MIN_GAS_PRICE_ULUNA` (28.325)**, matching Station `gasPriceStep` / Columbus-5 norms. Without this, **insufficient fee** errors occur at broadcast: high `gas_wanted` but **Fee.amount** computed with a tiny gas price (repro stack: **`increase_allowance`** on `/trade` or `/limits` before the CW20 **`send`** + `place_limit_order` tx — [GitLab #127](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/127)). |
| **Universal broadcast path** | Every on-chain submit uses **`broadcastTerraExecuteContracts`**; do not add parallel `wallet.broadcastTx` call sites. Gas limits live in **`terraGas.ts`**; sequence helpers (`executeCw20AllowanceThen`, `placeLimitOrderWithAllowance`) live in **`transactions.ts`** / **`pair.ts`**. [GitLab #127](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/127). |
| **Station extension + LocalTerra fee steps** | With **`VITE_NETWORK=local`**, [`wallet.ts`](../frontend-dapp/src/services/terraclassic/wallet.ts) still calls **`applyStationKeplrShimSignDefaults()`**, **`ensureStationLocalNetworkRegistered`** (always **`addNetwork`** to refresh **`gasPrices`**, before + after connect when native API exists), and **`experimentalSuggestChain`** when legacy `addNetwork` is unavailable. Patched **`KeplrExtension`**: per-sign **`preferNoSetFee`**, post-sign fee guard compares wallet **`signed.fee`** to **`stdDoc.fee`** (**no** second `signAmino` retry — [#208](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/208)). Run **`npm ci`** ([`cosmesPatch127.test.ts`](../frontend-dapp/src/services/terraclassic/__tests__/cosmesPatch127.test.ts)). **QA on LocalTerra:** **Keplr (extension)** and/or **dev/simulated wallet** only — **not Terra Station** (see [§ Station extension signing](#station-extension-signing)). [GitLab #127](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/127), [#235](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/235). |
| **Station extension signing** {#station-extension-signing} | **`StationExtension` === `KeplrExtension`** in cosmes; stack traces naming **`KeplrExtension`** are normal. Station’s Keplr shim must **not** use **`signDirect`** for extension txs — patched **`StationController`** sets **`useAminoSigning = true`** ([GitLab #208](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/208)). [`applyStationKeplrShimSignDefaults()`](../frontend-dapp/src/services/terraclassic/stationExtensionConfig.ts) on connect + before broadcast; **`withTerraWalletSignLock`** serializes extension `broadcastTx`. **Mainnet / columbus-5:** `experimentalSuggestChain` before/after connect refreshes **`gasPriceStep`** — **Terra Station P0 verification** (swap, limits, bids, wrap/unwrap) belongs here or on pre-release **columbus-5** staging with non-economic tokens. **LocalTerra connect:** [#207](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/207) **`addNetwork`** always (refresh stale **`gasPrices`**, [#127](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/127)) before + after connect when supported. Post-sign guard compares **`signed.fee`** to **`stdDoc.fee`**. False **`extension popup was closed`** → retail copy in [`terraBroadcast.ts`](../frontend-dapp/src/services/terraclassic/terraBroadcast.ts) / [`humanizeOffChainError.ts`](../frontend-dapp/src/utils/humanizeOffChainError.ts). **Known limitation — Station on LocalTerra:** Station ships a built-in **`localterra`** entry with stale **~0.015 uluna/gas**; our LocalTerra node ante handler requires **28.325 uluna/gas** (`localterra-cl8y`). Station **ignores** dApp fee overrides (`preferNoSetFee`, `experimentalSuggestChain`, `addNetwork`), so Station-signed txs on LocalTerra fail with insufficient fees (e.g. **3000 uluna** vs **~23M uluna** required). **Do not use Station for LocalTerra fee/signing QA** — use **Keplr** or the **dev/simulated wallet** ([#235](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/235)). Agents: [`skills/AGENTS_FRONTEND_STATION_SIGNING.md`](../skills/AGENTS_FRONTEND_STATION_SIGNING.md) · [`skills/AGENTS_TERRACLASSIC_GAS.md`](../skills/AGENTS_TERRACLASSIC_GAS.md). |
| Limit / bid placement sequence | **`placeLimitOrderWithAllowance`** → **`executeCw20AllowanceThen`** → two **`broadcastTerraExecuteContracts`** calls (`increase_allowance` then CW20 `send` / `place_limit_order`). Allowance tx is the usual first failure when fee math is wrong. |
| Limit cancel | **`cancelLimitOrder`** → `CANCEL_LIMIT_ORDER_GAS_LIMIT` (**1M**; tax pair→EOA refund OOGs at 450k, [#625](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/625)). |
| Limit parked-expired claim ([GitLab #141](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/141), batch [#246](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/246), **Claim all** [#253](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/253)) | **`claimExpiredLimitOrder`** → `CLAIM_EXPIRED_LIMIT_ORDER_GAS_LIMIT`; **`claimExpiredLimitOrders`** → `gasLimitForLimitOrderCancelBatch(n)`. Shared hook: [`useLimitExpiredClaimMutation`](../frontend-dapp/src/hooks/useLimitExpiredClaimMutation.ts). |
| Limit place — native balance vs **two** fees ([GitLab #132](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/132)) | The UI must not broadcast `increase_allowance` unless bank **uluna** ≥ `estimateLimitOrderPlaceSequenceUlunaFeesTotal()` (same model as two separate `Fee.amount` values). See [`limitOrderNativeGasBalanceGate.ts`](../frontend-dapp/src/utils/limitOrderNativeGasBalanceGate.ts), [`docs/limit-orders.md` § dApp retail form](./limit-orders.md#dapp-retail-form-wires-invariants). |
| Limit place batch / tax Send ([#206](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/206) / [#625](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/625)) | Retail place is CW20 `send` → `place_limit_order_batch` `n=1`. **`PLACE_LIMIT_ORDER_BATCH_BASE_GAS_LIMIT` is 1M** (was 400k) so tax `Send` + pair place is **1.18M**, not 580k OOG. Send-inner `place_limit_order` maps to **`PLACE_LIMIT_ORDER_GAS_LIMIT` (1.2M)**, not `SWAP_GAS_LIMIT`. Swarm `gas.ts` lockstep. |
| Fee-discount register / deregister ([GitLab #384](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/384), FT-3 / FT-4) | `/tiers` **`register`** / **`deregister`** execute msgs must map in **`getGasLimitForTx`** to **`REGISTER_FEE_DISCOUNT_GAS_LIMIT` (300k)** and **`DEREGISTER_FEE_DISCOUNT_GAS_LIMIT` (250k)** — not **`BASE_GAS_LIMIT` (200k)**. Measured LocalTerra tier-1 register ≈ **204,438** gas (exceeded 200k fallback → wallet “needed more gas than estimated”). dApp does **not** LCD-simulate execute gas; per-message fallbacks are the canonical envelope ([`terraClassicFeeEstimate.ts`](../frontend-dapp/src/services/terraclassic/terraClassicFeeEstimate.ts)). Verify: `make verify-issue-384`. |
| Retail execute inventory / `BASE_GAS_LIMIT` guardrail ([GitLab #475](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/475), Mint drip [#474](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/474), UST1 window [#506](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/506)) | Every retail `executeTerraContract*` shape (and CW20 `send` inner) must be registered in **`getGasLimitForTx`** with a named constant, or explicitly allowlisted for **`BASE_GAS_LIMIT`**. **Allowlist (intentional 200k only):** `increase_allowance` / `decrease_allowance`. **Mint faucet `drip`:** **`FAUCET_DRIP_GAS_LIMIT` (400k)** — missing mapping caused deterministic mainnet OOG after soft-launch (#473). **CW20 `send` → `unwrap`:** **`UNWRAP_GAS_LIMIT` (800k)** (not legacy `SWAP_GAS_LIMIT`; LCD sim ~562k; 400k/550k OOG on mainnet). **CW20 `send` → ust1-window `{ deposit }` / `{ withdraw }`:** **`UST1_WINDOW_SEND_GAS_LIMIT` (800k)**. Inventory + CI fixtures: [`terraGasRetailInventory.ts`](../frontend-dapp/src/services/terraclassic/terraGasRetailInventory.ts). Dev builds `console.warn` on unmapped fallback. Verify: `make verify-issue-475` / `make verify-issue-506`. Invariants **G-RETAIL-1 / G-RETAIL-2** in that module + [`skills/AGENTS_TERRACLASSIC_GAS.md`](../skills/AGENTS_TERRACLASSIC_GAS.md). |
| Pool add — CW20/CW20 path, native balance vs **three** fees ([GitLab #147](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/147)) | `provideLiquidity` (`pair.ts`) sends **three** txs: `increase_allowance` ×2 then `provide_liquidity`. The UI must not broadcast the first unless bank **uluna** ≥ `estimateProvideLiquidityCw20SequenceUlunaFeesTotal()` (three `Fee.amount` sums). Native/wrap paths use `executeTerraContractMulti` (one fee). **Rollback** after failed `provide_liquidity`: both `decrease_allowance` messages go out in **one** `executeTerraContractMulti` (one fee). See [`provideLiquidityNativeGasBalanceGate.ts`](../frontend-dapp/src/utils/provideLiquidityNativeGasBalanceGate.ts), [`docs/frontend.md` § Pool page](./frontend.md#pool-page--provide-liquidity-ui-invariants). |

**Operational alignment:** local/mainnet helper scripts use `terrad … --gas-adjustment 1.3` (see `scripts/deploy-dex-local.sh`). **`SWAP_GAS_BUFFER` is set to 1.3** so the dApp matches that default rather than a looser multiplier.

**Third-party / agent context:** see repository [`skills/AGENTS_TERRACLASSIC_GAS.md`](../skills/AGENTS_TERRACLASSIC_GAS.md) for a short playbook when changing gas constants or debugging `out of gas`. [`packages/localnet-trading-swarm/src/gas.ts`](../packages/localnet-trading-swarm/src/gas.ts) mirrors the same buffer for scripted swaps on LocalTerra ([GitLab #115](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/115)).

### Pay with any token (DEX-routed invoice) {#pay-with-any-token}

Shared checkout for paid protocol features ([GitLab **#595**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/595)). First consumers: community tax-token SKU unlocks and manager settings **batch** Save ([#592](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/592) / [#593](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/593)). Next: prepaid market-making subscription ([#597](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/597)). **Do not** copy Swap quote/execute into those pages.

Columbus-5 Create Token env (unset → page unavailable): `VITE_COMMUNITY_TAX_CODE_ID=11619`, `VITE_COMMUNITY_TOKEN_LAUNCHER=terra126pr5323xkhwas7y03azv48sqr2fy3fxxg0sxu8xhmjdxr8v5tzqahzwze` (code **11622**). Token CMM-admin banner: `terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2`. Launcher wasm admin is DEX 2-of-3. Pins: [`deployments/mainnet-ust1-wrap/REGISTRY.md`](../deployments/mainnet-ust1-wrap/REGISTRY.md) · [#601](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/601). LocalTerra: `make deploy-local` writes the **local** store id + launcher + QA tax token ([#620](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/620), [`AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED.md`](../skills/AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED.md)) — do not bake 11611/11619 into `.env.local`.

Code: [`payInvoice.ts`](../frontend-dapp/src/utils/payInvoice.ts), [`PayWithAnyToken.tsx`](../frontend-dapp/src/components/payments/PayWithAnyToken.tsx). Playbook: [`skills/AGENTS_FRONTEND_PAY_INVOICE.md`](../skills/AGENTS_FRONTEND_PAY_INVOICE.md). Verify: `make verify-issue-595`.

| Invariant | Meaning |
|-----------|---------|
| **I595-1** Canonical invoice | SKU / settings batch stay **50 UST1** (or `N × 50` for SKU count) on-chain. The module only acquires ≥ invoice of that CW20. |
| **I595-2** One broadcast | `buildPayInvoiceMsgs` → one `executeTerraContractMulti`. Two user txs is a fail. |
| **I595-3** Exact payee credit | Swap `to` = user; last `Send` amount = invoice. Excess invoice token refunds to the user. Do not set swap `to = payee`. |
| **I595-4** No pair FoT math | Existing router + pairs only (**H-01**). |
| **I595-5** Reuse solver | `GET /route/solve` (path) + `ReverseSimulateSwapOperations` (`amount_in`) + forward sim `out ≥ invoice`. Indexer exact-out is a follow-up. |
| **I595-6** Invoice is a floor | `minimum_receive` ≥ invoice. Slippage scales **max_in** only (`applySlippagePercentCeiling`). |
| **I595-7** No route | Frozen / gem-bridge / unroutable → disable + **No route**. |
| **I595-8** Same-asset Send | Pay token = invoice token → one `Send`, no router. |
| **I595-9** Native wrap | LUNC/USTC wrap then route when wrap env is set. |
| **I595-10** Payee from config | `Invoice.payee` from the caller (launcher env). Never URL/query. |
| **I595-11** No unlimited allowance | `Send` amount = quoted debit only. |
| **I595-12** Copy | “You pay ~X TOKEN (incl. DEX swap) → 50 UST1 fee”. CTA **Pay** / **Enable**. One Route row. |
| **I595-13** Gas | `PAY_INVOICE_SEND_GAS_LIMIT` (600k) + wrap+N-hop combo. `make verify-issue-475` stays green. |
| **I595-14** Launcher stays dumb | #592 accepts only invoice-token `Send`. Routing is not inside the launcher. |

**v1 settlement** is wallet multi-msg (swap to user, then exact invoice `Send`). An on-chain `invoice-payer` adapter is recommended later (one user `Send`, allowance-friendly) but is **not** required for #593 to import this module.

### Create Token (community tax) {#create-token-community-tax}

Retail create/manage for the #592 template ([GitLab **#593**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/593)). Identity + wallet helpers: [#604](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/604). SKU init + percent taxes: [#605](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/605). Routes: `/token/create`, `/token/migrate`, `/token/:addr/manage`, `/tokens`. Catalog: [#594](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/594). Playbook: [`skills/AGENTS_FRONTEND_CREATE_TOKEN.md`](../skills/AGENTS_FRONTEND_CREATE_TOKEN.md). Free listed-template adopt: [#626](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/626) / [`AGENTS_FRONTEND_TOKEN_MIGRATE.md`](../skills/AGENTS_FRONTEND_TOKEN_MIGRATE.md). Verify: `make verify-issue-593` · `make verify-issue-604` · `make verify-issue-605` · `make verify-issue-626` · `make verify-issue-628` · `make verify-issue-634`. Post-merge Coolify + LocalTerra retail: [#602](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/602) / `make verify-issue-602`.

| Invariant | Meaning |
|-----------|---------|
| **C593-1** Env gate | `VITE_COMMUNITY_TAX_CODE_ID` + `VITE_COMMUNITY_TOKEN_LAUNCHER`. Unset → unavailable + hidden More item. |
| **C593-2** Names | **Create Token** ≠ Mint ≠ Create Pair. |
| **C593-3** Pay card | SKU + settings Save import `PayWithAnyToken` (**I595**). |
| **C593-4** Invoices | Create `N × 50 UST1` (unique SKUs) → launcher; Enable Feature 50 → **launcher** (official path, [#606](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/606) **T606-1**); Save **50 UST1 flat** → token. Mint unpaid. |
| **C593-5** MintControl | Create-only. |
| **C593-6** Manager | Connected wallet vs LCD `manager`. Non-manager read-only. |
| **C593-7** Unverified admin | `ContractInfo.admin ≠ CMM` banner. |
| **C593-8** Template | Manage requires `code_id ==` env pin (columbus-5 **11630** after #635; **11626** was the #628 store). After #626 adopt, the same address is that pin and Manage shows tax SKUs (**P11**). 6036/10184/8266/8654 stay hidden. |
| **M626** Migrate Token | `/token/migrate` is free (no invoice). Source gate is `VITE_COMMUNITY_MIGRATE_CODE_IDS` (default 6036,10184,8266,8654) — not factory whitelist. **Do not append 3** ([#627](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/627)). Create Token links **Migrate here** and stays code-id-free (**M628-7**). Retail lead is one sentence (address stays); no env-var / 50 UST1 / cw2 essay on the card. Launcher stays CMM-only. Post-merge leftovers: [#628](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/628) / `make verify-issue-628`. |
| **M634** Migrate inventory | Confirm + success show CL8Y / other-DEX / GDEX venues. Register only factory-verified CL8Y after adopt when pins match. ALPHA/Open Terraport rows always overlay. `#633` owns new Create Pair register + Manage highest-LP. LocalTerra mintable analogue: [`localterra-634-migrate-inventory.sh`](../scripts/qa/localterra-634-migrate-inventory.sh). [#634](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/634) / `make verify-issue-634`. |
| **C593-9** Extra-debit Max | Swap/Trade sell max reduced by sell tax on **pair-direct and router-hop** sells (**T592-13**). Manager-directory wallets skip extra-debit (**#609** / **E609-7**); unknown exempt stays fail-closed. |
| **C593-10** Payee from env | Never URL. |
| **C593-11** No Swap dump | Not auto-listed (#562). After create, `/create` is copy-address + link only — no query prefill (**C542-11** / **P402-5**). |
| **C593-12** Free create | 0 SKU → launcher `CreateToken` execute (not 0-amount UST1 Send). Live on columbus-5 launcher `terra126pr5…` (code **11622**). |
| **C593-13** Instantiate caps | `max_buy + max_sell + max_transfer ≤ 2500`. Do not default each max to 2500. |
| **C593-14** Listed-pair tax copy | Create/Manage: buy/sell applies on every listed-pair swap. Swap/Trade: `Sell tax extra` / `Buy tax applies`. [#607](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/607) / [`AGENTS_COMMUNITY_TAX_ROUTER.md`](../skills/AGENTS_COMMUNITY_TAX_ROUTER.md). |
| **R633** Autoregister + manager skip | Official Create Pair registers tax-pin assets. Manage (manager + tax template) shows one highest-LP catch-up button when a factory pair is unregistered. `config.manager` skips buy/sell/transfer tax. LocalTerra factory B2 + manager Honest: [`localterra-633-autoregister.sh`](../scripts/qa/localterra-633-autoregister.sh). [#633](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/633) / [`AGENTS_COMMUNITY_TAX_AUTOREGISTER.md`](../skills/AGENTS_COMMUNITY_TAX_AUTOREGISTER.md). |
| **C604-1** Identity | Name/symbol `^[A-Za-z0-9]+$`, name 3–50, symbol 3–12 (submitted uppercase; name case preserved). Decimals integer **6–18**. Shared parsers in `communityTaxIdentity.ts` used by the page **and** hook builders. |
| **C604-2** Wallet helpers | Treasury/manager autofill the **connected** wallet when empty; never clobber a typed address; never `?manager=` / `?treasury=`. Helper copy is exactly `connected wallet` / `not connected wallet` (bech32-normalized). |
| **C604-3** Columbus-5 gap | Listed token **11611** does **not** gain instantiate identity checks until launcher `token_code_id` rotates after #589 GO + `AddWhitelistedCodeId`. Keep 11611 listed (**F6**). Frontend ships as a client gate first. |
| **C605-1** Percent taxes | Retail fields are **percent, 2 dp** (`2.50` → 250 bps). Never show “bps” on Create/Manage inputs. Combined cap still 25.00%. |
| **C605-2** SKU init | Selecting a SKU shows its init fields; unchecking drops those fields from the hook. Free create (0 SKU) still cannot include paid payloads (**C593-12**). |
| **C605-3** AutoLP create | Auto liquidity at create instantiates+binds the sister when launcher `autolp_code_id` is set. Unset → create blocked for that SKU (no 50 UST1 no-op). `SkimToLp` stays permissionless and is never called from `Transfer`/`Send` (**T592-10**). Pair must be this token’s factory-listed CL8Y pool; skim has a spread floor (**M610**, [#610](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/610)). |
| **C605-4** VariableRates gate | Without `variable_rates`, instantiate `max_*` must equal the current rate (no CLI headroom). Settings `buy_bps` / `sell_bps` require the SKU (`SkuNotUnlocked`). Not a no-op (audit M-1). Caps stay immutable after create. |

### Max amount / gas reserve {#max-amount-gas-reserve}

Retail **Max** (and pool **50%**) actions share [`AmountBalanceActions`](../frontend-dapp/src/components/common/AmountBalanceActions.tsx) and compute spendable amounts via [`maxSpendableAmount.ts`](../frontend-dapp/src/utils/maxSpendableAmount.ts) ([GitLab **#213**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/213)).

| Invariant | Meaning |
|-----------|---------|
| Single compute helper | **`computeMaxSpendableHumanAmount`** — all surfaces call this (Swap, Pool, trade limit/market, `/limits`); no inline `fromRawAmount(balance)` Max handlers. |
| BigInt reserve | Native **`uluna`** Max subtracts fee reserve in **raw micro-units** before **`fromRawAmount`**; never float subtraction on LUNC. |
| Fee envelope source | Reserves derive only from **`transactions.ts`** / **`terraGas.ts`** (`estimateNativeSwapUlunaFeesTotal`, `estimateProvideLiquidityNativeWrapUlunaFeesTotal`, existing sequence helpers). No magic uluna constants in UI. |
| CW20 Max unchanged | When pay asset is **not** native **`uluna`**, Max = full CW20 balance **unless** it is the community tax template (**C593-9** extra-debit sell). Native gas **submit** gates ([#132](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/132), [#147](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/147)) still block low-LUNC submits. |
| Native pay / wrap | Swap native pay, pool **Use native (auto-wrap)**, and native-input router paths subtract the action-specific reserve so one-click Max leaves **`Fee.amount`** payable from bank balance. |
| **`MaxAmountContext`** | Explicit per-surface context (`swap_native`, `swap_cw20`, `limit_place`, `market_swap`, `provide_liquidity_native_side`, `provide_liquidity_cw20`, `book_leg`) selects the correct fee envelope. |
| Limit **max mode** | Bid/Ask switch re-apply uses the same helper via [`useLimitEscrowMaxReapply`](../frontend-dapp/src/hooks/useLimitEscrowMaxReapply.ts). |
| Disabled Max | While balance is loading/error, or spendable raw is **0** after reserve, Max is disabled (no invalid drafts). |
| Book leg Max | Hybrid book override Max caps to **min(balance, pay amount)** — no gas reserve on the leg field. |
| Decimal drafts | Max output must pass **`isDecimalAmountDraft`** ([#169](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/169)). |

**Regression tests:** [`maxSpendableAmount.test.ts`](../frontend-dapp/src/utils/__tests__/maxSpendableAmount.test.ts), [`transactions.test.ts`](../frontend-dapp/src/services/terraclassic/__tests__/transactions.test.ts) (native swap / native wrap provide estimates).

**Third-party / agent context:** [`skills/AGENTS_TERRACLASSIC_GAS.md`](../skills/AGENTS_TERRACLASSIC_GAS.md) § Max amount / gas reserve.

### Local dev: Vite origin vs indexer CORS {#local-dev-indexer-cors}

The dApp reads **`VITE_INDEXER_URL`** (see [`frontend-dapp/.env.example`](../frontend-dapp/.env.example)) for browser `fetch` to the indexer API. **CORS is enforced on the `Origin` header**, which comes from the URL you open in the browser (`localhost` vs `127.0.0.1` are different origins). **`CORS_ORIGINS` on the indexer must list every origin you use for Vite** (typically both `http://localhost:5173` and `http://127.0.0.1:5173`, plus preview ports and the dedicated Playwright port **3173** — [`indexer/.env.example`](../indexer/.env.example), [`scripts/deploy-dex-local.sh`](../scripts/deploy-dex-local.sh), [`scripts/lib/indexer-cors-playwright.sh`](../scripts/lib/indexer-cors-playwright.sh), [GitLab **#625**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/625)). If they diverge, responses can show **200** in the Network panel while the browser still blocks the body (failed CORS) and `/pool` shows **Market data service unavailable**.

#### Local CORS proxy for remote soft-launch hosts {#local-dev-remote-cors-proxy}

When `.env.local` points at **remote** HTTPS backends (e.g. soft-launch `VITE_INDEXER_URL=https://indexer.dex.cl8y.com` and `VITE_TERRA_LCD_URL=https://terra-classic-lcd.publicnode.com`), production CORS will **not** allow `http://127.0.0.1:5173`. During `vite` serve, [`vite.config.ts`](../frontend-dapp/vite.config.ts) + [`src/dev/viteDevProxy.ts`](../frontend-dapp/src/dev/viteDevProxy.ts) automatically:

1. Proxy **`/__dev/indexer`** → the remote indexer host (`changeOrigin: true`)
2. Proxy **`/__dev/lcd`** → the remote LCD host
3. Rewrite browser-facing `import.meta.env.VITE_INDEXER_URL` / `VITE_TERRA_LCD_URL` to those same-origin paths

Keep the real HTTPS URLs in `.env.local` (so agents know the upstream). Set **`VITE_DEV_PROXY=0`** to disable. LocalTerra loopback URLs (`127.0.0.1` / `localhost`) are left alone. Implementation: [`vite.config.ts`](../frontend-dapp/vite.config.ts) + [`src/dev/viteDevProxy.ts`](../frontend-dapp/src/dev/viteDevProxy.ts).

After a successful **Place limit**, the UI polls **`GET .../limit-placements`** to auto-fill the cancel **Order ID**. Poll failures are logged as **`[limit-place] indexer poll failed:`** ([`warnIndexerPlacementPollFailed`](../frontend-dapp/src/utils/warnIndexerPlacementPollFailed.ts); [GitLab **#131**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/131)). On **`/trade`**, the ticket also surfaces **View order** / **Place another** next steps after a successful submit ([GitLab **#161**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/161); [§ Trade page — limit place success affordances](#trade-page-limit-place-success-affordances)). Operational detail: [`docs/indexer-invariants.md` § Local dev CORS](./indexer-invariants.md#local-dev-cors-localhost-vs-127001).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_LOCAL_REMOTE_CORS_PROXY.md`](../skills/AGENTS_FRONTEND_LOCAL_REMOTE_CORS_PROXY.md) · [`skills/AGENTS_LOCALNET_TRADING_SWARM.md`](../skills/AGENTS_LOCALNET_TRADING_SWARM.md) (local stack); indexer matrix: [`docs/environment-matrix.md`](./environment-matrix.md).

## Contract Message Format

The frontend uses TerraSwap-compatible message names:

- **Factory:** `create_pair { asset_infos }`, queries `config`, `pair`, `pairs`
- **Pair:** swap via CW20 Send with `{ swap: { belief_price, max_spread, to } }`, provide liquidity via `provide_liquidity { assets }`, withdraw via CW20 Send with `{ withdraw_liquidity: {} }`
- **Queries:** `pool` (reserves + total LP share), `simulation { offer_asset }`, `reverse_simulation { ask_asset }`
- **Types:** `AssetInfo` (`token` or `native_token`), `Asset` (`info` + `amount`), `PairInfo` (`asset_infos`, `contract_addr`, `liquidity_token`)

## Pages

| Route           | Description                                       |
|-----------------|---------------------------------------------------|
| `/`             | Swap interface — select tokens, enter amount, swap|
| `/pool`         | View pools, provide/withdraw liquidity            |
| `/create`       | Create a new token pair via the Factory           |

### Create pair — listed CW20 picker + custom paste {#create-pair-token-picker}

[`CreatePairPage`](../frontend-dapp/src/pages/CreatePairPage.tsx) offers a **listed-CW20 picker** plus a progressive-disclosure **Custom contract** paste field ([GitLab **#542**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/542)). The catalog is convenience only — factory `CreatePair` (CW20 + code-ID whitelist) remains the security boundary ([#376](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/376)).

| Invariant | Meaning |
|-----------|---------|
| **C542-1** | Token A / Token B can be chosen from bundled listed CW20s (cLUNC, cUSTC, CL8Y, USTR, UST1, vFDUSD when addresses resolve) without pasting. |
| **C542-2** | Native LUNC / USTC / `uluna` / `uusd` never appear as selectable create-pair tokens. |
| **C542-3** | Custom `terra1…` paste remains; #382 format/checksum errors still disable submit. |
| **C542-4** | Same token on both legs (picker, paste, or mixed case) disables submit with **Token addresses must be different**. |
| **C542-5** | `VITE_LUNC_C_*` / `VITE_USTC_C_*` / `VITE_UST1_*` / `VITE_VFDUSD_*` / `VITE_CL8Y_*` overlays win over columbus-5 `tokenlist.json` addresses (LocalTerra). |
| **C542-6** | Code-ID whitelist warning still runs for listed and pasted addresses; listed ≠ skip check. |
| **C542-7** | Submit calls existing `createPair` with two CW20 `AssetInfo`s. No native encoding. No factory/indexer API change. |
| **C542-8** | Swap / Mint / Trade pickers stay on their own universes (factory graph / faucet). Do **not** feed this catalog into Swap. |
| **C542-9** | Logos via `resolveTrustedTokenLogoUrl`; symbols/names text-only; search query capped at 128. |
| **C542-10** | UST1 AMM ≠ oracle notice stays ([#508](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/508) **U1**). |
| **C542-11** | Catalog is the bundled repo [`tokenlist/tokenlist.json`](../tokenlist/tokenlist.json) (plus env overlays). Soft-launch `VITE_TOKEN_*` gems append **only** when `retailExposeTestTokens()` ([#562](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/562) **P562-5**). **No** runtime HTTP for the list. Do **not** use tokenlist `decimals` for amounts. No `/create?a=&b=` query prefill. |

Helper: [`createPairTokenCatalog.ts`](../frontend-dapp/src/utils/createPairTokenCatalog.ts) (`getCreatePairCw20Options` / `buildCreatePairCw20Options`). UI: [`CreatePairTokenField`](../frontend-dapp/src/components/create/CreatePairTokenField.tsx) reuses [`TokenSearchSelect`](../frontend-dapp/src/components/trade/TokenSearchSelect.tsx) **control** with catalog ids — not `getAllTokens(pairs)`.

Sort empty browse economic-first then symbol ([#534](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/534) **P534-7**); gems last.

Regression: `make verify-issue-542`. Playbook: [`skills/AGENTS_FRONTEND_CREATE_PAIR_PICKER.md`](../skills/AGENTS_FRONTEND_CREATE_PAIR_PICKER.md).

### Create pair — token address validation {#create-pair-address-validation}

[`CreatePairPage`](../frontend-dapp/src/pages/CreatePairPage.tsx) validates both token contract fields with **`isValidTerraBech32Address`** / **`getTerraAddressInputError`** from [`terraAddressValidation.ts`](../frontend-dapp/src/utils/terraAddressValidation.ts) before enabling **Create Pair** ([GitLab **#382**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/382)). Custom paste and listed picks both go through this gate ([#542](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/542) **C542-3**).

| Invariant | Meaning |
|-----------|---------|
| Format gate | Wrong prefix, charset, or length → **Invalid Terra address format** (same regex as **`isValidTerraAddress`**). |
| Checksum gate | Structurally valid `terra1…` with bad bech32 checksum → **Invalid address: checksum does not match. Please check and re-enter the token address.** Submit stays disabled. |
| Both legs required | Empty fields do not count as valid; submit stays disabled until both addresses pass the gate and differ (case-insensitive). |
| Trade deep links unchanged | **`/trade/:pairAddr`** still uses format-only **`isValidTerraAddress`** — no checksum on URL segments ([§ Trade page unknown pair link](#trade-page-unknown-pair-link)). |
| Tx fallback | If a checksum error still reaches the chain, **`tryHumanizeTerraTxMessage`** maps `addr_validate` / `decoding bech32 failed` to the same retail copy ([§ User-facing errors](#user-facing-errors-humanization)). |

Regression: [`terraAddressValidation.test.ts`](../frontend-dapp/src/utils/__tests__/terraAddressValidation.test.ts), [`CreatePairPage.test.tsx`](../frontend-dapp/src/pages/CreatePairPage.test.tsx), [`createPairTokenCatalog.test.ts`](../frontend-dapp/src/utils/__tests__/createPairTokenCatalog.test.ts).

**UST1 secondary AMM notice (GitLab [#508](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/508)):** Create Pair lists copy from [`ust1SecondaryMarket.ts`](../frontend-dapp/src/utils/ust1SecondaryMarket.ts) clarifying that a new AMM market is **not** oracle mint/redeem (`/ust1`). Soft-launch `/mint` remains the faucet only. Runbook: [`runbooks/ust1-secondary-amm-pair.md`](./runbooks/ust1-secondary-amm-pair.md) (**U1**).

| `/charts`       | Pairs overview and per-pair charts (indexer)      |
| `/portfolio`    | **My Portfolio** — connected wallet summary, open quote positions, wallet-wide open limits, LP overview, recent swaps ([GitLab **#212**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/212), phase 2 [**#217**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/217)); alias `/my-portfolio` → `/portfolio` |
| `/trader`       | Trader profile lookup (indexer); optional `/:address` |
| `/trade`        | Trade UI — order book, **price chart**, tape, **limit + market** tickets ([GitLab #152](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/152)) |
| `/trade/:pairAddr` | Same as `/trade` with pair pre-selected       |
| `/limits`       | Limit order placements, lifecycle, and **wallet history** (fills, cancels, swaps on pair + CSV) — [GitLab **#163**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/163) |
| `/tiers`        | View fee discount tiers, register/deregister for a tier |
| `/mint`         | Soft-launch faucet Mint page (shown in More nav only when `VITE_FAUCET_ADDRESS` is set — [GitLab **#473**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/473); runbook [`soft-launch-faucet.md`](./runbooks/soft-launch-faucet.md)) |
| `/ust1`         | Always-on **UST1 ↔ vFDUSD** oracle mint/redeem via ust1-window CW20 Send (More nav when window env set — [GitLab **#506**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/506); **not** the faucet; runbook [`ust1-window-ui.md`](./runbooks/ust1-window-ui.md); skill [`AGENTS_UST1_WINDOW_UI.md`](../skills/AGENTS_UST1_WINDOW_UI.md)) |
| `/protocol`     | DEX **USD** global stats (volume + pool TVL) + **treasury fees** (24h/7d/30d USD + source/token mix) + **DEX hub prices** (cUSTC / LUNC / UST1 / USTR from `GET /api/v1/hub-prices`) + one CEX oracle card (USTC / LUNC / vFDUSD) + factory/router audit + hooks ([GitLab **#550**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/550) / [**#556**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/556) / [**#569**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/569) / [**#570**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/570) / [**#571**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/571) / [**#586**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/586)). Header realized P&amp;L on `/portfolio` + `/trader` uses the same hub snapshot ([#560](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/560)). |

### Protocol — global USD stats + unified oracle {#protocol-page}

[`ProtocolPage.tsx`](../frontend-dapp/src/pages/ProtocolPage.tsx) is the DEX census + reference-oracle surface ([GitLab **#550**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/550) / [**#556**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/556) / [**#569**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/569) / [**#570**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/570) / [**#586**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/586) / [**#652**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/652)). `/charts` does **not** duplicate this census ([#666](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/666)). Page order: title → **Global stats** (`protocol-global-stats`, tiles + optional UTC-day volume chart) → **Protocol fees** (`protocol-fee-stats`) → **DEX hub prices** (`protocol-dex-hub-prices`) → **one** CEX oracle card (`protocol-oracle`) → on-chain contracts (audit, [#378](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/378)) → hook events. Global stats / fees / oracle **stat chips** use `StatBox variant="flat"` ([#653](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/653)). Oracle sources/history and hook events may keep **one** table well.

| Invariant | Meaning |
|-----------|---------|
| **P550-1 Order** | Stats → **fees** (`protocol-fee-stats`, #586) → **DEX hub card** (#556 / #570) → CEX oracle. Do not merge factory/router into stats or fees. Do not clone `AddressRow` onto Swap confirmation. Do not add USTR as a fourth CEX tab. |
| **PFee-1–PFee-12** + **PFee-13** Fees | Trailing 24h/7d/30d treasury USD with **inline** flow Δ% in the same tile ([#652](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/652)). Source + token tables from `GET /api/v1/protocol/fees`. Retail labels include **UST1 mint** / **UST1 redeem** when `ust1_window_configured` ([#614](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/614) / **I614**). Idle `$0`; unpriced `—`; never `Infinity`. Not `traders.total_fees_paid`. Wrap/Unwrap rows appear when ingest sees mapper `notify_deposit` / `unwrap` `fee` ([GitLab #613](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/613)). Testids: `protocol-fee-stats`, `protocol-stat-fees-24h` / `7d` / `30d` (Δ% child `-chg`), `protocol-fees-by-source`, `protocol-fees-by-token`. Flat `StatBox` — no nested `card-glass`. |
| **P550-2 Tickers** | Tabs only `ustc` \| `lunc` \| `vfdusd`. `?ticker=` allowlisted; unknown / `javascript:` / `../` → `ustc`. |
| **P550-3 One card** | Snapshot, sources, and history share one `shell-panel`. Query keys include ticker. |
| **P550-4 USD headlines** | Volume uses `total_volume_*_usd`. Do **not** present mixed-unit `total_volume_24h` as volume. |
| **P569-1 Pool TVL** | **Total liquidity** is humanized AMM `pair_reserves` USD (`total_liquidity_usd`), not volume, not CG `liquidity_in_usd`, not book escrow. One cell: USD + inline 24h/30d snapshot Δ% (`protocol-stat-liquidity`; child testids `protocol-stat-liquidity-24h` / `-30d`). |
| **P569-2 Δ%** | 24h/30d liquidity is vs indexer snapshots. Missing / `null` / non-finite → em-dash (`formatProtocolPct`), never `0%` / `Infinity`. |
| **P550-9 vFDUSD** | Path `vfdusd` returns CEX **FDUSD/USD** (`first-digital-usd` / `FDUSDUSDT`; JSON `quote_asset=FDUSD`, `display_name=FDUSD/USD`). Protocol tab heading is **vFDUSD**; CEX StatBox is **FDUSD reference price**. Not Terra CW20 vFDUSD, not `$1`, not the `/ust1` window rate. Venus **1 vFDUSD Price** is [#571](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/571) ([#580](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/580)). |
| **V571-1–V571-10** | vFDUSD tab: CEX vs Venus split; indexer `eth_call` only; pin `0xC4eF4229FEc74Ccfe17B2bdeF7715fAC740BA0ba`. |
| **P550-11 Reference** | Feeds are advisory. Per-pair TWAP stays on Charts. |
| **H11–H16 Hub wrap** | Hub columns **cUSTC, LUNC, UST1, USTR**. LUNC/USD is #515 LUNC CEX (wrap 1:1). cUSTC and LUNC show configured wrap CW20 `AddressRow`s (`protocol-dex-hub-custc-token` / `protocol-dex-hub-lunc-token`); UST1/USTR keep **source pair** rows. Native `uluna` is not given a Finder URL. Explorer hrefs only via `getExplorerAddressUrl`. Four cells stay in the DOM on hub 502 (USD `—`). |

`unique_traders_24h` is on `GET /overview` for rollup/DoS safety ([#550](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/550) **AC7**) but is **not** a Protocol headline (dust-swap gaming; [#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/489)).

Volume boxes use **Last 24h / 7d / 30d vol** plus static trailing-window `title` / `aria-label` — not calendar buckets and not a lecture in the lead paragraph ([#576](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/576), [`trailingWindowCopy.ts`](../frontend-dapp/src/utils/trailingWindowCopy.ts)). Each volume tile also shows prior-window flow Δ% (`volume_change_*_pct`). A **UTC calendar-day** bar chart (`protocol-volume-daily-chart`, default 7d / toggle 30d) is additive and must not be read as the trailing tiles ([#652](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/652) **P652-1–P652-7**). Hide the chart when `GET /api/v1/protocol/volume/daily` is missing (old indexer). Do not call `GET /defillama/daily` from this page.

Regression: `make verify-issue-652` · `make verify-issue-550` · `make verify-issue-569` · `make verify-issue-586` · `make verify-issue-614` · `make verify-issue-570` · `make verify-issue-576` · `make verify-issue-571`. Playbook: [`skills/AGENTS_FRONTEND_PROTOCOL_STATS.md`](../skills/AGENTS_FRONTEND_PROTOCOL_STATS.md), [`skills/AGENTS_INDEXER_UST1_WINDOW_FEES.md`](../skills/AGENTS_INDEXER_UST1_WINDOW_FEES.md), [`skills/AGENTS_FRONTEND_PROTOCOL_HUB.md`](../skills/AGENTS_FRONTEND_PROTOCOL_HUB.md), [`skills/AGENTS_FRONTEND_TRAILING_WINDOW.md`](../skills/AGENTS_FRONTEND_TRAILING_WINDOW.md). Oracle API: [`runbooks/indexer-external-oracle.md`](./runbooks/indexer-external-oracle.md). Venus redeem: [`skills/AGENTS_INDEXER_VENUS_VFDUSD.md`](../skills/AGENTS_INDEXER_VENUS_VFDUSD.md). Overview TVL / fee / volume-Δ% rollup: [`runbooks/overview-global-stats-brin.md`](./runbooks/overview-global-stats-brin.md).

### My Portfolio (wallet-centric indexer exposure) {#my-portfolio}

Route **`/portfolio`** is the wallet-home surface for indexed trading exposure ([GitLab **#212**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/212); phase 2 [**#217**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/217)). It uses the connected address from **`useWalletStore`** only (no `?addr=` override). Public lookup of any address remains on **`/trader/:address`**.

| Invariant | Meaning |
|-----------|---------|
| **Disconnected** | Connect CTA only; **no** indexer/LCD calls that require an address. |
| **Positions API** | `GET /api/v1/traders/{addr}/positions` — returns `[]` for flat/unknown traders; not on-chain balances. |
| **Open limits API** | `GET /api/v1/traders/{addr}/limit-placements` — wallet-wide resting limits (`owner`); same cancel omission and **`lifecycle_status`** / **`?status=`** as pair route ([`indexer-invariants.md`](./indexer-invariants.md)); **`limit` ≤ 200**. UI: [`PortfolioOpenLimitsSection`](../frontend-dapp/src/components/portfolio/PortfolioOpenLimitsSection.tsx). |
| **LP overview** | Indexer `GET /api/v1/pairs` (max **50** pairs) + LCD CW20 **`balance`** per valid `lp_token` (concurrency **5**) via [`usePortfolioLpBalances`](../frontend-dapp/src/hooks/usePortfolioLpBalances.ts); skips invalid bech32 / per-pair LCD errors — **not** merged into positions table. |
| **Profile API** | `GET /api/v1/traders/{addr}` — **404** when the wallet has no indexed trader row; portfolio still shows positions + activity when present. **Total Volume (USD)** uses `total_volume_usd` ([#553](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/553)); raw `total_volume` is not displayed. |
| **LP vs trader** | Open positions are **swap-tracked quote exposure**; LP section is **on-chain LP token balances** — separate sections and copy; pool txs on **`/pool`**. |
| **P&amp;L semantics** | **Realized** indexer P&amp;L only — **no** unrealized mark-to-market on portfolio ([#217](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/217) defers until API/product agree). Per-pair amounts are **human token units** with the token symbol; cross-pair header totals are **USD** from `GET /api/v1/hub-prices` (UST1/USTR/cUSTC; never `$1` / `2.5×`) or **—** when units differ / unpriced ([#551](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/551) **P551-1–P551-5**, [#560](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/560) **P560-1–P560-6**). Best/Worst is **—**, not `N/A`. |
| **Privacy** | Trader routes are **public**; portfolio does not imply on-chain secrecy. |
| **Read-only** | No signing on portfolio; limits deep-link to **`/trade/{pairAddr}`** and **`/limits`**. |
| **Outage UX** | `MarketDataServiceOutageBanner` + `RetryError` parity with [`TraderPage`](../frontend-dapp/src/pages/TraderPage.tsx) for indexer-backed sections. |
| **Nav** | `Portfolio` in `PRIMARY_NAV_ITEMS` ([`navItems.ts`](../frontend-dapp/src/components/common/navItems.ts)); wallet menu **My Portfolio** link. |
| **Shared UI** | [`TraderSummaryStats`](../frontend-dapp/src/components/trader/TraderSummaryStats.tsx), [`TraderPositionsTable`](../frontend-dapp/src/components/trader/TraderPositionsTable.tsx) shared with trader profile. |

**Tests:** [`PortfolioPage.test.tsx`](../frontend-dapp/src/pages/PortfolioPage.test.tsx), [`traderPositionDisplay.test.ts`](../frontend-dapp/src/utils/__tests__/traderPositionDisplay.test.ts), [`TraderPositionsTable.test.tsx`](../frontend-dapp/src/components/trader/TraderPositionsTable.test.tsx), [`TraderSummaryStats.test.tsx`](../frontend-dapp/src/components/trader/TraderSummaryStats.test.tsx), [`usePortfolioLpBalances.test.ts`](../frontend-dapp/src/hooks/__tests__/usePortfolioLpBalances.test.ts), [`client.test.ts`](../frontend-dapp/src/services/indexer/__tests__/client.test.ts) (`getTraderPositions`, `getTraderLimitPlacements`), [`e2e/portfolio.spec.ts`](../frontend-dapp/e2e/portfolio.spec.ts), indexer [`api_traders.rs`](../indexer/tests/api_traders.rs). Regression: `make verify-issue-551`, `make verify-issue-560`.

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_PORTFOLIO.md`](../skills/AGENTS_FRONTEND_PORTFOLIO.md), [`skills/AGENTS_FRONTEND_PORTFOLIO_PNL.md`](../skills/AGENTS_FRONTEND_PORTFOLIO_PNL.md), [`skills/AGENTS_FRONTEND_HUB_PNL.md`](../skills/AGENTS_FRONTEND_HUB_PNL.md).

### Wallet swap and limit history (indexer) {#wallet-swap-limit-history}

When a wallet is connected, **`/limits`** shows indexed **limit fills** (maker), **cancellations** (owner attribute when present on-chain), and **AMM swaps** for the **selected pair** via trader-scoped indexer routes. **`/trade`** shows the same **swap** slice for the active pair. Rows include **timestamps**, **tx hashes** (with explorer links in the table), **fees** where the indexer stores them (`commission_amount` / `effective_fee_bps` on swaps; `commission_amount` on limit fills), and **size amounts** ([GitLab **#479**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/479)):

| Section | Amount columns | Source fields |
|---------|----------------|---------------|
| Swaps (AMM) | **Amount in** / **Amount out** | `offer_amount` / `return_amount` |
| Limit fills (maker) | **Token0** / **Token1** (base / quote) | `token0_amount` / `token1_amount` |
| Limit cancellations | _(none)_ | API has no amount fields |

Amount cells use **`formatTapeAmount`** (human units + symbol; GitLab [#557](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/557)). Indexer JSON/CSV stay **plain integer** strings (`offer_amount` / `return_amount`, no scientific notation) with additive `offer_decimals` / `ask_decimals`. Mobile keeps horizontal scroll (`data-testid="wallet-history-table-scroll"`, [#352](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/352)).

**CSV export** uses `GET .../trades?format=csv`, `.../limit-fills?format=csv`, and `.../limit-cancellations?format=csv` on the **`/api/v1/traders/{addr}/...`** paths (same `pair=` filter as the table). Client `limit` is capped at **`TRADER_HISTORY_CSV_MAX_LIMIT` (200)** to match the indexer clamp — see [`docs/indexer-invariants.md`](./indexer-invariants.md). Export is **HTTP-only** (no wallet signature). Failures show an inline alert (`wallet-history-csv-error`); `fetchTraderHistoryCsv` retries once on network/timeout. Formula-injection escaping (#432) stays server-side.

| Invariant | Meaning |
|-----------|---------|
| **Pair scope** | History + CSV stay filtered to the selected pair; do not expand to global wallet history here. |
| **Amount parity** | Swaps reuse TradesTable **human** amount semantics (#557); fills expose token0/token1 humanized with pair-leg decimals; cancellations stay Time / Order / Tx. CSV remains raw. |
| **CSV cap** | Never request or advertise export above indexer max **200**. |
| **CSV errors visible** | Download failures must not be silent; button re-enables after error. |
| **No signing for CSV** | Keplr only supplies the address in the URL path. |

Component: [`WalletIndexerHistoryPanel.tsx`](../frontend-dapp/src/components/trade/WalletIndexerHistoryPanel.tsx). Tests: [`WalletIndexerHistoryPanel.test.tsx`](../frontend-dapp/src/components/trade/__tests__/WalletIndexerHistoryPanel.test.tsx), [`e2e/wallet-history-163.spec.ts`](../frontend-dapp/e2e/wallet-history-163.spec.ts). Third-party agents: [`skills/AGENTS_FRONTEND_ORDER_HISTORY.md`](../skills/AGENTS_FRONTEND_ORDER_HISTORY.md).

### Form inputs — programmatic labels {#form-inputs-programmatic-labels}

Text inputs must expose an **accessible name** to assistive tech: pair **`htmlFor` on `<label>`** with matching **`id` on `<input>`**, wrap the control in `<label>`, or use `aria-label` / `aria-labelledby` when layout requires it. Placeholder text or visually adjacent copy alone is **not** a reliable accessible name.

| Invariant | Meaning |
|-----------|---------|
| Pair labels for text fields | Prefer **`htmlFor` + `id`**; use React **`useId()`** per component instance so IDs stay unique when the same widget appears more than once. |
| Consistency | Follow the same pattern as [`LimitOrderExpiryField.tsx`](../frontend-dapp/src/components/trade/LimitOrderExpiryField.tsx) (`idPrefix` + explicit ids) and pair search on [`ChartsPage.tsx`](../frontend-dapp/src/pages/ChartsPage.tsx). |
| Checkbox / radio | An enclosing `<label>` that wraps the input remains valid; do not unwrap existing correct patterns to “fix” unrelated fields. |

Scope and verification checklist: [GitLab **#143**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/143). Umbrella: [DEX **#133**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/133).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_A11Y_FORM_LABELS.md`](../skills/AGENTS_FRONTEND_A11Y_FORM_LABELS.md).

### Accessibility CI {#accessibility-ci}

Automated **WCAG 2.1 A + AA** checks on retail-critical routes via **`@axe-core/playwright`** in the **`e2e-smoke`** project ([GitLab **#214**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/214)). Vitest asserts component-level ARIA contracts on chart, order book, and wallet.

| Invariant | Meaning |
|-----------|---------|
| **Routes scanned** | `/` (Swap), `/trade`, `/charts` (chart canvas excluded), `/limits`, `/pool`, `/portfolio`, header **Connect wallet** dialog, connected **wallet menu** (`include: header`) ([#366](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/366)). |
| **Severity gate** | `assertNoCriticalA11yViolations` fails on **critical** or **serious** axe impacts only. |
| **Canvas exclusion** | Only `[data-testid="price-chart-lightweight-canvas"] canvas` — never exclude interactive controls. |
| **TradingView attribution** | `layout.attributionLogo: false` on lightweight-charts; visible **Charting by TradingView** link on `PriceChart` (outside `aria-hidden` canvas) satisfies Apache NOTICE without `aria-hidden-focus` on `#tv-attr-logo`. |
| **Chart AT summary** | `PriceChart` uses `role="region"` + `aria-labelledby` / `aria-describedby`; `sr-only` `aria-live="polite"` announces interval + last price; interval toggles use `aria-label` (`{iv} candle interval`). |
| **Order book** | Per-side `<table>` with `<th scope="col">`; rows expose `aria-label` with side, order id, price, size. Scrollable book body uses `tabIndex={0}` + `role="region"` so axe `scrollable-region-focusable` passes ([#214](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/214)). |
| **Order ticket tabs** | Limit / market: `tablist` + `tab` + `tabpanel` with `aria-controls` / `aria-labelledby`; pair paused → `role="alert"`. |
| **Wallet menu** | `role="menu"` contains **menuitems only** (balance header outside); focus first menuitem on open, return to trigger on close; `CopyButton` live region inside menuitem. |
| **Smoke without indexer** | `PLAYWRIGHT_SKIP_CHAIN=1` still runs axe on route shell; chart `region` is required when indexer + LCD are up (CI strict E2E). |
| **Rule disables** | Forbidden without comment in spec + row in this table. |

**Run locally:**

```bash
cd frontend-dapp
PLAYWRIGHT_SKIP_CHAIN=1 npm run test:e2e:smoke -- e2e/a11y-critical-routes.spec.ts
```

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_A11Y_CI.md`](../skills/AGENTS_FRONTEND_A11Y_CI.md); chart: [`skills/AGENTS_FRONTEND_PRICE_CHART.md`](../skills/AGENTS_FRONTEND_PRICE_CHART.md); wallet: [`skills/AGENTS_FRONTEND_WALLET_CHIP.md`](../skills/AGENTS_FRONTEND_WALLET_CHIP.md).

### Responsive shell & header navigation {#responsive-header-navigation}

Layout lives in [`Layout.tsx`](../frontend-dapp/src/components/common/Layout.tsx) with shell styles in [`index.css`](../frontend-dapp/src/index.css). Breakpoints are **CSS-first** for showing the bottom tab bar vs the sticky header row; **header density** for mid-width tablets is driven by `matchMedia` so the **More** menu can absorb overflow without crowding ([GitLab **#136**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/136)).

| Invariant | Meaning |
|-----------|---------|
| Mobile bottom nav | Viewports **`max-width: 767px`**: `.app-desktop-nav` is hidden; primary links use `.app-mobile-nav-shell` + **More** sheet (`MORE_NAV_ITEMS` only — theme toggle stays in that sheet). |
| Theme toggle placement | **Desktop/tablet (`min-width: 768px`)**: dark/light control in **sticky header** (`.app-header-controls`, `.app-header-theme-group`) — reachable without scrolling long routes ([GitLab **#170**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/170)). **Mobile**: same control in the **More** sheet only (footer chrome hidden). |
| Sound mute placement | Same surfaces as theme: sticky header (`.app-header-pref-group` + [`SoundEffectsToggle`](../frontend-dapp/src/components/common/SoundEffectsToggle.tsx)) on desktop/tablet; mobile **More** sheet (`.app-mobile-pref-group`). See [UI sound effects mute](#ui-sound-effects-mute) ([GitLab **#487**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/487)). |
| Tablet compact header | Viewports **`768px`–`1199px`**: header shows **Swap** inline plus **More**; Pool, Limits, Trade, and Charts appear **inside** the header More menu ahead of Trader / Protocol / Fee Tiers / Create Pair (`getHeaderMoreMenuItems(false)` in [`navItems.ts`](../frontend-dapp/src/components/common/navItems.ts)). Includes the **1024px–1199px** band where the full primary row previously overlapped wallet/controls ([GitLab **#136**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/136), [#483](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/483)). |
| Full desktop header | Viewports **`min-width: 1200px`**: all `PRIMARY_NAV_ITEMS` inline; header More lists **`MORE_NAV_ITEMS` only** (same as pre–#136 wide layout). |
| Nav → controls gap | At full-desktop widths, last nav control (**More**) and `.app-header-theme-group` must keep **≥ ~8px** horizontal gap (wallet connected or not). Desktop/tablet **omit** header [`NetworkBadge`](../frontend-dapp/src/components/wallet/NetworkBadge.tsx) — [`EnvironmentRibbon`](../frontend-dapp/src/components/legal/EnvironmentRibbon.tsx) is the primary network signal; mobile keeps the badge beside the wallet chip ([GitLab **#483**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/483)). |
| Footer environment ribbon | `.app-env-ribbon` renders inside `footer.app-footer-shell` on **all** viewports (including mobile, above the bottom tab bar). Sticky header is header-only for vertical density. |
| Sticky header clearance | `.app-top-sticky` uses an opaque `var(--bg-0)` background so scrolled page copy cannot bleed through the header card; Trade H1 clears the sticky header by **≥ ~16px** at `scrollY=0`. |
| Mobile vs header “More” active state | The bottom-tab **More** button highlights only for **`MORE_NAV_ITEMS`** routes; the header **More** trigger uses the expanded tablet list when compact so Pool/Charts/etc. still show an active affordance. |
| Header brand copy | Sticky header brand is **logo + “CL8Y DEX” title only** — no secondary kicker line (removed [GitLab **#136**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/136) regression fix; Terra Classic context stays in footer **`CL8Y DEX · Terra Classic`**). Below **`1024px`**, `.app-brand-copy` stays hidden ([GitLab **#52**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/52)). |
| Client-side tab navigation | Header and mobile shell tabs use [`AppShellNavLink`](../frontend-dapp/src/components/common/AppShellNavLink.tsx): plain left-click calls `navigate()` so **URL and `<Outlet>` update without hard refresh** even when a wallet extension swallows the default anchor handler ([GitLab **#182**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/182)). [`Layout`](../frontend-dapp/src/components/common/Layout.tsx) sets **`key={location.pathname}`** on `<Outlet />` so lazy route pages cannot stick on the prior tab after navigation ([GitLab **#138**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/138)). Modified clicks (new tab, etc.) are untouched. Regression: [`AppShellNavLink.test.tsx`](../frontend-dapp/src/components/common/__tests__/AppShellNavLink.test.tsx), E2E “desktop primary tabs change URL”, “navigates to Pool page”, NFA-after-route-change. |

Constants: `HEADER_FULL_NAV_MIN_WIDTH_PX` (`1200`), `TABLET_COMPACT_HEADER_MAX_WIDTH_PX` (`1199`), and row label tuples `DESKTOP_HEADER_NAV_ROW_LABELS` / `TABLET_COMPACT_HEADER_NAV_ROW_LABELS` for Playwright overlap checks (`frontend-dapp/e2e/navigation.spec.ts`). Footer ribbon placement, sticky header clearance, and nav→theme gap assertions live in the same file (**#483** density + footer ribbon regression).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_RESPONSIVE_HEADER.md`](../skills/AGENTS_FRONTEND_RESPONSIVE_HEADER.md), [`skills/AGENTS_FRONTEND_SHELL_NAV.md`](../skills/AGENTS_FRONTEND_SHELL_NAV.md), [`skills/AGENTS_FRONTEND_RISK_DISCLAIMERS.md`](../skills/AGENTS_FRONTEND_RISK_DISCLAIMERS.md) (environment ribbon), [`skills/AGENTS_FRONTEND_SOUND_MUTE.md`](../skills/AGENTS_FRONTEND_SOUND_MUTE.md) (SFX mute).

### UI sound effects mute {#ui-sound-effects-mute}

UI audio is centralized in [`sounds.ts`](../frontend-dapp/src/lib/sounds.ts) (`playButtonPress` / `playHover` / `playSuccess` / `playError`). Playback stays **opt-in per call site** — there is no global click interceptor. Users can opt out via a shell toggle ([GitLab **#487**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/487)).

| Invariant | Meaning |
|-----------|---------|
| Default ON | Missing or invalid `localStorage` value → sounds enabled (matches historical QA §10.3 expectations). |
| Single gate | Mute is enforced only inside `play()` via [`readSoundsEnabled()`](../frontend-dapp/src/utils/soundPreferences.ts). Do not fork mute checks at import call sites. |
| All SFX kinds | One preference disables press, hover, success, and error together. |
| Persistence | Key `cl8y-dex-sounds-enabled` stores `'1'` / `'0'`. Session cache applies immediately and survives write failures (private mode / quota) for the tab. Cross-tab sync until reload is out of scope for MVP. |
| Mute / unmute UX | Turning **off** does not play a press sound on that click. Turning **on** plays one confirmation press. Subsequent actions reflect the new preference without a hard refresh. |
| Shell placement | Compact **icon** controls (moon / sun / speaker) sit next to theme (header ≥768px; mobile More ≤767px). Names come from `aria-label` / `title`; `aria-pressed` on sound means **sounds enabled**. |
| Assets | Only local WAVs under `frontend-dapp/public/sounds/` — no remote audio URLs. |
| Visual errors remain primary | Toasts / UI error copy stay authoritative; SFX are supplemental. |

**Regression:** [`soundPreferences.test.ts`](../frontend-dapp/src/utils/soundPreferences.test.ts), [`sounds.test.ts`](../frontend-dapp/src/lib/sounds.test.ts), [`SoundEffectsToggle.test.tsx`](../frontend-dapp/src/components/common/__tests__/SoundEffectsToggle.test.tsx). Manual: [`QA_TEMPLATE.md`](../QA_TEMPLATE.md) §10.3.

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_SOUND_MUTE.md`](../skills/AGENTS_FRONTEND_SOUND_MUTE.md); theme adjacency: [`skills/AGENTS_FRONTEND_THEME_TOGGLE.md`](../skills/AGENTS_FRONTEND_THEME_TOGGLE.md).

### Keyboard focus visibility (WCAG 2.4.7) {#keyboard-focus-visible-wcag-247}

Interactive controls must expose a **visible keyboard focus indicator** when focused via Tab / Shift+Tab (`:focus-visible`). Pure `:focus` styling on components that also receive click focus can produce unwanted persistent rings for pointer users; industry practice is **`:focus-visible`** for custom rings ([WCAG 2.4.7 Focus Visible](https://www.w3.org/WAI/WCAG22/Understanding/focus-visible.html)). Implemented for header/nav, wallet/connect surfaces, tabs, primary buttons, form controls, and the Swap **You Pay** amount field ([GitLab **#144**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/144)).

| Invariant | Meaning |
|-----------|---------|
| Token alignment | Custom rings use **`var(--focus-ring)`** via `color-mix(in srgb, var(--focus-ring) 28%, transparent)` (same family as `.input-glass:focus-visible` in [`index.css`](../frontend-dapp/src/index.css)). |
| Inputs / triggers | `.input-glass`, `.select-glass`, `.token-select-trigger` use **`:focus-visible`** so mouse focus does not mimic keyboard emphasis where the UA supports it. |
| Focus ring footprint | `.token-select-trigger` reserves a **transparent** `0 0 0 2px` ring in the default `box-shadow` stack; `:focus-visible` only changes ring **color**, not size ([GitLab **#181**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/181)). |
| Shell & CTAs | `.btn-primary` / `.btn-muted` / `.btn-cta`, `.app-nav-link` (and related triggers), `.wallet-trigger` (+ `.wallet-trigger-connected`), `.network-badge`, `.tab-glass` / `.tab-glass-active`, `.side-control`, `.wallet-option-card`, and dropdown `.app-menu-link` / `.wallet-menu-item` define explicit `:focus-visible` rings; **active** nav rows compose the active `box-shadow` **plus** the outer ring. Buy/Sell side fills ([#563](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/563)) must keep the `.side-control:focus-visible` ring. |
| Menu backdrops | **`.app-menu-dismiss`** (shell More menu in [`Layout.tsx`](../frontend-dapp/src/components/common/Layout.tsx), connected wallet menu in [`WalletButton.tsx`](../frontend-dapp/src/components/wallet/WalletButton.tsx)) is a full-viewport **`type="button"`** with an **`aria-label`**; **`:focus-visible`** uses an inset ring ([GitLab **#187**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/187)). |
| Swap amount | The prominent pay amount `<input>` uses class **`swap-io-amount-input`** — do **not** strip focus with `focus:outline-none` without replacing it; ring styles sit beside `.swap-io-stack` in `index.css`. |

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_A11Y_FOCUS.md`](../skills/AGENTS_FRONTEND_A11Y_FOCUS.md).

### Portal listboxes (`MenuSelect` / `TokenSelect` / `TokenSearchSelect`) — layout stability {#portal-listbox-layout-stability}

Custom pair/token pickers use a **portaled** `<ul role="listbox">` positioned with **`position: fixed`** (not in-document flow) so opening a menu does not push chart, order book, or ticket columns ([GitLab **#181**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/181), W13-C2). Swap token search adds mobile trigger invariants ([GitLab **#498**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/498)).

| Invariant | Meaning |
|-----------|---------|
| **Fixed menu** | [`computePortalListboxStyle`](../frontend-dapp/src/components/ui/portalListboxPosition.ts) + [`usePortalListbox`](../frontend-dapp/src/components/ui/PortalListbox.tsx) set coords on the **first open frame** (sync read of anchor `getBoundingClientRect()` during render; window **and** `visualViewport` scroll/resize bump a reducer). `.token-select-dropdown` also declares `position: fixed` in CSS as a safety net. |
| **Visual viewport + in-app chrome (#632)** | Viewport source is [`readPortalListboxViewport`](../frontend-dapp/src/lib/portalListboxViewport.ts) (**V632-1–V632-4**): `visualViewport` occlusion + DEX tab bar + optional Keplr/Station in-app reserve + 44px finger gap. Do not size against `innerHeight` alone. Coarse/narrow browse does not focus a text field (**V632-5**). Agent playbook: [`skills/AGENTS_FRONTEND_PORTAL_LISTBOX_VIEWPORT.md`](../skills/AGENTS_FRONTEND_PORTAL_LISTBOX_VIEWPORT.md). |
| **Stable trigger** | Wrapper `.token-select-root` uses `contain: layout` and `min-height: 48px` matching `.token-select-trigger`. Trade `#trade-pair-select` sits in `shrink-0` `max-w-xl` shell so flex rows do not compress the control when the menu opens. |
| **Swap leading logo (#498)** | [`TokenSearchSelect`](../frontend-dapp/src/components/trade/TokenSearchSelect.tsx) keeps the selected token’s leading logo mounted while open and applies **`.token-select-trigger--with-leading-logo`** (reserved `padding-left`) so open/close does not shift the symbol. Do **not** toggle logo/padding on `open`. Until the user edits, the input keeps showing the selected label (`queryDraft === null`) to avoid an empty flash; focus selects the label so the next keystroke replaces it. |
| **Scrollbar gutter** | `html { scrollbar-gutter: stable; }` avoids horizontal reflow when overlay scrollbars would otherwise appear/disappear. Portaled `.token-select-dropdown` also uses `scrollbar-gutter: stable` so option rows do not jump when the list becomes scrollable. |
| **CLS budget** | Lighthouse CLS on `/trade` after opening the pair menu should stay **&lt; 0.1**; surrounding content must not jump (eyeball + Lighthouse). On phone-width `/swap`, opening the pay token combobox must keep trigger and amount-field Y/X stable (see E2E below). |
| **Keyboard (APG listbox)** | Arrow Up/Down (wrap), Home/End, typeahead by option label/symbol prefix (case-insensitive, 500ms buffer), Enter/Space to select, Escape/Tab to close with focus restored to trigger. Listbox exposes `aria-activedescendant`; options use `role="option"` + `aria-selected`. Shared hook: [`usePortalListboxKeyboard`](../frontend-dapp/src/components/ui/usePortalListboxKeyboard.ts) ([GitLab **#244**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/244), gap **M5**). **`openRef`** preserves multi-char typeahead between the first printable key and React `open` commit. |

**Regression tests:** [`frontend-dapp/e2e/trade-pair-select-cls.spec.ts`](../frontend-dapp/e2e/trade-pair-select-cls.spec.ts) (pair menu bounding-box deltas + #632 phone clearance); [`frontend-dapp/e2e/swap-token-select-cls.spec.ts`](../frontend-dapp/e2e/swap-token-select-cls.spec.ts) (mobile Swap token combobox — **#498**); [`frontend-dapp/e2e/swap-token-select-viewport.spec.ts`](../frontend-dapp/e2e/swap-token-select-viewport.spec.ts) (visualViewport clearance — **#632**); unit tests in [`portalListboxPosition.test.ts`](../frontend-dapp/src/components/ui/__tests__/portalListboxPosition.test.ts), [`portalListboxViewport.test.ts`](../frontend-dapp/src/lib/__tests__/portalListboxViewport.test.ts), [`portalListboxKeyboard.test.ts`](../frontend-dapp/src/components/ui/__tests__/portalListboxKeyboard.test.ts), [`MenuSelect.keyboard.test.tsx`](../frontend-dapp/src/components/ui/__tests__/MenuSelect.keyboard.test.tsx), [`TokenSelect.keyboard.test.tsx`](../frontend-dapp/src/components/ui/__tests__/TokenSelect.keyboard.test.tsx), [`TokenSearchSelect.test.tsx`](../frontend-dapp/src/components/trade/__tests__/TokenSearchSelect.test.tsx) (leading logo + label on open; coarse/narrow browse).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_PORTAL_LISTBOX_CLS.md`](../skills/AGENTS_FRONTEND_PORTAL_LISTBOX_CLS.md); visual viewport / in-app: [`skills/AGENTS_FRONTEND_PORTAL_LISTBOX_VIEWPORT.md`](../skills/AGENTS_FRONTEND_PORTAL_LISTBOX_VIEWPORT.md) (**V632-1–V632-8**, `make verify-issue-632`); token search: [`skills/AGENTS_FRONTEND_TOKEN_SEARCH.md`](../skills/AGENTS_FRONTEND_TOKEN_SEARCH.md); keyboard listbox APG: [`skills/AGENTS_FRONTEND_PORTAL_LISTBOX_KEYBOARD.md`](../skills/AGENTS_FRONTEND_PORTAL_LISTBOX_KEYBOARD.md); trade layout: [`skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](../skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md).

### Pair search combobox (`PairSearchSelect`) — Trade / Limits {#pair-search-combobox}

Trade and Limit Orders use [`PairSearchSelect`](../frontend-dapp/src/components/trade/PairSearchSelect.tsx) instead of a full factory pair dropdown ([GitLab **#314**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/314), pair switching regression [**#301**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/301)).

| Invariant | Meaning |
|-----------|---------|
| **Indexer search** | Debounced (≥300ms) `GET /api/v1/pairs?q=&sort=relevance&limit=20`; empty query still fetches `sort=volume_24h` for volume badges, then **re-ranks** the factory universe (GitLab **#534**). |
| **Min query length** | ≥2 chars unless the query looks like a `terra1…` address ([`pairSearchQuery.ts`](../frontend-dapp/src/utils/pairSearchQuery.ts)). |
| **Factory gate** | Results are filtered to factory-registered pairs (`factoryPairs` prop) so only routable pairs appear. |
| **Degraded mode** | After the first indexer error in the session, combobox search uses `filterFactoryPairsByLocalSearch` on factory pairs (menu label, display symbols, contract/denom ids, localStorage-cached CW20 symbol/name, registry entries, two-token `XXX YYY` / `XXX/YYY` queries) without further indexer calls. Empty degraded browse uses the same catalog rank. Typed symbol search (e.g. `EMBER`) works without the indexer when token metadata was cached from a prior `token_info` read. Shows a dim **Offline search** hint in the listbox. |
| **Accessibility** | Input uses `role="combobox"` + portaled `listbox`; Arrow keys / Enter / Escape match portal listbox keyboard patterns. Coarse/narrow: button combobox + in-menu search ([#632](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/632) **V632-5**). |
| **Liquidity badge** | Options show indexed 24h quote volume when `volume_quote_24h > 0`, formatted with **quote-token decimals** (`formatQuoteVolume24h`) — never `formatNum(raw)` (**P534-4**). |

Charts pair `MenuSelect` uses the same catalog rank on the loaded page ([pair catalog rank](#pair-catalog-rank)).

**Regression tests:** [`pairSearchQuery.test.ts`](../frontend-dapp/src/utils/__tests__/pairSearchQuery.test.ts); [`pairCatalogRank.test.ts`](../frontend-dapp/src/utils/__tests__/pairCatalogRank.test.ts); [`PairSearchSelect.issue301.test.tsx`](../frontend-dapp/src/components/trade/__tests__/PairSearchSelect.issue301.test.tsx); [`PairSearchSelect.issue534.test.tsx`](../frontend-dapp/src/components/trade/__tests__/PairSearchSelect.issue534.test.tsx); Trade page pair-switch test in [`TradePage.test.tsx`](../frontend-dapp/src/pages/TradePage.test.tsx); indexer [`list_pairs_relevance_ordering`](../indexer/tests/api_pairs.rs); Trade/Limits page tests mock `getPairs`.

**Product parity:** Trade/Limits search **pairs**; Swap searches **tokens** — see [Token search combobox](#token-search-combobox).

### Pair catalog rank — economic first, gems last {#pair-catalog-rank}

Empty pair browse (Trade / Limits `PairSearchSelect`, Charts pair menu) must not follow factory-creation order or raw `volume_24h` when that buries economic markets under faucet gems ([GitLab **#534**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/534)). Shared helper: [`pairCatalogRank.ts`](../frontend-dapp/src/utils/pairCatalogRank.ts). Agent playbook: [`skills/AGENTS_FRONTEND_PAIR_CATALOG_RANK.md`](../skills/AGENTS_FRONTEND_PAIR_CATALOG_RANK.md).

| ID | Rule |
|----|------|
| **P534-1** | **Economic first.** A pair is economic iff **both** legs are real tokens (cLUNC / LUNC / `uluna`, cUSTC / USTC / `uusd`, UST1, USTR, CL8Y / TCL8Y, vFDUSD, …). Either leg in the gem set (EMBER, CORAL, JADE, ONYX, RUBY, TOPAZ, QUARTZ, PEARL + LocalTerra extras OPAL, COBALT, SLATE, AMBER, IRON) → **test pair**, listed after economic rows under a **Test pairs** divider (not collapsed). |
| **P534-2** | **Hub grouping.** Among economic pairs, group by the highest-priority hub present: UST1, then cLUNC, cUSTC, USTR, CL8Y, vFDUSD. All UST1 markets (UST1/cUSTC, UST1/USTR, cLUNC/UST1) stay adjacent. |
| **P534-3** | **Volume within a group** uses **human** quote volume `raw / 10^quoteDecimals` (mixed 6/18-dec safe). Do not sort 18-dec USTR raw against 6-dec cUSTC raw. |
| **P534-4** | **Vol badge / pool 24h vol** format `volume_quote_24h` with quote decimals (`formatQuoteVolume24h`). Indexer JSON stays a **raw** integer (same class as #522 prices vs this volume field). `formatNum(raw)` on USTR prints `19,297,048T`. |
| **P534-5** | Bare `/trade` auto-pick uses `firstCatalogPairAddress` (first catalog-ranked factory pair), not `pairs[0]` factory-creation order. Deep links unchanged. |
| **P534-6** | **Typed search** keeps indexer `relevance` / local haystack order. Catalog rank applies to **empty browse** only. |
| **P534-7** | Swap token combobox empty browse uses the same gem vs economic split (`compareTokenCatalog`). |
| **P534-8** | Do **not** fold UST1 into the gem set (**U6**). Gems stay faucet/test; economic hubs stay registry + wrap aliases. |

`GET /api/v1/pairs?sort=volume_24h` remains raw-quote for API clients. The dApp overlays catalog rank on pickers **and** on the `/pool` table **default** ([#547](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/547) **P547-3**). User-chosen `/pool` column sorts (volume, fee, created, name) use indexer `sort`/`order` and are **not** re-ranked by catalog. On production those pages still **omit gems** ([#562](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/562) **P562-3**).

On **production** (`VITE_NETWORK=mainnet`, `VITE_SHOW_TEST_TOKENS` unset) gems are **omitted** from retail discovery rather than ranked last — see [Production hide of test tokens](#production-hide-test-tokens) ([#562](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/562)). LocalTerra still uses P534-1–P534-8 as written.

### Production hide of test tokens {#production-hide-test-tokens}

Soft-launch gemstone CW20s stay on columbus-5. The production dApp (`https://dex.cl8y.com`) must not list them in Swap / Trade / Pool / Charts / Create browse or typeahead, and must not advertise gem hops on economic quotes ([GitLab **#562**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/562)). Shared helpers: [`pairCatalogRank.ts`](../frontend-dapp/src/utils/pairCatalogRank.ts) (`retailExposeTestTokens`, `COLUMBUS5_GEM_ADDRESSES`, `filterRetailDiscovery*`). Agent playbook: [`skills/AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md`](../skills/AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md).

| ID | Rule |
|----|------|
| **P562-1** | **Expose flag.** `retailExposeTestTokens()` is true iff `VITE_NETWORK !== 'mainnet'` **or** build-arg `VITE_SHOW_TEST_TOKENS=true`. Not a runtime query (`?showGems=1` forbidden). Default production image leaves the override unset. |
| **P562-2** | **Identity.** Hide by hardcoded columbus-5 gem **addresses** (all eight, including QUARTZ/PEARL) **and** `GEM_SYMBOLS`. Unsetting Coolify `VITE_TOKEN_*` must not re-list gems. A gem contract spoofing `symbol=UST1` stays hidden; a listed hub/tokenlist token is never a gem even if someone names it RUBY (**X1**). UST1 / cLUNC / cUSTC / USTR / CL8Y / vFDUSD are never gems (**U6** / **P534-8**). |
| **P562-3** | **Discovery.** Production empty browse and typed search omit gems from Swap `TokenSearchSelect`, Trade/Limits `PairSearchSelect`, Charts pair menu, and `/pool` (catalog **and** user column sorts). **Test pairs** divider is absent when nothing remains to group. LocalTerra still lists gems last under **Test pairs** (**P534-1**). |
| **P562-4** | **Defaults.** Swap pay/receive and bare `/trade` auto-pick use economic tokens (`defaultRetailSwapTokenPair` / `firstCatalogPairAddress` after the hide filter), never `tokens[0]` gem. A leftover gem selection is reset to the economic default (**X10**). |
| **P562-5** | **Create Pair.** Listed catalog does not append `SOFT_LAUNCH_MINTABLE_TOKENS` when `!retailExposeTestTokens()`. Custom paste of a gem CW20 still uses checksum + code-id check — no client-only “blocked token” toast. |
| **P562-6** | **Quotes.** When both legs are economic, `findRoute` drops test pairs from the BFS graph and `quoteCw20ViaRouteSolve` returns `null` if any hop token is a gem (fail closed; displayed hops = executed hops). Gem↔gem remains allowed as an exit hatch. |
| **P562-7** | **Balances / exit.** `/portfolio`, `/trader`, history, and LP rows still render gem symbols and amounts. Deep link `/trade/<gem-pair-addr>` may resolve; optional static copy: **Legacy noneconomic market.** Do not put hidden gems back in global browse. Factory token universe remains the Swap gate (#481) — filter gems *inside* that set. |
| **P562-8** | **Mint / faucet.** Production Coolify **unsets** `VITE_FAUCET_ADDRESS` so Mint nav stays hidden (**F11**). Operator should **Pause** the faucet (and optionally `RemoveMinter`) per **F9** — UI hide is not the only control. No KYC. Do not fold UST1 mint into `/mint`. No always-on “test tokens removed” banner. |

`GET /api/v1/pairs` and `route/solve` JSON may still include gem rows for integrators. The dApp overlays the hide.

**Regression:** `make verify-issue-562`. LocalTerra rank/picker tests stay on `VITE_NETWORK=local` (`make verify-issue-534` / `#542` / `#547` / `#481`). LocalTerra P1: [`e2e/retail-test-tokens-562.spec.ts`](../frontend-dapp/e2e/retail-test-tokens-562.spec.ts) (Swap pay still lists EMBER). Stacked post-merge: `make verify-issue-573` ([#573](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/573) **M573-4**).

### Token search combobox (`TokenSearchSelect`) — Swap {#token-search-combobox}

Swap **YOU PAY** / **YOU RECEIVE** use [`TokenSearchSelect`](../frontend-dapp/src/components/trade/TokenSearchSelect.tsx) — a visible search combobox aligned with pair search UX ([GitLab **#481**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/481)). Mint keeps button-trigger [`TokenSelect`](../frontend-dapp/src/components/ui/TokenSelect.tsx) (small faucet list).

| Invariant | Meaning |
|-----------|---------|
| **Factory token universe** | Options come only from the `tokens` prop (`getAllTokens(pairs)` + native-wrap enrichment). Do **not** introduce an external/arbitrary token list or derive Swap options from `getPairs(q)`. |
| **Client-only filter** | Search is entirely client-side via [`tokenSearchQuery.ts`](../frontend-dapp/src/utils/tokenSearchQuery.ts) (works with indexer down). Haystack = id/denom, display symbol, localStorage-cached CW20 symbol/name, registry. No `GET /api/v1/tokens?q=` yet (optional follow-up if factory counts outgrow comfortable client filtering). |
| **Debounce / min chars / cap** | Debounce **300ms**; filter starts at ≥2 chars (or `terra1…` address ≥20); typed hits capped at **20**. Empty / too-short query browses the **allowed** list sorted **economic-first then display symbol** ([GitLab **#534**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/534), **P534-7**). Production omits gems from that list ([#562](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/562) **P562-3**). |
| **excludeToken** | Other leg is omitted from options; search tricks cannot select it. `onChange` only emits ids present in the gated options list. |
| **Query DoS / XSS** | Input `maxLength` / truncate at 128 chars; symbols/names render as **text only** (no `dangerouslySetInnerHTML`); logo URLs still pass [`resolveTrustedTokenLogoUrl`](../frontend-dapp/src/utils/tokenLogoAllowlist.ts). |
| **Accessibility** | Input `role="combobox"` + `aria-autocomplete="list"` + portaled `listbox`; Arrow / Enter / Escape / Tab. Typed query + Enter commits **first hit** (same #350 rule as pair search). Coarse/narrow browse uses a button combobox so the IME does not open until the user taps the in-menu search ([#632](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/632) **V632-5**). |
| **Mobile layout stability (#498)** | Leading logo stays mounted while open; trigger uses **`.token-select-trigger--with-leading-logo`**. Selected label remains until the user edits (`queryDraft`); focus selects the label. See [Portal listboxes — layout stability](#portal-listbox-layout-stability). |
| **Visual viewport / in-app (#632)** | Shared portal geometry uses `visualViewport` + tab/in-app/finger insets (**V632-1–V632-4**). Do not document “use Chrome only” (**WC-M7**). Viewport meta stays default `resizes-visual`. |
| **Quote path unchanged** | Selection still updates the same token id string; routing/simulation/execution are untouched. |
| **N630-1** Native product tickers | Visible option / trigger / `TokenDisplay` / `TokenIdentity` text for bank `uluna` / `uusd` is **LUNC** / **USTC**, never the denom ([GitLab **#630**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/630)). Shared helper [`registryProductSymbol`](../frontend-dapp/src/utils/tokenRegistry.ts) runs **before** indexer `symbol` in [`useTokenDisplayInfo`](../frontend-dapp/src/hooks/useTokenDisplayInfo.ts) and [`getTokenDisplaySymbol`](../frontend-dapp/src/utils/tokenDisplay.ts). |
| **N630-2** Wrap rows stay distinct | **cLUNC** / **cUSTC** stay four-way distinct from LUNC / USTC even when indexer/on-chain say `LUNC-C` / `USTC-C` ([#507](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/507)). |
| **N630-3** Ids unchanged | `onChange`, `data-testid` (`token-option-uluna`), balances, route/solve, and execute stay `uluna` / `uusd` (or wrap CW20 address). |
| **N630-4** Registry wins | A stale or spoofed indexer `symbol` (`uluna`, `UST1`, HTML) cannot relabel known natives or wrap CW20s. Unknown CW20s still use indexer / `token_info`. |
| **N630-5** Unknown natives fail closed | `ufoo` / `ibc/…` display as the raw denom (or indexer symbol if present). Do not invent tickers. |
| **N630-6** Search both forms | Haystack keeps denom **and** product ticker so `LUNC` / `uluna` / `USTC` / `uusd` all hit the native row. |
| **N630-7** Copy stays denom | [#541](#token-identity) copy payload is `uluna` / `uusd`; natives stay copy-only. No always-on “uluna means LUNC” essay ([#489](#retail-copy-cognitive-load)). |
| **N630-8** Indexer catalog | First native insert + denom-as-symbol repair write `LUNC` / `USTC` for `uluna` / `uusd` only. Other denoms stay denom/denom. Wrap CW20 rows untouched. |

**Regression tests:** [`tokenSearchQuery.test.ts`](../frontend-dapp/src/utils/__tests__/tokenSearchQuery.test.ts); [`TokenSearchSelect.test.tsx`](../frontend-dapp/src/components/trade/__tests__/TokenSearchSelect.test.tsx); [`TokenSearchSelect.issue630.test.tsx`](../frontend-dapp/src/components/trade/__tests__/TokenSearchSelect.issue630.test.tsx); [`useTokenDisplayInfo.test.tsx`](../frontend-dapp/src/hooks/__tests__/useTokenDisplayInfo.test.tsx); E2E helpers [`e2e/helpers/token-select.ts`](../frontend-dapp/e2e/helpers/token-select.ts) target `combobox` (in-menu `searchbox` when the trigger is a button); mobile CLS [`e2e/swap-token-select-cls.spec.ts`](../frontend-dapp/e2e/swap-token-select-cls.spec.ts) (**#498**); clearance [`e2e/swap-token-select-viewport.spec.ts`](../frontend-dapp/e2e/swap-token-select-viewport.spec.ts) (**#632**); LocalTerra gems-still-listed P1 [`e2e/retail-test-tokens-562.spec.ts`](../frontend-dapp/e2e/retail-test-tokens-562.spec.ts) (**#562** / **#573**). Issue-scoped: `make verify-issue-630` · `make verify-issue-632`.

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_TOKEN_SEARCH.md`](../skills/AGENTS_FRONTEND_TOKEN_SEARCH.md); native tickers: [`skills/AGENTS_FRONTEND_NATIVE_TICKERS.md`](../skills/AGENTS_FRONTEND_NATIVE_TICKERS.md); CLS: [`skills/AGENTS_FRONTEND_PORTAL_LISTBOX_CLS.md`](../skills/AGENTS_FRONTEND_PORTAL_LISTBOX_CLS.md); visual viewport: [`skills/AGENTS_FRONTEND_PORTAL_LISTBOX_VIEWPORT.md`](../skills/AGENTS_FRONTEND_PORTAL_LISTBOX_VIEWPORT.md); keyboard notes in [`skills/AGENTS_FRONTEND_PORTAL_LISTBOX_KEYBOARD.md`](../skills/AGENTS_FRONTEND_PORTAL_LISTBOX_KEYBOARD.md).

### Trader profile (indexer JSON + route error recovery) {#trader-profile-indexer}

Trader profile data comes from **`GET /api/v1/traders/:address`** (see [`client.ts`](../frontend-dapp/src/services/indexer/client.ts)). Before the UI renders stats, the response is normalized by [`traderProfilePayload.ts`](../frontend-dapp/src/services/indexer/traderProfilePayload.ts) so **arrays, `null` bodies, or partial objects** from a buggy proxy or indexer never reach the page as a “truthy” trader object (which previously could crash the route tree and strand users behind the route error UI — [GitLab #126](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/126)).

| Invariant | Meaning |
|-----------|---------|
| Parse or fail | `getTrader` runs `parseIndexerTraderPayload` on raw JSON; invalid shapes throw and become a **React Query error**, not a render-time exception. |
| **Total Volume (USD) (#553)** | [`TraderSummaryStats`](../frontend-dapp/src/components/trader/TraderSummaryStats.tsx) formats `total_volume_usd` with [`formatIndexedVolumeUsd`](../frontend-dapp/src/utils/chartsOverviewStats.ts). Never `formatNum(total_volume)`. Unpriced → `—`. |
| Route error boundary reset | `/trader` routes use `resetKeys` tied to `useParams().address` so switching between `/trader` and `/trader/:addr` (or between addresses) **clears a prior route error** without a full page reload ([`App.tsx`](../frontend-dapp/src/App.tsx) `TraderRouteShell`). |
| Deduped React in Vite | [`vite.config.ts`](../frontend-dapp/vite.config.ts) sets `resolve.dedupe` for `react` / `react-dom` to avoid rare **dual-React** dev bundles that surface as `useContext`/`Invalid hook call` when lazy chunks load. |

**Third-party / agent context:** [`skills/AGENTS_BUNDLE_DEV_WALLET.md`](../skills/AGENTS_BUNDLE_DEV_WALLET.md) (wallet + local QA); [`skills/AGENTS_LOCALNET_TRADING_SWARM.md`](../skills/AGENTS_LOCALNET_TRADING_SWARM.md) (indexer-backed flows).

### Trade page — price chart invariants {#trade-page-price-chart-invariants}

The **price chart** on `/trade` and `/charts` is rendered with **TradingView [lightweight-charts](https://github.com/tradingview/lightweight-charts)** (open-source canvas charting). It is **not** the hosted TradingView terminal/widget product—naming in code review and issues should keep that distinction clear.

| Invariant | Behavior |
|-----------|----------|
| Successful empty candles | `GET /api/v1/pairs/{addr}/candles` may return `[]` or rows that all fail OHLC validation; the UI **must not** show a blank panel. Use the empty state in `PriceChart` + `PriceChartEmptyState`. |
| **Indexer candle parsing (#226)** | [`indexerCandlesToChartPoints`](../frontend-dapp/src/components/charts/priceChartCandles.ts) **drops** rows (no coercion): missing/non-empty `open`/`close`, invalid `open_time`, or any OHLC field that is not **`Number.isFinite`** after `parseFloat` (covers `NaN`, `Infinity`, `1e309`, non-numeric strings, empty `high`/`low`). Output series never contains `NaN`. Client-side only — API contract unchanged. Regression: [`priceChartCandles.test.ts`](../frontend-dapp/src/components/charts/__tests__/priceChartCandles.test.ts). |
| **Stale `getCandles` on pair switch (#226)** | React Query key `['candles', pairAddress, interval]` plus query cancellation must ensure a **slower** response for pair A cannot overwrite pair B after the user switches. Canvas remounts with `key={pairAddress}`; no extra ref guard unless a failing test proves otherwise. Regression: `PriceChart.test.tsx` — *does not apply stale slower getCandles…*. |
| Single candle | After mapping/filtering, **one** valid OHLC point is enough for lightweight-charts to draw one candlestick; no empty-state for that case. |
| Loading vs empty | **First** load (no candle payload yet): full-panel **Loading chart…**. **Interval switch** on the **same pair** while refetching: keep `PriceChartLightweightCanvas` mounted (`placeholderData` keeps prior rows only when `queryKey[1]` is unchanged — not on pair switch; see [pair switch latency](#trade-page-pair-switch-latency)), overlay **`data-testid="price-chart-interval-loading"`** — do **not** unmount the plot (async `createChart` races froze the selector after ~5 switches — [GitLab **#148**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/148)). **Pair switch** remounts the canvas (`key={pairAddress}`) and shows **Loading chart…** until the new pair’s candles resolve ([#148](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/148) QA). Empty state applies only when the request **succeeded**, fetch settled, and there are zero valid points. |
| **Interval / timeframe** | `PriceChartLightweightCanvas` is created **once per mount**; interval changes call **`setData`** on the existing series. Stale async inits are dropped via **`chartInitIdRef`** ([GitLab **#148**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/148)). |
| **Time-scale zoom / scroll (#336)** | Background candle refetches (`refetchInterval: 30_000`) must **not** reset user zoom/scroll. **`timeScale().fitContent()`** runs only on initial mount and indicator toggle — not on routine data refresh. Live updates use **`series.update()`** only for the **last** bar and/or appended bars via [`priceChartLightweightSeriesSync.ts`](../frontend-dapp/src/components/charts/priceChartLightweightSeriesSync.ts). **`setData`** is used for first load, interval switch, truncated history, or a rewrite of any historical bar (pair invert **#524** / USD invert **#543** keeps times and rewrites OHLC — `update` on an older bar throws `Cannot update oldest data`). Regression: [`priceChartLightweightSeriesSync.test.ts`](../frontend-dapp/src/components/charts/__tests__/priceChartLightweightSeriesSync.test.ts), [`PriceChartLightweightCanvas.test.tsx`](../frontend-dapp/src/components/charts/__tests__/PriceChartLightweightCanvas.test.tsx). |
| Reference line | When the chart is empty, an optional **24h close** from `getPairStats` (`close_price`) may display; query is enabled only for that state so normal pairs are not blocked. |
| Accessibility | Canvas stays `aria-hidden` on `PriceChartLightweightCanvas`. Empty state uses `role="img"` + `aria-label`. When candles render, `PriceChart` exposes `role="region"` and an `aria-live` text summary (interval + last price) — see [§ Accessibility CI](#accessibility-ci) ([#214](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/214)). |
| **USD price scale (Y-axis)** | Spot **Price (USD)** is non-negative. The candlestick pane’s autoscale **must not** extend the right price scale below **zero** or below the **lowest visible candle `low`** (whichever is higher). Implemented via `autoscaleInfoProvider` + [`priceChartPriceScale.ts`](../frontend-dapp/src/components/charts/priceChartPriceScale.ts) ([GitLab **#151**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/151)). **Adaptive `priceFormat`** (`precision` / `minMove`) follows visible display-USD magnitude so `$0.000047` and `$0.012258` are readable — do **not** keep a fixed 2-dp format ([GitLab **#543**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/543)). Compact `formatNum` `T`/`K` must not be used as a price-axis formatter (**P522-5**). |
| **Chart viewport (layout)** | The plot region must **shrink inside** resizable `/trade` panels: `PriceChart` is **`h-full flex flex-col min-h-0`**; the candle mount uses **`flex-1 min-h-0`** with **`min-h-[min(52vh,280px)]`** (no fixed `560px` height). `TradePage` chart cards use **`flex flex-col min-h-0`** so the canvas is not clipped by **`overflow-hidden`** on first paint. `PriceChartLightweightCanvas` passes **`layout.panes.enableResize: false`** to `createChart` and reapplies width/height after layout via a **double `requestAnimationFrame`** and **`ResizeObserver`** ([GitLab **#151**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/151), contract tests [#227](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/227), lifecycle tests [#225](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/225)). |
| Volume histogram | **Pane 1** is a histogram of **human** quote volume per candle (`volume_quote` ÷ `10^asset_1.decimals`). When quote volume is zero, the UI uses **human** base volume (`volume_base` ÷ `10^asset_0.decimals`) — see [`priceChartCandles.ts`](../frontend-dapp/src/components/charts/priceChartCandles.ts) ([GitLab **#564**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/564)). Sub-label **“Volume (quote, else base)”** documents this in the chart header. Volume is **not** inverted as price (**C543-8**). |
| Indicators | Optional **MA 7**, **MA 25** (line overlays on pane 0) and **RSI 14** (separate pane, scale 0–100 with 70/30 guides) are toggled from the **Indicators** menu. The chart instance is **created once**; toggles call lightweight-charts **`removeSeries` / `addSeries`** and, for RSI, **`addPane` / `removePane(2)`** so overlays reliably appear and disappear without an async full re-init race ([`priceChartLightweightIndicatorSync.ts`](../frontend-dapp/src/components/charts/priceChartLightweightIndicatorSync.ts)). Pure math lives in [`priceChartIndicators.ts`](../frontend-dapp/src/components/charts/priceChartIndicators.ts) ([GitLab **#150**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/150)). |
| Fullscreen | **Expand** uses the Fullscreen API on the chart card (`PriceChart` root); **Exit** restores normal layout. |
| **Automated tests** | Default Vitest stubs the library ([`lightweightChartsJsdomMock.ts`](../frontend-dapp/src/test/lightweightChartsJsdomMock.ts)) for React/indexer wiring, including **createChart option contract** + canvas lifecycle regressions in [`PriceChartLightweightCanvas.test.tsx`](../frontend-dapp/src/components/charts/__tests__/PriceChartLightweightCanvas.test.tsx) ([#227](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/227), [#225](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/225)) and malformed-candle / stale-pair race coverage ([#226](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/226)). Real **lightweight-charts** init, `setData`, indicators, volume fallback, USD autoscale (real `getVisibleLogicalRange()` after zoom — [#229](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/229)), **500/1500** (+ CI **2000**) large-candle guards, and post-layout canvas sizing run in `npm run test:charts` ([GitLab **#211**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/211), [#225](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/225), [#229](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/229), [docs/testing.md § Real lightweight-charts in Vitest](./testing.md#real-lightweight-charts-in-vitest-gitlab-211)). Harness: [`chartRealLibraryHarness.ts`](../frontend-dapp/src/test/chartRealLibraryHarness.ts). **Playwright** ([**#228**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/228)): [`e2e/price-chart-smoke.spec.ts`](../frontend-dapp/e2e/price-chart-smoke.spec.ts) asserts browser canvas mount + fullscreen aria ([docs/testing.md § Price chart Playwright smoke](./testing.md#price-chart-playwright-smoke-gitlab-228)). |

**Last price headline (non-axis):** Beside the **Price (USD)** title, the chart shows a **Last** value when resolvable: **latest tape USD** from `resolveTapeLastPriceUsd` (prefers indexer `trades[].price_usd`, else human quote-per-base × quote catalog / #515 oracle — [GitLab **#522**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/522)). Parents must **not** pass raw `trades[0].price` as `tapeLastPriceUsd`. If tape USD is missing, the **last candle’s close** (candles store USD when `price_usd` was known) is used. Implementation: [`pairPriceUsd.ts`](../frontend-dapp/src/utils/pairPriceUsd.ts), `resolveTradeChartHeadlineUsd` in [`chartHeadlinePrice.ts`](../frontend-dapp/src/components/charts/chartHeadlinePrice.ts), `PriceChart` + `data-testid="trade-chart-headline-price"`. Tracked in [GitLab **#149**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/149), [**#522**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/522).

Implementation: [`frontend-dapp/src/components/charts/PriceChart.tsx`](../frontend-dapp/src/components/charts/PriceChart.tsx), [`chartHeadlinePrice.ts`](../frontend-dapp/src/components/charts/chartHeadlinePrice.ts) (headline — **#149**), [`priceChartCandles.ts`](../frontend-dapp/src/components/charts/priceChartCandles.ts), [`priceChartCandlesPlaceholder.ts`](../frontend-dapp/src/components/charts/priceChartCandlesPlaceholder.ts), [`priceChartIndicators.ts`](../frontend-dapp/src/components/charts/priceChartIndicators.ts), [`priceChartLightweightIndicatorSync.ts`](../frontend-dapp/src/components/charts/priceChartLightweightIndicatorSync.ts), [`priceChartLightweightSeriesSync.ts`](../frontend-dapp/src/components/charts/priceChartLightweightSeriesSync.ts) (incremental `update` / viewport preservation — **#336**), [`PriceChartLightweightCanvas.tsx`](../frontend-dapp/src/components/charts/PriceChartLightweightCanvas.tsx) (USD scale clamp via [`priceChartPriceScale.ts`](../frontend-dapp/src/components/charts/priceChartPriceScale.ts)). GitLab: [**#113**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/113), [**#148**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/148), [**#149**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/149), [**#150**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/150), [**#151**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/151), [**#211**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/211) (real-library Vitest), [**#225**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/225) (canvas lifecycle tests), [**#226**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/226) (candle parsing + stale pair race), [**#227**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/227) (jsdom stub contract tests), [**#229**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/229) (large-candle perf + real visible-range autoscale in Vitest), [**#336**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/336) (time-scale zoom preserved on 30s refetch).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_PRICE_CHART.md`](../skills/AGENTS_FRONTEND_PRICE_CHART.md); trade workspace layout: [`skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](../skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md); pair invert: [`skills/AGENTS_FRONTEND_TRADE_PAIR_INVERT.md`](../skills/AGENTS_FRONTEND_TRADE_PAIR_INVERT.md).

### Trade page — pair display invert (UST1 other-side Price USD) {#trade-pair-display-invert}

Factory pairs keep a fixed **base/quote** (`asset_0` / `asset_1`). Indexer `#466` / `#522` `price` / `price_usd` and on-chain `place_limit_order` stay **token1 per token0**. `/trade` and `/charts` may **display** the reciprocal so UST1-as-base markets show the floating token’s dollars by default ([GitLab **#524**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/524)).

| ID | Invariant |
|----|-----------|
| **T524-1** | Invert is **UI-only** for factory `price` / `price_usd` meaning and on-chain submit. Do not change indexer `swap_events.price` / `price_usd` or CG/CMC `last_price`. Additive candle `*_human` OHLC ([#543](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/543)) is allowed so the dApp can `invertUsd` per bar — do **not** replace stored factory USD with a reciprocal. |
| **T524-2** | On-chain messages stay factory-oriented. Convert display quote-per-display-base with `displayPriceToFactoryToken1PerToken0` before place, update-price, crossing/price gates, and market simulate/swap. Book **Edit** drafts stay factory; the ticket converts at the edge. |
| **T524-3** | Default invert **only** when factory `asset_0` is UST1 and `asset_1` is not. Detect UST1 by known contract allowlist and/or exact symbol `UST1` (trim/casefold). Never substring-match `cUSTC`. If UST1 is already quote, stay factory-oriented. |
| **T524-4** | One orientation state for chart headline, candles, pair pill, ticket **Buy {displayBase}**, Buy/Sell labels, and limit “When 1 {displayBase} is worth”. |
| **T524-5** | Bid/ask radiogroup is Buy/Sell of the **displayed** base. Pair invert is a **separate** pill + ticket icon — not `LimitOrderSideFlipButton`. |
| **T524-6** | Reciprocal safety: drop `≤ 0` / non-finite prices. **Human** book/limit invert uses `invertOhlc` (`1/x` + high/low swap). **USD candles** use `invertUsd` (`price_usd / price`) per bar — **never** `1 / price_usd` ([#543](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/543)). Never pass `NaN`/`Infinity` to lightweight-charts ([#226](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/226)). Y-axis stays non-negative ([#151](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/151)). Invert keeps candle times and rewrites historical OHLC — sync must **`setData`**, not `series.update()` from the oldest bar (`Cannot update oldest data`). |
| **T524-7** | Persist invert per `pairAddr` in **sessionStorage** (`cl8y-dex-trade-pair-invert:`). Pair switch re-keys. Cleared storage uses the product default (not another user’s last choice). |
| **T524-8** | `PairSearchSelect` click still opens search ([#181](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/181)). Invert is not on the combobox. Selected **label text** may follow display order. |
| **T524-9** | Route stays `/trade/:pairAddr` (factory pair). No second contract. |
| **T524-10** | `/limits` standalone does **not** silently invert. `/charts` uses the same default + pill. |
| **T524-11** | No `token0` / `token1` / `bid` / `ask` / **ORDER TICKET** in retail chrome. Invert `aria-label` names both symbols. Never describe invert as mint/redeem (**U1**, [#508](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/508)). |

**Side map (UST1 = `asset_0`, inverted):** display **Buy {other}** → factory **ask**; display **Sell {other}** → factory **bid**. Non-inverted: Buy → bid.

**USD invert (headline + candles, [#543](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/543)):** displayed Last **and** each candle OHLC = factory `price_usd` when not inverted; when inverted = `invertUsd(price_usd, human_quote_per_base)` = `price_usd / price` **per bar**. **Never** `1 / price_usd` on a USD series. **Never** plot human quote-per-base on a control labeled **Price (USD)** (**P522-5**). Bars with NULL / non-positive USD are dropped (no human fallback on the USD axis). SMA/RSI run on the **display USD** series after invert. Volume stays quote/base volume. Indexer candle USD is **as-of** ingest or idle mark ([#568](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/568)); the dApp must **not** overlay CoinGecko or `GET /oracle/history` onto pair charts.

| ID | Invariant |
|----|-----------|
| **C543-1** | Default UST1/USTR invert (pill `USTR/UST1`): Last and candle last-close are USD of 1 USTR (`0.012…` class), not `~1.0`. |
| **C543-2** | Same pair after switch-side: Last and candles are USD of 1 UST1 (`~$1`), not `1/0.012 ≈ 81`. |
| **C543-3** | UST1/cUSTC default: Last and candles are USD of 1 cUSTC (`~$0.005` class). Switch-side: both UST1 USD. |
| **C543-4** | cLUNC/UST1 factory: Last and candles are USD of 1 cLUNC (`~$0.000047`). Y-axis / last-value must not render `0.00` as the only digits. |
| **C543-5** | cLUNC/UST1 after switch-side: Last and candles are USD of 1 UST1 (`~$1`), **not** `~21260`. |
| **C543-6** | Non-UST1 pairs stay factory USD of `asset_0` (no default invert). |
| **C543-7** | `/trade` and `/charts` share the same series math and pill state. |
| **C543-8** | SMA/RSI follow display USD. Volume unchanged. Invert rewrite → `setData`. |
| **C543-9** | Empty / all-dropped USD bars use the existing empty state; no `NaN` / `Infinity` / negative axis. |

Implementation: [`tradePairDisplayOrientation.ts`](../frontend-dapp/src/utils/tradePairDisplayOrientation.ts) (`invertUsd` vs `invertOhlc`), [`priceChartCandles.ts`](../frontend-dapp/src/components/charts/priceChartCandles.ts) (`applyChartDisplayInvert`), [`priceChartPriceScale.ts`](../frontend-dapp/src/components/charts/priceChartPriceScale.ts) (`usdCandlePriceFormat`), [`PriceChart.tsx`](../frontend-dapp/src/components/charts/PriceChart.tsx), [`usePairDisplayOrientation.ts`](../frontend-dapp/src/hooks/usePairDisplayOrientation.ts), [`priceChartLightweightSeriesSync.ts`](../frontend-dapp/src/components/charts/priceChartLightweightSeriesSync.ts) (invert → `setData`), [`TradeOrderTicket.tsx`](../frontend-dapp/src/components/trade/TradeOrderTicket.tsx). Regression: `make verify-issue-524`, `make verify-issue-543`, `make verify-issue-568`.

**Idle / mark bars ([#568](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/568)):** catalog-quoted pairs may have candles with `trade_count = 0` and zero volume when USTC/LUNC/hub ticks with no swap. Plot the USD OHLC; histogram value stays 0. Do not treat marks as NAV or settlement.

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_TRADE_PAIR_INVERT.md`](../skills/AGENTS_FRONTEND_TRADE_PAIR_INVERT.md), [`skills/AGENTS_FRONTEND_USD_CANDLE_INVERT.md`](../skills/AGENTS_FRONTEND_USD_CANDLE_INVERT.md), [`skills/AGENTS_INDEXER_CANDLE_USD_MARK.md`](../skills/AGENTS_INDEXER_CANDLE_USD_MARK.md).

### Trade page — market context (tape, hybrid tag, limit-only book) {#trade-page-market-context}

Readability for traders used to centralized exchanges ([GitLab **#149**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/149)):

| Invariant | Meaning |
|-----------|---------|
| **Recent trades columns** | Headers **Pair** (pay → receive), **Amount in** / **Amount out** (human offer / ask token amounts — [#557](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/557)), plus **Price** (human quote-per-base, **not** USD — [#522](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/522); inverted pages use the reciprocal of that human price — [#524](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/524)), **Tx**. Column `<th>` elements carry `title` tooltips for the offer/ask semantics. Component: [`TradesTable.tsx`](../frontend-dapp/src/components/ui/TradesTable.tsx). See [§ Tape amounts (human scale)](#tape-amounts-human-scale). |
| **`hybrid` badge** | Uppercase styling on the badge text; native **`title`** explains hybrid **AMM + limit order** execution and points integrators to **`docs/integrators.md`** for fee attribution across events. |
| **Order ticket — type tabs** | **Limit** vs **Market** tabs on `/trade` (`TradeOrderTicket`). Market uses global slippage (`useDexStore`); quotes via indexer **`GET /route/solve`** (solver-optimized pool/book split, same as Swap — [#501](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/501), always-on [#596](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/596)); Advanced manual book leg uses `POST`; **no hybrid-off control**. Shows expected receive + min after slippage with retail disclosure ([GitLab **#152**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/152), [#414](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/414)). |
| **Order ticket — post-only limit preflight** | Before `place_limit_order`, the UI compares the typed price to the **book head** from `GET .../limit-book?limit=1` (best bid / best ask). Bids with price **≥ best ask** and asks with price **≤ best bid** are blocked with inline copy — pure client guard; the pair still inserts by book walk on-chain. Helpers: [`limitOrderNonCrossing.ts`](../frontend-dapp/src/utils/limitOrderNonCrossing.ts), hook [`useTradeBestBookPrices.ts`](../frontend-dapp/src/hooks/useTradeBestBookPrices.ts). The **`/limits` ladder panel** applies the same guard **per rung** via `describeLimitCrossingBlockerWithRef` (book head first; when the opposite side is empty, falls back to indexed tape / AMM pool reference like the retail ticket) and shows **`N of M rungs will cross the market…`** when any rung crosses ([#297](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/297), [#385](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/385)). Complements the **tape-reference** gate in [§ Trade page — limit order price field](#trade-page-limit-order-price) ([GitLab **#154**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/154)). |
| **Market quote — route preview (#302)** | When a market amount is quoted, [`TradeMarketOrderPanel`](../frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx) shows a single **Route** row (`data-testid="trade-market-route-summary"`) inside `trade-market-quote`, using the same `computeSwapRouteDisplay` helper and indexer-op precedence as Swap ([#158](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/158)). Row renders only when `marketRouteLine` is truthy; multihop paths appear when hybrid quoting returns indexer `router_operations`. Agent checklist: [`skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md`](../skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md). |

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](../skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md) (layout + this section for labeling), [`skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md`](../skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md) (swap + trade market route row), [`skills/AGENTS_FRONTEND_TAPE_AMOUNTS.md`](../skills/AGENTS_FRONTEND_TAPE_AMOUNTS.md) (human Amount in/out/Price — [#557](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/557)).

### Tape amounts (human scale) {#tape-amounts-human-scale}

Public tape (`TradesTable` on `/charts`, `/trade`, `/trader`, `/portfolio`) and pair wallet history must show **human** token amounts, not raw chain integers ([GitLab **#557**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/557)). Integrator JSON/CSV keep **plain integer digit strings** for `offer_amount` / `return_amount` (never `1e+19`); additive `offer_decimals` / `ask_decimals` (fills: `token0_decimals` / `token1_decimals`) come from indexed `assets.decimals` (asset id), never from wasm events or symbol matching.

| ID | Rule |
|----|------|
| **T557-1** | Amount in / out = `formatTokenAmount(raw, decimals)` then compact. Never `formatNum(raw)`. |
| **T557-2** | 18-dec amounts compact as `T` only when the **human** size is ≥ 1e12. |
| **T557-3** | Charts and Trade share `TradesTable`. |
| **T557-4** | Mixed-pair Trader/Portfolio rows use **per-trade** API decimals. |
| **T557-5** | Wallet pair history uses the same helpers; CSV download stays raw. |
| **T557-6** | Tape **Price** is human quote-per-base (`formatPairPrice`). Never USD. Never compact `T` from raw 18/6. |
| **T557-7** | Invert (#524) reciprocates **human** Price only. Amount in/out stay offer → ask. |
| **T557-8** | JSON/CSV raw amount columns stay **plain integer digit strings** (no scientific notation). Decimals are additive. |
| **T557-9** | Missing / out-of-range decimals (`<0` or `>38`) → `—`. Zero with known decimals → `0`. No `NaN` / `Infinity`. |
| **T557-10** | Amount cells include the pay/receive symbol after humanizing. |
| **T557-11** | Buy/sell color follows display-base (paying display-quote is a buy). Amounts stay offer → ask. |

Helpers: [`tradeTapeDisplay.ts`](../frontend-dapp/src/utils/tradeTapeDisplay.ts). Regression: `make verify-issue-557`.

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_TAPE_AMOUNTS.md`](../skills/AGENTS_FRONTEND_TAPE_AMOUNTS.md).

### Trade route — onboarding IA, CTA hierarchy, progressive disclosure {#trade-route-onboarding-ia}

Retail trade IA for Swap vs Trade vs Limits and calmer first paint on `/trade` ([GitLab **#417**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/417), parent [#411](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/411)):

| Invariant | Meaning |
|-----------|---------|
| **First-visit onboarding strip** | [`TradeOnboardingStrip`](../frontend-dapp/src/components/common/TradeOnboardingStrip.tsx) on `/`, `/trade`, and `/limits` until dismissed (`data-testid="trade-onboarding-strip"`). Copy links to **Swap** and explains when to use **Trade** vs **Limits**. Dismiss persists in `localStorage` ([`tradeOnboarding.ts`](../frontend-dapp/src/utils/tradeOnboarding.ts)). Must not block wallet connect or submit buttons. |
| **Money-action CTA sizing** | Primary trade submits use [`TRADE_MONEY_CTA_CLASS`](../frontend-dapp/src/utils/tradeMoneyCta.ts) (`py-3 text-sm font-semibold` minimum) on **Place limit**, **Market buy/sell**, and ladder place — aligned with Swap `btn-primary btn-cta` weight. `data-testid` hooks unchanged (`trade-limit-submit`, `trade-market-submit`, `ladder-place-submit`). |
| **Market slippage presets** | Shared [`SlippageProtectionPresets`](../frontend-dapp/src/components/common/SlippageProtectionPresets.tsx): label above a `role="group"` 3-up grid (`data-testid="trade-market-slippage-presets"`). Chips use `min-h-11` (~44px); `data-testid="trade-market-slippage-preset-{pct}"`. Do not wrap 0.5% onto the label row ([#528](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/528)). |
| **Progressive disclosure on `/trade`** | **Recent trades (tape)** and **wallet swap history** default **collapsed** on first visit. Sub-desktop: [`TradeWorkspaceDisclosure`](../frontend-dapp/src/components/trade/TradeWorkspaceDisclosure.tsx) (`trade-sub-lg-tape-disclosure`, `trade-wallet-history-disclosure`). Desktop: independent bottom-row tape panel with Expand/Collapse (`trade-desktop-tape-panel`, `trade-desktop-tape-toggle`) — **not** a splitter inside the chart column ([#561](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/561)). Expansion persists via [`tradeWorkspacePanels.ts`](../frontend-dapp/src/utils/tradeWorkspacePanels.ts). Pause/blacklist banners remain visible when applicable ([#395](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/395), [#388](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/388)). |

**Verify:** `make test-frontend` — [`TradePage.test.tsx`](../frontend-dapp/src/pages/TradePage.test.tsx), [`TradeOnboardingStrip.test.tsx`](../frontend-dapp/src/components/common/__tests__/TradeOnboardingStrip.test.tsx). Manual: clear `cl8y-dex-trade-onboarding-dismissed`, `cl8y-dex-trade-tape-expanded`, and `cl8y-dex-trade-wallet-history-expanded` in DevTools → reload `/trade` → confirm collapsed tape/history and onboarding strip; mobile bottom nav must remain usable.

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_TRADE_ONBOARDING_IA.md`](../skills/AGENTS_FRONTEND_TRADE_ONBOARDING_IA.md), [`skills/AGENTS_FRONTEND_SLIPPAGE_PRESET_ALIGN.md`](../skills/AGENTS_FRONTEND_SLIPPAGE_PRESET_ALIGN.md) (#528 chip group).

**Cursor agents:** When iterating on merge readiness and CI for this area, the **Babysit PR** Cursor skill complements the [Testing](./testing.md) doc (comment triage, conflict resolution, green pipelines).

<a id="trade-page-limit-ticket-sticky-cta"></a>

### Trade page — ticket heading + Buy/Sell side colors {#trade-page-ticket-heading}

`/trade` ticket heading shows the full **Buy {displayBase}** / **Sell {displayBase}** (or **Select a pair**). The compact ticket-header Connect Wallet / address chip is **removed**; shell header + footer money CTA remain ([GitLab **#563**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/563)). Buy/Sell side controls use semantic green/red fills. Styling only — escrow mapping unchanged.

| Invariant | Meaning |
|-----------|---------|
| **T563-1 Full heading** | `trade-ticket-heading` uses wrap (`.trade-ticket-heading`), not CSS `truncate`. Hub symbols (cLUNC, cUSTC, UST1, USTR) render in full — never `Buy c…`. Long symbols wrap; invert stays `shrink-0`. |
| **T563-2 No header chip** | No compact Connect Wallet / truncated bech32 in `trade-ticket-header` (disconnected or connected). Connected identity stays in the shell [`WalletButton`](../frontend-dapp/src/components/wallet/WalletButton.tsx). |
| **T563-3 Connect paths** | Disconnected: shell header **and** ticket footer **Connect Wallet** (`TRADE_MONEY_CTA_CLASS`). Do not add a fourth opener. Footer remains when the header is scrolled off. |
| **T563-4 Invert** | `trade-ticket-pair-invert` stays tappable. Heading + side labels track **displayed** base ([#524](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/524)). Colors follow bid=Buy / ask=Sell, not token0. |
| **T563-5 Side colors** | Buy = `side-buy-idle` / `side-buy-selected`; Sell = `side-sell-*`. Non-color cues: `aria-checked` + stronger fill / font-weight. Idle Sell ≠ `alert-error`. Contrast on dark and light. `.side-control:focus-visible` rings stay. |
| **T563-6 Shared + blue CTAs** | Same [`LimitOrderBidAskSideSelector`](../frontend-dapp/src/components/trade/LimitOrderBidAskSideSelector.tsx) on `/trade` and `/limits`. Limit/Market tabs stay `tab-glass*`. Place/Market CTAs stay `btn-primary`. Book Bid/Ask columns unchanged. |
| **T563-7 Behavior** | Radiogroup + Arrow/Home/End ([#153](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/153)). Bid still escrows token1; ask token0. Side change still clears / MAX-reapplies ([#155](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/155)). Click Buy still `onSideChange('bid')`. |
| **T563-8 Docs** | This section + [`design-system.md`](./design-system.md) side-fill exception. Verify: `make verify-issue-563`. |

**Code:** [`TradeOrderTicket.tsx`](../frontend-dapp/src/components/trade/TradeOrderTicket.tsx) (`data-testid="trade-ticket-header"`), [`index.css`](../frontend-dapp/src/index.css) (`.trade-ticket-heading`, `.side-control*`), theme `--side-buy*` / `--side-sell*`.

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_TRADE_TICKET_HEADING.md`](../skills/AGENTS_FRONTEND_TRADE_TICKET_HEADING.md). Footer dock: [`skills/AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md`](../skills/AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md) (`#527`).

### Trade page — ticket footer CTA {#trade-page-ticket-footer-cta}

`/trade` order-ticket money CTAs (**Place limit** / **Update price** / **Market buy|sell** / **Connect Wallet**) dock to the **bottom of the ticket card** as a flex `shrink-0` footer — not `position: sticky` inside the scrollport ([GitLab **#527**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/527)). Opacity and guards-in-flow from [#500](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/500) still apply (same opacity class as sticky header [#482](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/482)). Visibility without hunting the ticket body is [#348](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/348).

| Invariant | Meaning |
|-----------|---------|
| **T527-1 Dock, do not float** | At ticket scroll-top on Chromium, the money CTA **bottom** aligns with `trade-order-ticket-card` **bottom** (≤ 8px). It must not sit between Pay % chips and Expiry, and must not overlap Pay / Receive / Expiry / Advanced. |
| **T527-2 Shared footer slot** | Limit and Market render into the same `data-testid="trade-ticket-submit-footer"` (`.trade-ticket-submit-footer`). No second in-flow Market CTA mid-form. |
| **T527-3 #348 visibility** | Desktop ~1280×720: CTA fully visible in the ticket column without scrolling the ticket body. Tablet chart\|ticket row: CTA stays inside `trade-sub-lg-ticket-col`. |
| **T527-4 #500 opacity + guards** | Footer uses layered `var(--panel-bg-strong), var(--bg-1)` (+ backdrop blur). Do **not** use missing tokens (e.g. `--card`) or translucent mixes. `trade-limit-inline-guards` is **not** a descendant of the footer. |
| **T527-5 No sticky / fixed / portal** | Prefer `flex` + `shrink-0`. Do **not** use `position: sticky`, `position: fixed`, or a document-body portal footer. CTA remains a descendant of `trade-order-ticket-card` (sibling of `trade-order-ticket-scroll`). |
| **T527-6 Layout only** | No change to `place_limit_order`, invert convert-on-submit ([#524](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/524)), crossing / pause / blacklist / gas gates. |
| **T527-7 One ticket mount** | Do not remount a second `TradeOrderTicket` ([#178](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/178)). |
| **T527-8 z-index** | Footer stays under wallet modal, risk/NFA, clickwrap, toasts, and `#trade-pair-select` ([#181](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/181)). Do not raise `z-index` to “win” those surfaces. |
| **T527-9 Testids** | Keep `trade-limit-submit`, `trade-limit-update-price-submit`, `trade-limit-inline-guards`, `trade-order-ticket-scroll`, `trade-market-submit`. Footer wrapper is `trade-ticket-submit-footer` (replaces `trade-limit-submit-sticky`). |
| **T527-10 `/limits` out of scope** | Standalone place card stays in-flow. Do not add sticky/fixed chrome there unless a helper is shared. |
| **Footer payload** | CTA + broadcast pending link + tx result alerts only. |
| **#530 open limits** | `trade-ticket-placements-anchor` stays in the scroll body **above** the footer (not a child of Place limit). Compact **Cancel** must not sit under `trade-ticket-submit-footer`. |

Implementation: [`TradeOrderTicket.tsx`](../frontend-dapp/src/components/trade/TradeOrderTicket.tsx), [`TradeTicketSubmitFooter.tsx`](../frontend-dapp/src/components/trade/TradeTicketSubmitFooter.tsx), [`TradeMarketOrderPanel.tsx`](../frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx) (`dockSubmit`), styles in [`index.css`](../frontend-dapp/src/index.css). Verify: `make verify-issue-527` — `TradePage.test.tsx` (DOM order) and `e2e/trade-page-responsive.spec.ts` (bottom alignment + `elementFromPoint`).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md`](../skills/AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md) (`#527`); opacity / guards: [`skills/AGENTS_FRONTEND_TRADE_LIMIT_STICKY_CTA.md`](../skills/AGENTS_FRONTEND_TRADE_LIMIT_STICKY_CTA.md) (`#500`).

### Trade page — pair switch latency {#trade-page-pair-switch-latency}

Pair selector changes must feel responsive on `/trade` ([GitLab **#180**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/180), W13-C2). Prior behavior waited for **`getPair`** before mounting **`PriceChart`**, serializing candle loads (~8–10s on local indexers) with no loading affordance.

| Invariant | Meaning |
|-----------|---------|
| **Parallel fetch** | When `pairAddr` is valid, mount **`PriceChart`** immediately. **`getCandles`**, **`getTrades`**, and **`limitBookPage`** queries start in parallel with **`getPair`** — do not gate the chart on `indexerPairQuery.data`. |
| **Loading status** | While any workspace query for the active pair is in flight (`useIsFetching` + [`isTradePairWorkspaceQuery`](../frontend-dapp/src/utils/tradePairWorkspaceFetching.ts)), render [`TradePairSwitchStatus`](../frontend-dapp/src/components/trade/TradePairSwitchStatus.tsx) (`data-testid="trade-pair-switch-loading"`). The chart panel still shows **Loading chart…** from `PriceChart`. |
| **Prefetch** | [`prefetchTradePairWorkspace`](../frontend-dapp/src/utils/tradePairPrefetch.ts) warms pair metadata, candles (default `1h`), tape, and both book sides on route change, pair `onChange`, and `MenuSelect` **`onOptionIntent`** (hover/focus another row). |
| **No stale pair on switch** | Do **not** use `placeholderData: keepPreviousData` on pair-keyed indexer reads — wrong symbols/tape after a switch. **`PriceChart`** may keep prior candle rows **only when `pairAddress` is unchanged** (interval refetch); remount canvas with **`key={pairAddress}`** on pair change ([#148](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/148)). |
| **404 retry unchanged** | [`TradeChartSlot`](../frontend-dapp/src/pages/TradePage.tsx) still shows **`RetryError`** when `getPair` fails logically; see [§ Trade page — chart pair fetch retry](#trade-page-chart-retry) ([#177](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/177)). |

**Verify (manual):** switch pairs with DevTools Network + stopwatch — loading banner appears immediately; chart/book/tape settle within ~1–2s on `localterra`; Lighthouse INP on `#trade-pair-select` under 200ms when other work is warm.

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_TRADE_PAIR_SWITCH.md`](../skills/AGENTS_FRONTEND_TRADE_PAIR_SWITCH.md); layout: [`skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](../skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md).

### Trade page — chart pair fetch retry {#trade-page-chart-retry}

When **`getPair`** fails for the active `/trade/:pairAddr` route (e.g. indexer **404** for an unknown or malformed `terra1…` deep link), the chart panel shows **`RetryError`** with humanized copy — not the global outage banner ([GitLab **#177**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/177)).

| Invariant | Meaning |
|-----------|---------|
| **Manual retry must refetch** | Do not rely on bare **`query.refetch()`** alone for reads with **`staleTime: 60_000`**; use **`useQueryManualRetry`** ([`useQueryManualRetry.ts`](../frontend-dapp/src/hooks/useQueryManualRetry.ts)): **`invalidateQueries`** on the exact key, then **`refetch({ cancelRefetch: false })`**. |
| **Loading feedback** | While retrying, hide **`RetryError`** and show the chart **`Skeleton`** (`isFetching` after an error). **`RetryError`** may show **Retrying…** when the panel stays mounted. |
| **404 vs outage** | **`isIndexerUnavailableError`** treats **404** as a logical miss (chart retry panel), not **`trade-indexer-outage-banner`**. |
| **Regression** | [`TradePage.test.tsx`](../frontend-dapp/src/pages/TradePage.test.tsx) (404 + Retry), [`useQueryManualRetry.test.tsx`](../frontend-dapp/src/hooks/__tests__/useQueryManualRetry.test.tsx). |

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_QUERY_RETRY.md`](../skills/AGENTS_FRONTEND_QUERY_RETRY.md).

### Market data loading & outage (global) {#market-data-loading-outage}

Retail surfaces that depend on **indexer HTTP** share banner and loading primitives ([GitLab **#215**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/215)). The `/trade` route remains the reference for panel-level degradation and pair-switch loading — see [§ Trade page — indexer outage banner](#trade-page-indexer-outage-banner) and [§ Trade page — pair switch latency](#trade-page-pair-switch-latency).

| Invariant | Meaning |
|-----------|---------|
| **4xx vs outage** | Use [`isIndexerUnavailableError`](../frontend-dapp/src/utils/indexerErrors.ts) at query boundaries. **4xx** (404 pair miss, **400** route-solve simulation failure, etc.) → not-found / retry / LCD fallback — **not** the global market-data banner ([GitLab **#177**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/177), **#326**). Outage = transport errors or **5xx** only. |
| **LCD vs market data** | [`LcdConnectivityBanner`](../frontend-dapp/src/components/common/LcdConnectivityBanner.tsx) / [`LcdQueryGate`](../frontend-dapp/src/components/common/LcdQueryGate.tsx) cover **chain LCD** only — do not label RPC failures as “market data service.” |
| **Retail copy** | No `VITE_INDEXER_URL`, hostnames, or “indexer unavailable” in user-visible banners ([GitLab **#174**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/174)). Strings: [`marketDataServiceCopy.ts`](../frontend-dapp/src/utils/marketDataServiceCopy.ts); trade panel lines stay in [`indexerTradeOutageCopy.ts`](../frontend-dapp/src/utils/indexerTradeOutageCopy.ts). |
| **Detection** | [`detectMarketDataOutage`](../frontend-dapp/src/utils/marketDataOutage.ts) ORs transport errors across passed queries (trade re-exports as `detectTradeIndexerOutage`). |
| **Banner UI** | [`MarketDataServiceOutageBanner`](../frontend-dapp/src/components/common/MarketDataServiceOutageBanner.tsx) — trade keeps `data-testid="trade-indexer-outage-banner"` and **inline** layout; Charts/Trader/Pool/Protocol/`/limits`/`/` (swap) use stacked layout + page `data-testid`s (`charts-market-data-outage-banner`, `limits-market-data-outage-banner`, `swap-market-data-outage-banner`, etc.). |
| **Loading status** | [`MarketDataLoadingStatus`](../frontend-dapp/src/components/common/MarketDataLoadingStatus.tsx); trade pair switch wraps it as [`TradePairSwitchStatus`](../frontend-dapp/src/components/trade/TradePairSwitchStatus.tsx) (`trade-pair-switch-loading`); `/limits` uses `limits-pair-switch-loading` on the same primitive. |
| **`/limits` detection** | [`detectMarketDataOutage`](../frontend-dapp/src/utils/marketDataOutage.ts) on `indexer-pair-limit-orders` + `pair-trades-limit-orders` only — **not** placements list transport (logical empty OK). Lead: [`LIMITS_MARKET_DATA_OUTAGE_LEAD`](../frontend-dapp/src/utils/marketDataServiceCopy.ts); limit-reference **tail** reuses trade [#166] string from [`indexerTradeOutageCopy.ts`](../frontend-dapp/src/utils/indexerTradeOutageCopy.ts). Vitest: [`LimitOrdersPage.test.tsx`](../frontend-dapp/src/pages/LimitOrdersPage.test.tsx) ([GitLab **#218**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/218)). |
| **E2E** | Indexer-stopped specs run via `make test-e2e-indexer-outage` (Playwright project **`e2e-indexer-outage`**, `E2E_INDEXER_OUTAGE=1`) — see [docs/testing.md § Frontend E2E — indexer outage](./testing.md#frontend-e2e-indexer-outage). Default strict `npm run test:e2e` does not stop the indexer ([#201](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/201), [#219](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/219)). |

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md`](../skills/AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md).

### Swap page (`/` and `/swap`) — market data outage {#swap-page-market-data-outage}

When the swap **`simulation`** query observes **indexer transport / 5xx** errors (`isIndexerUnavailableError` or `indexerTransportFailed` on pool fallback — [GitLab **#241**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/241); **not** **400** route-solve failures — [GitLab **#326**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/326)), the page shows a stacked [`MarketDataServiceOutageBanner`](../frontend-dapp/src/components/common/MarketDataServiceOutageBanner.tsx) above the form (`data-testid="swap-market-data-outage-banner"`).

| Invariant | Meaning |
|-----------|---------|
| **4xx vs outage** | Indexer **404** / **400** on route solve do **not** show the global banner — LCD fallback or retry paths ([#177](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/177), [#326](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/326)). |
| **No stale quotes** | When `simQuery.isError`, receive line and fee/slippage blocks hide prior cached data; CTA reads **Quote unavailable** and stays disabled until a fresh sim succeeds. |
| **Pool fallback** | Indexer failure during quote sets `indexerTransportFailed` on successful LCD pool sim — banner still shows; CTA may remain enabled when the pool quote is valid (parity with Trade market panel). |
| **Wrap / unwrap** | Wrap-only sim does not call indexer — no outage banner on pure wrap paths; swap CTA stays available when other gates pass. |
| **LCD separate** | Factory pair list uses LCD — [`LcdQueryGate`](../frontend-dapp/src/components/common/LcdQueryGate.tsx) only; do not conflate with market-data copy ([#171](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/171)). |
| **Detection** | [`detectSwapIndexerOutage`](../frontend-dapp/src/utils/swapIndexerOutage.ts) on the sim query slice + optional `indexerTransportFailed` flag. Lead: [`SWAP_MARKET_DATA_OUTAGE_LEAD`](../frontend-dapp/src/utils/marketDataServiceCopy.ts). Vitest: [`SwapPage.test.tsx`](../frontend-dapp/src/pages/SwapPage.test.tsx). E2E: [`swap-indexer-outage.spec.ts`](../frontend-dapp/e2e/swap-indexer-outage.spec.ts). |

**Third-party / agent context:** [§ Market data loading & outage (global)](#market-data-loading-outage), [`skills/AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md`](../skills/AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md).

### Swap wrap safety CTA (SEC-A02) {#swap-wrap-safety-cta-sec-a02}

Native wrap paths on `/` query the **wrap mapper** for governance pause and per-denom rate limits before enabling submit ([`wrapMapper.ts`](../frontend-dapp/src/services/terraclassic/wrapMapper.ts)).

| Gate | Submit label | Disabled |
|------|--------------|----------|
| `config.paused === true` | **Wrapping is Temporarily Paused** | yes |
| `used + wrap_amount > max_amount_per_window` | **Rate Limit Exceeded** | yes |

Pause is evaluated **before** rate limit in the CTA precedence chain ([`SwapPage.tsx`](../frontend-dapp/src/pages/SwapPage.tsx)). This is **on-chain wrap quota**, not indexer HTTP **429**.

**Regression tests:** [`SwapPage.test.tsx`](../frontend-dapp/src/pages/SwapPage.test.tsx) (`SEC-A02` / GitLab **#389**); Playwright LCD mocks in [`wrap-swap.spec.ts`](../frontend-dapp/e2e/wrap-swap.spec.ts). Agent: [`skills/AGENTS_FRONTEND_SWAP_SAFETY_CTA.md`](../skills/AGENTS_FRONTEND_SWAP_SAFETY_CTA.md); matrix: [docs/testing.md § Swap wrap safety CTA](./testing.md#swap-wrap-safety-cta-sec-a02-gitlab-389).

### Pair pause disabled CTAs (SEC-B05 / L6) {#pair-pause-disabled-ctas-sec-b05}

Per-pair governance pause ([invariant **L6**](./contracts-security-audit.md)) blocks swaps and LP actions on-chain. The dApp queries LCD **`is_paused`** via [`getPairPaused`](../frontend-dapp/src/services/terraclassic/pair.ts) (shared hook [`usePairPaused`](../frontend-dapp/src/hooks/usePairPaused.ts)) and disables submit CTAs **before** broadcast.

| Page | Query scope | Submit label when paused | Banner `data-testid` |
|------|-------------|--------------------------|----------------------|
| `/` ([`SwapPage.tsx`](../frontend-dapp/src/pages/SwapPage.tsx)) | All pair contract addresses on the active swap route (direct, multihop, native, indexer ops) | **Pair is paused** | `swap-pair-paused-banner` |
| `/pool` ([`PoolPage.tsx`](../frontend-dapp/src/pages/PoolPage.tsx)) | Selected pool card’s pair | **Pair is paused** (provide + withdraw) | `pool-pair-paused-banner` |
| `/trade`, `/limits` | Selected pair | Ticket / book copy (see below) | — |

On `/`, **wrap-mapper pause** (SEC-A02) is evaluated **before** pair pause in the CTA precedence chain. Pair pause does **not** apply to pure wrap/unwrap paths with no pool hop.

**Regression tests:** [`SwapPage.test.tsx`](../frontend-dapp/src/pages/SwapPage.test.tsx) and [`PoolPage.test.tsx`](../frontend-dapp/src/pages/PoolPage.test.tsx) (`SEC-B05` / GitLab **#395**); trade/limit path: [`TradePage.test.tsx`](../frontend-dapp/src/pages/TradePage.test.tsx) (GitLab **#87** / **#199**). Retail FAQ: [user-incident-faq.md § Pair pause](./user-incident-faq.md#pair-pause).

### Code-id freeze (GitLab #585) {#code-id-freeze-gitlab-585}

Listed-CW20 **F6** write paths fail closed when live `ContractInfo.code_id` ≠ the listing pin or the live id is not factory-whitelisted ([#582](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/582)). Queries (`Simulation` / `HybridSimulation`) stay ungated, so a quote can still appear while execute is blocked. The dApp and indexer **surface** that freeze; they do **not** replace on-chain gating.

| Invariant | Meaning |
|-----------|---------|
| **F585-1** | Indexer `route/solve` skips frozen hops at path enumeration (`find_path` + `build_adjacency`). |
| **F585-2** | Pair list/detail expose `code_id_frozen`; frozen pairs stay listed for charts. |
| **F585-3** | LCD / `GetAssetCodeIds` failures are **fail-open** for routing and UI (keep last known; unknown ≠ frozen). Pre-1.15.0 pairs are not frozen. On-chain execute still fail-closes. |
| **F585-4** | Freeze LCD probe is off the `route/solve` request path (60s background cache). |
| **F585-5** | Execute errors (`AssetCodeIdDrift`, not factory-whitelisted, guard unavailable, unpinned) are humanized — not a generic failed tx. |
| **F585-6** | Swap / Trade / Pool / Charts / Limits show freeze state; banner says quotes can still appear; CTA **Market frozen**. |
| **F585-7** | Does **not** un-gate on-chain cancel / claim / withdraw. |
| **F585-8** | Does **not** add FoT / balance-delta swap math (**H-01**). |

| Page | Signal | Submit label | Banner `data-testid` |
|------|--------|--------------|----------------------|
| `/` ([`SwapPage.tsx`](../frontend-dapp/src/pages/SwapPage.tsx)) | LCD probe on route hops | **Market frozen** | `swap-pair-code-id-frozen-banner` |
| `/trade` ([`TradeOrderTicket.tsx`](../frontend-dapp/src/components/trade/TradeOrderTicket.tsx)) | LCD + optional indexer hint | disabled place/cancel | `trade-pair-code-id-frozen-banner` |
| `/pool` | LCD probe; table **Frozen** badge from `code_id_frozen` | **Market frozen** | `pool-pair-code-id-frozen-banner` / `pool-row-code-id-frozen` |
| `/charts` | indexer `code_id_frozen` **or** LCD | — | `charts-pair-code-id-frozen-banner` |
| `/limits` | LCD probe | disabled book edit (same as pause) | `limits-pair-code-id-frozen-banner` |

Hook: [`usePairCodeIdFreeze`](../frontend-dapp/src/hooks/usePairCodeIdFreeze.ts) (`indexerHintFrozen` **OR** LCD `probePairCodeIdFreeze`). Copy: [`assetCodeIdFreeze.ts`](../frontend-dapp/src/utils/assetCodeIdFreeze.ts). Indexer: [`asset_code_id_freeze.rs`](../indexer/src/indexer/asset_code_id_freeze.rs).

**Regression:** `make verify-issue-585`. Playbook: [`skills/AGENTS_FRONTEND_CODE_ID_FREEZE.md`](../skills/AGENTS_FRONTEND_CODE_ID_FREEZE.md). Retail FAQ: [user-incident-faq.md § Code-id freeze](./user-incident-faq.md#code-id-freeze). On-chain: [`AGENTS_CW20_CODE_ID_PIN.md`](../skills/AGENTS_CW20_CODE_ID_PIN.md).

### Trading blacklist disabled CTAs (SEC-E01) {#trading-blacklist-disabled-ctas-sec-e01}

Factory trading blacklist ([ADR 0003](./adr/0003-governance-trading-blacklist.md), [security model § Trading blacklist](../security-model.md#trading-blacklist-compliance--incident-response)) gates user write actions in the dApp via [`useTradingBlacklist`](../frontend-dapp/src/hooks/useTradingBlacklist.ts) and [`describeTradingBlacklistBlock`](../frontend-dapp/src/services/terraclassic/blacklist.ts).

| Page | Gated CTAs | Submit label when blocked (connected) |
|------|------------|---------------------------------------|
| `/` ([`SwapPage.tsx`](../frontend-dapp/src/pages/SwapPage.tsx)) | Swap | **Trading restricted** |
| `/pool` ([`PoolPage.tsx`](../frontend-dapp/src/pages/PoolPage.tsx)) | Provide + withdraw liquidity | **Trading restricted** |
| `/trade` ([`TradeOrderTicket.tsx`](../frontend-dapp/src/components/trade/TradeOrderTicket.tsx)) | Limit place, update price, cancel (form + my limits) | disabled (label unchanged) |
| `/limits` ([`LimitOrdersPage.tsx`](../frontend-dapp/src/pages/LimitOrdersPage.tsx)) | Limit place, cancel (form + book + my limits) | disabled (label unchanged) |

**Regression tests:** [`SwapPage.test.tsx`](../frontend-dapp/src/pages/SwapPage.test.tsx), [`PoolPage.test.tsx`](../frontend-dapp/src/pages/PoolPage.test.tsx), [`TradePage.test.tsx`](../frontend-dapp/src/pages/TradePage.test.tsx), [`LimitOrdersPage.test.tsx`](../frontend-dapp/src/pages/LimitOrdersPage.test.tsx) (`SEC-E01` / GitLab **#425**); shared mocks: [`tradingBlacklistMocks.ts`](../frontend-dapp/src/test/tradingBlacklistMocks.ts). E2E LCD mock (swap only): [`e2e/blacklist-swap.spec.ts`](../frontend-dapp/e2e/blacklist-swap.spec.ts).

### Trade page — indexer outage banner {#trade-page-indexer-outage-banner}

When the **`getPair`** query on `/trade` fails with an **indexer transport / non-OK** error (`isIndexerUnavailableError` in [`indexerErrors.ts`](../frontend-dapp/src/utils/indexerErrors.ts)), the page shows a warning **above** the workspace (`data-testid="trade-indexer-outage-banner"`).

| Invariant | Meaning |
|-----------|---------|
| **No false “chain fallback”** | The banner must **not** claim the order book or tickets still work “on chain” while [`OrderBookPanel`](../frontend-dapp/src/components/trade/OrderBookPanel.tsx) and related flows read depth, tape, candles, or routing through **indexer HTTP** (book pages use [`getPairLimitBookPage`](../frontend-dapp/src/services/indexer/client.ts), which is unreachable when the indexer is down — [GitLab **#164**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/164)). |
| **Limit reference vs book (#166)** | The banner **tail** may state one **narrow** exception: **limit price** buy-below / sell-above checks can fall back to **AMM pool** reserves via **LCD** when tape is missing ([GitLab **#166**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/166)) — this does **not** restore order-book depth or hybrid routing; see [§ Trade page — limit order price field](#trade-page-limit-order-price). |
| **No internal URLs in retail copy** | The banner must **not** render `VITE_INDEXER_URL`, hostnames, or “indexer unavailable at …” — use **market data service** wording ([GitLab **#174**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/174)). |
| **Single source of strings** | Shared title from [`marketDataServiceCopy.ts`](../frontend-dapp/src/utils/marketDataServiceCopy.ts); trade lead, tail, and panel lines in [`indexerTradeOutageCopy.ts`](../frontend-dapp/src/utils/indexerTradeOutageCopy.ts); [`TradePage.tsx`](../frontend-dapp/src/pages/TradePage.tsx) composes via [`MarketDataServiceOutageBanner`](../frontend-dapp/src/components/common/MarketDataServiceOutageBanner.tsx). |
| **Funds safety (SEC-E05)** | Banner **lead** must state that on-chain wallet balances, LP shares, and limit order escrows are **unaffected** — only display/routing may be limited ([GitLab **#427**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/427)). Vitest: [`indexerTradeOutageCopy.test.ts`](../frontend-dapp/src/utils/__tests__/indexerTradeOutageCopy.test.ts). |

**Third-party / agent context:** [§ Market data loading & outage (global)](#market-data-loading-outage), [`skills/AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md`](../skills/AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md), [`skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](../skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md); retail error funnel: [`skills/AGENTS_FRONTEND_USER_ERRORS.md`](../skills/AGENTS_FRONTEND_USER_ERRORS.md).

### Trade page — deep order book pagination {#trade-page-deep-order-book}

CEX-style **full-depth** resting limits on `/trade` and `/limits` ([GitLab **#194**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/194), remainder of **#102** / DEX-P1-001):

| Invariant | Meaning |
|-----------|---------|
| **Indexer path (primary)** | Depth comes from **`GET /api/v1/pairs/{addr}/limit-book?side=bid\|ask&limit=L&after_order_id=OPTIONAL`** ([ADR 0002](./adr/0002-limit-book-surfacing.md)). The dApp does **not** call LCD for book rows in production UI — there is no silent LCD fallback when the indexer is down ([GitLab **#164**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/164)). |
| **Legacy shallow** | **`limit-book-shallow`** (max 20) remains for integrators; **`OrderBookPanel`** uses paginated **`limit-book`** only. |
| **Page size** | UI requests **`LIMIT_BOOK_UI_PAGE_SIZE` (45)** per HTTP page (≤ indexer max **100**). Shared constant: [`limitBookPagination.ts`](../frontend-dapp/src/utils/limitBookPagination.ts). |
| **Keyset cursor** | React Query **`useInfiniteQuery`** via [`useLimitBookInfinite`](../frontend-dapp/src/hooks/useLimitBookInfinite.ts): first page omits `after_order_id`; **`fetchNextPage`** passes `next_after_order_id` from the prior response. Query key: **`['limitBookPage', pairAddr, side]`**. |
| **Non-blocking load** | Initial render shows a spinner per side; **Load more depth** fetches the next page **asynchronously** (no synchronous main-thread walk). Cumulative **Total** column recomputes across merged pages in memory. |
| **Prefetch** | [`prefetchTradePairWorkspace`](../frontend-dapp/src/utils/tradePairPrefetch.ts) warms **page 1** for both sides on pair switch via **`prefetchInfiniteQuery`** (same cache shape as [`useLimitBookInfinite`](../frontend-dapp/src/hooks/useLimitBookInfinite.ts); [#354](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/354)) — not the full book. |
| **Head / preflight** | Best bid/ask for post-only preflight uses **`limit=1`** via [`useTradeBestBookPrices`](../frontend-dapp/src/hooks/useTradeBestBookPrices.ts) — separate query keys (`tradeBestBook`), not the infinite book cache. |
| **Invalidations** | Place / cancel / claim success invalidates **`limitBookPage`** (and legacy **`limitBookPagePreview`**) for the pair so depth stays fresh. |
| **Insert hints (#261)** | New limit placement on `/trade` and `/limits` resolves **`hint_after_order_id`** from merged **`useLimitBookInfinite`** pages via [`resolveLimitInsertHintAfter`](../frontend-dapp/src/utils/limitBookInsertHint.ts) and passes it through **`placeLimitOrderWithAllowance`**. Omit hint when book unknown (pagination gap / head insert). Price-edit path unchanged (**#247**). Invariant **L14**: [contracts-security-audit.md](./contracts-security-audit.md). |
| **Rate limits** | Indexer may return **429** when `RATE_LIMIT_RPS > 0`; the UI shows side-level **Book unavailable** on fetch error — clients should not hammer full-book walks in a tight loop. |

Implementation: [`OrderBookPanel.tsx`](../frontend-dapp/src/components/trade/OrderBookPanel.tsx), [`LimitOrdersPage.tsx`](../frontend-dapp/src/pages/LimitOrdersPage.tsx). Indexer tests: [`api_limit_book_deep.rs`](../indexer/tests/api_limit_book_deep.rs). Integrator HTTP semantics: [§ On-chain limit book (LCD proxy)](./integrators.md#on-chain-limit-book-lcd-proxy).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_DEEP_ORDER_BOOK.md`](../skills/AGENTS_FRONTEND_DEEP_ORDER_BOOK.md); row actions: [`skills/AGENTS_FRONTEND_ORDER_BOOK_ROW_ACTIONS.md`](../skills/AGENTS_FRONTEND_ORDER_BOOK_ROW_ACTIONS.md).

### Trade page — order book row actions (cancel, edit, cancel-all) {#trade-book-row-actions}

CEX-style controls on the **Bids / Asks** depth tables on `/trade` ([GitLab **#162**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/162)):

| Invariant | Meaning |
|-----------|---------|
| **Shared cancel mutation** | `TradePage` constructs one `useLimitOrderCancelMutation(pairAddr, walletAddress)` and passes it to both `OrderBookPanel` and `TradeOrderTicket` so row cancels and the **Manage — Cancel resting limit** form share loading, errors, and query invalidations. |
| **Order id on every row** | Each row shows **`#order_id`** above the price (paginated **`limit-book`** row from indexer → LCD). |
| **Row actions (wallet-owned rows only)** | When `order.owner ===` connected address, **Edit** prefills the limit ticket (`LimitBookTicketDraft`: `orderId`, side, price, `fromRawAmount(remaining)`, optional `expiresAt`, advisory `hintAfterOrderId`) and switches to the **Limit** tab. **Price-only** change → **Update price** submits `ExecuteMsg::UpdateLimitOrderPrice` (GitLab **#247**). Size/side/expiry change → blocked with cancel-first copy. **×** runs the same cancel path as the ticket after `window.confirm`. |
| **Cancel all mine** | Submits one cancel tx per **active** indexed placement for the wallet on the pair (`GET .../limit-placements` + lifecycle partition), sequentially via `mutateAsync`, with confirm + stop-on-first-error alert. Hidden unless both `cancelLimitOrderMutation` and `onPrefillLimitTicket` are wired (same `/trade` bundle). |
| **Paused pair** | Row **Edit** / **×** and **Cancel all mine** stay disabled when `get_pair_paused` is true (L6 / GitLab #120), matching the ticket. |
| **`data-testid`s** | `trade-book-edit-{bid\|ask}-{order_id}`, `trade-book-cancel-{bid\|ask}-{order_id}`, `trade-book-cancel-all-mine`. |
| **Single ticket on `/trade`** | Exactly one mounted [`TradeOrderTicket`](../frontend-dapp/src/components/trade/TradeOrderTicket.tsx) at a time: sub-desktop vs desktop layouts are gated with [`useMediaQuery`](../frontend-dapp/src/hooks/useMediaQuery.ts) on [`TRADE_DESKTOP_LAYOUT_MEDIA_QUERY`](../frontend-dapp/src/utils/tradePageLayout.ts) (`1024px`, Tailwind `lg`). Two CSS-hidden tickets used to mount together; the hidden instance consumed `LimitBookTicketDraft` before the visible ticket applied it ([GitLab **#178**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/178)). |
| **Edit UX (#247 price update)** | **Edit** does not open a modal. Prefills the limit ticket and selects the **Limit** tab. Change **price only** → **Update price** (one chain tx, same `order_id`, no CW20). Change size/side/expiry → cancel the resting order first, then place. Test ids: `trade-limit-edit-context`, `trade-limit-update-price-submit`. |
| **E2E smoke (#338)** | [`trade-book-edit-178.spec.ts`](../frontend-dapp/e2e/trade-book-edit-178.spec.ts) asserts edit-context **after** Edit (not on page load). Blocks [#292](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/292) smoke until green. See [e2e README § Trade book Edit](../frontend-dapp/e2e/README.md#trade-book-edit-smoke-gitlab-338). |

Implementation: [`useLimitOrderCancelMutation.ts`](../frontend-dapp/src/hooks/useLimitOrderCancelMutation.ts), [`useLimitOrderUpdatePriceMutation.ts`](../frontend-dapp/src/hooks/useLimitOrderUpdatePriceMutation.ts), [`limitOrderPriceEdit.ts`](../frontend-dapp/src/utils/limitOrderPriceEdit.ts), [`OrderBookPanel.tsx`](../frontend-dapp/src/components/trade/OrderBookPanel.tsx), [`TradePage.tsx`](../frontend-dapp/src/pages/TradePage.tsx), [`TradeOrderTicket.tsx`](../frontend-dapp/src/components/trade/TradeOrderTicket.tsx). Types: [`limitBookTicketDraft.ts`](../frontend-dapp/src/types/limitBookTicketDraft.ts).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_ORDER_BOOK_ROW_ACTIONS.md`](../skills/AGENTS_FRONTEND_ORDER_BOOK_ROW_ACTIONS.md); trade layout: [`skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](../skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md). Open-row Cancel: [`skills/AGENTS_FRONTEND_LIMIT_CANCEL_OPEN.md`](../skills/AGENTS_FRONTEND_LIMIT_CANCEL_OPEN.md) (`#530`).

### My open limits — Cancel vs stale ● row {#open-limits-cancel-reconciliation}

**My open limits** on `/trade` (compact) and `/limits` must reconcile indexer `active` with chain before offering Cancel ([GitLab **#530**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/530)). Fills never flip placement lifecycle; a fully filled UST1 ask can still render `●order #1 · Sell UST1 · 82.044…`.

| Invariant | Meaning |
|-----------|---------|
| **LCD first** | `queryOrderStatus` / `useLimitOrderStatuses`. `Active` → Cancel. `ParkedRefund` → Claim. Query failure stays unset — **not** `Unknown` (**L21**). |
| **Unknown classification** | Fill row → **Filled**. Indexed or local cancel → **Already cancelled**. Else **No longer on the book**. No enabled Cancel. |
| **Disabled copy** | Paused → `Unavailable (pair paused)`. Blacklist → `Trading restricted`. Gone/filled/cancelled use those labels — never a mute `Cancel`. |
| **`/trade` reachability** | `trade-ticket-placements-anchor` stays in `trade-order-ticket-scroll` **above** `trade-ticket-submit-footer`. Compact Cancel is not a child of Place limit. |
| **Invert-safe** | Cancel execute is `{ cancel_limit_order: { order_id } }` on the selected pair. #524 invert does not add fields. |
| **Testids** | `trade-cancel-placement-{id}`, `limits-page-cancel-placement-{id}`, `trade-book-cancel-{bid\|ask}-{id}`, `trade-ticket-placements-anchor`. |

Helpers: [`limitPlacementOpenReconcile.ts`](../frontend-dapp/src/utils/limitPlacementOpenReconcile.ts). Product invariants **F530-1–F530-8**: [`docs/limit-orders.md` § Open-row Cancel reconciliation](./limit-orders.md#open-row-cancel-reconciliation-gitlab-530). Verify: `make verify-issue-530`.

### Limit orders page (`/limits`) — market data outage {#limits-page-market-data-outage}

When **`getPair`** or **`getTrades`** on `/limits` fails with **`isIndexerUnavailableError`**, the page shows a stacked [`MarketDataServiceOutageBanner`](../frontend-dapp/src/components/common/MarketDataServiceOutageBanner.tsx) above the book (`data-testid="limits-market-data-outage-banner"`). Pair switch shows [`MarketDataLoadingStatus`](../frontend-dapp/src/components/common/MarketDataLoadingStatus.tsx) as `limits-pair-switch-loading` while `indexer-pair-limit-orders` / `pair-trades-limit-orders` fetch ([GitLab **#218**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/218), follow-up to **#215**).

| Invariant | Meaning |
|-----------|---------|
| **404 vs outage** | Indexer **404** on `getPair` does **not** show the global banner — same boundary as `/trade` ([#177](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/177)). |
| **Shared book panel** | [`OrderBookPanel`](../frontend-dapp/src/components/trade/OrderBookPanel.tsx) keeps `trade-book-unavailable-*` testids on `/limits`; page banner is additional context ([#164](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/164)). |
| **Limit reference (#166)** | Banner **tail** matches trade: pool-reserve exception for buy-below / sell-above only — does not restore book depth or placements. |
| **Place gate** | [`evaluateLimitOrderPricePlaceGate`](../frontend-dapp/src/utils/limitOrderPricePlaceGate.ts) still blocks submit when reference cannot resolve during outage. |
| **LCD separate** | Factory pair list / escrow balances use LCD — [`LcdQueryGate`](../frontend-dapp/src/components/common/LcdQueryGate.tsx) only; do not conflate with market-data copy ([#171](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/171)). |

**Third-party / agent context:** [§ Market data loading & outage (global)](#market-data-loading-outage), [`skills/AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md`](../skills/AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md), [`skills/AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md`](../skills/AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md).

### Limit orders page (`/limits`) — order book row actions {#limits-page-order-book-row-actions}

The standalone **Limit Orders** route reuses the same [`OrderBookPanel`](../frontend-dapp/src/components/trade/OrderBookPanel.tsx) and [`useLimitOrderCancelMutation`](../frontend-dapp/src/hooks/useLimitOrderCancelMutation.ts) wiring as `/trade` ([GitLab **#162**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/162)) so traders are not forced to scroll to **Cancel limit** and type an **Order ID** after finding it elsewhere.

| Invariant | Meaning |
|-----------|---------|
| **One cancel mutation** | [`LimitOrdersPage.tsx`](../frontend-dapp/src/pages/LimitOrdersPage.tsx) calls `useLimitOrderCancelMutation(pairAddr, address)` once; row **×**, **Cancel all mine**, and the manual cancel form all call `mutate(orderId)` on that instance. |
| **Edit → ticket** | **Edit** on a wallet-owned row runs the same `LimitBookTicketDraft` prefill as `/trade` (side, price, remaining size into the place form below the book). |
| **`data-testid`s** | Same as [§ Trade page — order book row actions](#trade-book-row-actions) (`trade-book-*`) because the component is shared; scope Playwright to the `/limits` route when asserting. |
| **Manual cancel form** | Kept for integrators and edge cases; helper copy points users to row actions first. |

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_ORDER_BOOK_ROW_ACTIONS.md`](../skills/AGENTS_FRONTEND_ORDER_BOOK_ROW_ACTIONS.md).

### Trade page — limit order price field (reference, deviation, USD anchor) {#trade-page-limit-order-price}

Retail safety for typed **token1 per token0** limits ([GitLab **#154**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/154), **#166** — indexer outage must not bypass validation):

| Invariant | Meaning |
|-----------|---------|
| **Reference rate** | Primary: newest indexed swap (**last trade**), converted to **human** token1/token0 using `offer_asset` / `ask_asset` symbols and **decimals** from `GET /api/v1/pairs/{addr}` — same ordering as on-chain limits (`docs/limit-orders.md` § Ordering). **Fallback:** when tape is missing or unparseable but wallet **LCD** can still read the pair contract, the UI uses **AMM pool reserves** (`pair` → `pool` query) for a constant-product **spot** token1/token0, using indexer decimals when the pair row exists else **registry-only** decimals (no guessed defaults for unknown CW20s). |
| **% deviation** | Signed \((\text{typed} - \text{ref}) / \text{ref}\); **red** when the direction is invalid for the side; **amber** when \(\lvert \text{pct} \rvert \geq 50\) in an otherwise valid direction. |
| **Headline-scaled USD** | `anchorUsdForLimitPrice` scales the chart **tape headline** linearly with typed price vs reference so the line matches the headline when the typed price equals the reference (same `tapeLastPriceUsd` from `resolveTapeLastPriceUsd` / `price_usd` as `PriceChart` — [#522](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/522)). This is an **anchor estimate**, not a fresh oracle quote per token. When the reference comes from the **pool** (no tape), headline USD may stay **—** until tape returns. |
| **Submit gate** | **Bid:** `typed price >= reference` → disabled submit + error copy (buy limit must be **below** reference). **Ask:** `typed price <= reference` → disabled submit. When the user typed a **positive** limit and **no** reference can be resolved (no tape, pool empty, unknown decimals, or LCD error while loading pool), submit is **blocked** with explicit copy — never silently skip the guard ([GitLab **#166**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/166)). While the pool fallback query is **in flight**, submit stays disabled with a **warning** tone. |
| **Rate label + chips (#488/#489/#495)** | Primary card uses **When 1 {token} is worth** (token0 symbol). **% chips** sit under the rate field with **side-aware signs**: bid → `0%−` / `−1%` / `−5%` / `−10%` (strictly **below** ref); ask → `0%+` / `+1%` / `+5%` / `+10%` (strictly **above** ref). Magnitude `0` maps to ±`LIMIT_PRICE_NEAR_MARKET_DEVIATION_PERCENT` (0.01%) so chips never land on equality (invalid per #154). No on-card instructional essay and no **Place limit ⓘ** tooltip; invalid-direction errors stay inline. |
| **Human ↔ raw (#529)** | Typed price, refs, gates, and book display stay **human** token1/token0. On-chain / indexer book `price` is **raw** (`× 10^(dec1 − dec0)` vs human). Submit scales via [`limitOrderPriceScale.ts`](../frontend-dapp/src/utils/limitOrderPriceScale.ts); #524 invert applies to the human factory price only. |

Implementation: [`limitOrderPriceReference.ts`](../frontend-dapp/src/utils/limitOrderPriceReference.ts) (`resolveLimitOrderPriceRef`, pool spot helpers, `signedLimitPriceDeviationPercent` / `limitPriceFromRefDeviationChip`), [`useLimitOrderPriceRefBundle.ts`](../frontend-dapp/src/hooks/useLimitOrderPriceRefBundle.ts) (tape + `getPool` wiring for [`TradeOrderTicket.tsx`](../frontend-dapp/src/components/trade/TradeOrderTicket.tsx) and [`LimitOrdersPage.tsx`](../frontend-dapp/src/pages/LimitOrdersPage.tsx)), [`limitOrderPricePlaceGate.ts`](../frontend-dapp/src/utils/limitOrderPricePlaceGate.ts), [`LimitOrderPriceField.tsx`](../frontend-dapp/src/components/trade/LimitOrderPriceField.tsx) (`LimitOrderPriceInputWithContext`), plus [`TradePage.tsx`](../frontend-dapp/src/pages/TradePage.tsx) for pair context. `LimitOrderEscrowPlaceGuardMessage` accepts the price gate result for inline errors.

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md`](../skills/AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md), [`skills/AGENTS_LIMIT_PRICE_DECIMALS.md`](../skills/AGENTS_LIMIT_PRICE_DECIMALS.md).

### Trade page — limit order pre-submit summary (resting semantics, fees) {#trade-page-limit-order-pre-submit-summary}

Before **Place limit**, the ticket and standalone **`/limits`** form show a **pre-sign summary** so traders are not comparing resting limits to market-style quote lines ([GitLab **#157**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/157)):

| Invariant | Meaning |
|-----------|---------|
| **Signing fields (SEC-I05 / #461)** | Labeled rows before the wallet opens: **Action** (`Place Limit Order`), **Pair**, **Side** (Buy/Sell base), **Amount** (escrow), **Chain** (`getNetworkBadgeCopy().fullLabel`) — same anti-phishing anchor as swap/pool pre-sign cards. |
| **Compact copy (#489)** | No instructional paragraphs on the pre-sign card — labeled rows only, plus optional **Docs** link for resting-limit semantics. Market-style slippage / min-received lines belong on the **Market** tab / Swap card, not here. |
| **% vs reference** | Same signed deviation as under the price field: \((\text{typed} - \text{ref}) / \text{ref} \times 100\) from [`limitPriceDeviationPercent`](../frontend-dapp/src/utils/limitOrderPriceReference.ts), using the resolved tape or pool reference. |
| **Maker placement fee** | Retail copy: **small fee at placement** with human **percent** (`bpsToPercentLabel`) plus bps detail — not bps-only ([#419](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/419)). On-chain: **`floor(effective_fee_bps / 2)`** bps of escrow at placement ([`orderbook.rs`](../smartcontracts/contracts/pair/src/orderbook.rs)). **I14 / [#537](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/537):** apply `limit_discount_bps` only when this pair’s `discount_registry` matches `VITE_FEE_DISCOUNT_ADDRESS`; otherwise show full `maker_fee_bps(fee_bps)` (unwired pairs charge 90 bps at 180 pair fee regardless of CL8Y tier). |
| **Est. network fee** | Minimum **uluna** for **`increase_allowance` + `place_limit_order`** via [`estimateLimitOrderPlaceSequenceUlunaFeesTotal`](../frontend-dapp/src/services/terraclassic/transactions.ts) — informational; wallet extensions may still adjust `gas_wanted`. |
| **`data-testid`s** | **`trade-limit-pre-submit-summary`** on `/trade`; **`limits-page-pre-submit-summary`** on `/limits`; field rows suffixed `-action`, `-pair`, `-side`, `-amount`, `-chain`. |

Implementation: [`LimitOrderPreSubmitSummary.tsx`](../frontend-dapp/src/components/trade/LimitOrderPreSubmitSummary.tsx), [`useLimitOrderMakerFeeRates.ts`](../frontend-dapp/src/hooks/useLimitOrderMakerFeeRates.ts), [`limitOrderFeeSummary.ts`](../frontend-dapp/src/utils/limitOrderFeeSummary.ts); wired in [`TradeOrderTicket.tsx`](../frontend-dapp/src/components/trade/TradeOrderTicket.tsx) and [`LimitOrdersPage.tsx`](../frontend-dapp/src/pages/LimitOrdersPage.tsx).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md`](../skills/AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md) (cross-links #157 with price reference + this summary).

### Trade page — limit place success affordances {#trade-page-limit-place-success-affordances}

After **Place limit** succeeds on **`/trade`**, the order ticket shows **next-step** controls so traders are not left at a dead end ([GitLab **#161**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/161)):

| Invariant | Meaning |
|-----------|---------|
| **View order** | Scrolls the compact **My limits (indexer)** list into view. When the post-place poll has resolved an **`order_id`**, the UI scrolls to the matching row (`data-testid="trade-placement-active-{order_id}"`) and applies a short **highlight** ring on that `<li>`. If the row is not in the DOM yet, the scroll target is the placements anchor (`data-testid="trade-ticket-placements-anchor"`). |
| **Place another** | Clears the success mutation state, resets limit fields to the same defaults as a fresh ticket (**price `1`**, empty amount, **`expires_at` unset**, **placement gas preset Medium → 32 steps**), clears the cancel **Order ID** line, clears the “last indexed” helper, and **focuses + selects** the limit price input (`useId()`-scoped `htmlFor` / `id` on [`LimitOrderPriceInputWithContext`](../frontend-dapp/src/components/trade/LimitOrderPriceField.tsx)). |
| **Indexer lag copy** | While **`lastIndexedOrderId`** is still **null** after success, helper copy explains that the indexer may lag and that **View order** can be tapped again once the row appears. |
| **Stable test ids** | `trade-limit-post-place-actions`, `trade-limit-view-order-btn`, `trade-limit-place-another-btn` — use in Playwright when extending funded limit flows. |

Implementation: [`TradeOrderTicket.tsx`](../frontend-dapp/src/components/trade/TradeOrderTicket.tsx), [`LimitOrderMyPlacementsPanel.tsx`](../frontend-dapp/src/components/trade/LimitOrderMyPlacementsPanel.tsx) (`highlightOrderId`).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](../skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md).

### Trade page — responsive layout (sub-desktop) {#trade-page-responsive-layout}

Below **`lg` (`min-width: 1024px`)**, [`TradePage.tsx`](../frontend-dapp/src/pages/TradePage.tsx) uses a **CSS Grid** layout. Tablet portrait (**`768px`–`1023px`**, Tailwind **`md:`**–**`lg:`**) gets a **two-column top row** (price chart left, limit **order ticket** right) so iPad-class viewports are not forced into a phone-only vertical stack ([GitLab **#146**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/146)). Desktop (`≥1024px`) is a **non-resizable CSS grid** (book \| chart \| ticket + independent tape row) — see [§ Trade page — desktop workspace](#trade-page-desktop-workspace) ([#561](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/561)). Header density for the same band is documented above ([GitLab **#136**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/136)); this section is the **trade workspace** counterpart.

| Invariant | Meaning |
|-----------|---------|
| **`<768px` (default grid)** | Single column, DOM order: **order book** → **order ticket** → **chart** (when pair resolved) → **recent trades**. |
| **`768px`–`1023px` (`md:` / `<lg:`)** | Two-column grid: **chart** `row-start-1` / `col-start-1`, **order ticket** `row-start-1` / `col-start-2`, **order book** full width `row-start-2`, **recent trades** full width `row-start-3`. |
| **`≥1024px` (`lg:`)** | CSS grid: book \| chart \| ticket on the top row; **Recent trades** independent bottom row. **No** drag-resize handles ([#561](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/561)). |
| **`useMediaQuery` on TradePage** | Used only to mount **one** workspace tree (single `TradeOrderTicket`, [#178](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/178)). Not a layout-from-URL API. |
| **`data-testid="trade-sub-lg-workspace"`** | Marks the sub-desktop grid root so Playwright (and agents) can scope headings — the desktop tree also contains an order book + chart and would otherwise duplicate roles. |
| **Price chart (flex chain)** | Immediate wrapper around `PriceChart` is **`h-full flex flex-col min-h-0`** with **no** extra `card-glass` (desktop chart cell and sub-lg chart column). Keeps candles from clipping ([GitLab **#151**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/151), **L561-1**). |

Regression coverage: [`frontend-dapp/e2e/trade-page-responsive.spec.ts`](../frontend-dapp/e2e/trade-page-responsive.spec.ts).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](../skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md).

### Trade page — desktop workspace (no drag-resize) {#trade-page-desktop-workspace}

Desktop `/trade` (`lg`, `min-width: 1024px`) is a **CSS grid** with boolean panel toggles — not `react-resizable-panels` ([GitLab **#561**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/561)). Implementation: [`TradeDesktopWorkspace.tsx`](../frontend-dapp/src/components/trade/TradeDesktopWorkspace.tsx), prefs in [`tradeWorkspacePanels.ts`](../frontend-dapp/src/utils/tradeWorkspacePanels.ts).

| Invariant | Meaning |
|-----------|---------|
| **L561-1 Single chart surface** | `PriceChart` `shell-panel-strong` is the only chrome on the chart. No wrapping `card-glass`. |
| **L561-2 One chrome layer** | Applies [§ One chrome layer / anti-nesting](#one-chrome-layer) (**C653**) on `/trade`: sibling panels, not a panel-of-panels. Swap IO cards stay the interactive exception. Metric grids (Charts / Trader / Protocol) use `StatBox variant="flat"` — see [#653](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/653). |
| **L561-3 Independent tape** | Recent trades is a bottom-row panel (`trade-desktop-tape-panel`), not nested in the chart column. Expand/collapse is a button (`trade-desktop-tape-toggle`), not a splitter. |
| **L561-4 No drag-resize** | No `PanelResizeHandle` in the default layout. P10 asserts handles are absent. |
| **L561-5 Hide book** | `trade-desktop-book-toggle` hides the book; chart width grows (`3.2fr` vs remaining ticket `1fr`). The ticket must not take the vacated width. Restore control stays visible. |
| **L561-6 Hide ticket** | `trade-desktop-ticket-toggle` hides the ticket (`hidden` + `inert` + `interactive={false}`). Chart width grows; the book stays `1fr`. No focus/submit from the hidden tree. |
| **L561-7 Both hidden** | Chart uses the remaining workspace width (`1fr` only). Default (both visible: `1fr` / `2.2fr` / `1fr`) keeps the chart the largest region. |
| **L561-8 Persistence** | Tape: `cl8y-dex-trade-tape-expanded`. Book: `cl8y-dex-trade-book-visible`. Ticket: `cl8y-dex-trade-ticket-visible`. Values `'1'` / `'0'` only; anything else → default (sides visible, tape collapsed). |
| **L561-9 Sub-lg + one ticket** | Sub-`lg` grid unchanged in structure ([#146](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/146)). Exactly one `TradeOrderTicket` ([#178](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/178)). |
| **L561-10 Footer / candles / portal** | Visible ticket keeps `trade-ticket-submit-footer` dock ([#527](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/527)). Chart flex chain ([#151](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/151)). Pair selector portal does not shift layout ([#181](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/181)). |
| **L561-11 Book Edit** | Edit while ticket hidden re-shows the ticket and applies the draft. Hide uses `inert` (not unmount) so fields persist on the **same pair**. Pair URL change remounts the page via Layout keyed Outlet ([#358](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/358)), so a hidden ticket still binds the **current** `pairAddr` (A9). |
| **L561-12 Verify** | `make verify-issue-561`. |

**Hide vs unmount:** ticket/book stay mounted. **No** `?layout=` query flags (A5). Pause / blacklist / indexer banners stay above the workspace (A4).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](../skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md).

### Trade page — invalid pair deep link {#trade-page-invalid-pair-link}

When `/trade/:pairAddr` contains a segment that is **not** a valid Terra pair contract address ([`isValidTerraAddress`](../frontend-dapp/src/utils/constants.ts) via [`tradePairRoute.ts`](../frontend-dapp/src/utils/tradePairRoute.ts)), the UI must **not** leave the garbage string in the URL, show it in the pair selector, or block the empty-route auto-pick of the first factory pair behind a truthy invalid param ([GitLab **#176**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/176)).

| Invariant | Meaning |
|-----------|---------|
| **URL cleanup** | On invalid segment, `navigate('/trade', { replace: true, state: { invalidPair } })` so share links do not keep non-`terra1` garbage in the address bar. Notice state survives [`Layout`](../frontend-dapp/src/components/common/Layout.tsx) keyed-`<Outlet key={location.pathname} />` remount ([#358](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/358)). |
| **Notice + CTA** | [`InvalidPairLinkNotice`](../frontend-dapp/src/components/trade/InvalidPairLinkNotice.tsx) renders **`role="alert"`** with title **Invalid pair link**, quotes the bad segment (truncated when long), and a **Select a trading pair** button that scrolls to and focuses `#trade-pair-select`. |
| **Selector value** | `pairAddr` state stays **empty** until the user picks a pair or a valid deep link loads — `MenuSelect` must not display the raw invalid segment as the trigger label. |
| **Queries disabled** | Indexer / LCD pair queries use **`isTradePairRouteParam(pairAddr)`** (not bare `startsWith('terra1')`) so malformed `terra1…` prefixes do not fire API calls. |
| **Auto-pick guard** | Default redirect to the first factory pair runs only when **`pairAddr` is empty**, pairs exist, and the invalid-link notice is **not** showing (user must dismiss or pick via CTA). Bare `/trade` only — **not** when a valid `:pairAddr` segment is present ([GitLab **#357**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/357)); use [`shouldAutoPickDefaultTradePair`](../frontend-dapp/src/utils/tradePairRoute.ts). |
| **`data-testid`s** | `trade-invalid-pair-link-notice`, `trade-invalid-pair-link-value`, `trade-invalid-pair-link-cta`. |

Copy constants: [`tradeInvalidPairLinkCopy.ts`](../frontend-dapp/src/utils/tradeInvalidPairLinkCopy.ts). Regression: [`TradePage.test.tsx`](../frontend-dapp/src/pages/TradePage.test.tsx), [`tradePairRoute.test.ts`](../frontend-dapp/src/utils/__tests__/tradePairRoute.test.ts).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_TRADE_INVALID_PAIR_LINK.md`](../skills/AGENTS_FRONTEND_TRADE_INVALID_PAIR_LINK.md).

### Trade page — unknown pair deep link (valid `terra1…`, not on factory) {#trade-page-unknown-pair-link}

When `/trade/:pairAddr` passes [`isValidTerraAddress`](../frontend-dapp/src/utils/constants.ts) but the address is **not** in the paginated factory pair list ([`getUnknownTradePairRouteParam`](../frontend-dapp/src/utils/tradePairRoute.ts)), the UI must treat it as **pair not found** — not as a valid selection and not as an open-ended “not indexed yet” indexer miss ([GitLab **#175**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/175)). **No bech32 checksum** on URL segments (users may paste lowercase); factory membership is the gate before any indexer/LCD pair workspace fetch.

| Invariant | Meaning |
|-----------|---------|
| **Factory gate** | After `allPairs` succeeds, unknown detection compares `routePair` to `pairs[].contract_addr` only — do not set `pairAddr` until the segment is known. |
| **URL cleanup** | Same as [invalid pair deep link](#trade-page-invalid-pair-link): `navigate('/trade', { replace: true, state: { unknownPair } })` (location state survives Layout keyed-Outlet remount — [#358](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/358)). |
| **Notice + CTA** | [`PairNotFoundLinkNotice`](../frontend-dapp/src/components/trade/PairNotFoundLinkNotice.tsx) — title **Pair not found**, quotes the segment, CTA focuses `#trade-pair-select`. |
| **Queries disabled** | Indexer / LCD workspace queries stay off while `pairAddr` is empty (no 404 storm for regex-valid garbage). |
| **Auto-pick guard** | Blocked while `unknownPairNotice` is set (mirror invalid-link notice). Also blocked while any valid-format `:pairAddr` is in the URL — known-pair route sync owns that case ([GitLab **#357**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/357); [`shouldAutoPickDefaultTradePair`](../frontend-dapp/src/utils/tradePairRoute.ts)). |
| **vs #176** | Charset-invalid `terra1…` (e.g. `terra1damThat'scrazy`) uses **Invalid pair link** ([#176](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/176)), not this notice. |
| **vs #177** | Chart **RetryError** after **404** applies only when `pairAddr` is a **known** factory pair (indexer lag/outage), not unknown deep links. |
| **Workspace gate** | Book/chart/ticket mount only when [`shouldShowTradeWorkspace`](../frontend-dapp/src/utils/tradePairRoute.ts) is true — not while a valid-format deep link awaits factory resolution (`isPendingTradePairRouteResolution`) or while invalid/unknown notices are visible. Prevents an empty workspace when `getPair` has no data ([#175](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/175) follow-up). |
| **Indexer 404 fallback** | If `getPair` returns **404** for a route segment that is **not** on the factory list (race/stale state), sync **`PairNotFoundLinkNotice`** and clear `pairAddr` via `isIndexerPairNotFoundError` — do not treat as indexer outage. |
| **`data-testid`s** | `trade-pair-not-found-link-notice`, `trade-pair-not-found-link-value`, `trade-pair-not-found-link-cta`. |

Copy: [`tradeUnknownPairLinkCopy.ts`](../frontend-dapp/src/utils/tradeUnknownPairLinkCopy.ts). Regression: [`TradePage.test.tsx`](../frontend-dapp/src/pages/TradePage.test.tsx), [`tradePairRoute.test.ts`](../frontend-dapp/src/utils/__tests__/tradePairRoute.test.ts).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_TRADE_INVALID_PAIR_LINK.md`](../skills/AGENTS_FRONTEND_TRADE_INVALID_PAIR_LINK.md) (§ Unknown pair).

### Trade page — known pair deep link (non-default) {#trade-page-known-pair-deep-link}

When `/trade/:pairAddr` contains a **valid** `terra1…` segment that **is** on the factory list, the route→state sync effect must set `pairAddr` from the URL. The default-pick effect (`pairs[0]` + `navigate`) must **not** run in the same commit while `pairAddr` is still empty — otherwise deep links and pair-selector switches to non-default pairs snap back to the first factory pair ([GitLab **#357**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/357)).

| Invariant | Meaning |
|-----------|---------|
| **Route owns known pairs** | After `allPairs` succeeds, `isKnownFactoryTradePair(routePair, pairs)` → `setPairAddr(routePair)`; URL stays on the requested segment. |
| **Default-pick scope** | [`shouldAutoPickDefaultTradePair`](../frontend-dapp/src/utils/tradePairRoute.ts) is true only for bare `/trade` (no valid `:pairAddr`, no invalid/unknown notice, not pending deep-link resolution). |
| **Pending deep link** | While factory list is loading and the URL has a valid-format segment, default-pick stays off (`isPendingTradePairRouteResolution`) so unknown-pair detection can run after LCD resolves ([#175](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/175)). |
| **Regression** | [`TradePage.test.tsx`](../frontend-dapp/src/pages/TradePage.test.tsx) (`keeps non-default deep link after factory pairs resolve`), [`tradePairRoute.test.ts`](../frontend-dapp/src/utils/__tests__/tradePairRoute.test.ts). |

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_TRADE_INVALID_PAIR_LINK.md`](../skills/AGENTS_FRONTEND_TRADE_INVALID_PAIR_LINK.md) (§ Known pair deep link).

### Limit place — Bid / Ask side control (trade + limits page) {#limit-place-bid-ask-side}

On-chain semantics are unchanged: **Bid escrows token1; Ask escrows token0** (pair asset ordering — see [`pair.ts`](../frontend-dapp/src/services/terraclassic/pair.ts) and contract docs).

| Invariant | Meaning |
|-----------|---------|
| **Control type** | Limit **side** is a WAI-ARIA **`radiogroup`** with two **`role="radio"`** `<button type="button">` controls (`side-control` + `side-buy-*` / `side-sell-*` semantic fills, [GitLab **#563**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/563)), not native `<input type="radio">`, so the active side updates in the same React commit as `onSideChange` without browser-native controlled-radio timing quirks ([GitLab **#153**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/153)). Limit vs Market tabs stay **`tab-glass*`**. |
| **Roving tabindex** | The selected side has **`tabIndex={0}`**; the other **`tabIndex={-1}`** (one tab stop for the group). **ArrowRight / ArrowDown** move selection and focus toward Ask; **ArrowLeft / ArrowUp** toward Bid; **End** selects Ask; **Home** selects Bid (from the Ask control). |
| **`data-testid`s** | **`{idPrefix}-side-radiogroup`**, **`{idPrefix}-side-bid`**, **`{idPrefix}-side-ask`**. **`/trade`** uses **`idPrefix="trade-ticket"`** ([`TradeOrderTicket.tsx`](../frontend-dapp/src/components/trade/TradeOrderTicket.tsx)); **`/limits`** uses **`idPrefix="limit-orders"`** ([`LimitOrdersPage.tsx`](../frontend-dapp/src/pages/LimitOrdersPage.tsx)). |
| **Button copy** | **`/trade`** and **`/limits`** both use **Buy {base}** / **Sell {base}** via [`tradeDirectionSideLabels.ts`](../frontend-dapp/src/utils/tradeDirectionSideLabels.ts) ([#412](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/412), [#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/489)). On `/trade`, **base** is the **displayed** base after pair invert ([#524](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/524)); `/limits` stays factory token0. Historical retail copy **Bid (escrow …)** / **Ask (escrow …)** on the place card is retired; **bid**/**ask** remain on-chain enums and order-book column titles only. |
| **Focus visibility** | Buttons use **`.side-control`**, which defines **`:focus-visible`** rings aligned with [Keyboard focus visibility (WCAG 2.4.7)](#keyboard-focus-visible-wcag-247). Do not drop the ring when applying green/red fills. |

**Implementation:** [`LimitOrderBidAskSideSelector.tsx`](../frontend-dapp/src/components/trade/LimitOrderBidAskSideSelector.tsx).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_LIMIT_ORDER_SIDE_SELECTOR.md`](../skills/AGENTS_FRONTEND_LIMIT_ORDER_SIDE_SELECTOR.md), [`skills/AGENTS_FRONTEND_TRADE_TICKET_HEADING.md`](../skills/AGENTS_FRONTEND_TRADE_TICKET_HEADING.md) (#563 colors).

### Limit place — escrow amount (headline USD, side switch) {#limit-place-escrow-amount}

Retail sizing for the **Amount** field on **`/trade`** and **`/limits`** ([GitLab **#155**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/155)):

| Invariant | Meaning |
|-----------|---------|
| **Headline USD (escrow)** | When the amount is non-empty, the ticket shows **Headline USD (escrow): ≈ $…** using the same tape headline + reference as the limit price USD anchor ([`escrowAmountUsdAnchorNotional`](../frontend-dapp/src/utils/limitOrderPriceReference.ts)): **Ask** escrows **token0** → `amount × tapeHeadlineUsd`; **Bid** escrows **token1** → `amount × (tapeHeadlineUsd / refToken1PerToken0)`. |
| **Missing headline / ref** | When headline `price` is absent or invalid, the line shows **—** (same coverage as “Headline-scaled USD” under the price field; pool-only ref without tape). |
| **Side switch clears manual amounts** | **Bid ↔ Ask** clears a **manually typed** amount so the field never keeps the prior token’s numeric string. |
| **MAX reapplies on side switch** | If the amount was set via **Max**, switching sides clears until the new escrow balance loads, then **re-applies** full balance for the new escrow token. |
| **`data-testid`s** | `limit-order-escrow-amount-input`, `limit-order-escrow-usd-notional` — scope Playwright to the limit form. |

Implementation: [`useLimitOrderForm.ts`](../frontend-dapp/src/hooks/useLimitOrderForm.ts) (`LimitEscrowAmountSource`, `onLimitAmountInputChange`, `onLimitAmountMax`, `resetLimitEscrowAmount`, `setLimitEscrowAmountFromDraft`, `setLimitEscrowAmountFromMaxReapply`), [`LimitOrderEscrowAmountField.tsx`](../frontend-dapp/src/components/trade/LimitOrderEscrowAmountField.tsx), [`TradeOrderTicket.tsx`](../frontend-dapp/src/components/trade/TradeOrderTicket.tsx), [`LimitOrdersPage.tsx`](../frontend-dapp/src/pages/LimitOrdersPage.tsx).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_LIMIT_ORDER_SIDE_SELECTOR.md`](../skills/AGENTS_FRONTEND_LIMIT_ORDER_SIDE_SELECTOR.md), [`skills/AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md`](../skills/AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md).

### Pool page — LP risk disclosure {#pool-lp-risk-disclosure}

When the **Provide Liquidity** panel is open on `/pool`, a short **impermanent loss** notice appears before amount inputs ([#366](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/366)):

| Invariant | Meaning |
|-----------|---------|
| **Copy** | States that LP value can **diverge** from simply holding the underlying assets when pool prices move — directional risk, not guaranteed yield. |
| **Placement** | Visible in the add-liquidity card (`data-testid="pool-il-risk-notice"`) without wallet connect. |
| **Docs link** | `Learn more` → this section (`#pool-lp-risk-disclosure`). |

**Code:** `frontend-dapp/src/pages/PoolPage.tsx` (`POOL_LP_RISK_DOC`).

Retail “how do I add LUNC?” is **not** this section. See [§ Retail LUNC liquidity how-to](#retail-lunc-liquidity-howto) and [`user-lunc-liquidity.md`](./user-lunc-liquidity.md).

### Retail LUNC liquidity how-to {#retail-lunc-liquidity-howto}

In-product opt-in guide so a new user can add (or correctly attempt) LUNC LP without opening GitLab ([#531](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/531)). Primary surface: `/pool#lp-howto`. Human backup: [`user-lunc-liquidity.md`](./user-lunc-liquidity.md). Agent playbook: [`skills/AGENTS_FRONTEND_POOL_LP_HOWTO.md`](../skills/AGENTS_FRONTEND_POOL_LP_HOWTO.md).

| Invariant | Meaning |
|-----------|---------|
| **H531-1** | Same-origin how-to on `/pool` names **Pool provide/withdraw** and optional **limit maker**. This file is not the only guide. |
| **H531-2** | States pools use wrapped LUNC; pick native LUNC on Add to auto-wrap **or** `/wrap` first; bank LUNC still required for **gas**. |
| **H531-3** | Retail Add is **one-sided** (token + pair + amount). Two-sided is **Advanced** (empty-pool bootstrap). Off-ratio Advanced provide still **donates** excess; retail zap does not. |
| **H531-4** | States there is **no** incentive program currently. No APR / points / farm UI. |
| **H531-5** | Withdraw via `/pool`; LP tokens are the share. |
| **H531-6** | Limits (if mentioned) are maker escrow, **not** LP shares; in-app `/trade` or `/limits` only. |
| **H531-7** | No always-on lecture on Swap / Trade / Limits / Wrap. `/pool` how-to (hint **and** `<details>`) is dismissible; `#lp-howto` restores ([#547](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/547)). |
| **H531-8** | How-to lives on `/pool`, so tablet **More** and phone bottom-nav Pool still reach it. |
| **H531-9** | Does not replace IL, pause, blacklist, gas, ratio warning, clickwrap, or NFA. |
| **H531-10** | Static React copy (no `innerHTML` of indexer/wallet strings). In-app links only (Pool / Wrap / Trade / Limits / Create Pair). Mentions **Create Pair** LUNC creation fee; creating a pair is not required to LP an existing pool. Unwrap is not free. |

**Code:** [`poolLpHowtoCopy.ts`](../frontend-dapp/src/utils/poolLpHowtoCopy.ts), [`PoolLpHowto.tsx`](../frontend-dapp/src/components/pool/PoolLpHowto.tsx), [`poolLpHowto.ts`](../frontend-dapp/src/utils/poolLpHowto.ts) (`cl8y-dex-pool-lp-howto-section-dismissed`), footer / Portfolio links to `/pool#lp-howto`.

**Verify:** `make verify-issue-531`. One-sided default: `make verify-issue-533` / [`AGENTS_FRONTEND_POOL_ONE_SIDED.md`](../skills/AGENTS_FRONTEND_POOL_ONE_SIDED.md).

### One-sided pool add / withdraw (Z533) {#pool-one-sided-liquidity}

Retail `/pool` default is **one-sided** zap add/withdraw ([#533](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/533)). Two-sided provide/withdraw stays under **Advanced** (empty-pool first deposit / power users).

| Invariant | Meaning |
|-----------|---------|
| **Z533-1** | Retail add: Token (wallet `> 0`), Pair (factory), Amount. No wrap checkbox, no second amount, no on-card slippage chips. |
| **Z533-2** | Retail withdraw: LP (wallet LP `> 0`), token to receive, amount. No “receive wrapped” checkbox. |
| **Z533-3** | Native `uluna` / `uusd` wrap or unwrap automatically. Pools still hold CW20 only. |
| **Z533-4** | Zap-in targets the **post-swap** pool ratio. Retail must not `provide_liquidity` off-ratio (no silent donate). |
| **Z533-5** | Empty pool: one-sided disabled. First deposit stays two-sided Advanced (`MINIMUM_LIQUIDITY`). |
| **Z533-6** | Factory pairs only. Pool-only zap swap (`poolOnlyHybridParams`). Same quote snapshot on submit as Swap (#356). |
| **Z533-7** | Slippage on every leg (`min_return` / `slippage_tolerance` / `min_assets`). Do not pass `slippage_tolerance: null`. Default 5% from Settings (#497). |
| **Z533-8** | Unwrap **only** the zap-out amount. Never unwrap the rest of the wallet’s cLUNC/cUSTC. |
| **Z533-9** | Gas/Max cover the full wrap + swap + provide (or withdraw + swap + unwrap) sequence. Keep pause, blacklist, mapper pause, treasury match, IL, clickwrap, NFA, Expert Mode, pre-sign. |
| **Z533-10** | LP amounts use CW20 decimals **18**. No APR / farm chrome. No new pair/router `Zap` execute unless LocalTerra rehearsal proves multi-msg cannot work. |

**Code:** [`oneSidedLiquidity.ts`](../frontend-dapp/src/utils/oneSidedLiquidity.ts), [`oneSidedLiquidityTx.ts`](../frontend-dapp/src/utils/oneSidedLiquidityTx.ts), [`OneSidedAddCard.tsx`](../frontend-dapp/src/components/pool/OneSidedAddCard.tsx), [`OneSidedWithdrawCard.tsx`](../frontend-dapp/src/components/pool/OneSidedWithdrawCard.tsx).

**Verify:** `make verify-issue-533`. Playwright smoke (5 workers): `frontend-dapp/e2e/pool-one-sided-533.spec.ts`. LocalTerra tx P4–P8 (1 worker): `frontend-dapp/e2e/pool-one-sided-533-tx.spec.ts` via `make verify-issue-539` / `sg docker -c 'CI=1 make test-e2e'` (wrap-mapper split-fee instantiate is [#539](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/539)). Zap execution floors: `make verify-issue-559` / [`AGENTS_FRONTEND_POOL_ZAP_FLOORS.md`](../skills/AGENTS_FRONTEND_POOL_ZAP_FLOORS.md).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_POOL_ONE_SIDED.md`](../skills/AGENTS_FRONTEND_POOL_ONE_SIDED.md).

### One-sided zap execution floors (Z559) {#pool-one-sided-zap-floors}

Retail zap multi-msg legs are sized to the **previous leg’s floor**, not the optimistic quote ([#559](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/559)). A swap fill in `(min_return, quote)` must succeed provide (or fail `min_return` / `slippage_tolerance`) — never CW20 `Overflow: Cannot Sub`.

| Invariant | Meaning |
|-----------|---------|
| **Z559-1** | Execution amounts follow floors (`min_return` / `min_assets`); quotes may be optimistic. Zap-in `provideAsk ≤ swapMinReturn`. Do not `TransferFrom` the quoted ask. |
| **Z559-2** | Zap-in provide is ratio-trimmed to conservative post-swap reserves (worse fill → higher ask reserve). Leftover stays in the wallet; no silent pre-existing ask spend (**Z533-4**). |
| **Z559-3** | Zap-out `swapAmount ≤ min_assets[sold]`. Unwrap send ≤ `min(wanted withdrawn, min_assets[wanted]) + swapMinReturn` (**Z533-8**). |
| **Z559-4** | Pre-sign min-swap is human token units, not raw uints. Conservative LP dust → `Amount too small`. Empty pool still `Empty pool. Use Advanced.` |

**Code:** [`conservativeZapInProvide`](../frontend-dapp/src/utils/oneSidedLiquidity.ts) / [`conservativeZapOutExecution`](../frontend-dapp/src/utils/oneSidedLiquidity.ts), [`quoteOneSidedAdd`](../frontend-dapp/src/utils/oneSidedLiquidityQuote.ts), [`formatHumanMinSwapLine`](../frontend-dapp/src/utils/oneSidedLiquidityCopy.ts).

**Verify:** `make verify-issue-559`. Units T-Z1–T-Z12. LocalTerra P9: `frontend-dapp/e2e/pool-one-sided-533-tx.spec.ts`. `make verify-issue-533` must still pass.

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_POOL_ZAP_FLOORS.md`](../skills/AGENTS_FRONTEND_POOL_ZAP_FLOORS.md).

### Pool page — provide liquidity (UI invariants)

The **Provide Liquidity** card mirrors on-chain `provide_liquidity` math for the **Estimated LP** line (see `docs/contracts-terraclassic.md` and `smartcontracts/contracts/pair/src/contract.rs`):

- **First deposit** (both reserves `0`): user LP ≈ `sqrt(amount_a × amount_b) − 1000` LP smallest units (LP CW20 `decimals` = **18**; 1000 = `MINIMUM_LIQUIDITY` locked forever — see [issue #124](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/124)).
- **Later deposits:** user LP = `min(amount_a × total_share / reserve_a, amount_b × total_share / reserve_b)` (integer floor on each term, then `min`).

**Wallet balance queries** use the same React Query key prefix as the Swap page: `['tokenBalance', address, <asset id>]`, where the asset id is the CW20 `terra1…` address or, when “Use native (auto-wrap)” is checked, the bank **denom** string (e.g. `uluna`), via `getTokenBalance` in `src/services/terraclassic/queries.ts`.

**Limit / ladder CW20 escrow** — retail limit place and ladder gates use [`useLimitOrderEscrowBalance`](../frontend-dapp/src/hooks/useLimitOrderEscrowBalance.ts) (same key shape as swap). [`useTokenBalance`](../frontend-dapp/src/hooks/useTokenBalance.ts) re-exports that hook for generic call sites; the module file must exist or Vite fails on routes that import ladder gates ([GitLab **#231**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/231)). Agent skill: [`skills/AGENTS_LIMIT_ORDER_BATCH_LADDER.md`](../skills/AGENTS_LIMIT_ORDER_BATCH_LADDER.md).

**CW20/CW20 add liquidity — native LUNC vs three fees ([GitLab #147](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/147)):** when neither asset uses “Use native (auto-wrap)”, `provideLiquidity` runs **three** sequential transactions (`increase_allowance` on token A and B, then `provide_liquidity`). Before the first broadcast, bank **uluna** must cover **`estimateProvideLiquidityCw20SequenceUlunaFeesTotal()`** (same gas limits and `effectiveGasPriceUluna()` as [`transactions.ts`](../frontend-dapp/src/services/terraclassic/transactions.ts)); gate: [`provideLiquidityNativeGasBalanceGate.ts`](../frontend-dapp/src/utils/provideLiquidityNativeGasBalanceGate.ts), [`PoolPage.tsx`](../frontend-dapp/src/pages/PoolPage.tsx). The native multi-msg path uses `executeTerraContractMulti` (single combined fee). **Rollback:** if `provide_liquidity` fails after allowances were raised, cleanup uses **`executeTerraContractMulti`** with two CW20 **`decrease_allowance`** messages (**one** Keplr prompt / **one** fee), not two separate txs — avoiding extra gas loss from duplicate rollback broadcasts ([GitLab #147](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/147)).

**Ratio warning:** if the two typed amounts are not in the current pool price ratio, the contract still executes, but the **smaller** LP term sets the mint; the excess on the other side is effectively donated to the pool (same as Astroport/TerraSwap behavior). Warning element: `data-testid="pool-provide-ratio-warning"`.

**Auto-fill counterpart ([#480](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/480)):** on a non-empty pool, editing Asset A or B auto-fills the other side from `floor(edited × reserve_other / reserve_edited)` when the counterpart field is empty. **Max** and **50%** always force-sync the counterpart (`forceSync: true`). Empty pool (both reserves `0`): no auto-fill. After auto-fill, if the user edits the filled side to a different value, the other side is left unchanged so the ratio warning can appear. Native-wrap paths use **net** post-tax amounts for ratio math (`provideRawAdd*` semantics); see [`poolProvideCounterpart.ts`](../frontend-dapp/src/utils/poolProvideCounterpart.ts) and [`skills/AGENTS_FRONTEND_POOL_PROVIDE_WITHDRAW_PREVIEW.md`](../skills/AGENTS_FRONTEND_POOL_PROVIDE_WITHDRAW_PREVIEW.md).

**Withdraw receive preview ([#480](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/480)):** when an LP amount is entered, the withdraw panel shows **expected** underlying tokens at 0% slippage (`pool-withdraw-estimated-receive`) and **minimum** after the selected slippage tolerance (`pool-withdraw-minimum-receive`). Labels use wrapped token symbols when “Receive as wrapped tokens” is checked; otherwise native symbols when unwrap is available.

### Pool page — pre-sign confirmation summary (SEC-I05) {#pool-page-pre-sign-summary}

Before the wallet extension opens on **`/pool`**, provide and withdraw show a **compact labeled summary card** so traders catch phishing or wrong-network mistakes ([#462](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/462) / SEC-I05 F-03):

| Invariant | Meaning |
|-----------|---------|
| **Labeled fields** | Action (`Provide Liquidity` / `Withdraw Liquidity`), pair symbols, consolidated amount line(s), and chain full name. |
| **Form snapshot** | Amounts match the typed values on the provide/withdraw form at submit time (not a separate quote path). Provide: both asset amounts. Withdraw: LP amount plus expected underlying token amounts (pro-rata, 0% slippage) when pool data is loaded. |
| **Chain name** | From [`getNetworkBadgeCopy()`](../frontend-dapp/src/utils/networkDisplay.ts) — must align with the network badge / env strip. |
| **`data-testid`s** | Provide panel: `pool-provide-pre-submit-summary` with `-action`, `-pair`, `-amount`, `-chain`; withdraw panel: `pool-withdraw-pre-submit-summary` with the same suffixes. |
| **Compact layout** | Four rows only (no swap-style intro paragraph) — token inputs above already show deposit amounts; the card repeats security anchors. |

Implementation: [`PoolPreSubmitSummary.tsx`](../frontend-dapp/src/components/pool/PoolPreSubmitSummary.tsx); wired in [`OneSidedAddCard`](../frontend-dapp/src/components/pool/OneSidedAddCard.tsx) / [`OneSidedWithdrawCard`](../frontend-dapp/src/components/pool/OneSidedWithdrawCard.tsx) and in [`PoolAdvancedManage.tsx`](../frontend-dapp/src/components/pool/PoolAdvancedManage.tsx) (two-sided Advanced, opened from the table **Manage** expand — [#547](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/547)).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_POOL_SIGNING_CONFIRMATION.md`](../skills/AGENTS_FRONTEND_POOL_SIGNING_CONFIRMATION.md), [`skills/AGENTS_FRONTEND_POOL_PROVIDE_WITHDRAW_PREVIEW.md`](../skills/AGENTS_FRONTEND_POOL_PROVIDE_WITHDRAW_PREVIEW.md).

E2E for pool flows runs with the dev-wallet fixture; Playwright worker count is pinned in [`.cursor/rules/playwright-workers.mdc`](../.cursor/rules/playwright-workers.mdc) (5 workers) to keep the Vite `webServer` stable.

| GitLab | Role |
|--------|------|
| [#109](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/109) | Add-LP balances, Max / 50%, LP estimate, tests |
| [#147](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/147) | CW20/CW20 add LP: native LUNC preflight for three sequential txs (`provideLiquidityNativeGasBalanceGate.ts`) |
| [#112](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/112) | Pool list: indexer vs factory (badges; Router-known filter removed in #547) |
| [#547](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/547) | `/pool` sortable table, catalog default, Charts deep links |
| [#462](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/462) | Pre-sign summary card for provide/withdraw (SEC-I05 F-03) |
| [#480](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/480) | Provide auto-fill + withdraw receive preview |

### Liquidity pools list (indexer vs factory) {#liquidity-pools-list-indexer-vs-factory}

The pool list (`/pool`) is a **sortable table** sourced from indexer `GET /api/v1/pairs` ([#547](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/547), playbook [`AGENTS_FRONTEND_POOL_TABLE.md`](../skills/AGENTS_FRONTEND_POOL_TABLE.md)). That order is **not** the on-chain factory’s `pairs` cursor order. Page chrome is **search + live status + one-sided CTAs + table** — no “Liquidity Pools” title, list-source essay, or indexer/factory counts ([#489](#retail-copy-cognitive-load)).

**Invariants (dApp, P547-1–P547-10):**

| ID | Meaning |
|----|---------|
| **P547-1** | Primary list is a `<table>` (`data-testid="pool-pairs-table"`), not stacked `PoolCard`. |
| **P547-2** | Sortable headers: label + caret next to the text; `aria-sort` on the active column. No Sort/Order dropdowns. |
| **P547-3** | Default (empty search, no column click) is **catalog rank**: fetch `limit=500` `sort=volume_24h&order=desc`, client `sortIndexerPairsByCatalog`, paginate 20. UST1-hub economic pairs first, gems last on LocalTerra (**P534-1–P534-4**). Production omits gems entirely ([#562](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/562) **P562-3**). |
| **P547-4** | Column sort uses indexer keys (`symbol`, `volume_24h`, `fee`, `created`) with **no** catalog overlay. Vol uses `formatQuoteVolume24h`. Visible header stays **Vol**; `title` discloses trailing 24h ([#576](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/576) **U6**). Created **cells** show `—` because `GET /api/v1/pairs` list JSON has no timestamp (sort still hits indexer `created`). |
| **P547-5** | Every row Charts control is a same-origin `Link` to `/charts/:pairAddr` via `chartsPairHref`. Invalid bech32 / `javascript:` / HTML → no navigation. |
| **P547-6** | No Router-known checkbox (`pool-filter-router` removed). Missing factory membership is a compact **Factory** / **Indexer** mark, not a list filter. |
| **P547-7** | How-to hint **and** `<details>` dismiss together; `#lp-howto` restores (**H531-7**). |
| **P547-8** | No header eligibility essay (`pool-fee-discount-eligibility-note`). Search, one-sided cards, indexer outage banner, and registry-outage warning stay. |
| **P547-9** | Default paint does **not** `getPool` / `getPairFeeConfig` per row (A8). Advanced two-sided + I14 LCD fee chrome mount on **Manage** expand (`PoolAdvancedManage`). |
| **P547-10** | `#489`: no always-on indexer-vs-factory lectures. Engineering notes stay in this file. |

**Factory set (still O(1) badges):** One React Query for `getAllPairsPaginated(FACTORY_PAIRS_MAX_FOR_POOL_LIST)` (stale time 60s), query key `factoryPairsForPoolList`. Missing factory membership ≠ “safe to hide.” Cap: `FACTORY_PAIRS_MAX_FOR_POOL_LIST` in `pairListBadges.ts`.

**Charts deep link:** `/charts` and `/charts/:pairAddr`. Invalid param: stay on Charts, short notice, no XSS. Unknown valid `terra1`: “Pair not found,” no crash. Helpers: [`chartsPairRoute.ts`](../frontend-dapp/src/utils/chartsPairRoute.ts).

**Code:** `frontend-dapp/src/pages/PoolPage.tsx`, `PoolPairsTable.tsx`, `PoolAdvancedManage.tsx`, `poolListQuery.ts`. Issues: [glab#547](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/547), [glab#112](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/112).

**Verify:** `make verify-issue-547`.

**Agent workflow (optional):** For reviewable follow-up PRs or merge-ready checks in Cursor, use the **split to PRs** and **babysit** skills from your [Cursor skills](https://docs.cursor.com/context/skills) path (e.g. `~/.cursor/skills-cursor/` on a developer machine).

## Fee Discount Service

The `feeDiscount.ts` service in `src/services/` handles all interactions with the fee-discount contract:

**Queries:**
- `getConfig()` — returns governance address and CL8Y token address
- `getDiscount(trader)` — returns the trader's current discount in basis points
- `getTier(tierId)` — returns a single tier's details (`min_cl8y_balance`, discount bps); ladder in [`docs/reference/fee-discount-tiers.md`](reference/fee-discount-tiers.md)
- `getTiers()` — returns all configured tiers
- `getRegistration(wallet)` — returns the wallet's current tier registration (or null)
- `isTrustedRouter(router)` — checks if an address is a trusted router

**Executions:**
- `register(tierId)` — self-register for a tier (EOA only); gas envelope **`REGISTER_FEE_DISCOUNT_GAS_LIMIT` (300k)** in [`terraGas.ts`](../frontend-dapp/src/services/terraclassic/terraGas.ts) ([#384](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/384))
- `deregister()` — remove own registration; gas envelope **`DEREGISTER_FEE_DISCOUNT_GAS_LIMIT` (250k)** ([#384](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/384))

### Swap Page Integration

The Swap page displays the effective fee after discount **only when the selected pair’s `DISCOUNT_REGISTRY` is set and matches `VITE_FEE_DISCOUNT_ADDRESS`** (invariant **I14**, [GitLab #537](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/537)). When a connected wallet has a registered tier **and** that pair is wired, the UI shows:
- The base pair fee (e.g., 0.30%)
- The discount percentage from the trader's tier
- The effective fee after discount (e.g., 0.15% for a 50% discount)

Unwired pairs (registry `None`, e.g. economic pairs before the #535 factory sweep) show plain `fee_bps` with **no** strikethrough and hide the Hold-CL8Y CTA for that pair — HybridSimulation with `trader` already quotes the full fee. Probe: [`pairDiscountRegistry.ts`](../frontend-dapp/src/utils/pairDiscountRegistry.ts) + [`getPairDiscountRegistry`](../frontend-dapp/src/services/terraclassic/pairDiscountRegistry.ts). Agent playbook: [`skills/AGENTS_FRONTEND_PAIR_FEE_DISCOUNT.md`](../skills/AGENTS_FRONTEND_PAIR_FEE_DISCOUNT.md).

**Registry outage warning (GitLab [#374](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/374)):** When LCD `get_registration` / `get_discount` fails or the indexer reports `fee_discount_registry_ok: false` (`GET /api/v1/health/fee-discount`), registered traders see a non-blocking amber banner (`data-testid="swap-fee-discount-registry-warning"`) — swap submit stays enabled; on-chain execution may still charge full pair fee. Unregistered wallets with healthy LCD reads keep the **Hold CL8Y & register…** CTA instead **when the pair is wired** (I14). Logic: [`feeDiscountRegistryWarning.ts`](../frontend-dapp/src/utils/feeDiscountRegistryWarning.ts) + [`useFeeDiscountRegistryStatus`](../frontend-dapp/src/hooks/useFeeDiscountRegistryStatus.ts). Agent playbook: [`skills/AGENTS_FEE_DISCOUNT_TIERS.md`](../skills/AGENTS_FEE_DISCOUNT_TIERS.md) § Registry outage observability.

### Pair fee-tier chrome vs on-chain registry (GitLab [#537](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/537)) {#pair-fee-tier-chrome}

`getTraderDiscount` always queries `VITE_FEE_DISCOUNT_ADDRESS`. On-chain fees use the **pair’s** `DISCOUNT_REGISTRY`. When that is `None`, execute charges full `fee_bps` (maker place `floor(fee_bps/2)`). The dApp must not strikethrough a phantom discount.

| ID | Rule |
|----|------|
| **F537-1** | Advertise CL8Y discount only when pair `DISCOUNT_REGISTRY` is set and equals `VITE_FEE_DISCOUNT_ADDRESS`. |
| **F537-2** | Unwired pair: show full `fee_bps` / full maker place fee; no strikethrough. |
| **F537-3** | Do not invent a client-side discount the pair will not apply. |
| **F537-4** | Prefer pair `GetDiscountRegistry` (wasm **1.14.0+**, [#538](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/538)); LCD raw key `discount_registry` is fallback for 1.13.x or LCDs that reject the query. Probe failure → fail-closed (full fee chrome). |
| **F537-5** | Hide pair-scoped Hold-CL8Y / “not registered” CTA on unwired pairs. Global registry-outage banner is unchanged. |
| **F537-6** | HybridSimulation / route-solve with `trader` remains execute-aligned; this issue is fee **chrome**, not quote math. |

Canonical invariant **I14**: [`docs/reference/fee-discount-tiers.md`](reference/fee-discount-tiers.md). Agent playbook: [`skills/AGENTS_FRONTEND_PAIR_FEE_DISCOUNT.md`](../skills/AGENTS_FRONTEND_PAIR_FEE_DISCOUNT.md). Verify: `make verify-issue-537`. Post-migrate inherit + smart-query-first: `make verify-issue-538` ([#538](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/538), **F538-1–F538-3**).

### Post-migrate inherit + smart-query-first (GitLab [#538](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/538)) {#post-migrate-inherit-538}

| ID | Rule |
|----|------|
| **F538-1** | After factory 1.8.0 migrate, `config.discount_registry` is the fee-discount contract (All/Batch or `UpdateConfig`). Columbus-5 ops already set this. |
| **F538-2** | New LocalTerra `create_pair` `GetDiscountRegistry` matches the factory pointer with **no** follow-up `SetDiscountRegistry`. |
| **F538-3** | Swap / Pool / Trade `getPairDiscountRegistry` prefers `GetDiscountRegistry`; raw LCD is fallback. Fail-closed chrome (**F537-2**) is unchanged. |

### Pool page fee-discount UX (GitLab [#476](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/476)) {#pool-page-fee-discount-ux}

Pool cards reuse the same fee-discount status hook as Swap:

| Signal | UI |
|--------|-----|
| Connected **unregistered** + healthy registry + **pair wired** (I14) | Fee badge shows base pair fee + **· not registered**; CTA `data-testid="pool-fee-discount-unregistered-cta"` → `/tiers` |
| Connected **registered** + `discount_bps > 0` + **pair wired** | [`FeeDisplay`](../frontend-dapp/src/components/ui/FeeDisplay.tsx) strikethrough base + effective % (unchanged math) |
| Pair `DISCOUNT_REGISTRY` unset or ≠ `VITE_FEE_DISCOUNT_ADDRESS` ([#537](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/537)) | Full `fee_bps`; **no** strikethrough; hide pair CTA / “not registered” (do not advertise a discount the pair will not apply) |
| Registered + registry unreachable | Non-blocking amber banner `data-testid="pool-fee-discount-registry-warning"` (same copy as Swap); provide/withdraw stay enabled |
| `VITE_FEE_DISCOUNT_ADDRESS` empty | No discount queries, no CTA, no outage banner |

**Eligibility (invariant I12):** Discount applies only after the wallet **holds the configured `cl8y_token` CW20** (env `VITE_CL8Y_TOKEN_ADDRESS` / fee-discount `config.cl8y_token`) **and** successfully **`Register`s** a self-serve tier on `/tiers`. Holding alone, or a differently named/bridged asset (e.g. reporter “CL8Y-cb”), yields `discount_bps: 0` and a plain base fee — that is expected, not a missing on-chain feature. Shared copy: [`feeDiscountUiCopy.ts`](../frontend-dapp/src/utils/feeDiscountUiCopy.ts). Pool header note: `data-testid="pool-fee-discount-eligibility-note"`.

**CL8Y decimals:** [`tokenRegistry.ts`](../frontend-dapp/src/utils/tokenRegistry.ts) lists CL8Y at **18** decimals so `/tiers` Hold labels match `min_cl8y_balance` wei (LocalTerra TCL8Y is also 18 — [#383](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/383)).

**Tests:** [`PoolPage.feeDiscountRegistryBanner.test.tsx`](../frontend-dapp/src/pages/PoolPage.feeDiscountRegistryBanner.test.tsx), [`SwapPage.feeDiscountRegistryBanner.test.tsx`](../frontend-dapp/src/pages/SwapPage.feeDiscountRegistryBanner.test.tsx), [`pairDiscountRegistry.test.ts`](../frontend-dapp/src/utils/__tests__/pairDiscountRegistry.test.ts), [`useLimitOrderMakerFeeRates.test.tsx`](../frontend-dapp/src/hooks/__tests__/useLimitOrderMakerFeeRates.test.tsx). QA: [`QA_TEMPLATE.md`](../QA_TEMPLATE.md) § 3.1.6–3.1.10. Regression: `make verify-issue-537` / `make verify-issue-538`.

**Expected slippage, Expert Mode & max spread (GitLab [#134](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/134), [#293](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/293)):** When the indexer returns `slippage_percent` on `route/solve`, the trade summary shows **Expected slippage** — symmetric deviation vs fair cross-rate token prices (`spot_amount_out`). The dApp prefers wallet `return_amount` vs spot when both are present ([`swapRouteSlippage.ts`](../frontend-dapp/src/utils/swapRouteSlippage.ts)). **Expert Mode** (Settings checkbox, default **off**, persisted in `localStorage`) blocks submit when expected slippage **> 30%** with **Slippage is too high** and an **Enable Expert Mode** affordance that opens a warning modal ([`ExpertModeModal.tsx`](../frontend-dapp/src/components/swap/ExpertModeModal.tsx)). **≥ 99%** always shows an extreme-slippage alert, even with Expert Mode enabled. Multihop and indexer quotes also run **per-hop pair simulation** preflight (factory resolve + `simulation` / `hybrid_simulation`) so hop spread is visible as secondary context and submit is disabled when any hop would exceed the user’s **Slippage tolerance** (`max_spread`). Failed txs that still surface `Max spread assertion` from the chain are mapped to short retail copy in [`humanizeTerraTxError.ts`](../frontend-dapp/src/utils/humanizeTerraTxError.ts) via [`terraBroadcast.ts`](../frontend-dapp/src/services/terraclassic/terraBroadcast.ts) and `TxResultAlert`. **Pool-only CW20 swap** gas uses the buffered one-hop router envelope (**830k**), not legacy **600k**, so wallet fee displays (~23 vs ~36 LUNC) stay aligned with on-chain headroom; LocalTerra post-sign guards reject fee/gas rewrites below **95%** of the dApp envelope ([#127](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/127)). Full invariants: [`docs/swap-max-spread-ux.md`](./swap-max-spread-ux.md).

**Swap Settings — retail vs Advanced (GitLab [#413](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/413)):** Opening **Settings** on `/` shows **retail** controls only: slippage presets/custom, **transaction deadline** (5/10/20/30m presets + custom, 30s–60m clamp via `useDexStore`), and **Expert Mode** (`data-testid="swap-expert-mode-toggle"`). Integrator tooling — direct-pair **limit book leg** (hybrid `book_input` / `max_maker_fills`, only when `isDirect && !isWrapOrUnwrap`) and **Indexer route check** (BFS hop dump) — live behind a collapsed-by-default **Advanced** disclosure ([`SwapAdvancedSettings.tsx`](../frontend-dapp/src/components/swap/SwapAdvancedSettings.tsx), `data-testid="swap-advanced-settings-toggle"`). Power users who expand Advanced persist that state in `localStorage` ([`swapSettingsAdvanced.ts`](../frontend-dapp/src/utils/swapSettingsAdvanced.ts)). Agent checklist: [`skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md`](../skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md) § Settings progressive disclosure.

**Route preview (GitLab [#158](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/158)):** The **Route** line lives in the same trade-summary card as **Price impact** and **Min received** (no separate “quote source” strip, no paired `Route (indexer)` / `Route` labels). The displayed token path follows the same precedence as submit: indexer-shaped `router_operations` when present, otherwise the client BFS route, native wrap path, or a direct `from → to`. Code: [`swapRouteDisplay.ts`](../frontend-dapp/src/utils/swapRouteDisplay.ts). Agent checklist: [`skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md`](../skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md).

**Submit–quote alignment (GitLab [#356](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/356), [#360](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/360), [#418](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/418), [#484](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/484), [#496](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/496))** {#submit-quote-alignment--calculating-ux}

Sim queries debounce pay amount and hybrid book leg (**350ms**, [#346](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/346)). Submit must use the **same** debounced pay raw, book leg, and max-makers snapshot as the displayed quote — not live typed values while debounce, placeholder, or refetch is in flight. [`useSubmitAlignedSimQuote`](../frontend-dapp/src/hooks/useSubmitAlignedSimQuote.ts) bundles `submitPayRaw`, `minReceived`, `simData`, and `snapshottedHybrid` via [`buildSubmitAlignedSimPayload`](../frontend-dapp/src/utils/quoteDebounce.ts) for Swap and Trade market `swapMutation`; [`isSubmitQuoteStale`](../frontend-dapp/src/utils/quoteDebounce.ts) gates the submit button. Hybrid book-leg splits for sim/submit use debounced pay total, debounced book leg, and debounced max makers. **Default hybrid quote path ([#501](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/501), always-on [#596](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/596)):** Swap and Trade market use [`quoteCw20ViaRouteSolve`](../frontend-dapp/src/utils/cw20RouteSolveQuote.ts) (`GET /route/solve`) and submit solver `hybrid` from `indexerOperations`. There is **no** retail hybrid opt-out. **Advanced manual book leg ([#418](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/418)):** [`quoteDirectHybridSwap`](../frontend-dapp/src/utils/directHybridQuote.ts) aligns indexer `POST` + LCD quotes with submit — no pool-only receive line while a manual book leg is configured.

**Calculating… / slow multihop ([#484](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/484), [#485](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/485), [#496](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/496)):** Periodic sim refresh uses [`simQuoteRefetchInterval`](../frontend-dapp/src/utils/quoteDebounce.ts) so React Query does **not** cancel/restart an in-flight quote every 10s (default `cancelRefetch`). Swap **You Receive** and Trade **Expected receive** use [`shouldShowSimReceiveCalculating`](../frontend-dapp/src/utils/quoteDebounce.ts): **same-input** background refetch keeps the prior amount (`isFetching && !hasSettledQuote` only — #484); **pay amount/token (query key) change** shows Calculating/Quoting while typed pay ≠ debounced key or `isPlaceholderData` from `keepPreviousData` (#496). Pass `hasSettledQuote = !!data && !isPlaceholderData`. Indexer `GET`/`POST` `/route/solve` use a **45s** client timeout (`INDEXER_ROUTE_SOLVE_TIMEOUT_MS`) and accept an AbortSignal from the sim `queryFn`. **#485:** while the first CW20 multihop quote is in flight, Swap polls `GET /api/v1/route/solve/progress` (~1 Hz) and may replace static Calculating with indexer `label` (e.g. Searching 2 of 5 paths…) after ~500ms; `aria-live="polite"` for screen readers. Agent checklists: [`skills/AGENTS_FRONTEND_SWAP_QUOTE_REFETCH.md`](../skills/AGENTS_FRONTEND_SWAP_QUOTE_REFETCH.md), [`skills/AGENTS_INDEXER_ROUTE_SOLVE_PROGRESS.md`](../skills/AGENTS_INDEXER_ROUTE_SOLVE_PROGRESS.md).

| Invariant | Meaning |
|-----------|---------|
| **Single submit snapshot** | Pay raw, min received, indexer ops, hybrid params, and route display refer to one settled sim result. |
| **No live/debounced skew** | `swapMutation` reads `submitPayRaw` (debounced), not live `inputAmount` / `marketAmountHuman`, when min received comes from `simQuery.data`; hybrid `book_input` / `max_maker_fills` come from the same debounced snapshot. |
| **Stale submit blocked** | Submit disabled when typed raw ≠ debounced key, live book leg ≠ debounced book leg, live max makers ≠ snapshotted max makers, `isPlaceholderData`, or `simQuery.isFetching` for the active debounced key. |
| **No refetch pile-up (#484)** | `refetchInterval` skips while `fetchStatus === 'fetching'`; route-solve HTTP budget allows slow distant-pair solves to finish. |
| **Receive stale-OK same inputs (#484)** | Prior receive amount stays visible during background refetch of the **same** sim key; Calculating is for first load / no settled quote for that key. |
| **Receive loading on pay change (#496)** | Changing pay amount or pay token clears/loads You Receive (and Trade Expected receive) until a quote for the new inputs settles — `keepPreviousData` must not leave the old estimate looking current. |
| **Live search progress (#485)** | While the first distant-pair `getRouteSolve` is in flight, Swap may show indexer progress (`Searching x of y…`) after ~500ms via [`useRouteSolveProgress`](../frontend-dapp/src/hooks/useRouteSolveProgress.ts) — display-only; does not change submit/receive gates. Trade market stays on pair-scoped **Quoting…**. See [`skills/AGENTS_INDEXER_ROUTE_SOLVE_PROGRESS.md`](../skills/AGENTS_INDEXER_ROUTE_SOLVE_PROGRESS.md). |

### Swap page — pre-sign confirmation summary (SEC-D11) {#swap-page-pre-sign-summary}

Before the wallet extension opens on **`/`** / **`/swap`**, a labeled summary card helps traders catch phishing or wrong-network mistakes ([#409](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/409) / SEC-D11):

| Invariant | Meaning |
|-----------|---------|
| **Labeled fields** | Action (`Swap`), pair symbols, factory-sourced pair contract address(es) when known ([#449](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/449)), pay amount, estimated receive, max spread (`slippageTolerance%`), min return after slippage floor, and chain full name. |
| **Submit snapshot** | Amounts and min return match [`useSubmitAlignedSimQuote`](../frontend-dapp/src/hooks/useSubmitAlignedSimQuote.ts) — same debounced pay + sim as on-chain submit ([#356](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/356)). |
| **Chain name** | From [`getNetworkBadgeCopy()`](../frontend-dapp/src/utils/networkDisplay.ts) — must align with the network badge / env strip. |
| **`data-testid`s** | Panel: `swap-pre-submit-summary`; rows: `swap-confirm-action`, `swap-confirm-pair`, `swap-confirm-pair-contracts` / `swap-confirm-pair-contract` / `swap-confirm-hop-pair-{n}`, `swap-confirm-offer`, `swap-confirm-receive`, `swap-confirm-max-spread`, `swap-confirm-min-return`, `swap-confirm-chain`. |
| **Trade market mirror** | [`TradeMarketOrderPanel`](../frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx) reuses the component with action `Market swap` and root `trade-market-pre-submit-summary`. |

Implementation: [`SwapPreSubmitSummary.tsx`](../frontend-dapp/src/components/swap/SwapPreSubmitSummary.tsx); wired in [`SwapPage.tsx`](../frontend-dapp/src/pages/SwapPage.tsx) when a positive pay amount has a quote.

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_SWAP_SIGNING_CONFIRMATION.md`](../skills/AGENTS_FRONTEND_SWAP_SIGNING_CONFIRMATION.md).

### Swap page — MEV / submission posture {#swap-mev-posture}

This section documents how swaps reach the chain and the MEV/front-running risks traders should understand. It is **documentation only** — there is no MEV-protection setting in the dApp UI ([GitLab **#168**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/168), [**#299**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/299)).

#### How transactions are submitted

All swaps and trades are signed in the connected wallet and broadcast to the **public** Terra Classic mempool via the wallet’s normal RPC/LCD path. This dApp does **not** operate a private RPC, bundle relay, or MEV-protection channel. There is no opt-in protected submission path in this build, and no MEV-protection toggle will be added without a real protected path wired end-to-end.

#### MEV and front-running risks

- **Public mempool exposure:** Once a signed transaction enters the public mempool, validators and searchers can observe it before inclusion. Large or predictable swaps may be sandwiched or front-run.
- **Slippage protection is the on-chain guard:** **Slippage protection** (retail label; on-chain `max_spread` on pair/router messages) is the primary contract-level protection against sandwich and front-running losses. Keep it tight for large trades. Default protection is **5%** for new sessions ([#497](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/497); `DEFAULT_SLIPPAGE_TOLERANCE_PERCENT`). The Swap Settings **retail** panel exposes slippage presets (**0.5 / 1 / 5%**), **transaction deadline** (5/10/20/30m + custom, default 5 min), and a **High slippage protection increases front-running risk** warning when protection is **strictly above** 5% ([#413](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/413)). Agent checklist: [`skills/AGENTS_FRONTEND_DEFAULT_SLIPPAGE.md`](../skills/AGENTS_FRONTEND_DEFAULT_SLIPPAGE.md). Preset chips **0.5 / 1 / 5%** must stay one aligned group (label is not a `flex-wrap` sibling that can orphan 0.5%) — [#528](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/528); [`skills/AGENTS_FRONTEND_SLIPPAGE_PRESET_ALIGN.md`](../skills/AGENTS_FRONTEND_SLIPPAGE_PRESET_ALIGN.md). See [§ Slippage protection preset alignment](#slippage-protection-preset-align).
- **No UI disclosure panel:** Per product decision ([#299](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/299)), MEV posture is **not** surfaced in the Swap or Trade UI — an informational card would imply a user-controllable setting that does not exist.

| Invariant | Meaning |
|-----------|---------|
| **Public mempool default** | Wallet → public Terra Classic mempool; no private relay or bundle. |
| **No MEV toggle** | Do not add a cosmetic or disabled “MEV protection” control in the UI. |
| **Slippage is executable protection** | `max_spread` from Settings (**Slippage protection** label) is enforced on-chain; see [`docs/swap-max-spread-ux.md`](./swap-max-spread-ux.md). |
| **Docs-only disclosure** | MEV risks live in this section and linked docs, not in Swap/Trade Settings. |

Related: [`docs/swap-max-spread-ux.md`](./swap-max-spread-ux.md) (price impact / max spread) · [`docs/limit-orders.md`](./limit-orders.md) (hybrid routing disclosure — GitLab #111).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_MEV_POSTURE.md`](../skills/AGENTS_FRONTEND_MEV_POSTURE.md).

### Slippage protection preset alignment {#slippage-protection-preset-align}

Retail **0.5 / 1 / 5%** chips on `/trade` Market and Swap Settings are one control ([GitLab **#528**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/528)). Shared primitive: [`SlippageProtectionPresets.tsx`](../frontend-dapp/src/components/common/SlippageProtectionPresets.tsx). Values stay [`SLIPPAGE_TOLERANCE_PRESETS_PERCENT`](../frontend-dapp/src/utils/slippageProtectionCopy.ts) (`[0.5, 1.0, 5.0]`); default remains **5%** ([#497](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/497)).

| Invariant | Meaning |
|-----------|---------|
| **S528-1 Label is not a wrap sibling** | Visible **Slippage protection** sits above the chip `role="group"`. Do not put the label in the same `flex-wrap` list as the buttons. |
| **S528-2 One 3-up group** | Chips live in `grid grid-cols-3` (`data-testid="trade-market-slippage-presets"` / `swap-slippage-presets`). They share a baseline (top/bottom ≤ 2px). If the ticket is narrow they shrink together — never orphan 0.5% on the label row. |
| **S528-3 Swap Custom outside the group** | Custom input (`swap-slippage-custom`) is a sibling of the group, not a child. It stacks **below** the three chips (`flex-col`); it must not sit between 0.5% and 1%. |
| **S528-4 Trade touch targets** | Trade chips keep `TRADE_SLIPPAGE_PRESET_CLASS` (`min-h-11`, [#417](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/417)). No Custom on Trade Market. |
| **S528-5 Mapping unchanged** | `max_spread = (percent / 100).toString()` (`maxSpreadFromSlippagePercent`). Fresh store still defaults to 5 → `"0.05"`. |
| **S528-6 Chips stay in the ticket body** | Do not move presets into the [#527](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/527) money-CTA footer. `elementFromPoint` on a chip must hit that chip. |
| **S528-7 One group, one store write** | One mounted preset group per surface. Click writes `useDexStore.slippageTolerance` only (plus Swap Custom sanitize). |
| **S528-8 Pool withdraw out of scope** | Pool **0.5 / 1.0 / 2.0** chips stay on [`PoolAdvancedManage.tsx`](../frontend-dapp/src/components/pool/PoolAdvancedManage.tsx). Do not reuse this helper if it would change those values. |
| **S528-9 Custom sanitize** | Swap Custom uses `sanitizeSlippageCustomInput` (digits + one `.`). Values `< 0.01` show range error and do not persist; `> 50` clamp to 50. High-warn only when store **> 5**. |
| **S528-10 A11y** | Group `aria-labelledby` the visible label. No `tabindex` on the label. Tab order 0.5 → 1 → 5. `:focus-visible` on `.tab-glass` unchanged ([#144](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/144)). |

**Verify:** `make verify-issue-528` — RTL selection + Custom sanitize + Playwright P1–P10 (`e2e/slippage-preset-align-528.spec.ts`) when LocalTerra is up.

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_SLIPPAGE_PRESET_ALIGN.md`](../skills/AGENTS_FRONTEND_SLIPPAGE_PRESET_ALIGN.md), [`skills/AGENTS_FRONTEND_DEFAULT_SLIPPAGE.md`](../skills/AGENTS_FRONTEND_DEFAULT_SLIPPAGE.md).

### Tiers Page

The `/tiers` page is the only retail surface that **registers** a wallet for a fee-discount tier ([I12](./reference/fee-discount-tiers.md)). Holding CL8Y alone does not apply a discount. Canonical minima / bps live in [`docs/reference/fee-discount-tiers.md`](./reference/fee-discount-tiers.md) — do **not** duplicate that numeric ladder here.

The page:
- Lists **self-register** tiers only (`governance_only: false`, IDs **1–9**; I3)
- Shows **Hold {human} CL8Y** via `formatTokenAmountAbbrev` at 18 decimals (I12 / [#476](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/476))
- Shows `{discount}%` + **fee discount** and `{eff}%` + **eff. fee*** from `discount_bps` + factory `default_fee_bps` (I4). Do not apply a wallet `get_discount` to the published ladder. Pair-scoped chrome stays on Swap / Pool / Trade ([#537](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/537) / I14)
- Shows How it works **Limit place*** from `limit_discount_bps` (I13 / [#514](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/514); tier 9 place = 0)
- Lets a connected, non-governance wallet **Register** / **Deregister** (msgs and gas unchanged — [#384](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/384))

#### Phone-width card + How it works ([#651](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/651))

QA **11.1.4** is the phone-width `/tiers` checklist. Layout only — do not change execute msgs, `GetTiers` / `GetRegistration`, or invent discounts.

| ID | Rule |
|----|------|
| **T651-1** | At ≤390px CSS width (and 375×667), each self-register card shows **Tier N** on one line and **Hold {n} CL8Y** as one unbreakable phrase (`whitespace-nowrap` on `data-testid="tier-hold-{id}"`). Live abbrev is OK (`Hold 7.5K CL8Y`). No `truncate` / ellipsis that hides magnitude. |
| **T651-2** | Do **not** reserve an empty Register column (`w-28`) when the button is absent. Disconnected and current-tier rows have no dead 112px gap. |
| **T651-3** | Fee cluster phrases stay intact: `{pct}` + `fee discount` and `{pct}` + `eff. fee*` (`whitespace-nowrap`). No one-word-per-line wrap of `FEE` / `DISCOUNT`. |
| **T651-4** | When Register is shown, it is a full-width second row on `<md` (`min-h-11` ≥ 44px), `data-testid="register-tier-{id}"`. One button per card; `onRegister(tier_id)` matches that row. `z-index` stays below header Connect / Legal / WalletConnect (`z-[9999]`). |
| **T651-5** | Connected + registered: **Your Status** shows Active + Deregister; the current card has no Register; other self-register rows still do. Governance 0 / 255 never appear in the self-register list. |
| **T651-6** | How it works on `<md` is stacked labeled rows (`data-testid="tiers-how-it-works-mobile"`), not a squeezed `grid-cols-5`. Desktop ≥768 keeps the five-column table. **Limit place*** stays visible (I13). |
| **T651-7** | Display-only: no new indexer / contract surface; LCD fields render as text (no `dangerouslySetInnerHTML`); no ghost disabled Register when disconnected; pending Register disables every row button. |
| **T651-8** | `btn-primary` / `btn-muted` / `shell-panel-strong`; `var(--ink)` / `--mint` (alias blue). No `*-neo`. No always-on fee-trivia banner ([#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/489)). Not a `visualViewport` picker issue ([#632](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/632)). |

**Verify:** `make verify-issue-651` — Vitest `TiersPage.test.tsx` + docs/skills **T651-1–T651-8**. Playwright `e2e/fee-tiers.spec.ts` phone viewports (390 / 375) when LocalTerra is up (5 workers, no `e2e-tx`).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_TIERS_PHONE.md`](../skills/AGENTS_FRONTEND_TIERS_PHONE.md), [`skills/AGENTS_FEE_DISCOUNT_TIERS.md`](../skills/AGENTS_FEE_DISCOUNT_TIERS.md).

## Environment Variables

| Variable        | Example                    | Description               |
|-----------------|----------------------------|---------------------------|
| `VITE_NETWORK`  | `mainnet` / `testnet` / `local` | Target chain         |
| `VITE_FACTORY_ADDRESS`  | `terra1abc...`      | Factory contract address  |
| `VITE_ROUTER_ADDRESS`   | `terra1xyz...`      | Router contract address   |
| `VITE_FEE_DISCOUNT_ADDRESS` | `terra1def...`  | Fee discount registry contract address |

See `.env.example` for the full list.
