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
