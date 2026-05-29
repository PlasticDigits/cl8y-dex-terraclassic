# Testing

## Philosophy

CL8Y DEX tests focus on real contract behavior — no blockchain mocks. Unit tests exercise pure logic, integration tests deploy to a simulated chain environment, and E2E tests drive the actual frontend against LocalTerra.

## P2 testing epic (GitLab #199)

Consolidated coverage for production-review P2 gaps ([`TEST_GAP_MATRIX.md`](./reviews/20260409T030009Z/TEST_GAP_MATRIX.md)). Agent playbook: [`skills/AGENTS_TESTING_P2_EPIC.md`](../skills/AGENTS_TESTING_P2_EPIC.md).

| Area | Issue | Primary automated test | Notes |
|------|-------|------------------------|-------|
| Indexer hybrid attrs on `swap_events` | [#82](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/82) | [`indexer/tests/swap_events_hybrid_columns.rs`](../indexer/tests/swap_events_hybrid_columns.rs) | `book_return_amount`, `limit_book_offer_consumed`, `effective_fee_bps` |
| Book-leg fee discount | [#83](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/83) | `limit_order_tests::hybrid_book_fill_uses_taker_discounted_effective_fee_bps` | Same `effective_fee_bps` as pool path |
| Frontend hybrid message shape | [#84](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/84) | [`pair.test.ts`](../frontend-dapp/src/services/terraclassic/__tests__/pair.test.ts), [`router.hybrid.test.ts`](../frontend-dapp/src/services/terraclassic/router.hybrid.test.ts) | Direct pair + router `execute_swap_operations` |
| Pause blocks swap + limits | [#87](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/87) | `pause_blocks_swap_and_place_cancel_refunds_escrow`; [`TradePage.test.tsx`](../frontend-dapp/src/pages/TradePage.test.tsx) | L6 — see [`contracts-security-audit.md`](./contracts-security-audit.md) |
| Post-deploy smoke | [#86](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/86) | Manual: [`scripts/smoke-pool-swap.sh`](../scripts/smoke-pool-swap.sh) | LCD `pool` + optional `simulation`; run after deploy |
| Stubs / mocks catalog | [#105](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/105) | Policy below + issue #105 | LCD stub vs AMM-sim orderbook |
| Charts integration | [#104](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/104) | [`ChartsPage.integration.test.tsx`](../frontend-dapp/src/pages/ChartsPage.integration.test.tsx) | CI runs `npm run test:integration` |
| Price chart real `lightweight-charts` (Vitest) | [#211](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/211) | `*.charts.test.{ts,tsx}` via `npm run test:charts` | Separate from jsdom stub; see below |

**Post-deploy smoke (#86):**

```bash
export PAIR_ADDR=terra1...          # required
export OFFER_TOKEN=terra1...        # optional — enables Simulation query
export TERRA_LCD_URL=http://127.0.0.1:1317
./scripts/smoke-pool-swap.sh
```

See also [`docs/deployment-guide.md`](./deployment-guide.md) and [`docs/runbooks/launch-checklist.md`](./runbooks/launch-checklist.md).

## Test Types

### Indexer (Rust)

- **Unit tests (`cargo test --lib`):** parser stress tests, candle OHLC merge invariants, position clamping, oracle `f64` conversion, CG ticker shape validation — **no database required**.
- **Integration tests (`cargo test --tests`):** require PostgreSQL (set `TEST_DATABASE_URL` or use the default URL with valid credentials). They assert API allowlists, caps, CORS, rate limiting (429), and sanitized 500 responses.

```bash
cd indexer
cargo test --lib          # fast, no Postgres
cargo test --tests        # needs Postgres + migrations
```

#### Local Postgres setup (agents)

**Agent playbook:** [`skills/AGENTS_LOCAL_POSTGRES_DEV.md`](../skills/AGENTS_LOCAL_POSTGRES_DEV.md) — default user `cl8y_legal`, `make reset` when an old Docker volume still has `postgres:postgres`, `setup-postgres-dev-databases.sh`, and what `deploy-dex-local` writes to `indexer/.env`.

#### Shared Postgres and test parallelism

Integration tests call [`tests/common/mod.rs`](../indexer/tests/common/mod.rs) helpers that **truncate and re-seed** the same database. With default Cargo/Rust test parallelism, multiple integration test **binaries** and multiple **tests per binary** can run concurrently against that DB, which can surface as duplicate unique keys (e.g. on `assets.denom`) or foreign-key violations—not application bugs.

When using a **single** shared test database (typical local or CI), prefer serialized execution:

```bash
cd indexer
export TEST_DATABASE_URL="${TEST_DATABASE_URL:-postgres://cl8y_legal:cl8y_legal@127.0.0.1:5432/dex_indexer_test}"
cargo test --tests -j 1 -- --test-threads=1
```

- **`-j 1`** — run one integration test crate at a time (reduces cross-crate contention).
- **`--test-threads=1`** — run tests inside each binary one at a time (reduces intra-crate contention).

Start Postgres (`docker compose up -d postgres`) and run `./scripts/setup-postgres-dev-databases.sh` (or `make deploy-local`) so `dex_indexer_test` exists before the first run.

See [Indexer invariants](./indexer-invariants.md) for the full matrix and the same note under **Running tests**.

**Stubs, mocks, and test stand-ins:** intentional test doubles are cataloged in [GitLab issue #105](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/105) and summarized under [P2 testing epic (#199)](./testing.md#p2-testing-epic-gitlab-199). Key indexer spots: `indexer/tests/common/lcd_mock.rs` (LCD HTTP stub only) vs `indexer/src/api/orderbook_sim.rs` (**production** AMM curve-walk for CG/CMC — not the on-chain FIFO book; see **#210**). Agent playbook: [`skills/AGENTS_TESTING_P2_EPIC.md`](../skills/AGENTS_TESTING_P2_EPIC.md).

### Unit Tests (Rust)

Test individual contract functions in isolation using `cosmwasm_std::testing` helpers.

```bash
cd smartcontracts
cargo test
```

### Unit Tests (Frontend)

Test React components and hooks with Vitest and jsdom. **CosmWasm / LCD I/O** is typically **stubbed at the service layer** so unit tests stay fast and deterministic. That does **not** replace integration coverage for features that depend on indexer HTTP or chart data: use the **integration** Vitest config (below) or dedicated issues (e.g. GitLab [**#104**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/104) for charts).

```bash
make test-frontend        # single run (nvm via scripts/with-node.sh)
# or:
bash scripts/with-node.sh --cwd frontend-dapp -- npm run test
bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run
```

Config: `vitest.config.ts`

**Regression:** Trade/Charts **price chart** empty-candle UX and `getPairStats` fallback are covered in `src/components/charts/__tests__/PriceChart.test.tsx` (see GitLab [**#113**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/113) and [Trade page — price chart invariants](./frontend.md#trade-page--price-chart-invariants)).

#### Real `lightweight-charts` in Vitest (GitLab #211)

TradingView **[lightweight-charts](https://github.com/tradingview/lightweight-charts)** (open-source canvas library — **not** the hosted TradingView widget) has two Vitest layers:

| Layer | Config / command | What runs |
|-------|------------------|-----------|
| **Fast stub** (default) | `vitest.config.ts` → `npm run test:run` | `lightweightChartsJsdomMock.ts` — React/indexer wiring, `createChart` spies, `setData` payloads |
| **Real library** | `vitest.config.charts.ts` → `npm run test:charts` | Imports actual `lightweight-charts`; Node `canvas` shim in `src/test/chartsSetup.ts`; files matching `*.charts.test.{ts,tsx}` |

```bash
make test-frontend-charts   # from repo root
# or:
bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:charts
```

**CI:** the `frontend` job runs `npm run test:charts` after unit tests ([#211](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/211)).

**Agent playbook:** [`skills/AGENTS_FRONTEND_PRICE_CHART.md`](../skills/AGENTS_FRONTEND_PRICE_CHART.md). Chart invariants: [Trade page — price chart invariants](./frontend.md#trade-page--price-chart-invariants).

**Do not** load `lightweightChartsJsdomMock.ts` in the charts config. Pure helpers (`priceChartCandles`, `priceChartIndicators`, `priceChartPriceScale`) stay in default unit tests only.

### Integration Tests (Frontend)

Longer-running tests are kept out of the default `npm run test:run` suite. **Charts + indexer HTTP** coverage uses `vitest.config.integration.ts`: tests call a real indexer (`VITE_INDEXER_URL`, default `http://127.0.0.1:3001`) with PostgreSQL migrations applied. They are **not** skipped when the stack is down — the run fails so CI catches broken wiring. E2E and other flows may still use LocalTerra where documented.

**Charts integration (local)**

**Primary path:** from repo root with Postgres and the indexer API on `:3001` running (host Postgres or QA stack):

```bash
make test-charts-integration
```

This runs [`scripts/test-charts-integration.sh`](../scripts/test-charts-integration.sh): ensures the target database exists, applies `sqlx migrate run`, seeds fixtures idempotently, verifies indexer `/health`, then `npm run test:integration` via `scripts/with-node.sh`. Limit-order pool ref tests ([GitLab **#166**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/166)) also need LocalTerra LCD reachable (defaults `http://localhost:1317`; override `VITE_TERRA_LCD_URL` / `VITE_TERRA_RPC_URL` — same as [`frontend-dapp/.env.example`](../frontend-dapp/.env.example)).

**Fixture invariant:** the seeded pair address is `terra1paircontractabc` — must stay in sync with [`frontend-dapp/src/test/chartsIntegrationConstants.ts`](../frontend-dapp/src/test/chartsIntegrationConstants.ts) and [`indexer/scripts/seed-charts-integration.sql`](../indexer/scripts/seed-charts-integration.sql). Override the database with `CHARTS_INT_DATABASE_URL` (defaults to `DATABASE_URL` / `dex_indexer` from [`scripts/lib/postgres-dev.env`](../scripts/lib/postgres-dev.env)); override indexer URL with `VITE_INDEXER_URL` (default `http://127.0.0.1:3001`). See GitLab [**#205**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/205) and [`skills/AGENTS_TESTING_P2_EPIC.md`](../skills/AGENTS_TESTING_P2_EPIC.md).

**Manual steps** (when debugging individual phases):

1. Start PostgreSQL (for example `docker compose up -d postgres` from the repo root).
2. Create a database (once): `CREATE DATABASE cl8y_charts_int;` (name can match your `DATABASE_URL`).
3. Run migrations and seed minimal pair + candles:

   ```bash
   export DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/cl8y_charts_int
   cd indexer && sqlx migrate run && psql "$DATABASE_URL" -f scripts/seed-charts-integration.sql
   ```

4. Start the indexer API (same `DATABASE_URL` plus required env from `indexer/.env.example`: at minimum `FACTORY_ADDRESS`, **`CORS_ORIGINS`** (for browser integration tests / local Vite, include both `http://localhost:5173` and `http://127.0.0.1:5173` — [GitLab **#131**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/131)), `LCD_URLS`).

5. Run Vitest integration:

   ```bash
   cd frontend-dapp
   VITE_INDEXER_URL=http://127.0.0.1:3001 npm run test:integration
   ```

**Manual rollback SQL** (not run by `sqlx migrate`): paired `.down.sql` for selected migrations lives under [`indexer/migrations/revert/`](../indexer/migrations/revert/) — e.g. limit-order lifecycle columns ([GitLab **#142**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/142)) ship beside [`20260509160000_limit_order_placement_lifecycle.sql`](../indexer/migrations/20260509160000_limit_order_placement_lifecycle.sql).

**Note:** Default Vitest stubs `lightweight-charts` under jsdom via `src/test/lightweightChartsJsdomMock.ts` (including `LineSeries` for MA/RSI lines). **Real-library** chart init, `setData`, indicators, volume fallback, and USD autoscale paths run in `npm run test:charts` ([#211](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/211)). Pixel-level zoom/pan and layout regressions remain Playwright / manual QA.

Config: `vitest.config.integration.ts`

### E2E Tests (Playwright)

Full browser tests against the running dApp + LocalTerra. **Strict on-chain policy** ([GitLab **#201**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/201), [**#103**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/103)): default CI/local tx specs **fail** when LCD, funds, routes, or pairs are missing — no silent `test.skip` for those gaps.

```bash
make test-e2e-tx              # one command: LocalTerra + deploy + strict tx project
# Or from repo root with nvm (scripts/with-node.sh — see .nvmrc):
bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:e2e
bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:e2e:tx
bash scripts/with-node.sh --cwd frontend-dapp -- env PLAYWRIGHT_SKIP_CHAIN=1 npm run test:e2e:smoke
bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:e2e:ui
```

After `nvm use` in a shell, you may `cd frontend-dapp` and run the same `npm run test:e2e*` scripts directly.

Config: `playwright.config.ts` (`e2e-smoke`, `e2e-tx`, `e2e-indexer-outage` projects). Agent playbook: [`skills/AGENTS_E2E_STRICT_CHAIN.md`](../skills/AGENTS_E2E_STRICT_CHAIN.md).

#### Frontend E2E — indexer outage {#frontend-e2e-indexer-outage}

Market-data-down Playwright specs live in project **`e2e-indexer-outage`** (`**/*-indexer-outage.spec.ts`). They require the indexer HTTP API to be **stopped** while LocalTerra/Vite remain up, with **`E2E_INDEXER_OUTAGE=1`**. Default `npm run test:e2e` and the strict **`e2e`** CI job **exclude** this project — avoids flaking the strict chain suite ([#201](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/201)).

| Layer | Target |
|-------|--------|
| **CI** | GitHub Actions job **`frontend-e2e-indexer-outage`** (`.github/workflows/test.yml`) — LocalTerra + deploy + Postgres + indexer sanity → stop → Playwright |
| **Local one-command** | `make test-e2e-indexer-outage` → [`scripts/test-e2e-indexer-outage.sh`](../scripts/test-e2e-indexer-outage.sh) |
| **Manual** | Stop indexer on `:3001`, set pair from deploy, then run specs |

```bash
# Preferred local path (starts indexer, verifies /api/v1/overview, stops, runs specs):
make test-e2e-indexer-outage

# Manual (after deploy + indexer was running on :3001):
cd frontend-dapp
export E2E_TRADE_PAIR="$(bash ../scripts/lib/e2e-trade-pair-from-deploy.sh)"
E2E_INDEXER_OUTAGE=1 npm run test:e2e:indexer-outage
```

**Env vars:** `E2E_INDEXER_OUTAGE=1` (required for specs to run); `E2E_TRADE_PAIR` (optional — defaults to first deploy pair via `.qa-deploy-stamp` / factory LCD); `VITE_INDEXER_URL` (default `http://127.0.0.1:3001`).

Vitest covers Charts/Trader/Pool/**Limits** outage banners with mocked transport errors (`npm run test:run`; `/limits`: [`LimitOrdersPage.test.tsx`](../frontend-dapp/src/pages/LimitOrdersPage.test.tsx), GitLab **#218**). Product invariants: [docs/frontend.md § Market data loading & outage](./frontend.md#market-data-loading-outage); agent: [`skills/AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md`](../skills/AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md). Tracking: [GitLab **#219**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/219).

**Header / tablet compact nav:** `e2e/navigation.spec.ts` asserts no horizontal overlap for the **Swap + More** row at 773×743, **1024–1098px** (follow-up cram band), and other tablet widths; the full primary row at 1280px; desktop **Swap → Pool → Trade** tab transitions without reload at 1440px ([GitLab **#182**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/182)); and that **“Terra Classic ecosystem”** does not appear in the shell (header brand is logo + title only). Invariants: [docs/frontend.md § Responsive shell & header navigation](./frontend.md#responsive-header-navigation) ([GitLab **#136**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/136)); shell nav playbook [`skills/AGENTS_FRONTEND_SHELL_NAV.md`](../skills/AGENTS_FRONTEND_SHELL_NAV.md).

**Connected wallet chip — network label:** same spec file — desktop **`Local`** short label on the trigger at 1280px, mobile LUNC without visible network text, connected chip vs **More** non-overlap at 773px ([GitLab **#186**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/186); [docs/frontend.md § Connected wallet chip — network & mobile](./frontend.md#connected-wallet-chip-network-mobile)).

**Local stack for strict on-chain tests (default CI path):**

1. `docker compose up -d localterra`
2. From repo root: `bash scripts/deploy-dex-local.sh` (writes `frontend-dapp/.env.local`, deploys contracts, seeds CW20 balances on the dev account `terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v`).
3. `make test-e2e` or `bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:e2e`

Before tests, **`e2e/global-setup.ts`** waits for the LCD and runs **`scripts/e2e-provision-dev-wallet.sh`**, which **mints factory CW20s** to the dev wallet when any listed token balance is below **`E2E_DEV_MIN_CW20_U128`** (default `1000000000000` raw units), then **`scripts/e2e-seed-hybrid-book.sh`**, which idempotently places a **resting bid** on the first dual-CW20 pair when the bid book head is empty (GitLab [**#193**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/193)). **Invariant:** pair `OrderBookHead` returns a bare **`u64`** on LCD (`{"data":13}`), not `{ head_order_id }`; the seed script must treat an existing head as success so global setup can re-run ([GitLab **#138**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/138)). Native gas denoms **uluna** / **uusd** are expected from genesis (`docker/init-chain.sh`), not from the provision script.

**Single-file pool tx run (documented in `frontend-dapp/e2e/README.md`):**

```bash
bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/pool-tx.spec.ts --project=e2e-tx
```

**Hybrid swap E2E (strict tx path, GitLab [#193](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/193)):** `bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/hybrid-swap.spec.ts --project=e2e-tx` — requires global-setup seeding. See [`frontend-dapp/e2e/README.md`](../frontend-dapp/e2e/README.md) and [`skills/AGENTS_E2E_HYBRID_SWAP.md`](../skills/AGENTS_E2E_HYBRID_SWAP.md).

**Limit order tx E2E (strict place + cancel, GitLab [#195](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/195)):** `bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/limit-orders-tx.spec.ts --project=e2e-tx` — first **unpaused** dual-CW20 pair via LCD `is_paused`. See [`frontend-dapp/e2e/README.md`](../frontend-dapp/e2e/README.md) and [`skills/AGENTS_E2E_LIMIT_ORDERS_TX.md`](../skills/AGENTS_E2E_LIMIT_ORDERS_TX.md).

**Optional chain (skip instead of fail):** set `PLAYWRIGHT_SKIP_CHAIN=1` (or legacy `REQUIRE_LOCALTERRA=0`) for local UI-only runs (`npm run test:e2e:smoke`). **Do not** set this in CI. Default is strict (unset).

**GitLab [#138](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/138) verification (risk surfacing + E2E blockers):** after LocalTerra + `deploy-dex-local.sh` + indexer on `VITE_INDEXER_URL` (see [`docs/frontend.md` § Risk surfacing](./frontend.md#legal-risk-surfacing)):

```bash
cd frontend-dapp && npm ci && npm run test:unit
bash scripts/e2e-seed-hybrid-book.sh && bash scripts/e2e-seed-hybrid-book.sh   # second run must skip
bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/navigation.spec.ts -g "NFA footer"
bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/navigation.spec.ts -g "navigates to Pool"
```

Fixes: **`bd763be`** (`cosmesPatch127.test.ts`, `e2e-seed-hybrid-book.sh` bare `u64` head), **`f58cce5`** (`Outlet key={pathname}` shell tab nav). Agent playbooks: [`skills/AGENTS_FRONTEND_RISK_DISCLAIMERS.md`](../skills/AGENTS_FRONTEND_RISK_DISCLAIMERS.md), [`skills/AGENTS_FRONTEND_SHELL_NAV.md`](../skills/AGENTS_FRONTEND_SHELL_NAV.md).

### Trading swarm for UI load (localnet)

The [`@cl8y-dex/localnet-trading-swarm`](../packages/localnet-trading-swarm) package drives **five** concurrent bot wallets against LocalTerra to stress the dApp (swap, hybrid, router multi-hop, limit orders, LP flows) and optional indexer-backed views. It is **not** run in CI by default; manual QA runs it after `deploy-dex-local.sh`.

```bash
# From repo root (requires LocalTerra + frontend-dapp/.env.local)
./scripts/localnet-trading-swarm.sh
# Optional: validate wiring without txs
./scripts/localnet-trading-swarm.sh -- --dry-run
# Optional: JSON stats on SIGINT (mean inter-tx gap per bot ~20s target)
./scripts/localnet-trading-swarm.sh -- --stats
```

Contract message shapes align with [`docs/contracts-terraclassic.md`](./contracts-terraclassic.md), [`docs/limit-orders.md`](./limit-orders.md), and frontend Terra services. Full invariants: [`packages/localnet-trading-swarm/README.md`](../packages/localnet-trading-swarm/README.md); agent playbook: [`skills/AGENTS_LOCALNET_TRADING_SWARM.md`](../skills/AGENTS_LOCALNET_TRADING_SWARM.md). Issue: [GitLab #119](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/119).

### Fee Discount Contract Tests

Canonical tier numbers: [`docs/reference/fee-discount-tiers.md`](reference/fee-discount-tiers.md) (GitLab [#198](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/198)). Drift check: `make check-fee-discount-tier-docs`. Agent playbook: [`skills/AGENTS_FEE_DISCOUNT_TIERS.md`](../skills/AGENTS_FEE_DISCOUNT_TIERS.md).

The fee-discount contract has unit tests covering:

- **Tier management:** `AddTier`, `UpdateTier`, `RemoveTier` — validates governance-only access, duplicate tier rejection, and bps bounds (≤10000)
- **Registration:** `Register` for self-registration (EOA-only enforcement), `RegisterWallet` for governance-controlled registration, rejection of contracts attempting self-registration
- **Deregistration:** `Deregister` (self), `DeregisterWallet` (governance), lazy deregistration triggered by insufficient balance
- **Discount queries:** `GetDiscount` returns correct bps for registered traders, returns 0 for unregistered traders, fires deregistration when CL8Y balance is below threshold
- **Trusted routers:** `AddTrustedRouter`, `RemoveTrustedRouter`, `IsTrustedRouter` query
- **Governance tiers:** Tier 0 and Tier 255 cannot be self-registered, only governance can assign them
- **Config updates:** `UpdateConfig` governance-only access

### Integration Tests (Contracts)

The integration test harness in `smartcontracts/tests/` deploys the full contract suite (Factory, Pair, Router, Fee Discount) to a simulated chain and tests:

- End-to-end swap with discount: register a tier (from `STANDARD_PRODUCTION_TIERS` / canonical doc), execute swap, verify reduced commission
- Swap without registration: verify full fee applied
- Balance drop: transfer CL8Y away, swap, verify discount revoked and deregistration fired
- Router trusted forwarding: swap via Router passes trader address correctly
- Factory `SetDiscountRegistryAll`: verify all pairs receive the registry address (small deployments only)
- Factory `SetDiscountRegistryBatch`: verify cursor `next_start_after` + `has_more` advance until complete ([GitLab #123](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/123), covered in `factory_coverage_tests`)
- Blacklist (Tier 255): verify wallet receives zero discount

**Hybrid / limit book (L8 regression):** [`limit_order_tests.rs`](../smartcontracts/tests/src/limit_order_tests.rs) — single-hop hybrid sim vs execute, two-hop router with hybrid on the first leg (`router_two_hop_first_leg_hybrid_matches_simulate`), and **3-hop router with hybrid on ≥2 legs** (`router_three_hop_two_legs_hybrid_matches_simulate`, [GitLab **#192**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/192)). Agent playbook: [`skills/AGENTS_TESTING_MULTIHOP_HYBRID.md`](../skills/AGENTS_TESTING_MULTIHOP_HYBRID.md).

**Indexer route solve (hybrid merge):** integration tests in [`indexer/tests/api_route_solve.rs`](../indexer/tests/api_route_solve.rs) — POST `hybrid_by_hop` merge + LCD mock, GET default hybrid + `hybrid_optimize` on 2- and **3-hop** paths ([GitLab **#192**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/192), default hybrid GET [**#191**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/191)). Invariants: [indexer-invariants.md](./indexer-invariants.md).

## Coverage

### Frontend (Vitest)

```bash
cd frontend-dapp
npx vitest run --coverage
```

Coverage reports are generated in `frontend-dapp/coverage/` in text, JSON, and HTML formats (configured via `vitest.config.ts`).

### Smart contracts (Rust / LLVM)

Instrumented line coverage for the CosmWasm workspace uses [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov):

```bash
cargo install cargo-llvm-cov
cd smartcontracts
cargo llvm-cov test --workspace --lcov --output-path lcov.info
# Optional HTML report:
cargo llvm-cov report --html --output-dir target/llvm-cov-html
```

Or from the repo root: `make coverage-contracts` (writes `smartcontracts/lcov.info`).

Use coverage to find **untested business logic**, not as a vanity metric — see [contracts-security-audit.md](./contracts-security-audit.md) for invariant-to-test mapping.

## CI

The GitHub Actions workflow (`.github/workflows/test.yml`) runs:
1. `cargo fmt --check` + `cargo clippy` + contract tests via `cargo llvm-cov test` (LCOV artifact) + WASM builds
2. `tsc --noEmit` + `npm run lint` + `npm run test:run`
3. **Frontend charts integration:** PostgreSQL service → `sqlx migrate run` → `seed-charts-integration.sql` → release indexer binary → `npm run test:integration` against `http://127.0.0.1:3001` (local equivalent: `make test-charts-integration`, GitLab [#205](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/205))

See [the workflow file](../.github/workflows/test.yml) for details.

## Writing Tests

- Place unit tests next to source files: `MyComponent.test.tsx`
- Place integration tests next to source files: `MyComponent.integration.test.tsx`
- Place E2E tests in `frontend-dapp/e2e/`
- Use `renderWithProviders()` from `src/test/helpers.tsx` for component tests
- Use the dev-wallet fixture from `e2e/fixtures/dev-wallet.ts` for E2E tests
