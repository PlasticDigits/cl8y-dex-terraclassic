# Agent playbook: P2 testing epic (GitLab #199)

**Epic:** [GitLab **#199**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/199) — consolidated P2 test gaps from the production review ([`TEST_GAP_MATRIX.md`](../docs/reviews/20260409T030009Z/TEST_GAP_MATRIX.md)).

Use this when adding or extending coverage for hybrid indexing, fee discount on book legs, frontend hybrid message shapes, pause UX, post-deploy smoke, stub policy, or charts integration.

**Local automation only** — no hosted GitHub/GitLab CI ([#234](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/234)). Reference job → Make mapping: [docs/testing.md § CI](../docs/testing.md#ci).

## Coverage map

| Area | Sub-issue | Automated evidence | Manual gate |
|------|-----------|-------------------|-------------|
| Indexer hybrid wasm attrs on `swap_events` | [#82](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/82) | [`indexer/tests/swap_events_hybrid_columns.rs`](../indexer/tests/swap_events_hybrid_columns.rs); parser unit tests in [`parser.rs`](../indexer/src/indexer/parser.rs) | — |
| Fee discount on **book** leg | [#83](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/83) | `limit_order_tests::hybrid_book_fill_uses_taker_discounted_effective_fee_bps` | — |
| `SwapPage` / router **hybrid message shape** | [#84](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/84) | [`pair.test.ts`](../frontend-dapp/src/services/terraclassic/__tests__/pair.test.ts) (direct CW20 `send` → `swap`); [`router.hybrid.test.ts`](../frontend-dapp/src/services/terraclassic/router.hybrid.test.ts) (`execute_swap_operations`); [`routeOperations.test.ts`](../frontend-dapp/src/services/indexer/__tests__/routeOperations.test.ts) (indexer merge) | — |
| **Pause** blocks swap + limits + cancel UX | [#87](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/87) | `limit_order_tests::pause_blocks_swap_and_place_cancel_refunds_escrow` (L6); [`TradePage.test.tsx`](../frontend-dapp/src/pages/TradePage.test.tsx) pause banner + disabled limit CTA | E2E: strict specs skip/fail on paused pair via [`hybrid-e2e.ts`](../frontend-dapp/e2e/helpers/hybrid-e2e.ts) |
| Post-deploy **smoke** | [#86](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/86), [#368](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/368) | `make smoke-pool-swap`; wired in `make start-qa` | [`scripts/smoke-pool-swap.sh`](../scripts/smoke-pool-swap.sh) + [`scripts/lib/smoke-deploy-env.sh`](../scripts/lib/smoke-deploy-env.sh) |
| **Stubs / mocks catalog** | [#105](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/105) | Policy in [`docs/testing.md`](../docs/testing.md) | Review stub list when adding new test doubles |
| **Charts** integration (not jsdom-only) | [#104](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/104), [#230](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/230) | [`ChartsPage.integration.test.tsx`](../frontend-dapp/src/pages/ChartsPage.integration.test.tsx); reference job **`frontend-charts-integration`** | Local: `make test-charts-integration` — [`docs/testing.md` § Integration Tests (Frontend)](../docs/testing.md#integration-tests-frontend); GitLab [#205](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/205) |
| **Charts** real library Vitest | [#211](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/211), [#230](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/230) | `*.charts.test.{ts,tsx}`; reference job **`frontend-charts-vitest`** | Local: `make test-frontend-charts`; Ubuntu `canvas` deps in [`docs/testing.md`](../docs/testing.md#real-lightweight-charts-in-vitest-gitlab-211) |

## Invariants cross-links

| Invariant | Doc | Tests |
|-----------|-----|-------|
| Hybrid swap attrs indexed | [`indexer-invariants.md`](../docs/indexer-invariants.md) — hybrid swap columns | `#82` integration + parser |
| Book leg fee discount | **L4** / fee composition — [`contracts-security-audit.md`](../docs/contracts-security-audit.md) | `#83` |
| Pause freezes trading | **L6** — [`contracts-security-audit.md`](../docs/contracts-security-audit.md) | `#87`; [`AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md`](./AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md) |
| Hybrid quote = execute | **L8** — [`AGENTS_TESTING_MULTIHOP_HYBRID.md`](./AGENTS_TESTING_MULTIHOP_HYBRID.md) | Multihop router + indexer (#192) |
| Charts integration fixture pair | [`docs/testing.md`](../docs/testing.md#integration-tests-frontend) — `CHARTS_INTEGRATION_PAIR_ADDRESS` | `#104` / `#205`; seed SQL + Vitest constant must match |
| Charts fixture candle window | Seed SQL + indexer `DEFAULT_CANDLE_LOOKBACK_DAYS` (90d) | [`docs/indexer-invariants.md`](../docs/indexer-invariants.md); re-seed moves `open_time` to current hour |
| Limit-order integration pair | [`limitOrderIntegrationConstants.ts`](../frontend-dapp/src/test/limitOrderIntegrationConstants.ts); `make test-charts-integration` queries factory | `#166`; needs LocalTerra + `make deploy-local` for 7/7 |
| Charts Vitest layers (HTTP vs canvas) | [`docs/testing.md`](../docs/testing.md#integration-tests-frontend) — matrix ([#230](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/230)) | `#104` integration stub; `#211` / `#229` real library |

## Commands

Indexer integration tests need Postgres — see [`AGENTS_LOCAL_POSTGRES_DEV.md`](./AGENTS_LOCAL_POSTGRES_DEV.md) (`cl8y_legal`, `indexer/.env`, serialized `-j 1`).

```bash
# Indexer hybrid columns (#82) — needs Postgres
cd indexer && cargo test --test swap_events_hybrid_columns -j 1 -- --test-threads=1

# Contract book-leg discount + pause (#83, #87)
cd smartcontracts
cargo test hybrid_book_fill_uses_taker_discounted_effective_fee_bps
cargo test pause_blocks_swap_and_place_cancel_refunds_escrow

# Frontend hybrid msgs + pause banner (#84, #87)
cd frontend-dapp && npm run test:run -- router.hybrid.test.ts TradePage.test.tsx pair.test.ts

# Real lightweight-charts Vitest (#211)
make test-frontend-charts

# Charts integration (#104, #205) — Postgres seed + indexer on :3001
make test-charts-integration
# Override DB or indexer URL when needed:
# CHARTS_INT_DATABASE_URL=postgres://... VITE_INDEXER_URL=http://127.0.0.1:3001 make test-charts-integration

# Post-deploy smoke (#86) — manual after deploy
make smoke-pool-swap
# QA_SKIP_SMOKE=1 make start-qa  # skip smoke during indexer-only debugging
```

## Stub policy (summary)

Full catalog: [GitLab **#105**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/105) and [`docs/testing.md`](../docs/testing.md).

| Stand-in | Location | Production analogue |
|----------|----------|---------------------|
| Wiremock LCD | `indexer/tests/common/lcd_mock.rs` | Real Terra LCD |
| Hybrid CG/CMC orderbook | `hybrid_orderbook_sim.rs` + `orderbook_sim.rs` (**#220** pool+limits, **#210** pool leg); playbook [`AGENTS_INDEXER_AMM_ORDERBOOK_SIM.md`](./AGENTS_INDEXER_AMM_ORDERBOOK_SIM.md); compliance [`docs/CG_CMC_COMPLIANCE.md`](../docs/CG_CMC_COMPLIANCE.md) (**#224**, CMC array **#223**); tests `api_cmc.rs` + `api_orderbook_lcd_mock.rs` | Live CEX L2; raw FIFO without pool (`limit-book` API) |
| Vitest `vi.mock` (Terra services) | `frontend-dapp/src/**/__tests__` | Wallet + LCD I/O |
| `lightweightChartsJsdomMock.ts` | jsdom unit tests (`test:run`); records `createChart` / `applyOptions` / `addSeries` (pane index, `autoscaleInfoProvider`) — contract tests [#227](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/227); stub catalog [#105](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/105) | Browser `lightweight-charts` |
| `vitest.config.charts.ts` + `chartsSetup.ts` | Real library Vitest ([#211](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/211)) | Same; `npm run test:charts` |
| E2E `REQUIRE_LOCALTERRA=0` | Playwright global setup | Full LocalTerra stack |

**Rule:** Do not add `#[ignore]` / permanent skips without a linked follow-up issue. Prefer strict fail in default local automation (reference job `e2e`) when the stack is required.

## When to extend this epic

Add rows here (update `#199` checklist) when:

- New hybrid wasm attrs appear on pair swap events → extend parser + `swap_events_hybrid_columns.rs`.
- Router or SwapPage changes hybrid param serialization → extend `router.hybrid.test.ts` / `pair.test.ts`.
- Pause semantics change (L6) → contract test + Trade/Limits Vitest + E2E helpers.
- New chart/indexer HTTP coupling → extend integration Vitest config, not jsdom-only mocks alone.

## Related playbooks

- [`AGENTS_LOCAL_POSTGRES_DEV.md`](./AGENTS_LOCAL_POSTGRES_DEV.md) — local Postgres setup for indexer tests
- [`AGENTS_TESTING_MULTIHOP_HYBRID.md`](./AGENTS_TESTING_MULTIHOP_HYBRID.md) — L8 multihop (#192)
- [`AGENTS_E2E_HYBRID_SWAP.md`](./AGENTS_E2E_HYBRID_SWAP.md) — strict hybrid tx E2E (#193)
- [`AGENTS_E2E_LIMIT_ORDERS_TX.md`](./AGENTS_E2E_LIMIT_ORDERS_TX.md) — limit place/cancel tx (#195)
- [`AGENTS_FRONTEND_PRICE_CHART.md`](./AGENTS_FRONTEND_PRICE_CHART.md) — chart UX (#113, #104)
