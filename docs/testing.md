# Testing

## Philosophy

CL8Y DEX tests focus on real contract behavior — no blockchain mocks. Unit tests exercise pure logic, integration tests deploy to a simulated chain environment, and E2E tests drive the actual frontend against LocalTerra.

## Test Types

### Unit Tests (Rust)

Test individual contract functions in isolation using `cosmwasm_std::testing` helpers.

```bash
cd smartcontracts
cargo test
```

### Unit Tests (Frontend)

Test React components and hooks with Vitest and jsdom. Contract calls are stubbed at the service layer.

```bash
cd frontend-dapp
npm run test:run          # single run
npm run test              # watch mode
```

Config: `vitest.config.ts`

### Integration Tests (Frontend)

Longer-running tests that interact with a running LocalTerra instance. Separated to avoid slowing down the unit test suite.

```bash
cd frontend-dapp
npx vitest run --config vitest.config.integration.ts
```

Config: `vitest.config.integration.ts`

### E2E Tests (Playwright)

Full browser tests against the running dApp + LocalTerra.

```bash
cd frontend-dapp
npx playwright test           # headless
npx playwright test --ui      # interactive UI
```

Config: `playwright.config.ts`

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

```bash
cd frontend-dapp
npx vitest run --coverage
```

Coverage reports are generated in `frontend-dapp/coverage/` in text, JSON, and HTML formats (configured via `vitest.config.ts`).

## CI

The GitHub Actions workflow (`.github/workflows/test.yml`) runs:
1. `cargo fmt --check` + `cargo clippy` + `cargo test` + WASM builds
2. `tsc --noEmit` + `npm run lint` + `npm run test:run`

See [the workflow file](../.github/workflows/test.yml) for details.

## Writing Tests

- Place unit tests next to source files: `MyComponent.test.tsx`
- Place integration tests next to source files: `MyComponent.integration.test.tsx`
- Place E2E tests in `frontend-dapp/e2e/`
- Use `renderWithProviders()` from `src/test/helpers.tsx` for component tests
- Use the dev-wallet fixture from `e2e/fixtures/dev-wallet.ts` for E2E tests
