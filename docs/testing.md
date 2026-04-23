# Testing

## Philosophy

CL8Y DEX tests focus on real contract behavior — no blockchain mocks. Unit tests exercise pure logic, integration tests deploy to a simulated chain environment, and E2E tests drive the actual frontend against LocalTerra.

## Test Types

### Indexer (Rust)

- **Unit tests (`cargo test --lib`):** parser stress tests, candle OHLC merge invariants, position clamping, oracle `f64` conversion, CG ticker shape validation — **no database required**.
- **Integration tests (`cargo test --tests`):** require PostgreSQL (set `TEST_DATABASE_URL` or use the default URL with valid credentials). They assert API allowlists, caps, CORS, rate limiting (429), and sanitized 500 responses.

```bash
cd indexer
cargo test --lib          # fast, no Postgres
cargo test --tests        # needs Postgres + migrations
```

#### Shared Postgres and test parallelism

Integration tests call [`tests/common/mod.rs`](../indexer/tests/common/mod.rs) helpers that **truncate and re-seed** the same database. With default Cargo/Rust test parallelism, multiple integration test **binaries** and multiple **tests per binary** can run concurrently against that DB, which can surface as duplicate unique keys (e.g. on `assets.denom`) or foreign-key violations—not application bugs.

When using a **single** shared test database (typical local or CI), prefer serialized execution:

```bash
cd indexer
export TEST_DATABASE_URL="${TEST_DATABASE_URL:-postgres://postgres:postgres@127.0.0.1:5432/dex_indexer_test}"
cargo test --tests -j 1 -- --test-threads=1
```

- **`-j 1`** — run one integration test crate at a time (reduces cross-crate contention).
- **`--test-threads=1`** — run tests inside each binary one at a time (reduces intra-crate contention).

Start Postgres (for example `docker compose up -d postgres` from the repo root) and ensure the target database exists (e.g. `CREATE DATABASE dex_indexer_test;`) before the first run.

See [Indexer invariants](./indexer-invariants.md) for the full matrix and the same note under **Running tests**.

