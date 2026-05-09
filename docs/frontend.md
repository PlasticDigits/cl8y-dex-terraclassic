# Frontend Guide

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
│   ├── components/       # Reusable UI components
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

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_PRODUCTION_BUILD.md`](../skills/AGENTS_FRONTEND_PRODUCTION_BUILD.md).

## Wallet Integration

The dApp connects to Terra Classic wallets using the Station browser extension or WalletConnect for mobile. Key considerations:

- **Network detection:** the `VITE_NETWORK` env var controls which chain the dApp targets (`mainnet`, `testnet`, `local`).
- **Signing:** all transactions use the connected wallet's signer. The dApp never handles private keys in production; the Simulated Wallet (dev only) is an exception and is described below.

### Simulated (dev) wallet and `VITE_DEV_MNEMONIC` {#simulated-dev-wallet-and-vite_dev_mnemonic}

When `VITE_DEV_MODE=true`, the UI can offer a **Simulated Wallet** (no browser extension) implemented in [`devWallet.ts`](../frontend-dapp/src/services/terraclassic/devWallet.ts). Invariants:

| Invariant | Meaning |
|-----------|---------|
| No seed in app source | There is **no** default mnemonic in TypeScript. `VITE_DEV_MNEMONIC` must be supplied at dev time (e.g. `.env.development`, which Vite loads for `vite` / `npm run dev` but not for the default production `vite build`). |
| Same test vector as chain | For LocalTerra, use the same phrase as `TEST_MNEMONIC` in [`docker/init-chain.sh`](../docker/init-chain.sh). `scripts/deploy-dex-local.sh` writes it to `frontend-dapp/.env.development` after deploy. |
| Production build guard | `vite.config.ts` throws if `VITE_DEV_MNEMONIC` is present in the merged production env (prevents inlining a real seed into `dist/`). Tracked in [GitLab #118](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/118). |
| Address in UI | The connected address comes from the `MnemonicWallet` instance (`devWallet.address`), not a hardcoded constant, so a custom dev mnemonic is reflected correctly. |
| Secret scanning | [`.gitleaks.toml`](../.gitleaks.toml) adds a custom rule for BIP39-like quoted phrases under `frontend-dapp/src` (default gitleaks rules do not cover this pattern). |

**Third-party / agent context:** [`skills/AGENTS_BUNDLE_DEV_WALLET.md`](../skills/AGENTS_BUNDLE_DEV_WALLET.md).
- **CW20 allowances:** before `ProvideLiquidity`, the dApp must ensure both CW20 tokens have sufficient allowance for the Pair contract.

### Terra Classic gas limits (router `execute_swap_operations`) {#terra-classic-gas-limits}

The dApp does **not** LCD-simulate every swap before broadcast. Instead, `executeTerraContract` / `executeTerraContractMulti` in [`frontend-dapp/src/services/terraclassic/transactions.ts`](../frontend-dapp/src/services/terraclassic/transactions.ts) set **Cosmos `Fee.gas`** from typed constants in [`frontend-dapp/src/utils/constants.ts`](../frontend-dapp/src/utils/constants.ts). **Underestimating gas causes on-chain `out of gas` after the wallet signs** (users still pay fees for failed txs).

**Formula (pool-only `execute_swap_operations`, no hybrid hop):** for `hops = max(operations.length, 1)`,

`gasWanted = max( round(SWAP_GAS_PER_HOP × hops × SWAP_GAS_BUFFER) + hops × SWAP_MULTIHOP_GAS_PADDING_PER_HOP, hops × EXECUTE_SWAP_OPS_MIN_GAS_PER_HOP )`

Hybrid hops use `max(..., HYBRID_SWAP_GAS_LIMIT × hops)` in `transactions.ts`.

| Invariant | Meaning |
|-----------|---------|
| Buffer tracks chain variance | `SWAP_GAS_BUFFER` must cover wasm execution variance on columbus-5 / LocalTerra; raising it increases **LUNC fee** (`effectiveGasPriceUluna() × gas`) proportionally — trade off vs reliability. |
| Floor guards multi-hop | `EXECUTE_SWAP_OPS_MIN_GAS_PER_HOP` prevents totals from collapsing when buffer × base is still too small for some hop shapes. |
| Padding absorbs rounding | Per-hop padding exists so totals do not sit exactly on prior “just barely enough” values (historical 2-hop near-miss: ~1,320,097 used vs 1,320,000 wanted). |
| Pool-only regression | Single-hop pool swap `gasWanted` must exceed **753,321** gas used in repro [GitLab #115](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/115) (710k at buffer **1.1** was too low). With **`SWAP_GAS_BUFFER = 1.3`** (aligned with `terrad --gas-adjustment 1.3`), one-hop `gasWanted` is **830,000** before hybrid floors. |
| **Min uluna gas price (fee amount)** | `effectiveGasPriceUluna()` in [`constants.ts`](../frontend-dapp/src/utils/constants.ts) floors a low `VITE_GAS_PRICE_ULUNA` at **`MIN_GAS_PRICE_ULUNA` (28.325)**, matching Station `gasPriceStep` / Columbus-5 norms. Without this, **insufficient fee** errors occur at broadcast: high `gas_wanted` but **Fee.amount** computed with a tiny gas price (repro stack: **`increase_allowance`** on `/trade` or `/limits` before the CW20 **`send`** + `place_limit_order` tx — [GitLab #127](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/127)). |
| **Station extension + LocalTerra fee steps** | With **`VITE_NETWORK=local`**, [`wallet.ts`](../frontend-dapp/src/services/terraclassic/wallet.ts) calls **`window.station.keplr.experimentalSuggestChain`** **before and after** `StationController.connect` (same Keplr-shaped suggestion: **`feeCurrencies[].gasPriceStep` = 28.325 / 28.325 / 50**) so fee metadata refreshes after `enable()`. The **`@goblinhunt/cosmes`** patch ([`patches/@goblinhunt+cosmes+0.0.71-ghunt.21.patch`](../frontend-dapp/patches/@goblinhunt+cosmes+0.0.71-ghunt.21.patch)) updates **`KeplrExtension`**: every **`signAmino` / `signDirect`** passes explicit **`preferNoSetFee: true`** (and memo), and **`StationController.connectExtension`** passes **`useAmino: true`** into **`StationExtension`** when **`chainId`** matches **LocalTerra** (**case-insensitive** comparison to **`localterra`**) **or** the key is **Ledger** (**`isNanoLedger`** — amino on all chains). That avoids Station’s shim replacing fees on **`signDirect`** (~3000 uluna on 200k gas — [GitLab #127](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/127)). Approve the chain update in Station if prompted; reconnect after changing RPC/LCD env. **QA:** limit/bid + allowance-first flows on **Station and Terra Classic Keplr** on LocalTerra. |
| Limit / bid placement sequence | Place Limit / Place Bid uses **`increase_allowance`** (uses `BASE_GAS_LIMIT`) then **`placeLimitOrder`** ([`pair.ts`](../frontend-dapp/src/services/terraclassic/pair.ts): CW20 `send` with base64 `place_limit_order`, gas `PLACE_LIMIT_ORDER_GAS_LIMIT`). Both steps use the same fee helper; the allowance tx is the usual first failure when fee math is wrong. |
| Limit place — native balance vs **two** fees ([GitLab #132](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/132)) | The UI must not broadcast `increase_allowance` unless bank **uluna** ≥ `estimateLimitOrderPlaceSequenceUlunaFeesTotal()` (same model as two separate `Fee.amount` values). See [`limitOrderNativeGasBalanceGate.ts`](../frontend-dapp/src/utils/limitOrderNativeGasBalanceGate.ts), [`docs/limit-orders.md` § dApp retail form](./limit-orders.md#dapp-retail-form-wires-invariants). |

**Operational alignment:** local/mainnet helper scripts use `terrad … --gas-adjustment 1.3` (see `scripts/deploy-dex-local.sh`). **`SWAP_GAS_BUFFER` is set to 1.3** so the dApp matches that default rather than a looser multiplier.

**Third-party / agent context:** see repository [`skills/AGENTS_TERRACLASSIC_GAS.md`](../skills/AGENTS_TERRACLASSIC_GAS.md) for a short playbook when changing gas constants or debugging `out of gas`. [`packages/localnet-trading-swarm/src/gas.ts`](../packages/localnet-trading-swarm/src/gas.ts) mirrors the same buffer for scripted swaps on LocalTerra ([GitLab #115](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/115)).

### Local dev: Vite origin vs indexer CORS {#local-dev-indexer-cors}

The dApp reads **`VITE_INDEXER_URL`** (see [`frontend-dapp/.env.example`](../frontend-dapp/.env.example)) for browser `fetch` to the indexer API. **CORS is enforced on the `Origin` header**, which comes from the URL you open in the browser (`localhost` vs `127.0.0.1` are different origins). **`CORS_ORIGINS` on the indexer must list every origin you use for Vite** (typically both `http://localhost:5173` and `http://127.0.0.1:5173`, plus preview ports if applicable — [`indexer/.env.example`](../indexer/.env.example), [`scripts/deploy-dex-local.sh`](../scripts/deploy-dex-local.sh)). If they diverge, responses can show **200** in the Network panel while the browser still blocks the body (failed CORS).

