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

## Wallet Integration

The dApp connects to Terra Classic wallets using the Station browser extension or WalletConnect for mobile. Key considerations:

- **Network detection:** the `VITE_NETWORK` env var controls which chain the dApp targets (`mainnet`, `testnet`, `local`).
- **Signing:** all transactions use the connected wallet's signer. The dApp never handles private keys.
- **CW20 allowances:** before `ProvideLiquidity`, the dApp must ensure both CW20 tokens have sufficient allowance for the Pair contract.

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
| `/trade`        | Trade UI — order book, **price chart**, tape, limits |
| `/trade/:pairAddr` | Same as `/trade` with pair pre-selected       |
| `/limits`       | Limit order placements and lifecycle              |
| `/tiers`        | View fee discount tiers, register/deregister for a tier |

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

### Pool page — provide liquidity (UI invariants)

The **Provide Liquidity** card mirrors on-chain `provide_liquidity` math for the **Estimated LP** line (see `docs/contracts-terraclassic.md` and `smartcontracts/contracts/pair/src/contract.rs`):

- **First deposit** (both reserves `0`): user LP ≈ `sqrt(amount_a × amount_b) − 1000` micro-units (1000 = `MINIMUM_LIQUIDITY` locked forever).
- **Later deposits:** user LP = `min(amount_a × total_share / reserve_a, amount_b × total_share / reserve_b)` (integer floor on each term, then `min`).

**Wallet balance queries** use the same React Query key prefix as the Swap page: `['tokenBalance', address, <asset id>]`, where the asset id is the CW20 `terra1…` address or, when “Use native (auto-wrap)” is checked, the bank **denom** string (e.g. `uluna`), via `getTokenBalance` in `src/services/terraclassic/queries.ts`.

**Ratio warning:** if the two typed amounts are not in the current pool price ratio, the contract still executes, but the **smaller** LP term sets the mint; the excess on the other side is effectively donated to the pool (same as Astroport/TerraSwap behavior).

E2E for pool flows runs with the dev-wallet fixture; Playwright worker count is pinned in [`.cursor/rules/playwright-workers.mdc`](../.cursor/rules/playwright-workers.mdc) (5 workers) to keep the Vite `webServer` stable.

| GitLab | Role |
|--------|------|
| [#109](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/109) | Add-LP balances, Max / 50%, LP estimate, tests |

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