**Stubs, mocks, and test stand-ins:** intentional test doubles (Wiremock LCD, Vitest `vi.mock`, placeholder addresses, E2E skips, and how they differ from production paths like the AMM **simulated** orderbook) are cataloged in [GitLab issue #105](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/105). Key indexer spots: `indexer/tests/common/lcd_mock.rs` (LCD HTTP stub only) vs `indexer/src/api/orderbook_sim.rs` (curve-walk depth, not the on-chain FIFO book).

### Unit Tests (Rust)

Test individual contract functions in isolation using `cosmwasm_std::testing` helpers.

```bash
cd smartcontracts
cargo test
```

### Unit Tests (Frontend)

Test React components and hooks with Vitest and jsdom. **CosmWasm / LCD I/O** is typically **stubbed at the service layer** so unit tests stay fast and deterministic. That does **not** replace integration coverage for features that depend on indexer HTTP or chart data: use the **integration** Vitest config (below) or dedicated issues (e.g. GitLab [**#104**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/104) for charts).

```bash
cd frontend-dapp
npm run test:run          # single run
npm run test              # watch mode
```

Config: `vitest.config.ts`

**Regression:** Trade/Charts **price chart** empty-candle UX and `getPairStats` fallback are covered in `src/components/charts/__tests__/PriceChart.test.tsx` (see GitLab [**#113**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/113) and [Trade page — price chart invariants](./frontend.md#trade-page--price-chart-invariants)).

### Integration Tests (Frontend)

Longer-running tests are kept out of the default `npm run test:run` suite. **Charts + indexer HTTP** coverage uses `vitest.config.integration.ts`: tests call a real indexer (`VITE_INDEXER_URL`, default `http://127.0.0.1:3001`) with PostgreSQL migrations applied. They are **not** skipped when the stack is down — the run fails so CI catches broken wiring. E2E and other flows may still use LocalTerra where documented.

**Charts integration (local)**

1. Start PostgreSQL (for example `docker compose up -d postgres` from the repo root).
2. Create a database (once): `CREATE DATABASE cl8y_charts_int;` (name can match your `DATABASE_URL`).
3. Run migrations and seed minimal pair + candles:

   ```bash
   export DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/cl8y_charts_int
   cd indexer && sqlx migrate run && psql "$DATABASE_URL" -f scripts/seed-charts-integration.sql
   ```

   The seeded pair address is `terra1paircontractabc` (kept in sync with `frontend-dapp/src/test/chartsIntegrationConstants.ts`).

4. Start the indexer API (same `DATABASE_URL` plus required env from `indexer/.env.example`: at minimum `FACTORY_ADDRESS`, `CORS_ORIGINS`, `LCD_URLS`).

5. Run Vitest integration:

   ```bash
   cd frontend-dapp
   VITE_INDEXER_URL=http://127.0.0.1:3001 npm run test:integration
   ```

**Note:** `lightweight-charts` is stubbed under jsdom via `src/test/lightweightChartsJsdomMock.ts` so Node-based Vitest stays stable; the real chart library runs in the browser (manual QA / Playwright).

Config: `vitest.config.integration.ts`

### E2E Tests (Playwright)

Full browser tests against the running dApp + LocalTerra.

```bash
cd frontend-dapp
npx playwright test           # headless
npx playwright test --ui      # interactive UI
```

Config: `playwright.config.ts`

**Local stack for strict on-chain tests (default CI path):**

1. `docker compose up -d localterra`
2. From repo root: `bash scripts/deploy-dex-local.sh` (writes `frontend-dapp/.env.local`, deploys contracts, seeds CW20 balances on the dev account `terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v`).
3. `cd frontend-dapp && npx playwright test`

Before tests, **`e2e/global-setup.ts`** waits for the LCD and runs **`scripts/e2e-provision-dev-wallet.sh`**, which **mints factory CW20s** to the dev wallet when any listed token balance is below **`E2E_DEV_MIN_CW20_U128`** (default `1000000000000` raw units). Native gas denoms **uluna** / **uusd** are expected from genesis (`docker/init-chain.sh`), not from the script.

**Single-file pool tx run (documented in `frontend-dapp/e2e/README.md`):**

```bash
cd frontend-dapp
pnpm exec playwright test e2e/pool-tx.spec.ts
# or: npx playwright test e2e/pool-tx.spec.ts
```

**Optional chain (skip instead of fail):** set `REQUIRE_LOCALTERRA=0` so global setup and strict LCD/pool assertions are relaxed — for jobs that intentionally omit LocalTerra. Default is strict (unset or any value other than `0`).

### Fee Discount Contract Tests

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

- End-to-end swap with discount: register a tier, execute swap, verify reduced commission
- Swap without registration: verify full fee applied
- Balance drop: transfer CL8Y away, swap, verify discount revoked and deregistration fired
- Router trusted forwarding: swap via Router passes trader address correctly
- Factory `SetDiscountRegistryAll`: verify all pairs receive the registry address
- Blacklist (Tier 255): verify wallet receives zero discount

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
3. **Frontend charts integration:** PostgreSQL service → `sqlx migrate run` → `seed-charts-integration.sql` → release indexer binary → `npm run test:integration` against `http://127.0.0.1:3001`

See [the workflow file](../.github/workflows/test.yml) for details.

## Writing Tests

- Place unit tests next to source files: `MyComponent.test.tsx`
- Place integration tests next to source files: `MyComponent.integration.test.tsx`
- Place E2E tests in `frontend-dapp/e2e/`
- Use `renderWithProviders()` from `src/test/helpers.tsx` for component tests
- Use the dev-wallet fixture from `e2e/fixtures/dev-wallet.ts` for E2E tests
