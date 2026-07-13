# Frontend Guide

## Design system

Visual primitives (**Cyberminimalist Glass System**): [`design-system.md`](./design-system.md). Agent playbook: [`skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md`](../skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md) ([#415](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/415)).

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
| [`patches/@goblinhunt+cosmes+*.patch`](../frontend-dapp/patches/) | `KeplrExtension`: per-sign **`preferNoSetFee`**, post-sign fee guard vs **`stdDoc.fee`**; `StationController`: extension → **amino always** |
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

Browser **extension** wallets use the same `window` signals as [`getKeplrLikeExtension`](../frontend-dapp/src/services/terraclassic/keplrLikeExtension.ts) plus **`'station' in window`** for Station ([GitLab #139](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/139)). When an extension is detected, the row shows a **Ready** pill next to the **Extension** pill; when it is not, the row is visually subdued and an **Install** link appears — there is **no** separate **Not installed** pill (redundant with **Install**; frees horizontal space on narrow modals, [GitLab #160](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/160)). Long wallet names truncate with an ellipsis; the full name is available via **`title`** on the label. **WalletConnect** rows are unchanged (no extension install check). **Leap** is not listed ([GitLab #159](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/159)). Implementation: [`walletExtensionInstall.ts`](../frontend-dapp/src/services/terraclassic/walletExtensionInstall.ts), [`WalletModal.tsx`](../frontend-dapp/src/components/wallet/WalletModal.tsx), [`useWalletExtensionInstallSnapshot.ts`](../frontend-dapp/src/hooks/useWalletExtensionInstallSnapshot.ts).

| Invariant | Meaning |
|-----------|---------|
| Align with `getKeplrLikeExtension` | **Cosmostation** detection must stay in sync with [`keplrLikeExtension.ts`](../frontend-dapp/src/services/terraclassic/keplrLikeExtension.ts); if that mapping changes, update **`isBrowserWalletExtensionDetected`** and the Vitest suite in [`walletExtensionInstall.test.ts`](../frontend-dapp/src/services/terraclassic/__tests__/walletExtensionInstall.test.ts). **Leap** is intentionally **not** offered ([GitLab #159](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/159)). |
| Station vs Station shim | **Station** uses **`'station' in window`** (extension injected), not only `station.keplr`, so the row does not depend on the Keplr-shaped shim being present. |
| WalletConnect | **`WalletType.WALLETCONNECT`** options must **not** be treated as missing extensions; they are always offered as QR / mobile flows (detection returns “present” for install UI purposes). |
| No duplicate “missing” chrome | Missing extensions are communicated by the dimmed row + **Install** CTA only — do not add a second **Not installed** badge ([GitLab #160](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/160)). |
| Long labels | Wallet **name** column uses **`min-w-0`**, **`truncate`**, and **`title={name}`** so long names (e.g. **COSMOSTATION**) do not collide with the **Extension** / **Ready** badges on small viewports. |
| Re-check after install | The modal subscribes via **`useSyncExternalStore`** to **`window` `focus`** and **`visibilitychange`** so returning from a store install refreshes badges without a full page reload. |
| Regression tests | [`walletExtensionInstall.test.ts`](../frontend-dapp/src/services/terraclassic/__tests__/walletExtensionInstall.test.ts). |
| **Build gate** | QA checklist item 4: **`npm run build`** and **`npx vitest run`** in `frontend-dapp` must pass on `main` before closing [GitLab #139](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/139). See [Production build — Vite source maps § `tsc -b`](#vite-production-sourcemaps). |

**Third-party / agent context:** [`skills/AGENTS_BUNDLE_DEV_WALLET.md`](../skills/AGENTS_BUNDLE_DEV_WALLET.md) · [`skills/AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md`](../skills/AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md) (connect modal layout + install UX).

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
| **Out of scope (#140 B)** | Remaining AddressRow surfaces: pair chips, `TxResultAlert` tx copy — [#188](#addressrow-primitive) (wallet header done). Address explorer URLs: [#184](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/184) — see [Terra Classic block explorer URLs](#terra-classic-block-explorer-urls). Chip network label + mobile layout: [#186](#connected-wallet-chip-network-mobile) — done. |

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
| **Single API path** | [`copyToClipboard`](../frontend-dapp/src/utils/copyToClipboard.ts) wraps `writeText`; UI uses [`CopyButton`](../frontend-dapp/src/components/ui/CopyButton.tsx) only. |
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

**First consumers:** wallet menu (`wallet-menu-address-row`), pool LP token line (`pool-lp-token-address-row`), trader profile header (`trader-profile-address-row`).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_ADDRESS_ROW.md`](../skills/AGENTS_FRONTEND_ADDRESS_ROW.md).

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
| Environment strip | [`EnvironmentRibbon`](../frontend-dapp/src/components/legal/EnvironmentRibbon.tsx) under the sticky header stack — **local**, **testnet**, and **mainnet** builds all show chain context (not only the `LOCAL`-style header badge). Ribbon backgrounds must stay **opaque enough** that scrolled page copy cannot bleed through ([GitLab **#482**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/482)); on desktop/tablet the header omits duplicate [`NetworkBadge`](../frontend-dapp/src/components/wallet/NetworkBadge.tsx) so this strip is the primary network signal ([GitLab **#483**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/483)). |
| NFA + risk summary | [`legalCopy.ts`](../frontend-dapp/src/components/legal/legalCopy.ts) — reused by the modal and [`LegalFooterNotice`](../frontend-dapp/src/components/legal/LegalFooterNotice.tsx) in the **desktop footer** and, on narrow viewports only, a second instance inside [`Layout`](../frontend-dapp/src/components/common/Layout.tsx) (`.app-mobile-legal-strip`) because the footer chrome is hidden below 768px. Footer includes **Report suspicious activity** → GitLab security template ([#392](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/392), [`SECURITY.md`](../SECURITY.md)). |
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

### Simulated (dev) wallet and `VITE_DEV_MNEMONIC` {#simulated-dev-wallet-and-vite_dev_mnemonic}

When `VITE_DEV_MODE=true`, the UI can offer a **Simulated Wallet** (no browser extension) implemented in [`devWallet.ts`](../frontend-dapp/src/services/terraclassic/devWallet.ts). Invariants:

| Invariant | Meaning |
|-----------|---------|
| No seed in app source | There is **no** default mnemonic in TypeScript. `VITE_DEV_MNEMONIC` must be supplied at dev time (e.g. `.env.development`, which Vite loads for `vite` / `npm run dev` but not for the default production `vite build`). |
| Same test vector as chain | For LocalTerra, use the same phrase as `TEST_MNEMONIC` in [`docker/init-chain.sh`](../docker/init-chain.sh). `scripts/deploy-dex-local.sh` writes it to `frontend-dapp/.env.development` after deploy. |
| Production build guard | `vite.config.ts` throws if `VITE_DEV_MNEMONIC` is present in the merged env for any `vite build` unless `mode === 'development'` or `VITE_ALLOW_DEV_MNEMONIC=local-only` (staging/production bundles). Tracked in [GitLab #118](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/118), [#378](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/378). |
| WalletConnect project ID | `vite build --mode production` **requires** `VITE_WC_PROJECT_ID`; `wallet.ts` has no shared default ID in the bundle ([#378](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/378)). |
| Production CSP | `vite build --mode production` replaces `index.html` CSP with env-scoped `connect-src` (LCD/RPC/indexer + WalletConnect relay) via [`viteCsp.ts`](../frontend-dapp/viteCsp.ts). Dev `vite` keeps broad `https:` in `index.html` for local endpoints. |
| Protocol audit addresses | Factory and router addresses render on [`/protocol`](../frontend-dapp/src/pages/ProtocolPage.tsx) only (`protocol-contract-addresses`) — not on swap confirmation ([#378](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/378)). |
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
| CSP | Production: `script-src 'self'`; `connect-src` = LCD + RPC + indexer + WalletConnect (no `https:` wildcard). Dev: broader policy for Vite HMR | [`index.html`](../frontend-dapp/index.html), [`render.yaml`](../render.yaml) |
| Deploy addresses | Factory/router on `/protocol` only; optional LCD sanity check | [`ProtocolPage.tsx`](../frontend-dapp/src/pages/ProtocolPage.tsx), [`deployAddressVerification.ts`](../frontend-dapp/src/utils/deployAddressVerification.ts) |
| Token logos | Host allowlist; evil URLs → blockie | [`tokenLogoAllowlist.ts`](../frontend-dapp/src/utils/tokenLogoAllowlist.ts), [`TokenLogo.tsx`](../frontend-dapp/src/components/ui/TokenLogo.tsx) |
| Expert mode | Type `ENABLE EXPERT MODE` to enable; 30% block / 50% settings cap unchanged | [`ExpertModeModal.tsx`](../frontend-dapp/src/components/swap/ExpertModeModal.tsx), [`swapRouteSlippage.ts`](../frontend-dapp/src/utils/swapRouteSlippage.ts) |
| Footer security link | Public posture doc linked from every page footer (SEC-A01, [#387](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/387)) | [`LegalFooterNotice.tsx`](../frontend-dapp/src/components/legal/LegalFooterNotice.tsx), [`security-posture.md`](./security-posture.md) |

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_TRUST_BOUNDARIES.md`](../skills/AGENTS_FRONTEND_TRUST_BOUNDARIES.md).

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
| **Funds safety (SEC-E05)** | **`LCD_CONNECTIVITY_OUTAGE_MESSAGE`** must reassure users that on-chain wallet balances, LP shares, and positions are **unaffected** — only app display/reads are limited ([GitLab **#427**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/427)). Vitest: [`lcdConnectivity.test.ts`](../frontend-dapp/src/utils/__tests__/lcdConnectivity.test.ts), [`LcdQueryGate.test.tsx`](../frontend-dapp/src/components/common/__tests__/LcdQueryGate.test.tsx). |

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
| All on-chain submits | Applies to limit place/cancel, swaps, pool add/withdraw, and any path using **`broadcastTerraExecuteContracts`** — not only `/trade`. |

Implementation: [`terraTxTimeout.ts`](../frontend-dapp/src/utils/terraTxTimeout.ts), [`withPromiseTimeout.ts`](../frontend-dapp/src/utils/withPromiseTimeout.ts), [`terraBroadcast.ts`](../frontend-dapp/src/services/terraclassic/terraBroadcast.ts) (canonical sign/broadcast/poll + post-sign recovery), [`terraWalletSignTxRaw.ts`](../frontend-dapp/src/services/terraclassic/terraWalletSignTxRaw.ts), [`terraTxRecoveryPoll.ts`](../frontend-dapp/src/services/terraclassic/terraTxRecoveryPoll.ts), [`terraGas.ts`](../frontend-dapp/src/services/terraclassic/terraGas.ts) (gas + `Fee` build), [`transactions.ts`](../frontend-dapp/src/services/terraclassic/transactions.ts) (public `executeTerraContract*` wrappers).

Regression: [`withPromiseTimeout.test.ts`](../frontend-dapp/src/utils/__tests__/withPromiseTimeout.test.ts), [`transactions.test.ts`](../frontend-dapp/src/services/terraclassic/__tests__/transactions.test.ts) (broadcast / poll timeout cases), [`terraBroadcastRecovery.test.ts`](../frontend-dapp/src/services/terraclassic/__tests__/terraBroadcastRecovery.test.ts) ([#359](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/359)).

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


### User-facing errors (wallet, fetch, indexer, tx) {#user-facing-errors-humanization}

Friendly failure copy should flow through **`humanizeUserFacingError`** ([`frontend-dapp/src/utils/humanizeUserFacingError.ts`](../frontend-dapp/src/utils/humanizeUserFacingError.ts)): it applies **`tryHumanizeTerraTxMessage`** first (on-chain / LCD patterns from [GitLab #134](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/134)), then wallet and transport classifiers in [`humanizeOffChainError.ts`](../frontend-dapp/src/utils/humanizeOffChainError.ts) ([GitLab #145](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/145)).

| Invariant | Meaning |
|-----------|---------|
| Single funnel | Call **`humanizeUserFacingError`** / **`humanizeUserFacingErrorFromUnknown`** at leaf call sites, or rely on components that already apply it: **`RetryError`**, **`TxResultAlert`** (`type === 'error'` only), and the **`useWalletStore.connect`** catch (wallet modal). |
| Diagnostics elsewhere | Full throws remain in **`console.error`** / devtools; **ErrorBoundary** adds a collapsed **Technical details** block (chunk failures scrub dev URLs — see [§ Lazy route chunks](#lazy-route-chunks)). Post-sign fee/gas guard failures ([`extensionSignedFeeGuard.ts`](../frontend-dapp/src/utils/extensionSignedFeeGuard.ts), [GitLab #127](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/127)) keep developer diagnostics in logs; UI shows **`EXTENSION_SIGNED_FEE_UNDERSHOOT_USER_MESSAGE`** via **`tryHumanizeTerraTxMessage`** ([GitLab #371](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/371)). |
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
| Router `execute_swap_operations` ([#353](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/353)) | Native wrap + CW20 router paths use **`gasLimitForRouterExecuteSwapOperations`**: single-hop **1.4M** (`ROUTER_SINGLE_HOP_GAS_LIMIT`, measured ~1.28M); multi-hop floor **`ROUTER_SWAP_OPS_MIN_GAS_PER_HOP` (900k)** per hop. **`WRAP_GAS_LIMIT` = 400k** (measured ~301k). Direct pair `swap` stays **840k**. |
| **Min uluna gas price (fee amount)** | `effectiveGasPriceUluna()` in [`constants.ts`](../frontend-dapp/src/utils/constants.ts) floors a low `VITE_GAS_PRICE_ULUNA` at **`MIN_GAS_PRICE_ULUNA` (28.325)**, matching Station `gasPriceStep` / Columbus-5 norms. Without this, **insufficient fee** errors occur at broadcast: high `gas_wanted` but **Fee.amount** computed with a tiny gas price (repro stack: **`increase_allowance`** on `/trade` or `/limits` before the CW20 **`send`** + `place_limit_order` tx — [GitLab #127](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/127)). |
| **Universal broadcast path** | Every on-chain submit uses **`broadcastTerraExecuteContracts`**; do not add parallel `wallet.broadcastTx` call sites. Gas limits live in **`terraGas.ts`**; sequence helpers (`executeCw20AllowanceThen`, `placeLimitOrderWithAllowance`) live in **`transactions.ts`** / **`pair.ts`**. [GitLab #127](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/127). |
| **Station extension + LocalTerra fee steps** | With **`VITE_NETWORK=local`**, [`wallet.ts`](../frontend-dapp/src/services/terraclassic/wallet.ts) still calls **`applyStationKeplrShimSignDefaults()`**, **`ensureStationLocalNetworkRegistered`** (always **`addNetwork`** to refresh **`gasPrices`**, before + after connect when native API exists), and **`experimentalSuggestChain`** when legacy `addNetwork` is unavailable. Patched **`KeplrExtension`**: per-sign **`preferNoSetFee`**, post-sign fee guard compares wallet **`signed.fee`** to **`stdDoc.fee`** (**no** second `signAmino` retry — [#208](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/208)). Run **`npm ci`** ([`cosmesPatch127.test.ts`](../frontend-dapp/src/services/terraclassic/__tests__/cosmesPatch127.test.ts)). **QA on LocalTerra:** **Keplr (extension)** and/or **dev/simulated wallet** only — **not Terra Station** (see [§ Station extension signing](#station-extension-signing)). [GitLab #127](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/127), [#235](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/235). |
| **Station extension signing** {#station-extension-signing} | **`StationExtension` === `KeplrExtension`** in cosmes; stack traces naming **`KeplrExtension`** are normal. Station’s Keplr shim must **not** use **`signDirect`** for extension txs — patched **`StationController`** sets **`useAminoSigning = true`** ([GitLab #208](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/208)). [`applyStationKeplrShimSignDefaults()`](../frontend-dapp/src/services/terraclassic/stationExtensionConfig.ts) on connect + before broadcast; **`withTerraWalletSignLock`** serializes extension `broadcastTx`. **Mainnet / columbus-5:** `experimentalSuggestChain` before/after connect refreshes **`gasPriceStep`** — **Terra Station P0 verification** (swap, limits, bids, wrap/unwrap) belongs here or on pre-release **columbus-5** staging with non-economic tokens. **LocalTerra connect:** [#207](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/207) **`addNetwork`** always (refresh stale **`gasPrices`**, [#127](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/127)) before + after connect when supported. Post-sign guard compares **`signed.fee`** to **`stdDoc.fee`**. False **`extension popup was closed`** → retail copy in [`terraBroadcast.ts`](../frontend-dapp/src/services/terraclassic/terraBroadcast.ts) / [`humanizeOffChainError.ts`](../frontend-dapp/src/utils/humanizeOffChainError.ts). **Known limitation — Station on LocalTerra:** Station ships a built-in **`localterra`** entry with stale **~0.015 uluna/gas**; our LocalTerra node ante handler requires **28.325 uluna/gas** (`localterra-cl8y`). Station **ignores** dApp fee overrides (`preferNoSetFee`, `experimentalSuggestChain`, `addNetwork`), so Station-signed txs on LocalTerra fail with insufficient fees (e.g. **3000 uluna** vs **~23M uluna** required). **Do not use Station for LocalTerra fee/signing QA** — use **Keplr** or the **dev/simulated wallet** ([#235](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/235)). Agents: [`skills/AGENTS_FRONTEND_STATION_SIGNING.md`](../skills/AGENTS_FRONTEND_STATION_SIGNING.md) · [`skills/AGENTS_TERRACLASSIC_GAS.md`](../skills/AGENTS_TERRACLASSIC_GAS.md). |
| Limit / bid placement sequence | **`placeLimitOrderWithAllowance`** → **`executeCw20AllowanceThen`** → two **`broadcastTerraExecuteContracts`** calls (`increase_allowance` then CW20 `send` / `place_limit_order`). Allowance tx is the usual first failure when fee math is wrong. |
| Limit cancel | **`cancelLimitOrder`** → `CANCEL_LIMIT_ORDER_GAS_LIMIT` ([`transactions.ts`](../frontend-dapp/src/services/terraclassic/transactions.ts)). |
| Limit parked-expired claim ([GitLab #141](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/141), batch [#246](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/246), **Claim all** [#253](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/253)) | **`claimExpiredLimitOrder`** → `CLAIM_EXPIRED_LIMIT_ORDER_GAS_LIMIT`; **`claimExpiredLimitOrders`** → `gasLimitForLimitOrderCancelBatch(n)`. Shared hook: [`useLimitExpiredClaimMutation`](../frontend-dapp/src/hooks/useLimitExpiredClaimMutation.ts). |
| Limit place — native balance vs **two** fees ([GitLab #132](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/132)) | The UI must not broadcast `increase_allowance` unless bank **uluna** ≥ `estimateLimitOrderPlaceSequenceUlunaFeesTotal()` (same model as two separate `Fee.amount` values). See [`limitOrderNativeGasBalanceGate.ts`](../frontend-dapp/src/utils/limitOrderNativeGasBalanceGate.ts), [`docs/limit-orders.md` § dApp retail form](./limit-orders.md#dapp-retail-form-wires-invariants). |
| Fee-discount register / deregister ([GitLab #384](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/384), FT-3 / FT-4) | `/tiers` **`register`** / **`deregister`** execute msgs must map in **`getGasLimitForTx`** to **`REGISTER_FEE_DISCOUNT_GAS_LIMIT` (300k)** and **`DEREGISTER_FEE_DISCOUNT_GAS_LIMIT` (250k)** — not **`BASE_GAS_LIMIT` (200k)**. Measured LocalTerra tier-1 register ≈ **204,438** gas (exceeded 200k fallback → wallet “needed more gas than estimated”). dApp does **not** LCD-simulate execute gas; per-message fallbacks are the canonical envelope ([`terraClassicFeeEstimate.ts`](../frontend-dapp/src/services/terraclassic/terraClassicFeeEstimate.ts)). Verify: `make verify-issue-384`. |
| Retail execute inventory / `BASE_GAS_LIMIT` guardrail ([GitLab #475](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/475), Mint drip [#474](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/474)) | Every retail `executeTerraContract*` shape (and CW20 `send` inner) must be registered in **`getGasLimitForTx`** with a named constant, or explicitly allowlisted for **`BASE_GAS_LIMIT`**. **Allowlist (intentional 200k only):** `increase_allowance` / `decrease_allowance`. **Mint faucet `drip`:** **`FAUCET_DRIP_GAS_LIMIT` (400k)** — missing mapping caused deterministic mainnet OOG after soft-launch (#473). **CW20 `send` → `unwrap`:** **`UNWRAP_GAS_LIMIT` (400k)** (not legacy `SWAP_GAS_LIMIT`). Inventory + CI fixtures: [`terraGasRetailInventory.ts`](../frontend-dapp/src/services/terraclassic/terraGasRetailInventory.ts). Dev builds `console.warn` on unmapped fallback. Verify: `make verify-issue-475`. Invariants **G-RETAIL-1 / G-RETAIL-2** in that module + [`skills/AGENTS_TERRACLASSIC_GAS.md`](../skills/AGENTS_TERRACLASSIC_GAS.md). |
| Pool add — CW20/CW20 path, native balance vs **three** fees ([GitLab #147](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/147)) | `provideLiquidity` (`pair.ts`) sends **three** txs: `increase_allowance` ×2 then `provide_liquidity`. The UI must not broadcast the first unless bank **uluna** ≥ `estimateProvideLiquidityCw20SequenceUlunaFeesTotal()` (three `Fee.amount` sums). Native/wrap paths use `executeTerraContractMulti` (one fee). **Rollback** after failed `provide_liquidity`: both `decrease_allowance` messages go out in **one** `executeTerraContractMulti` (one fee). See [`provideLiquidityNativeGasBalanceGate.ts`](../frontend-dapp/src/utils/provideLiquidityNativeGasBalanceGate.ts), [`docs/frontend.md` § Pool page](./frontend.md#pool-page--provide-liquidity-ui-invariants). |

**Operational alignment:** local/mainnet helper scripts use `terrad … --gas-adjustment 1.3` (see `scripts/deploy-dex-local.sh`). **`SWAP_GAS_BUFFER` is set to 1.3** so the dApp matches that default rather than a looser multiplier.

**Third-party / agent context:** see repository [`skills/AGENTS_TERRACLASSIC_GAS.md`](../skills/AGENTS_TERRACLASSIC_GAS.md) for a short playbook when changing gas constants or debugging `out of gas`. [`packages/localnet-trading-swarm/src/gas.ts`](../packages/localnet-trading-swarm/src/gas.ts) mirrors the same buffer for scripted swaps on LocalTerra ([GitLab #115](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/115)).

### Max amount / gas reserve {#max-amount-gas-reserve}

Retail **Max** (and pool **50%**) actions share [`AmountBalanceActions`](../frontend-dapp/src/components/common/AmountBalanceActions.tsx) and compute spendable amounts via [`maxSpendableAmount.ts`](../frontend-dapp/src/utils/maxSpendableAmount.ts) ([GitLab **#213**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/213)).

| Invariant | Meaning |
|-----------|---------|
| Single compute helper | **`computeMaxSpendableHumanAmount`** — all surfaces call this (Swap, Pool, trade limit/market, `/limits`); no inline `fromRawAmount(balance)` Max handlers. |
| BigInt reserve | Native **`uluna`** Max subtracts fee reserve in **raw micro-units** before **`fromRawAmount`**; never float subtraction on LUNC. |
| Fee envelope source | Reserves derive only from **`transactions.ts`** / **`terraGas.ts`** (`estimateNativeSwapUlunaFeesTotal`, `estimateProvideLiquidityNativeWrapUlunaFeesTotal`, existing sequence helpers). No magic uluna constants in UI. |
| CW20 Max unchanged | When pay asset is **not** native **`uluna`**, Max = full CW20 balance; native gas **submit** gates ([#132](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/132), [#147](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/147)) still block low-LUNC submits. |
| Native pay / wrap | Swap native pay, pool **Use native (auto-wrap)**, and native-input router paths subtract the action-specific reserve so one-click Max leaves **`Fee.amount`** payable from bank balance. |
| **`MaxAmountContext`** | Explicit per-surface context (`swap_native`, `swap_cw20`, `limit_place`, `market_swap`, `provide_liquidity_native_side`, `provide_liquidity_cw20`, `book_leg`) selects the correct fee envelope. |
| Limit **max mode** | Bid/Ask switch re-apply uses the same helper via [`useLimitEscrowMaxReapply`](../frontend-dapp/src/hooks/useLimitEscrowMaxReapply.ts). |
| Disabled Max | While balance is loading/error, or spendable raw is **0** after reserve, Max is disabled (no invalid drafts). |
| Book leg Max | Hybrid book override Max caps to **min(balance, pay amount)** — no gas reserve on the leg field. |
| Decimal drafts | Max output must pass **`isDecimalAmountDraft`** ([#169](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/169)). |

**Regression tests:** [`maxSpendableAmount.test.ts`](../frontend-dapp/src/utils/__tests__/maxSpendableAmount.test.ts), [`transactions.test.ts`](../frontend-dapp/src/services/terraclassic/__tests__/transactions.test.ts) (native swap / native wrap provide estimates).

**Third-party / agent context:** [`skills/AGENTS_TERRACLASSIC_GAS.md`](../skills/AGENTS_TERRACLASSIC_GAS.md) § Max amount / gas reserve.

### Local dev: Vite origin vs indexer CORS {#local-dev-indexer-cors}

The dApp reads **`VITE_INDEXER_URL`** (see [`frontend-dapp/.env.example`](../frontend-dapp/.env.example)) for browser `fetch` to the indexer API. **CORS is enforced on the `Origin` header**, which comes from the URL you open in the browser (`localhost` vs `127.0.0.1` are different origins). **`CORS_ORIGINS` on the indexer must list every origin you use for Vite** (typically both `http://localhost:5173` and `http://127.0.0.1:5173`, plus preview ports if applicable — [`indexer/.env.example`](../indexer/.env.example), [`scripts/deploy-dex-local.sh`](../scripts/deploy-dex-local.sh)). If they diverge, responses can show **200** in the Network panel while the browser still blocks the body (failed CORS).

After a successful **Place limit**, the UI polls **`GET .../limit-placements`** to auto-fill the cancel **Order ID**. Poll failures are logged as **`[limit-place] indexer poll failed:`** ([`warnIndexerPlacementPollFailed`](../frontend-dapp/src/utils/warnIndexerPlacementPollFailed.ts); [GitLab **#131**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/131)). On **`/trade`**, the ticket also surfaces **View order** / **Place another** next steps after a successful submit ([GitLab **#161**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/161); [§ Trade page — limit place success affordances](#trade-page-limit-place-success-affordances)). Operational detail: [`docs/indexer-invariants.md` § Local dev CORS](./indexer-invariants.md#local-dev-cors-localhost-vs-127001).

**Third-party / agent context:** [`skills/AGENTS_LOCALNET_TRADING_SWARM.md`](../skills/AGENTS_LOCALNET_TRADING_SWARM.md) (local stack); indexer matrix: [`docs/environment-matrix.md`](./environment-matrix.md).

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

### Create pair — token address validation {#create-pair-address-validation}

[`CreatePairPage`](../frontend-dapp/src/pages/CreatePairPage.tsx) validates both token contract fields with **`isValidTerraBech32Address`** / **`getTerraAddressInputError`** from [`terraAddressValidation.ts`](../frontend-dapp/src/utils/terraAddressValidation.ts) before enabling **Create Pair** ([GitLab **#382**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/382)).

| Invariant | Meaning |
|-----------|---------|
| Format gate | Wrong prefix, charset, or length → **Invalid Terra address format** (same regex as **`isValidTerraAddress`**). |
| Checksum gate | Structurally valid `terra1…` with bad bech32 checksum → **Invalid address: checksum does not match. Please check and re-enter the token address.** Submit stays disabled. |
| Trade deep links unchanged | **`/trade/:pairAddr`** still uses format-only **`isValidTerraAddress`** — no checksum on URL segments ([§ Trade page unknown pair link](#trade-page-unknown-pair-link)). |
| Tx fallback | If a checksum error still reaches the chain, **`tryHumanizeTerraTxMessage`** maps `addr_validate` / `decoding bech32 failed` to the same retail copy ([§ User-facing errors](#user-facing-errors-humanization)). |

Regression: [`terraAddressValidation.test.ts`](../frontend-dapp/src/utils/__tests__/terraAddressValidation.test.ts), [`CreatePairPage.test.tsx`](../frontend-dapp/src/pages/CreatePairPage.test.tsx).

| `/charts`       | Pairs overview and per-pair charts (indexer)      |
| `/portfolio`    | **My Portfolio** — connected wallet summary, open quote positions, wallet-wide open limits, LP overview, recent swaps ([GitLab **#212**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/212), phase 2 [**#217**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/217)); alias `/my-portfolio` → `/portfolio` |
| `/trader`       | Trader profile lookup (indexer); optional `/:address` |
| `/trade`        | Trade UI — order book, **price chart**, tape, **limit + market** tickets ([GitLab #152](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/152)) |
| `/trade/:pairAddr` | Same as `/trade` with pair pre-selected       |
| `/limits`       | Limit order placements, lifecycle, and **wallet history** (fills, cancels, swaps on pair + CSV) — [GitLab **#163**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/163) |
| `/tiers`        | View fee discount tiers, register/deregister for a tier |
| `/mint`         | Soft-launch faucet Mint page (shown in More nav only when `VITE_FAUCET_ADDRESS` is set — [GitLab **#473**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/473); runbook [`soft-launch-faucet.md`](./runbooks/soft-launch-faucet.md)) |

### My Portfolio (wallet-centric indexer exposure) {#my-portfolio}

Route **`/portfolio`** is the wallet-home surface for indexed trading exposure ([GitLab **#212**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/212); phase 2 [**#217**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/217)). It uses the connected address from **`useWalletStore`** only (no `?addr=` override). Public lookup of any address remains on **`/trader/:address`**.

| Invariant | Meaning |
|-----------|---------|
| **Disconnected** | Connect CTA only; **no** indexer/LCD calls that require an address. |
| **Positions API** | `GET /api/v1/traders/{addr}/positions` — returns `[]` for flat/unknown traders; not on-chain balances. |
| **Open limits API** | `GET /api/v1/traders/{addr}/limit-placements` — wallet-wide resting limits (`owner`); same cancel omission and **`lifecycle_status`** / **`?status=`** as pair route ([`indexer-invariants.md`](./indexer-invariants.md)); **`limit` ≤ 200**. UI: [`PortfolioOpenLimitsSection`](../frontend-dapp/src/components/portfolio/PortfolioOpenLimitsSection.tsx). |
| **LP overview** | Indexer `GET /api/v1/pairs` (max **50** pairs) + LCD CW20 **`balance`** per valid `lp_token` (concurrency **5**) via [`usePortfolioLpBalances`](../frontend-dapp/src/hooks/usePortfolioLpBalances.ts); skips invalid bech32 / per-pair LCD errors — **not** merged into positions table. |
| **Profile API** | `GET /api/v1/traders/{addr}` — **404** when the wallet has no indexed trader row; portfolio still shows positions + activity when present. |
| **LP vs trader** | Open positions are **swap-tracked quote exposure**; LP section is **on-chain LP token balances** — separate sections and copy; pool txs on **`/pool`**. |
| **P&amp;L semantics** | **Realized** indexer P&amp;L only — **no** unrealized mark-to-market on portfolio ([#217](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/217) defers until API/product agree). |
| **Privacy** | Trader routes are **public**; portfolio does not imply on-chain secrecy. |
| **Read-only** | No signing on portfolio; limits deep-link to **`/trade/{pairAddr}`** and **`/limits`**. |
| **Outage UX** | `MarketDataServiceOutageBanner` + `RetryError` parity with [`TraderPage`](../frontend-dapp/src/pages/TraderPage.tsx) for indexer-backed sections. |
| **Nav** | `Portfolio` in `PRIMARY_NAV_ITEMS` ([`navItems.ts`](../frontend-dapp/src/components/common/navItems.ts)); wallet menu **My Portfolio** link. |
| **Shared UI** | [`TraderSummaryStats`](../frontend-dapp/src/components/trader/TraderSummaryStats.tsx), [`TraderPositionsTable`](../frontend-dapp/src/components/trader/TraderPositionsTable.tsx) shared with trader profile. |

**Tests:** [`PortfolioPage.test.tsx`](../frontend-dapp/src/pages/PortfolioPage.test.tsx), [`usePortfolioLpBalances.test.ts`](../frontend-dapp/src/hooks/__tests__/usePortfolioLpBalances.test.ts), [`client.test.ts`](../frontend-dapp/src/services/indexer/__tests__/client.test.ts) (`getTraderPositions`, `getTraderLimitPlacements`), [`e2e/portfolio.spec.ts`](../frontend-dapp/e2e/portfolio.spec.ts), indexer [`api_traders.rs`](../indexer/tests/api_traders.rs).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_PORTFOLIO.md`](../skills/AGENTS_FRONTEND_PORTFOLIO.md).

### Wallet swap and limit history (indexer) {#wallet-swap-limit-history}

When a wallet is connected, **`/limits`** shows indexed **limit fills** (maker), **cancellations** (owner attribute when present on-chain), and **AMM swaps** for the **selected pair** via trader-scoped indexer routes. **`/trade`** shows the same **swap** slice for the active pair. Rows include **timestamps**, **tx hashes** (with explorer links in the table), **fees** where the indexer stores them (`commission_amount` / `effective_fee_bps` on swaps; `commission_amount` on limit fills), and **size amounts** ([GitLab **#479**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/479)):

| Section | Amount columns | Source fields |
|---------|----------------|---------------|
| Swaps (AMM) | **Amount in** / **Amount out** | `offer_amount` / `return_amount` |
| Limit fills (maker) | **Token0** / **Token1** (base / quote) | `token0_amount` / `token1_amount` |
| Limit cancellations | _(none)_ | API has no amount fields |

Amount cells use the same **`formatNum(raw)`** display as public [`TradesTable`](../frontend-dapp/src/components/ui/TradesTable.tsx) (raw chain integers — parity, not a third format). Mobile keeps horizontal scroll (`data-testid="wallet-history-table-scroll"`, [#352](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/352)).

**CSV export** uses `GET .../trades?format=csv`, `.../limit-fills?format=csv`, and `.../limit-cancellations?format=csv` on the **`/api/v1/traders/{addr}/...`** paths (same `pair=` filter as the table). Client `limit` is capped at **`TRADER_HISTORY_CSV_MAX_LIMIT` (200)** to match the indexer clamp — see [`docs/indexer-invariants.md`](./indexer-invariants.md). Export is **HTTP-only** (no wallet signature). Failures show an inline alert (`wallet-history-csv-error`); `fetchTraderHistoryCsv` retries once on network/timeout. Formula-injection escaping (#432) stays server-side.

| Invariant | Meaning |
|-----------|---------|
| **Pair scope** | History + CSV stay filtered to the selected pair; do not expand to global wallet history here. |
| **Amount parity** | Swaps reuse TradesTable amount semantics; fills expose token0/token1; cancellations stay Time / Order / Tx. |
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
| Tablet compact header | Viewports **`768px`–`1199px`**: header shows **Swap** inline plus **More**; Pool, Limits, Trade, and Charts appear **inside** the header More menu ahead of Trader / Protocol / Fee Tiers / Create Pair (`getHeaderMoreMenuItems(false)` in [`navItems.ts`](../frontend-dapp/src/components/common/navItems.ts)). Includes the **1024px–1199px** band where the full primary row previously overlapped wallet/controls ([GitLab **#136**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/136), [#483](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/483)). |
| Full desktop header | Viewports **`min-width: 1200px`**: all `PRIMARY_NAV_ITEMS` inline; header More lists **`MORE_NAV_ITEMS` only** (same as pre–#136 wide layout). |
| Nav → controls gap | At full-desktop widths, last nav control (**More**) and `.app-header-theme-group` must keep **≥ ~8px** horizontal gap (wallet connected or not). Desktop/tablet **omit** header [`NetworkBadge`](../frontend-dapp/src/components/wallet/NetworkBadge.tsx) — [`EnvironmentRibbon`](../frontend-dapp/src/components/legal/EnvironmentRibbon.tsx) is the primary network signal; mobile keeps the badge beside the wallet chip ([GitLab **#483**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/483)). |
| Sticky stack seam & opacity | `.app-top-sticky` uses an opaque `var(--bg-0)` stack background; header↔ribbon vertical gap **≥ ~8px**; ribbon uses **panel-bg + network tint** (not a translucent wash alone) so scrolled page copy cannot bleed through; Trade H1 clears the ribbon by **≥ ~16px** at `scrollY=0` ([GitLab **#482**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/482)). |
| Mobile vs header “More” active state | The bottom-tab **More** button highlights only for **`MORE_NAV_ITEMS`** routes; the header **More** trigger uses the expanded tablet list when compact so Pool/Charts/etc. still show an active affordance. |
| Header brand copy | Sticky header brand is **logo + “CL8Y DEX” title only** — no secondary kicker line (removed [GitLab **#136**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/136) regression fix; Terra Classic context stays in footer **`CL8Y DEX · Terra Classic`**). Below **`1024px`**, `.app-brand-copy` stays hidden ([GitLab **#52**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/52)). |
| Client-side tab navigation | Header and mobile shell tabs use [`AppShellNavLink`](../frontend-dapp/src/components/common/AppShellNavLink.tsx): plain left-click calls `navigate()` so **URL and `<Outlet>` update without hard refresh** even when a wallet extension swallows the default anchor handler ([GitLab **#182**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/182)). [`Layout`](../frontend-dapp/src/components/common/Layout.tsx) sets **`key={location.pathname}`** on `<Outlet />` so lazy route pages cannot stick on the prior tab after navigation ([GitLab **#138**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/138)). Modified clicks (new tab, etc.) are untouched. Regression: [`AppShellNavLink.test.tsx`](../frontend-dapp/src/components/common/__tests__/AppShellNavLink.test.tsx), E2E “desktop primary tabs change URL”, “navigates to Pool page”, NFA-after-route-change. |

Constants: `HEADER_FULL_NAV_MIN_WIDTH_PX` (`1200`), `TABLET_COMPACT_HEADER_MAX_WIDTH_PX` (`1199`), and row label tuples `DESKTOP_HEADER_NAV_ROW_LABELS` / `TABLET_COMPACT_HEADER_NAV_ROW_LABELS` for Playwright overlap checks (`frontend-dapp/e2e/navigation.spec.ts`). Sticky seam / nav→theme gap assertions live in the same file under **GitLab #482** / **#483**.

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_RESPONSIVE_HEADER.md`](../skills/AGENTS_FRONTEND_RESPONSIVE_HEADER.md), [`skills/AGENTS_FRONTEND_SHELL_NAV.md`](../skills/AGENTS_FRONTEND_SHELL_NAV.md), [`skills/AGENTS_FRONTEND_RISK_DISCLAIMERS.md`](../skills/AGENTS_FRONTEND_RISK_DISCLAIMERS.md) (environment ribbon).

### Keyboard focus visibility (WCAG 2.4.7) {#keyboard-focus-visible-wcag-247}

Interactive controls must expose a **visible keyboard focus indicator** when focused via Tab / Shift+Tab (`:focus-visible`). Pure `:focus` styling on components that also receive click focus can produce unwanted persistent rings for pointer users; industry practice is **`:focus-visible`** for custom rings ([WCAG 2.4.7 Focus Visible](https://www.w3.org/WAI/WCAG22/Understanding/focus-visible.html)). Implemented for header/nav, wallet/connect surfaces, tabs, primary buttons, form controls, and the Swap **You Pay** amount field ([GitLab **#144**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/144)).

| Invariant | Meaning |
|-----------|---------|
| Token alignment | Custom rings use **`var(--focus-ring)`** via `color-mix(in srgb, var(--focus-ring) 28%, transparent)` (same family as `.input-glass:focus-visible` in [`index.css`](../frontend-dapp/src/index.css)). |
| Inputs / triggers | `.input-glass`, `.select-glass`, `.token-select-trigger` use **`:focus-visible`** so mouse focus does not mimic keyboard emphasis where the UA supports it. |
| Focus ring footprint | `.token-select-trigger` reserves a **transparent** `0 0 0 2px` ring in the default `box-shadow` stack; `:focus-visible` only changes ring **color**, not size ([GitLab **#181**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/181)). |
| Shell & CTAs | `.btn-primary` / `.btn-muted` / `.btn-cta`, `.app-nav-link` (and related triggers), `.wallet-trigger` (+ `.wallet-trigger-connected`), `.network-badge`, `.tab-glass` / `.tab-glass-active`, `.wallet-option-card`, and dropdown `.app-menu-link` / `.wallet-menu-item` define explicit `:focus-visible` rings; **active** nav rows compose the active `box-shadow` **plus** the outer ring. |
| Menu backdrops | **`.app-menu-dismiss`** (shell More menu in [`Layout.tsx`](../frontend-dapp/src/components/common/Layout.tsx), connected wallet menu in [`WalletButton.tsx`](../frontend-dapp/src/components/wallet/WalletButton.tsx)) is a full-viewport **`type="button"`** with an **`aria-label`**; **`:focus-visible`** uses an inset ring ([GitLab **#187**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/187)). |
| Swap amount | The prominent pay amount `<input>` uses class **`swap-io-amount-input`** — do **not** strip focus with `focus:outline-none` without replacing it; ring styles sit beside `.swap-io-stack` in `index.css`. |

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_A11Y_FOCUS.md`](../skills/AGENTS_FRONTEND_A11Y_FOCUS.md).

### Portal listboxes (`MenuSelect` / `TokenSelect`) — layout stability {#portal-listbox-layout-stability}

Custom pair/token pickers use a **portaled** `<ul role="listbox">` positioned with **`position: fixed`** (not in-document flow) so opening a menu does not push chart, order book, or ticket columns ([GitLab **#181**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/181), W13-C2).

| Invariant | Meaning |
|-----------|---------|
| **Fixed menu** | [`computePortalListboxStyle`](../frontend-dapp/src/components/ui/portalListboxPosition.ts) + [`usePortalListbox`](../frontend-dapp/src/components/ui/PortalListbox.tsx) set coords on the **first open frame** (sync read of anchor `getBoundingClientRect()` during render; scroll/resize bump a reducer). `.token-select-dropdown` also declares `position: fixed` in CSS as a safety net. |
| **Stable trigger** | Wrapper `.token-select-root` uses `contain: layout` and `min-height: 48px` matching `.token-select-trigger`. Trade `#trade-pair-select` sits in `shrink-0` `max-w-xl` shell so flex rows do not compress the control when the menu opens. |
| **Scrollbar gutter** | `html { scrollbar-gutter: stable; }` avoids horizontal reflow when overlay scrollbars would otherwise appear/disappear. |
| **CLS budget** | Lighthouse CLS on `/trade` after opening the pair menu should stay **&lt; 0.1**; surrounding content must not jump (eyeball + Lighthouse). |
| **Keyboard (APG listbox)** | Arrow Up/Down (wrap), Home/End, typeahead by option label/symbol prefix (case-insensitive, 500ms buffer), Enter/Space to select, Escape/Tab to close with focus restored to trigger. Listbox exposes `aria-activedescendant`; options use `role="option"` + `aria-selected`. Shared hook: [`usePortalListboxKeyboard`](../frontend-dapp/src/components/ui/usePortalListboxKeyboard.ts) ([GitLab **#244**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/244), gap **M5**). **`openRef`** preserves multi-char typeahead between the first printable key and React `open` commit. |

**Regression tests:** [`frontend-dapp/e2e/trade-pair-select-cls.spec.ts`](../frontend-dapp/e2e/trade-pair-select-cls.spec.ts) (bounding-box deltas); unit tests in [`portalListboxPosition.test.ts`](../frontend-dapp/src/components/ui/__tests__/portalListboxPosition.test.ts), [`portalListboxKeyboard.test.ts`](../frontend-dapp/src/components/ui/__tests__/portalListboxKeyboard.test.ts), [`MenuSelect.keyboard.test.tsx`](../frontend-dapp/src/components/ui/__tests__/MenuSelect.keyboard.test.tsx), [`TokenSelect.keyboard.test.tsx`](../frontend-dapp/src/components/ui/__tests__/TokenSelect.keyboard.test.tsx).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_PORTAL_LISTBOX_CLS.md`](../skills/AGENTS_FRONTEND_PORTAL_LISTBOX_CLS.md); keyboard listbox APG: [`skills/AGENTS_FRONTEND_PORTAL_LISTBOX_KEYBOARD.md`](../skills/AGENTS_FRONTEND_PORTAL_LISTBOX_KEYBOARD.md); trade layout: [`skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](../skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md).

### Pair search combobox (`PairSearchSelect`) — Trade / Limits {#pair-search-combobox}

Trade and Limit Orders use [`PairSearchSelect`](../frontend-dapp/src/components/trade/PairSearchSelect.tsx) instead of a full factory pair dropdown ([GitLab **#314**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/314), pair switching regression [**#301**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/301)).

| Invariant | Meaning |
|-----------|---------|
| **Indexer search** | Debounced (≥300ms) `GET /api/v1/pairs?q=&sort=relevance&limit=20`; empty query uses `sort=volume_24h&order=desc` (high-liquidity defaults). |
| **Min query length** | ≥2 chars unless the query looks like a `terra1…` address ([`pairSearchQuery.ts`](../frontend-dapp/src/utils/pairSearchQuery.ts)). |
| **Factory gate** | Results are filtered to factory-registered pairs (`factoryPairs` prop) so only routable pairs appear. |
| **Degraded mode** | After the first indexer error in the session, combobox search uses `filterFactoryPairsByLocalSearch` on factory pairs (menu label, display symbols, contract/denom ids, localStorage-cached CW20 symbol/name, registry entries, two-token `XXX YYY` / `XXX/YYY` queries) without further indexer calls. Typed symbol search (e.g. `EMBER`) works without the indexer when token metadata was cached from a prior `token_info` read. Shows a dim **Offline search** hint in the listbox. |
| **Accessibility** | Input uses `role="combobox"` + portaled `listbox`; Arrow keys / Enter / Escape match portal listbox keyboard patterns. |
| **Liquidity badge** | Options show indexed 24h quote volume when `volume_quote_24h > 0`. |

Charts keeps its separate search + sort + `MenuSelect` layout (unchanged).

**Regression tests:** [`pairSearchQuery.test.ts`](../frontend-dapp/src/utils/__tests__/pairSearchQuery.test.ts); [`PairSearchSelect.issue301.test.tsx`](../frontend-dapp/src/components/trade/__tests__/PairSearchSelect.issue301.test.tsx); Trade page pair-switch test in [`TradePage.test.tsx`](../frontend-dapp/src/pages/TradePage.test.tsx); indexer [`list_pairs_relevance_ordering`](../indexer/tests/api_pairs.rs); Trade/Limits page tests mock `getPairs`.

**Product parity:** Trade/Limits search **pairs**; Swap searches **tokens** — see [Token search combobox](#token-search-combobox).

### Token search combobox (`TokenSearchSelect`) — Swap {#token-search-combobox}

Swap **YOU PAY** / **YOU RECEIVE** use [`TokenSearchSelect`](../frontend-dapp/src/components/trade/TokenSearchSelect.tsx) — a visible search combobox aligned with pair search UX ([GitLab **#481**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/481)). Mint keeps button-trigger [`TokenSelect`](../frontend-dapp/src/components/ui/TokenSelect.tsx) (small faucet list).

| Invariant | Meaning |
|-----------|---------|
| **Factory token universe** | Options come only from the `tokens` prop (`getAllTokens(pairs)` + native-wrap enrichment). Do **not** introduce an external/arbitrary token list or derive Swap options from `getPairs(q)`. |
| **Client-only filter** | Search is entirely client-side via [`tokenSearchQuery.ts`](../frontend-dapp/src/utils/tokenSearchQuery.ts) (works with indexer down). Haystack = id/denom, display symbol, localStorage-cached CW20 symbol/name, registry. No `GET /api/v1/tokens?q=` yet (optional follow-up if factory counts outgrow comfortable client filtering). |
| **Debounce / min chars / cap** | Debounce **300ms**; filter starts at ≥2 chars (or `terra1…` address ≥20); typed hits capped at **20**. Empty / too-short query browses the **full** allowed list sorted by display symbol. |
| **excludeToken** | Other leg is omitted from options; search tricks cannot select it. `onChange` only emits ids present in the gated options list. |
| **Query DoS / XSS** | Input `maxLength` / truncate at 128 chars; symbols/names render as **text only** (no `dangerouslySetInnerHTML`); logo URLs still pass [`resolveTrustedTokenLogoUrl`](../frontend-dapp/src/utils/tokenLogoAllowlist.ts). |
| **Accessibility** | Input `role="combobox"` + `aria-autocomplete="list"` + portaled `listbox`; Arrow / Enter / Escape / Tab. Typed query + Enter commits **first hit** (same #350 rule as pair search). |
| **Quote path unchanged** | Selection still updates the same token id string; routing/simulation/execution are untouched. |

**Regression tests:** [`tokenSearchQuery.test.ts`](../frontend-dapp/src/utils/__tests__/tokenSearchQuery.test.ts); [`TokenSearchSelect.test.tsx`](../frontend-dapp/src/components/trade/__tests__/TokenSearchSelect.test.tsx); E2E helpers [`e2e/helpers/token-select.ts`](../frontend-dapp/e2e/helpers/token-select.ts) target `combobox` (not `button`).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_TOKEN_SEARCH.md`](../skills/AGENTS_FRONTEND_TOKEN_SEARCH.md); keyboard notes in [`skills/AGENTS_FRONTEND_PORTAL_LISTBOX_KEYBOARD.md`](../skills/AGENTS_FRONTEND_PORTAL_LISTBOX_KEYBOARD.md).

### Trader profile (indexer JSON + route error recovery) {#trader-profile-indexer}

Trader profile data comes from **`GET /api/v1/traders/:address`** (see [`client.ts`](../frontend-dapp/src/services/indexer/client.ts)). Before the UI renders stats, the response is normalized by [`traderProfilePayload.ts`](../frontend-dapp/src/services/indexer/traderProfilePayload.ts) so **arrays, `null` bodies, or partial objects** from a buggy proxy or indexer never reach the page as a “truthy” trader object (which previously could crash the route tree and strand users behind the route error UI — [GitLab #126](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/126)).

| Invariant | Meaning |
|-----------|---------|
| Parse or fail | `getTrader` runs `parseIndexerTraderPayload` on raw JSON; invalid shapes throw and become a **React Query error**, not a render-time exception. |
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
| **Time-scale zoom / scroll (#336)** | Background candle refetches (`refetchInterval: 30_000`) must **not** reset user zoom/scroll. **`timeScale().fitContent()`** runs only on initial mount and indicator toggle — not on routine data refresh. Live updates use **`series.update()`** from the first changed bar via [`priceChartLightweightSeriesSync.ts`](../frontend-dapp/src/components/charts/priceChartLightweightSeriesSync.ts); **`setData`** is reserved for first load, interval switch, or truncated history. Regression: [`priceChartLightweightSeriesSync.test.ts`](../frontend-dapp/src/components/charts/__tests__/priceChartLightweightSeriesSync.test.ts), [`PriceChartLightweightCanvas.test.tsx`](../frontend-dapp/src/components/charts/__tests__/PriceChartLightweightCanvas.test.tsx). |
| Reference line | When the chart is empty, an optional **24h close** from `getPairStats` (`close_price`) may display; query is enabled only for that state so normal pairs are not blocked. |
| Accessibility | Canvas stays `aria-hidden` on `PriceChartLightweightCanvas`. Empty state uses `role="img"` + `aria-label`. When candles render, `PriceChart` exposes `role="region"` and an `aria-live` text summary (interval + last price) — see [§ Accessibility CI](#accessibility-ci) ([#214](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/214)). |
| **USD price scale (Y-axis)** | Spot **Price (USD)** is non-negative. The candlestick pane’s autoscale **must not** extend the right price scale below **zero** or below the **lowest visible candle `low`** (whichever is higher). Implemented via `autoscaleInfoProvider` + [`priceChartPriceScale.ts`](../frontend-dapp/src/components/charts/priceChartPriceScale.ts) ([GitLab **#151**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/151)). |
| **Chart viewport (layout)** | The plot region must **shrink inside** resizable `/trade` panels: `PriceChart` is **`h-full flex flex-col min-h-0`**; the candle mount uses **`flex-1 min-h-0`** with **`min-h-[min(52vh,280px)]`** (no fixed `560px` height). `TradePage` chart cards use **`flex flex-col min-h-0`** so the canvas is not clipped by **`overflow-hidden`** on first paint. `PriceChartLightweightCanvas` passes **`layout.panes.enableResize: false`** to `createChart` and reapplies width/height after layout via a **double `requestAnimationFrame`** and **`ResizeObserver`** ([GitLab **#151**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/151), contract tests [#227](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/227), lifecycle tests [#225](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/225)). |
| Volume histogram | **Pane 1** is a histogram of **quote** volume per candle (`volume_quote`). When quote volume is zero (common on thin local indexers), the UI uses **base** volume (`volume_base`) so bars remain visible—see [`priceChartCandles.ts`](../frontend-dapp/src/components/charts/priceChartCandles.ts). Sub-label **“Volume (quote, else base)”** documents this in the chart header. |
| Indicators | Optional **MA 7**, **MA 25** (line overlays on pane 0) and **RSI 14** (separate pane, scale 0–100 with 70/30 guides) are toggled from the **Indicators** menu. The chart instance is **created once**; toggles call lightweight-charts **`removeSeries` / `addSeries`** and, for RSI, **`addPane` / `removePane(2)`** so overlays reliably appear and disappear without an async full re-init race ([`priceChartLightweightIndicatorSync.ts`](../frontend-dapp/src/components/charts/priceChartLightweightIndicatorSync.ts)). Pure math lives in [`priceChartIndicators.ts`](../frontend-dapp/src/components/charts/priceChartIndicators.ts) ([GitLab **#150**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/150)). |
| Fullscreen | **Expand** uses the Fullscreen API on the chart card (`PriceChart` root); **Exit** restores normal layout. |
| **Automated tests** | Default Vitest stubs the library ([`lightweightChartsJsdomMock.ts`](../frontend-dapp/src/test/lightweightChartsJsdomMock.ts)) for React/indexer wiring, including **createChart option contract** + canvas lifecycle regressions in [`PriceChartLightweightCanvas.test.tsx`](../frontend-dapp/src/components/charts/__tests__/PriceChartLightweightCanvas.test.tsx) ([#227](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/227), [#225](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/225)) and malformed-candle / stale-pair race coverage ([#226](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/226)). Real **lightweight-charts** init, `setData`, indicators, volume fallback, USD autoscale (real `getVisibleLogicalRange()` after zoom — [#229](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/229)), **500/1500** (+ CI **2000**) large-candle guards, and post-layout canvas sizing run in `npm run test:charts` ([GitLab **#211**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/211), [#225](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/225), [#229](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/229), [docs/testing.md § Real lightweight-charts in Vitest](./testing.md#real-lightweight-charts-in-vitest-gitlab-211)). Harness: [`chartRealLibraryHarness.ts`](../frontend-dapp/src/test/chartRealLibraryHarness.ts). **Playwright** ([**#228**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/228)): [`e2e/price-chart-smoke.spec.ts`](../frontend-dapp/e2e/price-chart-smoke.spec.ts) asserts browser canvas mount + fullscreen aria ([docs/testing.md § Price chart Playwright smoke](./testing.md#price-chart-playwright-smoke-gitlab-228)). |

**Last price headline (non-axis):** Beside the **Price (USD)** title, the chart shows a **Last** value when resolvable: **latest tape price (USD)** from the indexer when the parent passes `tapeLastPriceUsd` (e.g. newest row from `getTrades`, which is `ORDER BY id DESC`), otherwise the **last candle’s close** for the selected interval. Implementation: `resolveTradeChartHeadlineUsd` in [`chartHeadlinePrice.ts`](../frontend-dapp/src/components/charts/chartHeadlinePrice.ts), `PriceChart` + `data-testid="trade-chart-headline-price"`. Tracked in [GitLab **#149**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/149).

Implementation: [`frontend-dapp/src/components/charts/PriceChart.tsx`](../frontend-dapp/src/components/charts/PriceChart.tsx), [`chartHeadlinePrice.ts`](../frontend-dapp/src/components/charts/chartHeadlinePrice.ts) (headline — **#149**), [`priceChartCandles.ts`](../frontend-dapp/src/components/charts/priceChartCandles.ts), [`priceChartCandlesPlaceholder.ts`](../frontend-dapp/src/components/charts/priceChartCandlesPlaceholder.ts), [`priceChartIndicators.ts`](../frontend-dapp/src/components/charts/priceChartIndicators.ts), [`priceChartLightweightIndicatorSync.ts`](../frontend-dapp/src/components/charts/priceChartLightweightIndicatorSync.ts), [`priceChartLightweightSeriesSync.ts`](../frontend-dapp/src/components/charts/priceChartLightweightSeriesSync.ts) (incremental `update` / viewport preservation — **#336**), [`PriceChartLightweightCanvas.tsx`](../frontend-dapp/src/components/charts/PriceChartLightweightCanvas.tsx) (USD scale clamp via [`priceChartPriceScale.ts`](../frontend-dapp/src/components/charts/priceChartPriceScale.ts)). GitLab: [**#113**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/113), [**#148**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/148), [**#149**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/149), [**#150**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/150), [**#151**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/151), [**#211**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/211) (real-library Vitest), [**#225**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/225) (canvas lifecycle tests), [**#226**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/226) (candle parsing + stale pair race), [**#227**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/227) (jsdom stub contract tests), [**#229**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/229) (large-candle perf + real visible-range autoscale in Vitest), [**#336**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/336) (time-scale zoom preserved on 30s refetch).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_PRICE_CHART.md`](../skills/AGENTS_FRONTEND_PRICE_CHART.md); trade workspace layout: [`skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](../skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md).

### Trade page — market context (tape, hybrid tag, limit-only book) {#trade-page-market-context}

Readability for traders used to centralized exchanges ([GitLab **#149**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/149)):

| Invariant | Meaning |
|-----------|---------|
| **Recent trades columns** | Headers **Pair** (pay → receive), **Amount in** / **Amount out** (offer / ask token amounts), plus **Price**, **Tx**. Column `<th>` elements carry `title` tooltips for the offer/ask semantics. Component: [`TradesTable.tsx`](../frontend-dapp/src/components/ui/TradesTable.tsx). |
| **`hybrid` badge** | Uppercase styling on the badge text; native **`title`** explains hybrid **AMM + limit order** execution and points integrators to **`docs/integrators.md`** for fee attribution across events. |
| **Order ticket — type tabs** | **Limit** vs **Market** tabs on `/trade` (`TradeOrderTicket`). Market uses global slippage (`useDexStore`), optional **limit book + pool** hybrid routing, shows expected receive + min after slippage with retail disclosure lines ([GitLab **#152**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/152), [#414](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/414)). |
| **Order ticket — post-only limit preflight** | Before `place_limit_order`, the UI compares the typed price to the **book head** from `GET .../limit-book?limit=1` (best bid / best ask). Bids with price **≥ best ask** and asks with price **≤ best bid** are blocked with inline copy — pure client guard; the pair still inserts by book walk on-chain. Helpers: [`limitOrderNonCrossing.ts`](../frontend-dapp/src/utils/limitOrderNonCrossing.ts), hook [`useTradeBestBookPrices.ts`](../frontend-dapp/src/hooks/useTradeBestBookPrices.ts). The **`/limits` ladder panel** applies the same guard **per rung** via `describeLimitCrossingBlockerWithRef` (book head first; when the opposite side is empty, falls back to indexed tape / AMM pool reference like the retail ticket) and shows **`N of M rungs will cross the market…`** when any rung crosses ([#297](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/297), [#385](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/385)). Complements the **tape-reference** gate in [§ Trade page — limit order price field](#trade-page-limit-order-price) ([GitLab **#154**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/154)). |
| **Market quote — route preview (#302)** | When a market amount is quoted, [`TradeMarketOrderPanel`](../frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx) shows a single **Route** row (`data-testid="trade-market-route-summary"`) inside `trade-market-quote`, using the same `computeSwapRouteDisplay` helper and indexer-op precedence as Swap ([#158](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/158)). Row renders only when `marketRouteLine` is truthy; multihop paths appear when hybrid quoting returns indexer `router_operations`. Agent checklist: [`skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md`](../skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md). |

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](../skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md) (layout + this section for labeling), [`skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md`](../skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md) (swap + trade market route row).

### Trade route — onboarding IA, CTA hierarchy, progressive disclosure {#trade-route-onboarding-ia}

Retail trade IA for Swap vs Trade vs Limits and calmer first paint on `/trade` ([GitLab **#417**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/417), parent [#411](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/411)):

| Invariant | Meaning |
|-----------|---------|
| **First-visit onboarding strip** | [`TradeOnboardingStrip`](../frontend-dapp/src/components/common/TradeOnboardingStrip.tsx) on `/`, `/trade`, and `/limits` until dismissed (`data-testid="trade-onboarding-strip"`). Copy links to **Swap** and explains when to use **Trade** vs **Limits**. Dismiss persists in `localStorage` ([`tradeOnboarding.ts`](../frontend-dapp/src/utils/tradeOnboarding.ts)). Must not block wallet connect or submit buttons. |
| **Money-action CTA sizing** | Primary trade submits use [`TRADE_MONEY_CTA_CLASS`](../frontend-dapp/src/utils/tradeMoneyCta.ts) (`py-3 text-sm font-semibold` minimum) on **Place limit**, **Market buy/sell**, and ladder place — aligned with Swap `btn-primary btn-cta` weight. `data-testid` hooks unchanged (`trade-limit-submit`, `trade-market-submit`, `ladder-place-submit`). |
| **Market slippage presets** | Chips in [`TradeMarketOrderPanel`](../frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx) use `min-h-11` (~44px touch target); `data-testid="trade-market-slippage-preset-{pct}"`. |
| **Progressive disclosure on `/trade`** | **Recent trades (tape)** and **wallet swap history** default **collapsed** on first visit. Sub-desktop: [`TradeWorkspaceDisclosure`](../frontend-dapp/src/components/trade/TradeWorkspaceDisclosure.tsx) (`trade-sub-lg-tape-disclosure`, `trade-wallet-history-disclosure`). Desktop: collapsible resizable tape panel (`trade-desktop-tape-panel`, `trade-desktop-tape-toggle`). Expansion persists via [`tradeWorkspacePanels.ts`](../frontend-dapp/src/utils/tradeWorkspacePanels.ts). Pause/blacklist banners remain visible when applicable ([#395](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/395), [#388](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/388)). |

**Verify:** `make test-frontend` — [`TradePage.test.tsx`](../frontend-dapp/src/pages/TradePage.test.tsx), [`TradeOnboardingStrip.test.tsx`](../frontend-dapp/src/components/common/__tests__/TradeOnboardingStrip.test.tsx). Manual: clear `cl8y-dex-trade-onboarding-dismissed`, `cl8y-dex-trade-tape-expanded`, and `cl8y-dex-trade-wallet-history-expanded` in DevTools → reload `/trade` → confirm collapsed tape/history and onboarding strip; mobile bottom nav must remain usable.

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_TRADE_ONBOARDING_IA.md`](../skills/AGENTS_FRONTEND_TRADE_ONBOARDING_IA.md).

**Cursor agents:** When iterating on merge readiness and CI for this area, the **Babysit PR** Cursor skill complements the [Testing](./testing.md) doc (comment triage, conflict resolution, green pipelines).

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

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_ORDER_BOOK_ROW_ACTIONS.md`](../skills/AGENTS_FRONTEND_ORDER_BOOK_ROW_ACTIONS.md); trade layout: [`skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](../skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md).

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
| **Headline-scaled USD** | `anchorUsdForLimitPrice` scales the chart **tape headline** linearly with typed price vs reference so the line matches the headline when the typed price equals the reference (same `tapeLastPriceUsd` prop as `PriceChart`). This is an **anchor estimate**, not a fresh oracle quote per token. When the reference comes from the **pool** (no tape), headline USD may stay **—** until tape returns. |
| **Submit gate** | **Bid:** `typed price >= reference` → disabled submit + error copy (buy limit must be **below** reference). **Ask:** `typed price <= reference` → disabled submit. When the user typed a **positive** limit and **no** reference can be resolved (no tape, pool empty, unknown decimals, or LCD error while loading pool), submit is **blocked** with explicit copy — never silently skip the guard ([GitLab **#166**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/166)). While the pool fallback query is **in flight**, submit stays disabled with a **warning** tone. |
| **Tooltip** | **Place limit** label ships with an **ⓘ** control (`title` + `sr-only`) explaining buy-below / sell-above semantics. |

Implementation: [`limitOrderPriceReference.ts`](../frontend-dapp/src/utils/limitOrderPriceReference.ts) (`resolveLimitOrderPriceRef`, pool spot helpers), [`useLimitOrderPriceRefBundle.ts`](../frontend-dapp/src/hooks/useLimitOrderPriceRefBundle.ts) (tape + `getPool` wiring for [`TradeOrderTicket.tsx`](../frontend-dapp/src/components/trade/TradeOrderTicket.tsx) and [`LimitOrdersPage.tsx`](../frontend-dapp/src/pages/LimitOrdersPage.tsx)), [`limitOrderPricePlaceGate.ts`](../frontend-dapp/src/utils/limitOrderPricePlaceGate.ts), [`LimitOrderPriceField.tsx`](../frontend-dapp/src/components/trade/LimitOrderPriceField.tsx) (`LimitOrderPlaceLimitHeading`, `LimitOrderPriceInputWithContext`), plus [`TradePage.tsx`](../frontend-dapp/src/pages/TradePage.tsx) for pair context. `LimitOrderEscrowPlaceGuardMessage` accepts the price gate result for inline errors.

### Trade page — limit order pre-submit summary (resting semantics, fees) {#trade-page-limit-order-pre-submit-summary}

Before **Place limit**, the ticket and standalone **`/limits`** form show a **pre-sign summary** so traders are not comparing resting limits to market-style quote lines ([GitLab **#157**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/157)):

| Invariant | Meaning |
|-----------|---------|
| **Signing fields (SEC-I05 / #461)** | Labeled rows before the wallet opens: **Action** (`Place Limit Order`), **Pair**, **Side** (Buy/Sell base), **Amount** (escrow), **Chain** (`getNetworkBadgeCopy().fullLabel`) — same anti-phishing anchor as swap/pool pre-sign cards. |
| **Not immediate** | Copy states that the order **rests** until others fill it; it is **not** a taker swap “now”. |
| **No taker slippage / impact / min received** | A short line contrasts limits with **market** execution (those lines appear on the **Market** tab / hybrid swap quote, not on the resting limit path). |
| **% vs reference** | Same signed deviation as under the price field: \((\text{typed} - \text{ref}) / \text{ref} \times 100\) from [`limitPriceDeviationPercent`](../frontend-dapp/src/utils/limitOrderPriceReference.ts), using the resolved tape or pool reference. |
| **Maker placement fee** | Retail copy: **small fee at placement** with human **percent** (`bpsToPercentLabel`) plus bps detail — not bps-only ([#419](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/419)). On-chain: **`floor(effective_fee_bps / 2)`** bps of escrow at placement ([`orderbook.rs`](../smartcontracts/contracts/pair/src/orderbook.rs)). |
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

Below **`lg` (`min-width: 1024px`)**, [`TradePage.tsx`](../frontend-dapp/src/pages/TradePage.tsx) uses a **CSS Grid** layout instead of the desktop `react-resizable-panels` workspace. Tablet portrait (**`768px`–`1023px`**, Tailwind **`md:`**–**`lg:`**) gets a **two-column top row** (price chart left, limit **order ticket** right) so iPad-class viewports are not forced into a phone-only vertical stack ([GitLab **#146**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/146)). Header density for the same band is documented above ([GitLab **#136**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/136)); this section is the **trade workspace** counterpart.

| Invariant | Meaning |
|-----------|---------|
| **`<768px` (default grid)** | Single column, DOM order: **order book** → **order ticket** → **chart** (when pair resolved) → **recent trades**. |
| **`768px`–`1023px` (`md:` / `<lg:`)** | Two-column grid: **chart** `row-start-1` / `col-start-1`, **order ticket** `row-start-1` / `col-start-2`, **order book** full width `row-start-2`, **recent trades** full width `row-start-3`. |
| **`≥1024px` (`lg:`)** | Unchanged: horizontal `PanelGroup` (book \| chart+tape \| ticket) with resize handles. |
| **No `useMediaQuery` on TradePage** | Breakpoints are **Tailwind-only** for this page; keep header `matchMedia` logic in `Layout.tsx` / `navItems.ts` only unless a future interaction requires JS alignment. |
| **`data-testid="trade-sub-lg-workspace"`** | Marks the sub-desktop grid root so Playwright (and agents) can scope headings — the desktop panel tree also contains an order book + chart and would otherwise duplicate roles. |
| **Price chart card (flex chain)** | Where `PriceChart` sits inside **`overflow-hidden`** or `Panel` chrome, the immediate wrapper is **`h-full … flex flex-col min-h-0`** (desktop chart cell; sub-lg chart **`card-glass`**). Keeps the candle canvas from being clipped when header + minimum plot height exceed the panel ([GitLab **#151**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/151)). |

Regression coverage: [`frontend-dapp/e2e/trade-page-responsive.spec.ts`](../frontend-dapp/e2e/trade-page-responsive.spec.ts).

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
| **Control type** | Limit **side** is a WAI-ARIA **`radiogroup`** with two **`role="radio"`** `<button type="button">` controls (`tab-glass*` styling), not native `<input type="radio">`, so the active side updates in the same React commit as `onSideChange` without browser-native controlled-radio timing quirks ([GitLab **#153**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/153)). |
| **Roving tabindex** | The selected side has **`tabIndex={0}`**; the other **`tabIndex={-1}`** (one tab stop for the group). **ArrowRight / ArrowDown** move selection and focus toward Ask; **ArrowLeft / ArrowUp** toward Bid; **End** selects Ask; **Home** selects Bid (from the Ask control). |
| **`data-testid`s** | **`{idPrefix}-side-radiogroup`**, **`{idPrefix}-side-bid`**, **`{idPrefix}-side-ask`**. **`/trade`** uses **`idPrefix="trade-ticket"`** ([`TradeOrderTicket.tsx`](../frontend-dapp/src/components/trade/TradeOrderTicket.tsx)); **`/limits`** uses **`idPrefix="limit-orders"`** ([`LimitOrdersPage.tsx`](../frontend-dapp/src/pages/LimitOrdersPage.tsx)). |
| **`/trade` button copy** | Direction buttons read **Buy {base}** / **Sell {base}** (token0 display symbol) so buttons match the ticket heading ([GitLab **#412**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/412); supersedes [#300](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/300)). Helper: [`tradeDirectionSideLabels.ts`](../frontend-dapp/src/utils/tradeDirectionSideLabels.ts). **`/limits`** keeps escrow-oriented **Bid (escrow …)** / **Ask (escrow …)** labels. |
| **Focus visibility** | Buttons use **`tab-glass*`** classes, which define **`:focus-visible`** rings aligned with [Keyboard focus visibility (WCAG 2.4.7)](#keyboard-focus-visible-wcag-247). |

**Implementation:** [`LimitOrderBidAskSideSelector.tsx`](../frontend-dapp/src/components/trade/LimitOrderBidAskSideSelector.tsx).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_LIMIT_ORDER_SIDE_SELECTOR.md`](../skills/AGENTS_FRONTEND_LIMIT_ORDER_SIDE_SELECTOR.md).

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

Implementation: [`PoolPreSubmitSummary.tsx`](../frontend-dapp/src/components/pool/PoolPreSubmitSummary.tsx); wired in [`PoolPage.tsx`](../frontend-dapp/src/pages/PoolPage.tsx) when provide has both amounts or withdraw has an LP amount.

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_POOL_SIGNING_CONFIRMATION.md`](../skills/AGENTS_FRONTEND_POOL_SIGNING_CONFIRMATION.md), [`skills/AGENTS_FRONTEND_POOL_PROVIDE_WITHDRAW_PREVIEW.md`](../skills/AGENTS_FRONTEND_POOL_PROVIDE_WITHDRAW_PREVIEW.md).

E2E for pool flows runs with the dev-wallet fixture; Playwright worker count is pinned in [`.cursor/rules/playwright-workers.mdc`](../.cursor/rules/playwright-workers.mdc) (5 workers) to keep the Vite `webServer` stable.

| GitLab | Role |
|--------|------|
| [#109](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/109) | Add-LP balances, Max / 50%, LP estimate, tests |
| [#147](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/147) | CW20/CW20 add LP: native LUNC preflight for three sequential txs (`provideLiquidityNativeGasBalanceGate.ts`) |
| [#112](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/112) | Pool list: indexer vs factory, router badges, filter |
| [#462](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/462) | Pre-sign summary card for provide/withdraw (SEC-I05 F-03) |
| [#480](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/480) | Provide auto-fill + withdraw receive preview |

### Liquidity pools list (indexer vs factory) {#liquidity-pools-list-indexer-vs-factory}

The pool list (`/pool`) is **sourced and sorted** by the [indexer](./indexer-invariants.md) `GET /api/v1/pairs` API. That order is **not** the on-chain factory’s `pairs` cursor order.

**Invariants (dApp):**

| Invariant | Meaning |
|-----------|---------|
| “N pair(s) (indexer total)” | Total from the indexer (pagination + sort params). |
| “M on-chain (factory, router graph)” | Number of `PairInfo` rows returned by paginating the factory’s `pairs` query (capped; see `FACTORY_PAIRS_MAX_FOR_POOL_LIST` in `pairListBadges.ts`). This is the same set the Swap page loads for `findRoute` / BFS. |
| Row badge **In router (factory)** | The pair’s `pair_address` is in that factory-derived `Set` — **O(1)** per row; **no** per-card `pair` query to the factory. |
| Row badge **Indexer only** | Address not in the factory list for this page session (e.g. indexing ahead of factory registration, de-listed pair still in indexer, or address outside the factory fetch cap). |
| “Indexer only” filter | Restricts the **current page** of indexer results to rows that appear in the factory set. |

**Drift line:** If indexer total and factory list length differ, the page shows a short **status** note (indexing lag, caps, or allowlist effects).

**Query strategy:** One React Query for `getAllPairsPaginated(FACTORY_PAIRS_MAX_FOR_POOL_LIST)` (stale time 60s), shared conceptually with Swap’s on-chain graph but **separate** query key (`factoryPairsForPoolList`) to avoid clashing with Swap’s default `maxPairs` limit.

**Code:** `frontend-dapp/src/utils/pairListBadges.ts`, `frontend-dapp/src/pages/PoolPage.tsx`. Issue: [glab#112](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/112).

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

The Swap page displays the effective fee after discount. When a connected wallet has a registered tier, the UI shows:
- The base pair fee (e.g., 0.30%)
- The discount percentage from the trader's tier
- The effective fee after discount (e.g., 0.15% for a 50% discount)

**Registry outage warning (GitLab [#374](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/374)):** When LCD `get_registration` / `get_discount` fails or the indexer reports `fee_discount_registry_ok: false` (`GET /api/v1/health/fee-discount`), registered traders see a non-blocking amber banner (`data-testid="swap-fee-discount-registry-warning"`) — swap submit stays enabled; on-chain execution may still charge full pair fee. Unregistered wallets with healthy LCD reads keep the **Hold CL8Y & register…** CTA instead. Logic: [`feeDiscountRegistryWarning.ts`](../frontend-dapp/src/utils/feeDiscountRegistryWarning.ts) + [`useFeeDiscountRegistryStatus`](../frontend-dapp/src/hooks/useFeeDiscountRegistryStatus.ts). Agent playbook: [`skills/AGENTS_FEE_DISCOUNT_TIERS.md`](../skills/AGENTS_FEE_DISCOUNT_TIERS.md) § Registry outage observability.

### Pool page fee-discount UX (GitLab [#476](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/476)) {#pool-page-fee-discount-ux}

Pool cards reuse the same fee-discount status hook as Swap:

| Signal | UI |
|--------|-----|
| Connected **unregistered** + healthy registry | Fee badge shows base pair fee + **· not registered**; CTA `data-testid="pool-fee-discount-unregistered-cta"` → `/tiers` |
| Connected **registered** + `discount_bps > 0` | [`FeeDisplay`](../frontend-dapp/src/components/ui/FeeDisplay.tsx) strikethrough base + effective % (unchanged math) |
| Registered + registry unreachable | Non-blocking amber banner `data-testid="pool-fee-discount-registry-warning"` (same copy as Swap); provide/withdraw stay enabled |
| `VITE_FEE_DISCOUNT_ADDRESS` empty | No discount queries, no CTA, no outage banner |

**Eligibility (invariant I12):** Discount applies only after the wallet **holds the configured `cl8y_token` CW20** (env `VITE_CL8Y_TOKEN_ADDRESS` / fee-discount `config.cl8y_token`) **and** successfully **`Register`s** a self-serve tier on `/tiers`. Holding alone, or a differently named/bridged asset (e.g. reporter “CL8Y-cb”), yields `discount_bps: 0` and a plain base fee — that is expected, not a missing on-chain feature. Shared copy: [`feeDiscountUiCopy.ts`](../frontend-dapp/src/utils/feeDiscountUiCopy.ts). Pool header note: `data-testid="pool-fee-discount-eligibility-note"`.

**CL8Y decimals:** [`tokenRegistry.ts`](../frontend-dapp/src/utils/tokenRegistry.ts) lists CL8Y at **18** decimals so `/tiers` Hold labels match `min_cl8y_balance` wei (LocalTerra TCL8Y is also 18 — [#383](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/383)).

**Tests:** [`PoolPage.feeDiscountRegistryBanner.test.tsx`](../frontend-dapp/src/pages/PoolPage.feeDiscountRegistryBanner.test.tsx), [`SwapPage.feeDiscountRegistryBanner.test.tsx`](../frontend-dapp/src/pages/SwapPage.feeDiscountRegistryBanner.test.tsx). QA: [`QA_TEMPLATE.md`](../QA_TEMPLATE.md) § 3.1.6–3.1.10.

**Expected slippage, Expert Mode & max spread (GitLab [#134](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/134), [#293](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/293)):** When the indexer returns `slippage_percent` on `route/solve`, the trade summary shows **Expected slippage** — symmetric deviation vs fair cross-rate token prices (`spot_amount_out`). The dApp prefers wallet `return_amount` vs spot when both are present ([`swapRouteSlippage.ts`](../frontend-dapp/src/utils/swapRouteSlippage.ts)). **Expert Mode** (Settings checkbox, default **off**, persisted in `localStorage`) blocks submit when expected slippage **> 30%** with **Slippage is too high** and an **Enable Expert Mode** affordance that opens a warning modal ([`ExpertModeModal.tsx`](../frontend-dapp/src/components/swap/ExpertModeModal.tsx)). **≥ 99%** always shows an extreme-slippage alert, even with Expert Mode enabled. Multihop and indexer quotes also run **per-hop pair simulation** preflight (factory resolve + `simulation` / `hybrid_simulation`) so hop spread is visible as secondary context and submit is disabled when any hop would exceed the user’s **Slippage tolerance** (`max_spread`). Failed txs that still surface `Max spread assertion` from the chain are mapped to short retail copy in [`humanizeTerraTxError.ts`](../frontend-dapp/src/utils/humanizeTerraTxError.ts) via [`terraBroadcast.ts`](../frontend-dapp/src/services/terraclassic/terraBroadcast.ts) and `TxResultAlert`. **Pool-only CW20 swap** gas uses the buffered one-hop router envelope (**830k**), not legacy **600k**, so wallet fee displays (~23 vs ~36 LUNC) stay aligned with on-chain headroom; LocalTerra post-sign guards reject fee/gas rewrites below **95%** of the dApp envelope ([#127](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/127)). Full invariants: [`docs/swap-max-spread-ux.md`](./swap-max-spread-ux.md).

**Swap Settings — retail vs Advanced (GitLab [#413](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/413)):** Opening **Settings** on `/` shows **retail** controls only: slippage presets/custom, **transaction deadline** (5/10/20/30m presets + custom, 30s–60m clamp via `useDexStore`), and **Expert Mode** (`data-testid="swap-expert-mode-toggle"`). Integrator tooling — direct-pair **limit book leg** (hybrid `book_input` / `max_maker_fills`, only when `isDirect && !isWrapOrUnwrap`) and **Indexer route check** (BFS hop dump) — live behind a collapsed-by-default **Advanced** disclosure ([`SwapAdvancedSettings.tsx`](../frontend-dapp/src/components/swap/SwapAdvancedSettings.tsx), `data-testid="swap-advanced-settings-toggle"`). Power users who expand Advanced persist that state in `localStorage` ([`swapSettingsAdvanced.ts`](../frontend-dapp/src/utils/swapSettingsAdvanced.ts)). Agent checklist: [`skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md`](../skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md) § Settings progressive disclosure.

**Route preview (GitLab [#158](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/158)):** The **Route** line lives in the same trade-summary card as **Price impact** and **Min received** (no separate “quote source” strip, no paired `Route (indexer)` / `Route` labels). The displayed token path follows the same precedence as submit: indexer-shaped `router_operations` when present, otherwise the client BFS route, native wrap path, or a direct `from → to`. Code: [`swapRouteDisplay.ts`](../frontend-dapp/src/utils/swapRouteDisplay.ts). Agent checklist: [`skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md`](../skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md).

**Submit–quote alignment (GitLab [#356](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/356), [#360](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/360), [#418](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/418)):** Sim queries debounce pay amount and hybrid book leg (**350ms**, [#346](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/346)). Submit must use the **same** debounced pay raw, book leg, and max-makers snapshot as the displayed quote — not live typed values while debounce, placeholder, or refetch is in flight. [`useSubmitAlignedSimQuote`](../frontend-dapp/src/hooks/useSubmitAlignedSimQuote.ts) bundles `submitPayRaw`, `minReceived`, `simData`, and `snapshottedHybrid` via [`buildSubmitAlignedSimPayload`](../frontend-dapp/src/utils/quoteDebounce.ts) for Swap and Trade market `swapMutation`; [`isSubmitQuoteStale`](../frontend-dapp/src/utils/quoteDebounce.ts) gates the submit button. Hybrid book-leg splits for sim/submit use debounced pay total, debounced book leg, and debounced max makers. **Hybrid quote path:** when execution is hybrid, [`quoteDirectHybridSwap`](../frontend-dapp/src/utils/directHybridQuote.ts) aligns indexer + LCD quotes with submit — no pool-only receive line while a book leg is configured ([#418](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/418)).

| Invariant | Meaning |
|-----------|---------|
| **Single submit snapshot** | Pay raw, min received, indexer ops, hybrid params, and route display refer to one settled sim result. |
| **No live/debounced skew** | `swapMutation` reads `submitPayRaw` (debounced), not live `inputAmount` / `marketAmountHuman`, when min received comes from `simQuery.data`; hybrid `book_input` / `max_maker_fills` come from the same debounced snapshot. |
| **Stale submit blocked** | Submit disabled when typed raw ≠ debounced key, live book leg ≠ debounced book leg, live max makers ≠ snapshotted max makers, `isPlaceholderData`, or `simQuery.isFetching` for the active debounced key. |

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
- **Slippage protection is the on-chain guard:** **Slippage protection** (retail label; on-chain `max_spread` on pair/router messages) is the primary contract-level protection against sandwich and front-running losses. Keep it tight for large trades. The Swap Settings **retail** panel exposes slippage presets, **transaction deadline** (5/10/20/30m + custom, default 5 min), and a **High slippage protection increases front-running risk** warning when protection exceeds 5% ([#413](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/413)).
- **No UI disclosure panel:** Per product decision ([#299](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/299)), MEV posture is **not** surfaced in the Swap or Trade UI — an informational card would imply a user-controllable setting that does not exist.

| Invariant | Meaning |
|-----------|---------|
| **Public mempool default** | Wallet → public Terra Classic mempool; no private relay or bundle. |
| **No MEV toggle** | Do not add a cosmetic or disabled “MEV protection” control in the UI. |
| **Slippage is executable protection** | `max_spread` from Settings (**Slippage protection** label) is enforced on-chain; see [`docs/swap-max-spread-ux.md`](./swap-max-spread-ux.md). |
| **Docs-only disclosure** | MEV risks live in this section and linked docs, not in Swap/Trade Settings. |

Related: [`docs/swap-max-spread-ux.md`](./swap-max-spread-ux.md) (price impact / max spread) · [`docs/limit-orders.md`](./limit-orders.md) (hybrid routing disclosure — GitLab #111).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_MEV_POSTURE.md`](../skills/AGENTS_FRONTEND_MEV_POSTURE.md).

### Tiers Page

The `/tiers` page allows users to:
- View all available discount tiers with CL8Y requirements
- See their current CL8Y balance and eligible tiers
- Register for a tier (sends a `Register` transaction)
- Deregister from their current tier
- View their active registration status

## Environment Variables

| Variable        | Example                    | Description               |
|-----------------|----------------------------|---------------------------|
| `VITE_NETWORK`  | `mainnet` / `testnet` / `local` | Target chain         |
| `VITE_FACTORY_ADDRESS`  | `terra1abc...`      | Factory contract address  |
| `VITE_ROUTER_ADDRESS`   | `terra1xyz...`      | Router contract address   |
| `VITE_FEE_DISCOUNT_ADDRESS` | `terra1def...`  | Fee discount registry contract address |

See `.env.example` for the full list.
