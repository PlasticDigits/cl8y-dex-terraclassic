# Testing

**Master verification checklist:** [GitLab **#337**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/337) — executable Local/QA scenarios and **REG-00** / **LR-00** regression gates referenced below.

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
| Post-deploy smoke | [#86](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/86), [#368](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/368) | [`make smoke-pool-swap`](../Makefile); wired in [`make start-qa`](../scripts/qa/start-qa.sh) (`QA_SKIP_SMOKE=1` to skip) | LCD `pool` + optional `hybrid_simulation`; pair from `.qa-deploy-stamp` |
| Stubs / mocks catalog | [#105](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/105) | Policy below + issue #105 | LCD stub vs AMM-sim orderbook |
| Charts integration (indexer HTTP) | [#104](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/104), [#230](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/230) | [`ChartsPage.integration.test.tsx`](../frontend-dapp/src/pages/ChartsPage.integration.test.tsx) | Reference job `frontend-charts-integration` → `make test-charts-integration`; **stubbed** `lightweight-charts` — not canvas |
| Price chart real `lightweight-charts` (Vitest) | [#211](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/211), [#229](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/229), [#230](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/230) | `*.charts.test.{ts,tsx}` via `npm run test:charts` | Reference job `frontend-charts-vitest` → `make test-frontend-charts`; large-candle + real visible-range autoscale ([#229](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/229)) |
| Price chart candle parsing + stale pair race | [#226](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/226) | [`priceChartCandles.test.ts`](../frontend-dapp/src/components/charts/__tests__/priceChartCandles.test.ts), [`PriceChart.test.tsx`](../frontend-dapp/src/components/charts/__tests__/PriceChart.test.tsx) | Default `npm run test:run`; no Postgres |

**Post-deploy smoke (#86 / #368):**

```bash
make smoke-pool-swap
# or manually after deploy:
# source scripts/lib/smoke-deploy-env.sh && ./scripts/smoke-pool-swap.sh
```

`make start-qa` runs smoke after `qa-verify-deploy` (skip with `QA_SKIP_SMOKE=1`). `scripts/lib/smoke-deploy-env.sh` resolves `PAIR_ADDR` from `.qa-deploy-stamp` and `OFFER_TOKEN` from the pair `pool` query — no hardcoded testnet addresses.

See also [`docs/deployment-guide.md`](./deployment-guide.md) and [`docs/runbooks/launch-checklist.md`](./runbooks/launch-checklist.md).

## Test Types

### Indexer (Rust)

- **Unit tests (`cargo test --lib`):** parser stress tests, candle OHLC merge invariants, position clamping, oracle `f64` conversion, CG ticker shape validation — **no database required**.
- **Integration tests (`cargo test --tests`):** require PostgreSQL (set `TEST_DATABASE_URL` or use the default URL with valid credentials). They assert API allowlists, caps, CORS, rate limiting (429), sanitized 500 responses, and sanitized **502** LCD bodies ([#239](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/239); [`indexer/tests/security.rs`](../indexer/tests/security.rs), [`skills/AGENTS_INDEXER_API_LCD_SECURITY.md`](../skills/AGENTS_INDEXER_API_LCD_SECURITY.md)).

```bash
cd indexer
cargo test --lib          # fast, no Postgres
cargo test --tests        # needs Postgres + migrations
```

#### Local Postgres setup (agents)

**Agent playbook:** [`skills/AGENTS_LOCAL_POSTGRES_DEV.md`](../skills/AGENTS_LOCAL_POSTGRES_DEV.md) — default user `cl8y_legal`, superuser bootstrap via `setup-postgres-dev-databases.sh`, `make reset` when an old Docker volume still has `postgres:postgres`, and what `deploy-dex-local` writes to `indexer/.env`.

**Stack prerequisite (invariant PG-1):** The indexer and integration tests authenticate as **`cl8y_legal`**. Compose creates that role on a fresh volume (`POSTGRES_USER=cl8y_legal`). External or legacy Postgres that only ships **`postgres:postgres`** must either run [`scripts/setup-postgres-dev-databases.sh`](../scripts/setup-postgres-dev-databases.sh) (auto-creates `cl8y_legal` when `POSTGRES_SUPERUSER` is reachable) or provision the role manually — see [GitLab **#245**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/245) QA note and the skill § Stack prerequisite.

#### Shared Postgres and test parallelism

Integration tests call [`tests/common/mod.rs`](../indexer/tests/common/mod.rs) helpers that **truncate and re-seed** the same database. With default Cargo/Rust test parallelism, multiple integration test **binaries** and multiple **tests per binary** can run concurrently against that DB, which can surface as duplicate unique keys (e.g. on `assets.denom`) or foreign-key violations—not application bugs.

`seed_db` / `clean_db` take an exclusive **file lock** (`/tmp/cl8y-dex-indexer-test.seed.lock`, override with `TEST_DB_LOCK_FILE`) so parallel `cargo test` processes on one host do not interleave truncate/insert (GitLab **#210** orderbook verification). Prefer serialized execution anyway:

When using a **single** shared test database (typical local or QA host), prefer serialized execution:

```bash
cd indexer
export TEST_DATABASE_URL="${TEST_DATABASE_URL:-postgres://cl8y_legal:cl8y_legal@127.0.0.1:5432/dex_indexer_test}"
cargo test --tests -j 1 -- --test-threads=1
```

- **`-j 1`** — run one integration test crate at a time (reduces cross-crate contention).
- **`--test-threads=1`** — run tests inside each binary one at a time (reduces intra-crate contention).

Start Postgres (`docker compose up -d postgres`) and run `./scripts/setup-postgres-dev-databases.sh` (or `make deploy-local`) so `dex_indexer_test` exists before the first run.

**Cursor Cloud Agent (no wasm deploy):** `make setup-indexer-postgres` then `make test-indexer-integration` — see [`AGENTS.md`](../AGENTS.md) § Indexer integration tests (Postgres-only) and [GitLab **#335**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/335).

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

**Cosmes fork patch verification ([GitLab #367](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/367)):** `make test-frontend` runs [`cosmesPatch127.test.ts`](../frontend-dapp/src/services/terraclassic/__tests__/cosmesPatch127.test.ts), which (1) SHA-256-hashes `patches/@goblinhunt+cosmes+*.patch` against committed [`patches/.cosmes-patch-sha256`](../frontend-dapp/patches/.cosmes-patch-sha256) and (2) asserts patched fee-guard symbols exist in `node_modules/@goblinhunt/cosmes/dist/...` after `postinstall` / `patch-package`. Requires a normal `npm ci` (not `--ignore-scripts`). Operator docs: [`docs/frontend.md` § Forked cosmes](./frontend.md#cosmes-fork-patches).

**Regression:** Trade/Charts **price chart** empty-candle UX and `getPairStats` fallback are covered in `src/components/charts/__tests__/PriceChart.test.tsx` (see GitLab [**#113**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/113) and [Trade page — price chart invariants](./frontend.md#trade-page--price-chart-invariants)).

#### Real `lightweight-charts` in Vitest (GitLab #211)

TradingView **[lightweight-charts](https://github.com/tradingview/lightweight-charts)** (open-source canvas library — **not** the hosted TradingView widget) has two Vitest layers:

| Layer | Config / command | What runs |
|-------|------------------|-----------|
| **Fast stub** (default) | `vitest.config.ts` → `npm run test:run` | `lightweightChartsJsdomMock.ts` — React/indexer wiring, `createChart` / `applyOptions` / `addSeries` option capture, `setData` payloads; canvas **contract** + lifecycle in `PriceChartLightweightCanvas.test.tsx` ([#227](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/227), [#225](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/225); stub fidelity epic [#105](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/105)) |
| **Real library** | `vitest.config.charts.ts` → `npm run test:charts` | Imports actual `lightweight-charts`; Node `canvas` shim in `src/test/chartsSetup.ts`; files matching `*.charts.test.{ts,tsx}` (includes post-layout sizing [#225](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/225)) |

```bash
make test-frontend-charts   # from repo root
# or:
bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:charts
```

**Automation:** reference job **`frontend-charts-vitest`** → `make test-frontend-charts` (`npm run test:charts` with Node `canvas` OS deps: `libcairo`, etc.) — isolated from the fast `frontend` unit target so native binding failures do not block 600+ jsdom tests ([#230](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/230), [#211](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/211)). Treat as **required** in release checklists (not optional/skip silently).

**Local `canvas` deps (Ubuntu/Debian):** if `npm run test:charts` fails loading the native module, install the same packages as the reference workflow spec:

```bash
sudo apt-get install -y build-essential libcairo2-dev libgif-dev libjpeg-dev libpango1.0-dev librsvg2-dev
```

**Large-candle ceiling ([#229](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/229)):** Real-library tests always cover **500** and **1500** candles (per-test timeouts up to 25s). A **2000**-candle soak runs **only when `CI` is set** (`it.runIf(process.env.CI)`). Do not add 50k-row cases to CI — local-only if ever needed. Default suite timeout remains **15s** in `vitest.config.charts.ts`; autoscale regressions use the chart’s real `getVisibleLogicalRange()` after `setVisibleLogicalRange`, not a synthetic `original()` alone. Harness: [`chartRealLibraryHarness.ts`](../frontend-dapp/src/test/chartRealLibraryHarness.ts).

**Agent playbook:** [`skills/AGENTS_FRONTEND_PRICE_CHART.md`](../skills/AGENTS_FRONTEND_PRICE_CHART.md). Chart invariants: [Trade page — price chart invariants](./frontend.md#trade-page--price-chart-invariants).

**Do not** load `lightweightChartsJsdomMock.ts` in the charts config. Pure helpers (`priceChartCandles`, `priceChartIndicators`, `priceChartPriceScale`) stay in default unit tests only.

### Integration Tests (Frontend)

Longer-running tests are kept out of the default `npm run test:run` suite. **Charts + indexer HTTP** coverage uses `vitest.config.integration.ts`: tests call a real indexer (`VITE_INDEXER_URL`, default `http://127.0.0.1:3001`) with PostgreSQL migrations applied. They are **not** skipped when the stack is down — the run fails so automation catches broken wiring. E2E and other flows may still use LocalTerra where documented.

**Charts test layers (GitLab #230):**

| Layer | Command / reference job | Validates | Does **not** validate |
|-------|-------------------------|-----------|------------------------|
| Unit (jsdom stub) | `npm run test:run` / `frontend` | React wiring, stub contract ([#227](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/227)) | Real canvas / library |
| Real library Vitest | `npm run test:charts` / `frontend-charts-vitest` | `lightweight-charts` init, `setData`, autoscale, large candles ([#211](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/211), [#229](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/229)) | Indexer HTTP |
| Indexer HTTP integration | `npm run test:integration` / `frontend-charts-integration` | Live candles API → ChartsPage shell ([#104](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/104)) | Canvas render (stubbed) |
| Playwright | `npm run test:e2e` / `e2e` | Full browser + LocalTerra tx path | Chart pixel perf at 50k candles |

**Charts integration (local)**

**Primary path:** from repo root with Postgres and the indexer API on `:3001` running (host Postgres or QA stack):

```bash
make test-charts-integration
```

This runs [`scripts/test-charts-integration.sh`](../scripts/test-charts-integration.sh): ensures the target database exists, applies `sqlx migrate run`, seeds fixtures idempotently, verifies indexer `/health`, then `npm run test:integration` via `scripts/with-node.sh`. Limit-order pool ref tests ([GitLab **#166**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/166)) also need LocalTerra LCD reachable (defaults `http://localhost:1317`; override `VITE_TERRA_LCD_URL` / `VITE_TERRA_RPC_URL` — same as [`frontend-dapp/.env.example`](../frontend-dapp/.env.example)).

**Fixture invariants**

| Invariant | Where |
|-----------|--------|
| Charts pair address `terra1paircontractabc` | [`chartsIntegrationConstants.ts`](../frontend-dapp/src/test/chartsIntegrationConstants.ts) ↔ [`seed-charts-integration.sql`](../indexer/scripts/seed-charts-integration.sql) |
| Fixture candle `open_time` inside indexer default API window (90-day lookback when `from`/`to` omitted) | Seed SQL refreshes to current UTC hour on each run; see [`docs/indexer-invariants.md`](indexer-invariants.md) |
| Limit-order pool-ref pair (EMBER/CORAL) | Resolved from factory via LCD when LocalTerra is up; see [`limitOrderIntegrationConstants.ts`](../frontend-dapp/src/test/limitOrderIntegrationConstants.ts) and GitLab **#166** |

Override the database with `CHARTS_INT_DATABASE_URL` (defaults to `DATABASE_URL` / `dex_indexer` from [`scripts/lib/postgres-dev.env`](../scripts/lib/postgres-dev.env)); override indexer URL with `VITE_INDEXER_URL` (default `http://127.0.0.1:3001`). **Charts** tests (5) need Postgres + indexer only. **Limit-order** tests (2) run when the script resolves the factory pair into `VITE_LIMIT_ORDER_INTEGRATION_*`; otherwise Vitest skips them — full **7/7** needs LocalTerra LCD (`http://127.0.0.1:1317`) after `make deploy-local`. See GitLab [**#205**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/205) and [`skills/AGENTS_TESTING_P2_EPIC.md`](../skills/AGENTS_TESTING_P2_EPIC.md).

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

**Note:** Default Vitest stubs `lightweight-charts` under jsdom via `src/test/lightweightChartsJsdomMock.ts` (including `LineSeries` for MA/RSI lines). The stub records `createChart` / `applyOptions` / `addSeries` (pane index, `autoscaleInfoProvider`) for fast **contract** tests ([#227](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/227)); helpers: `lwChartTestDouble.getLastCreateChartOptions()`, `getLastApplyOptions()`, `getCandlestickAutoscaleProvider()`, `addSeriesCalls`. **Real-library** chart init, `setData`, indicators, volume fallback, USD autoscale (including visible-range zoom via real `getVisibleLogicalRange()` — [#229](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/229)), and large-candle perf guards run in `npm run test:charts` ([#211](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/211)). Pure clamp math stays in `priceChartPriceScale.test.ts` ([#151](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/151)). Pixel-level zoom/pan and layout regressions remain Playwright / manual QA.

Config: `vitest.config.integration.ts`

### E2E Tests (Playwright)

Full browser tests against the running dApp + LocalTerra. **Strict on-chain policy** ([GitLab **#201**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/201), [**#103**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/103)): default strict automation/local tx specs **fail** when LCD, funds, routes, or pairs are missing — no silent `test.skip` for those gaps.

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

#### Price chart Playwright smoke (GitLab #228) {#price-chart-playwright-smoke-gitlab-228}

Browser regression for **lightweight-charts** canvas presence and **fullscreen** aria toggles — complements Vitest ([#211](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/211)) and layout-only trade specs ([#146](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/146)).

| Layer | Spec / helper | What it proves |
|-------|----------------|----------------|
| **Strict automation / local** | `e2e/price-chart-smoke.spec.ts` (`e2e-smoke` project) | `/charts` and `/trade` mount `price-chart-lightweight-canvas` + child `canvas`; interval `1h`→`1d` keeps canvas; read-only chart without wallet |
| **Fullscreen (no indexer)** | Same file, mocked Fullscreen API via `e2e/helpers/price-chart.ts` | `aria-label` **Expand…** / **Exit…**, `aria-pressed`, denied enter does not remove button |
| **UI-only skip** | `PLAYWRIGHT_SKIP_CHAIN=1` | Entire spec **skipped** (trade workspace + indexer required for toolbar and canvas) |
| **Outage regression** | `*-indexer-outage.spec.ts` (separate job) | Unchanged — `trade-chart-unavailable` when indexer stopped ([#165](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/165), [#219](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/219)); swap banner ([#241](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/241)) |

```bash
# After LocalTerra + deploy-dex-local.sh + indexer (see scripts/e2e-start-indexer.sh):
bash scripts/e2e-start-indexer.sh
bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/price-chart-smoke.spec.ts --project=e2e-smoke
```

Reference job **`e2e`** (local: Postgres + `deploy-dex-local.sh` + `bash scripts/e2e-start-indexer.sh` + `make test-e2e`) starts the stack, then runs full Playwright ([#228](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/228)).

Chart invariants: [Trade page — price chart invariants](./frontend.md#trade-page--price-chart-invariants). Agent: [`skills/AGENTS_FRONTEND_PRICE_CHART.md`](../skills/AGENTS_FRONTEND_PRICE_CHART.md).

**Manual QA crosswalk** ([`QA_TEMPLATE.md`](../QA_TEMPLATE.md) §5.1 — GitLab #228 verification):

| QA row | Automated? | Where |
|--------|------------|-------|
| 5.1.1 Chart loads (`/charts`) | Yes | `e2e/price-chart-smoke.spec.ts` — `price-chart-lightweight-canvas` + child `canvas` |
| 5.1.5 / 5.1.7 Interval (1h / 1d) | Partial | Same spec — `/trade` interval click 1h→1d keeps canvas |
| 5.1.10 Loading state | Partial | Mobile viewport test — canvas **or** “Loading chart…” |
| 5.1.11 Error / outage | Yes (regression) | `e2e/*-indexer-outage.spec.ts` (separate job; not `price-chart-smoke`) |
| 5.1.12 Zoom / scroll | **Manual** | [#211](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/211) — no pixel assertions in Playwright |
| Fullscreen aria | Yes | `price-chart-smoke.spec.ts` + `e2e/helpers/price-chart.ts` (mocked Fullscreen API) |

#### Frontend E2E — indexer outage {#frontend-e2e-indexer-outage}

Market-data-down Playwright specs live in project **`e2e-indexer-outage`** (`**/*-indexer-outage.spec.ts`). They require the indexer HTTP API to be **stopped** while LocalTerra/Vite remain up, with **`E2E_INDEXER_OUTAGE=1`**. Default `npm run test:e2e` and the strict **`e2e`** reference job **exclude** this project — avoids flaking the strict chain suite ([#201](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/201)).

| Layer | Target |
|-------|--------|
| **Automation** | `make test-e2e-indexer-outage` / [`scripts/test-e2e-indexer-outage.sh`](../scripts/test-e2e-indexer-outage.sh) (workflow job name **`frontend-e2e-indexer-outage`** in [`.github/workflows/test.yml`](../.github/workflows/test.yml) is a portable spec only — this repo does not run GitHub Actions) |
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

**Env vars:** `E2E_INDEXER_OUTAGE=1` (required for specs to run); `VITE_E2E_INDEXER_OUTAGE=1` (set automatically — fast indexer transport failure in the browser); `E2E_TRADE_PAIR` (optional — defaults to first deploy pair via `.qa-deploy-stamp` / factory LCD); `VITE_INDEXER_URL` (default `http://127.0.0.1:3001`).

After sanity on `:3001`, Playwright runs with **`OUTAGE_E2E_INDEXER_URL`** (default `http://127.0.0.1:39991`, nothing listening) so a shared host cannot auto-restart the QA indexer on `:3001` and produce a false green. Local and reference-job paths use the same script: [`scripts/test-e2e-indexer-outage.sh`](../scripts/test-e2e-indexer-outage.sh).

**Local QA stack:** If an indexer is already listening on `:3001` (e.g. `make qa-start`), `test-e2e-indexer-outage.sh` reuses it for the sanity check, then stops **every** process bound to that port before Playwright. Restart afterward with `bash scripts/e2e-start-indexer.sh` or `make qa-start` (indexer only) if other work needs the API.

Vitest covers Charts/Trader/Pool/**Limits**/ **Swap** outage banners with mocked transport errors (`npm run test:run`; `/limits`: [`LimitOrdersPage.test.tsx`](../frontend-dapp/src/pages/LimitOrdersPage.test.tsx), GitLab **#218**; `/`: [`SwapPage.test.tsx`](../frontend-dapp/src/pages/SwapPage.test.tsx), GitLab **#241**). Product invariants: [docs/frontend.md § Market data loading & outage](./frontend.md#market-data-loading-outage); agent: [`skills/AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md`](../skills/AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md). Tracking: [GitLab **#219**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/219).

**Header / tablet compact nav:** `e2e/navigation.spec.ts` asserts no horizontal overlap for the **Swap + More** row at 773×743, **1024–1098px** (follow-up cram band), and other tablet widths; the full primary row at 1280px; desktop **Swap → Pool → Trade** tab transitions without reload at 1440px ([GitLab **#182**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/182)); and that **“Terra Classic ecosystem”** does not appear in the shell (header brand is logo + title only). Invariants: [docs/frontend.md § Responsive shell & header navigation](./frontend.md#responsive-header-navigation) ([GitLab **#136**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/136)); shell nav playbook [`skills/AGENTS_FRONTEND_SHELL_NAV.md`](../skills/AGENTS_FRONTEND_SHELL_NAV.md).

**Connected wallet chip — network label:** same spec file — desktop **`Local`** short label on the trigger at 1280px, mobile LUNC without visible network text, connected chip vs **More** non-overlap at 773px ([GitLab **#186**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/186); [docs/frontend.md § Connected wallet chip — network & mobile](./frontend.md#connected-wallet-chip-network-mobile)).

**Local stack for strict on-chain tests (default `e2e` automation path):**

LocalTerra must be **terrad v4 / SDK 0.53** with a fresh volume after digest bumps — see [`docs/localterra-sdk53.md`](./localterra-sdk53.md) ([GitLab **#292**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/292)).

1. `docker compose up -d localterra` (or `make reset && make start && make wait-healthy` after image bump)
2. From repo root: `bash scripts/deploy-dex-local.sh` (writes `frontend-dapp/.env.local`, deploys contracts, seeds CW20 balances on the dev account `terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v`).
3. `make test-e2e` or `bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:e2e`

**Cloud Agent VM:** global setup runs deploy/e2e shell scripts that need `docker exec` when host `:1317` hangs — wrap as `sg docker -c 'CI=1 make test-e2e'` (see [`AGENTS.md`](../AGENTS.md) and [`docs/localterra-sdk53.md`](./localterra-sdk53.md) **LT11** / **LT12**). Install Playwright Chromium via `./node_modules/.bin/playwright install chromium` (not bare `npx playwright`). Tx project stays at **1 worker**; smoke uses **5 workers**.

Before tests, **`e2e/global-setup.ts`** waits for the LCD and runs **`scripts/e2e-provision-dev-wallet.sh`**, which **mints factory CW20s** to the dev wallet when any listed token balance is below **`E2E_DEV_MIN_CW20_U128`** (default `10000000000000` raw units), then **`scripts/e2e-seed-hybrid-book.sh`**, which idempotently places a **resting bid** on the first dual-CW20 pair when the bid book head is empty (GitLab [**#193**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/193)). **Invariant:** pair `OrderBookHead` returns a bare **`u64`** on LCD (`{"data":13}`), not `{ head_order_id }`; the seed script must treat an existing head as success so global setup can re-run ([GitLab **#138**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/138)). Native gas denoms **uluna** / **uusd** are expected from genesis ([`docker/init-chain.sh`](../docker/init-chain.sh) — **10M LUNC** on SDK 0.53 LocalTerra; GitLab [**#372**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/372)), not from the provision script.

**Single-file pool tx run (documented in `frontend-dapp/e2e/README.md`):**

```bash
bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/pool-tx.spec.ts --project=e2e-tx
```

**Hybrid swap E2E (strict tx path, GitLab [#193](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/193)):** `bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/hybrid-swap.spec.ts --project=e2e-tx` — requires global-setup seeding. See [`frontend-dapp/e2e/README.md`](../frontend-dapp/e2e/README.md) and [`skills/AGENTS_E2E_HYBRID_SWAP.md`](../skills/AGENTS_E2E_HYBRID_SWAP.md).

**Limit order tx E2E (strict place + cancel, GitLab [#195](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/195)):** `bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/limit-orders-tx.spec.ts --project=e2e-tx` — first **unpaused** dual-CW20 pair via LCD `is_paused`. See [`frontend-dapp/e2e/README.md`](../frontend-dapp/e2e/README.md) and [`skills/AGENTS_E2E_LIMIT_ORDERS_TX.md`](../skills/AGENTS_E2E_LIMIT_ORDERS_TX.md).

**Claim all parked tx E2E (GitLab [#259](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/259)):** `bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/limit-orders-claim-all-tx.spec.ts --project=e2e-tx` — expiry-park harness + batch claim confirm gas copy. Requires indexer + [`scripts/e2e-seed-expired-parked-claim-all.sh`](../scripts/e2e-seed-expired-parked-claim-all.sh). See [`skills/AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md`](../skills/AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md).

**Optional chain (skip instead of fail):** set `PLAYWRIGHT_SKIP_CHAIN=1` (or legacy `REQUIRE_LOCALTERRA=0`) for local UI-only runs (`npm run test:e2e:smoke`). **Do not** set this in release automation checklists. Default is strict (unset).

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

**Python QA swarm (`make swarm-launch`):** separate from the TypeScript package — **25** swap workers + **5** limit makers + **3** `provide_liquidity` workers. All workers broadcast from **`test1`** (same mnemonic as the Simulated Wallet / Playwright dev wallet). Genesis and deploy seeds were raised **10×** so a full QA day with swarm volume does not exhaust LUNC gas ([#372](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/372)); after genesis changes run **`make reset`** before redeploy. `launch-swarm.sh` runs `bootstrap-swarm-liquidity` once so OE-1 swap pairs (EMBER/CORAL, TOPAZ/ONYX, ONYX/CORAL) stay deep after swap-only volume ([#293](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/293)). Unit tests: `make test-swarm-liquidity`. Funding headroom check (needs fresh reset + deploy): `make verify-localterra-funding-headroom`. Live OE-1 quote check (needs LocalTerra + indexer): `make verify-issue-293` — **acceptance** is `pool_only=true` direct-pool reciprocal (≤5%), `route/solve` **slippage enrichment** (`slippage_percent` + `spot_amount_out` present and math-consistent; at least one EMBER→CORAL direction **>30%** so retail Expert Mode guard is exercisable), and global best-execution route asymmetry traced (multi-hop vs direct is expected on LocalTerra, not a Swap decimal bug). **Prerequisite:** indexer DB must not retain stale duplicate quote assets from an earlier deploy — if `make verify-issue-293` reports missing slippage fields, rerun `make setup-cloud-localterra --fresh`.

### Fee Discount Contract Tests

Canonical tier numbers: [`docs/reference/fee-discount-tiers.md`](reference/fee-discount-tiers.md) (GitLab [#198](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/198)). Drift check: `make check-fee-discount-tier-docs`. Agent playbook: [`skills/AGENTS_FEE_DISCOUNT_TIERS.md`](../skills/AGENTS_FEE_DISCOUNT_TIERS.md). **Registry outage observability** ([#365](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/365), [#375](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/375)): integrator guidance in [`docs/integrators.md` § Fee-discount registry outage](./integrators.md#fee-discount-registry-outage); regression ladder `make verify-issue-365` (contract P5 test + indexer health API + frontend warning util — Postgres bootstrap via `make setup-indexer-postgres` when `indexer/.env` is missing; no LocalTerra).

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
- Factory `SetDiscountRegistryAll`: succeeds when `PAIR_COUNT` ≤ 10; rejects with `DiscountRegistryAllTooManyPairs` when larger ([GitLab #242](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/242), `test_set_discount_registry_all_rejects_when_pair_count_exceeds_cap`)
- Factory `SetDiscountRegistryBatch`: verify cursor `next_start_after` + `has_more` advance until complete ([GitLab #123](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/123), [#242](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/242) — `test_set_discount_registry_batch_covers_many_pairs` for 25 pairs / limit 10)
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

## CI {#ci}

**Invariants ([GitLab #234](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/234)):**

| Concept | Canonical wording |
|--------|-------------------|
| Where checks run | **Local / QA host** — `make …`, `scripts/*.sh` |
| [`.github/workflows/*.yml`](../.github/workflows/) | **Reference spec only** (job names, services, step order) |
| Job names (`e2e`, `frontend-charts-integration`, …) | **Labels** mapping to Make/scripts below |
| “CI green on main” | **Local automation checklist passed** or the named Make target |
| Hosted runners | **None today** — this repo does **not** run GitHub Actions or GitLab CI |

**Agents:** Do not tell third parties to wait for GitHub Actions on `main`. Point to [`docs/testing.md` § CI](#ci), [`.github/workflows/README.md`](../.github/workflows/README.md), and the relevant `skills/AGENTS_*.md` playbook.

### Reference job → local command

| Reference job (`test.yml`) | Local command |
|--------------------------|---------------|
| `docs-fee-discount-tiers` | `make check-fee-discount-tier-docs` |
| `contracts-terra` | `make lint-contracts` && `make test-contracts` (optional LCOV: `make coverage-contracts`) |
| `localnet-trading-swarm` | `cd packages/localnet-trading-swarm && npm ci && npx tsc -p tsconfig.json && npm run test:run` |
| `frontend` | `bash scripts/with-node.sh --cwd frontend-dapp -- npx tsc --noEmit` && `make lint-frontend` && `make test-frontend` |
| `frontend-charts-vitest` | `make test-frontend-charts` ([#230](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/230), [#211](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/211)) |
| `frontend-charts-integration` | `make test-charts-integration` ([#205](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/205)) |
| `indexer` | Postgres + `cd indexer && cargo fmt --check && cargo clippy -- -D warnings && cargo test` (integration: [§ Shared Postgres](#shared-postgres-and-test-parallelism)) |
| `e2e` | `make wait-localterra` → `bash scripts/deploy-dex-local.sh` → `make qa-verify-deploy` → `bash scripts/e2e-start-indexer.sh` → `make test-e2e` |
| `frontend-e2e-indexer-outage` | `make test-e2e-indexer-outage` ([#219](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/219)) |

**Wasm (release):** reference job in [`contracts-wasm-optimizer.yml`](../.github/workflows/contracts-wasm-optimizer.yml) → `make build-optimized`. Fast `cargo wasm` in `test.yml` is dev-only; see [deployment guide § Build Optimized WASM](./deployment-guide.md#1-build-optimized-wasm).

The reference workflow [`.github/workflows/test.yml`](../.github/workflows/test.yml) also documents step order for: contract `cargo fmt` / `clippy` / `llvm-cov`, indexer Postgres service container, and Playwright browser install. Full mapping: [`.github/workflows/README.md`](../.github/workflows/README.md).

**Agent playbooks:** [`skills/AGENTS_E2E_STRICT_CHAIN.md`](../skills/AGENTS_E2E_STRICT_CHAIN.md), [`skills/AGENTS_E2E_INDEXER_OUTAGE.md`](../skills/AGENTS_E2E_INDEXER_OUTAGE.md), [`skills/AGENTS_LOCAL_POSTGRES_DEV.md`](../skills/AGENTS_LOCAL_POSTGRES_DEV.md), [`skills/AGENTS_TESTING_P2_EPIC.md`](../skills/AGENTS_TESTING_P2_EPIC.md).

## Writing Tests

- Place unit tests next to source files: `MyComponent.test.tsx`
- Place integration tests next to source files: `MyComponent.integration.test.tsx`
- Place E2E tests in `frontend-dapp/e2e/`
- Use `renderWithProviders()` from `src/test/helpers.tsx` for component tests
- Use the dev-wallet fixture from `e2e/fixtures/dev-wallet.ts` for E2E tests