After a successful **Place limit**, the UI polls **`GET .../limit-placements`** to auto-fill the cancel **Order ID**. Poll failures are logged as **`[limit-place] indexer poll failed:`** ([`warnIndexerPlacementPollFailed`](../frontend-dapp/src/utils/warnIndexerPlacementPollFailed.ts); [GitLab **#131**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/131)). Operational detail: [`docs/indexer-invariants.md` § Local dev CORS](./indexer-invariants.md#local-dev-cors-localhost-vs-127001).

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
| `/charts`       | Pairs overview and per-pair charts (indexer)      |
| `/trader`       | Trader profile lookup (indexer); optional `/:address` |
| `/trade`        | Trade UI — order book, **price chart**, tape, limits |
| `/trade/:pairAddr` | Same as `/trade` with pair pre-selected       |
| `/limits`       | Limit order placements and lifecycle              |
| `/tiers`        | View fee discount tiers, register/deregister for a tier |

### Form inputs — programmatic labels {#form-inputs-programmatic-labels}

Text inputs must expose an **accessible name** to assistive tech: pair **`htmlFor` on `<label>`** with matching **`id` on `<input>`**, wrap the control in `<label>`, or use `aria-label` / `aria-labelledby` when layout requires it. Placeholder text or visually adjacent copy alone is **not** a reliable accessible name.

| Invariant | Meaning |
|-----------|---------|
| Pair labels for text fields | Prefer **`htmlFor` + `id`**; use React **`useId()`** per component instance so IDs stay unique when the same widget appears more than once. |
| Consistency | Follow the same pattern as [`LimitOrderExpiryField.tsx`](../frontend-dapp/src/components/trade/LimitOrderExpiryField.tsx) (`idPrefix` + explicit ids) and pair search on [`ChartsPage.tsx`](../frontend-dapp/src/pages/ChartsPage.tsx). |
| Checkbox / radio | An enclosing `<label>` that wraps the input remains valid; do not unwrap existing correct patterns to “fix” unrelated fields. |

Scope and verification checklist: [GitLab **#143**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/143). Umbrella: [DEX **#133**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/133).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_A11Y_FORM_LABELS.md`](../skills/AGENTS_FRONTEND_A11Y_FORM_LABELS.md).

### Responsive shell & header navigation {#responsive-header-navigation}

Layout lives in [`Layout.tsx`](../frontend-dapp/src/components/common/Layout.tsx) with shell styles in [`index.css`](../frontend-dapp/src/index.css). Breakpoints are **CSS-first** for showing the bottom tab bar vs the sticky header row; **header density** for mid-width tablets is driven by `matchMedia` so the **More** menu can absorb overflow without crowding ([GitLab **#136**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/136)).

| Invariant | Meaning |
|-----------|---------|
| Mobile bottom nav | Viewports **`max-width: 767px`**: `.app-desktop-nav` is hidden; primary links use `.app-mobile-nav-shell` + **More** sheet (`MORE_NAV_ITEMS` only — theme toggle stays in that sheet). |
| Tablet compact header | Viewports **`768px`–`1023px`**: header shows **Swap** inline plus **More**; Pool, Limits, Trade, and Charts appear **inside** the header More menu ahead of Trader / Protocol / Fee Tiers / Create Pair (`getHeaderMoreMenuItems(false)` in [`navItems.ts`](../frontend-dapp/src/components/common/navItems.ts)). |
| Full desktop header | Viewports **`min-width: 1024px`**: all `PRIMARY_NAV_ITEMS` inline; header More lists **`MORE_NAV_ITEMS` only** (same as pre–#136 wide layout). |
| Mobile vs header “More” active state | The bottom-tab **More** button highlights only for **`MORE_NAV_ITEMS`** routes; the header **More** trigger uses the expanded tablet list when compact so Pool/Charts/etc. still show an active affordance. |

Constants: `HEADER_FULL_NAV_MIN_WIDTH_PX` (`1024`) and row label tuples `DESKTOP_HEADER_NAV_ROW_LABELS` / `TABLET_COMPACT_HEADER_NAV_ROW_LABELS` for Playwright overlap checks (`frontend-dapp/e2e/navigation.spec.ts`).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_RESPONSIVE_HEADER.md`](../skills/AGENTS_FRONTEND_RESPONSIVE_HEADER.md).

### Keyboard focus visibility (WCAG 2.4.7) {#keyboard-focus-visible-wcag-247}

Interactive controls must expose a **visible keyboard focus indicator** when focused via Tab / Shift+Tab (`:focus-visible`). Pure `:focus` styling on components that also receive click focus can produce unwanted persistent rings for pointer users; industry practice is **`:focus-visible`** for custom rings ([WCAG 2.4.7 Focus Visible](https://www.w3.org/WAI/WCAG22/Understanding/focus-visible.html)). Implemented for header/nav, wallet/connect surfaces, tabs, primary buttons, form controls, and the Swap **You Pay** amount field ([GitLab **#144**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/144)).

| Invariant | Meaning |
|-----------|---------|
| Token alignment | Custom rings use **`var(--focus-ring)`** via `color-mix(in srgb, var(--focus-ring) 28%, transparent)` (same family as `.input-neo:focus-visible` in [`index.css`](../frontend-dapp/src/index.css)). |
| Inputs / triggers | `.input-neo`, `.select-neo`, `.token-select-trigger` use **`:focus-visible`** so mouse focus does not mimic keyboard emphasis where the UA supports it. |
| Shell & CTAs | `.btn-primary` / `.btn-muted` / `.btn-cta`, `.app-nav-link` (and related triggers), `.wallet-trigger` (+ `.wallet-trigger-connected`), `.network-badge`, `.tab-neo` / `.tab-neo-active`, `.wallet-option-card`, and dropdown `.app-menu-link` / `.wallet-menu-item` define explicit `:focus-visible` rings; **active** nav rows compose the active `box-shadow` **plus** the outer ring. |
| Swap amount | The prominent pay amount `<input>` uses class **`swap-io-amount-input`** — do **not** strip focus with `focus:outline-none` without replacing it; ring styles sit beside `.swap-io-stack` in `index.css`. |

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_A11Y_FOCUS.md`](../skills/AGENTS_FRONTEND_A11Y_FOCUS.md).

### Trader profile (indexer JSON + route error recovery) {#trader-profile-indexer}

Trader profile data comes from **`GET /api/v1/traders/:address`** (see [`client.ts`](../frontend-dapp/src/services/indexer/client.ts)). Before the UI renders stats, the response is normalized by [`traderProfilePayload.ts`](../frontend-dapp/src/services/indexer/traderProfilePayload.ts) so **arrays, `null` bodies, or partial objects** from a buggy proxy or indexer never reach the page as a “truthy” trader object (which previously could crash the route tree and strand users behind the route error UI — [GitLab #126](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/126)).

| Invariant | Meaning |
|-----------|---------|
| Parse or fail | `getTrader` runs `parseIndexerTraderPayload` on raw JSON; invalid shapes throw and become a **React Query error**, not a render-time exception. |
| Route error boundary reset | `/trader` routes use `resetKeys` tied to `useParams().address` so switching between `/trader` and `/trader/:addr` (or between addresses) **clears a prior route error** without a full page reload ([`App.tsx`](../frontend-dapp/src/App.tsx) `TraderRouteShell`). |
| Deduped React in Vite | [`vite.config.ts`](../frontend-dapp/vite.config.ts) sets `resolve.dedupe` for `react` / `react-dom` to avoid rare **dual-React** dev bundles that surface as `useContext`/`Invalid hook call` when lazy chunks load. |

**Third-party / agent context:** [`skills/AGENTS_BUNDLE_DEV_WALLET.md`](../skills/AGENTS_BUNDLE_DEV_WALLET.md) (wallet + local QA); [`skills/AGENTS_LOCALNET_TRADING_SWARM.md`](../skills/AGENTS_LOCALNET_TRADING_SWARM.md) (indexer-backed flows).

### Trade page — price chart invariants

The **price chart** on `/trade` and `/charts` is rendered with **TradingView [lightweight-charts](https://github.com/tradingview/lightweight-charts)** (open-source canvas charting). It is **not** the hosted TradingView terminal/widget product—naming in code review and issues should keep that distinction clear.

| Invariant | Behavior |
|-----------|----------|
| Successful empty candles | `GET /api/v1/pairs/{addr}/candles` may return `[]` or rows that all fail OHLC validation; the UI **must not** show a blank panel. Use the empty state in `PriceChart` + `PriceChartEmptyState`. |
| Single candle | After mapping/filtering, **one** valid OHLC point is enough for lightweight-charts to draw one candlestick; no empty-state for that case. |
| Loading vs empty | While React Query is loading, show the chart loading row; empty state applies only when the request **succeeded** and there are zero valid points. |
| Reference line | When the chart is empty, an optional **24h close** from `getPairStats` (`close_price`) may display; query is enabled only for that state so normal pairs are not blocked. |
| Accessibility | The empty panel uses `role="img"` and a descriptive `aria-label` so screen readers do not see a silent canvas. |

Implementation: [`frontend-dapp/src/components/charts/PriceChart.tsx`](../frontend-dapp/src/components/charts/PriceChart.tsx), [`priceChartCandles.ts`](../frontend-dapp/src/components/charts/priceChartCandles.ts) (pure mapping). Tracked in GitLab [**#113**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/113).

**Cursor agents:** When iterating on merge readiness and CI for this area, the **Babysit PR** Cursor skill complements the [Testing](./testing.md) doc (comment triage, conflict resolution, green pipelines).

### Trade page — responsive layout (sub-desktop) {#trade-page-responsive-layout}

Below **`lg` (`min-width: 1024px`)**, [`TradePage.tsx`](../frontend-dapp/src/pages/TradePage.tsx) uses a **CSS Grid** layout instead of the desktop `react-resizable-panels` workspace. Tablet portrait (**`768px`–`1023px`**, Tailwind **`md:`**–**`lg:`**) gets a **two-column top row** (price chart left, limit **order ticket** right) so iPad-class viewports are not forced into a phone-only vertical stack ([GitLab **#146**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/146)). Header density for the same band is documented above ([GitLab **#136**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/136)); this section is the **trade workspace** counterpart.

| Invariant | Meaning |
|-----------|---------|
| **`<768px` (default grid)** | Single column, DOM order: **order book** → **order ticket** → **chart** (when pair resolved) → **recent trades**. |
| **`768px`–`1023px` (`md:` / `<lg:`)** | Two-column grid: **chart** `row-start-1` / `col-start-1`, **order ticket** `row-start-1` / `col-start-2`, **order book** full width `row-start-2`, **recent trades** full width `row-start-3`. |
| **`≥1024px` (`lg:`)** | Unchanged: horizontal `PanelGroup` (book \| chart+tape \| ticket) with resize handles. |
| **No `useMediaQuery` on TradePage** | Breakpoints are **Tailwind-only** for this page; keep header `matchMedia` logic in `Layout.tsx` / `navItems.ts` only unless a future interaction requires JS alignment. |
| **`data-testid="trade-sub-lg-workspace"`** | Marks the sub-desktop grid root so Playwright (and agents) can scope headings — the desktop panel tree also contains an order book + chart and would otherwise duplicate roles. |

Regression coverage: [`frontend-dapp/e2e/trade-page-responsive.spec.ts`](../frontend-dapp/e2e/trade-page-responsive.spec.ts).

**Third-party / agent context:** [`skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](../skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md).

### Pool page — provide liquidity (UI invariants)

The **Provide Liquidity** card mirrors on-chain `provide_liquidity` math for the **Estimated LP** line (see `docs/contracts-terraclassic.md` and `smartcontracts/contracts/pair/src/contract.rs`):

- **First deposit** (both reserves `0`): user LP ≈ `sqrt(amount_a × amount_b) − 1000` LP smallest units (LP CW20 `decimals` = **18**; 1000 = `MINIMUM_LIQUIDITY` locked forever — see [issue #124](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/124)).
- **Later deposits:** user LP = `min(amount_a × total_share / reserve_a, amount_b × total_share / reserve_b)` (integer floor on each term, then `min`).

**Wallet balance queries** use the same React Query key prefix as the Swap page: `['tokenBalance', address, <asset id>]`, where the asset id is the CW20 `terra1…` address or, when “Use native (auto-wrap)” is checked, the bank **denom** string (e.g. `uluna`), via `getTokenBalance` in `src/services/terraclassic/queries.ts`.

**Ratio warning:** if the two typed amounts are not in the current pool price ratio, the contract still executes, but the **smaller** LP term sets the mint; the excess on the other side is effectively donated to the pool (same as Astroport/TerraSwap behavior).

E2E for pool flows runs with the dev-wallet fixture; Playwright worker count is pinned in [`.cursor/rules/playwright-workers.mdc`](../.cursor/rules/playwright-workers.mdc) (5 workers) to keep the Vite `webServer` stable.

| GitLab | Role |
|--------|------|
| [#109](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/109) | Add-LP balances, Max / 50%, LP estimate, tests |
| [#112](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/112) | Pool list: indexer vs factory, router badges, filter |

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
- `getTier(tierId)` — returns a single tier's details (min tokens, discount bps)
- `getTiers()` — returns all configured tiers
- `getRegistration(wallet)` — returns the wallet's current tier registration (or null)
- `isTrustedRouter(router)` — checks if an address is a trusted router

**Executions:**
- `register(tierId)` — self-register for a tier (EOA only)
- `deregister()` — remove own registration

### Swap Page Integration

The Swap page displays the effective fee after discount. When a connected wallet has a registered tier, the UI shows:
- The base pair fee (e.g., 0.30%)
- The discount percentage from the trader's tier
- The effective fee after discount (e.g., 0.15% for a 50% discount)

**Price impact & max spread (GitLab [#134](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/134)):** Multihop and indexer quotes use the router LCD `simulate_swap_operations` response, which only includes the final **amount**. The Swap page runs an additional **per-hop pair simulation** preflight (factory resolve + `simulation` / `hybrid_simulation`) so **price impact** reflects the worst hop under the same formula as pair `assert_max_spread`, and the submit button is disabled when any hop would exceed the user’s **Slippage tolerance** (`max_spread`). Failed txs that still surface `Max spread assertion` from the chain are mapped to short retail copy in [`humanizeTerraTxError.ts`](../frontend-dapp/src/utils/humanizeTerraTxError.ts). Full invariants: [`docs/swap-max-spread-ux.md`](./swap-max-spread-ux.md).

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
