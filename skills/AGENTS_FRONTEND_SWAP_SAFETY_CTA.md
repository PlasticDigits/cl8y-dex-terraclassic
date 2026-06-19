# Agent playbook: Swap wrap safety CTA copy (SEC-A02)

Use when changing **wrap mapper pause**, **on-chain wrap rate limit**, or **swap submit button** gating on `/` ([GitLab **#389**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/389), launch checklist SEC-A02).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [`SwapPage.tsx`](../frontend-dapp/src/pages/SwapPage.tsx) | Submit CTA precedence: wrap pause → blacklist → amount → rate limit |
| [`wrapMapper.ts`](../frontend-dapp/src/services/terraclassic/wrapMapper.ts) | `queryPausedState`, `checkRateLimitExceeded` |
| [`SwapPage.test.tsx`](../frontend-dapp/src/pages/SwapPage.test.tsx) | Vitest: exact copy + `toBeDisabled()` per state (isolated mocks) |
| [`wrap-swap.spec.ts`](../frontend-dapp/e2e/wrap-swap.spec.ts) | Playwright: LCD route mocks via [`wrap-mapper-lcd-mock.ts`](../frontend-dapp/e2e/helpers/wrap-mapper-lcd-mock.ts) |
| [docs/frontend.md § Swap wrap safety CTA](../docs/frontend.md#swap-wrap-safety-cta-sec-a02) | Product copy table |
| [docs/testing.md § Swap wrap safety CTA](../docs/testing.md#swap-wrap-safety-cta-sec-a02-gitlab-389) | Verification commands |
| [docs/testing.md § Wrap-mapper pause smoke (SEC-B06)](../docs/testing.md#wrap-mapper-pause-smoke-sec-b06-gitlab-396) | On-chain pause cycle (`make smoke-wrap-mapper-pause`) |
| [`smoke-wrap-mapper-pause.sh`](../scripts/smoke-wrap-mapper-pause.sh) | LocalTerra wrap/unwrap rejection + restore transcript |

## Invariants (SEC-A02)

| State | Submit label | Disabled |
|-------|--------------|----------|
| Wrap mapper `config.paused === true` | **Wrapping is Temporarily Paused** | yes |
| Wrap amount exceeds mapper `rate_limit` window | **Rate Limit Exceeded** | yes |

- **Pause wins over rate limit** when both would apply (`SwapPage` `buttonText` chain).
- Tests must cover **each state in isolation** — do not assert pause and rate limit in one combined regex ([`wrap-swap.spec.ts`](../frontend-dapp/e2e/wrap-swap.spec.ts) E12 + dedicated describe).
- Rate limit here is **on-chain wrap mapper quota**, not indexer HTTP **429** (indexer limits: [`AGENTS_INDEXER_API_LCD_SECURITY.md`](./AGENTS_INDEXER_API_LCD_SECURITY.md)).

## Verification

```bash
# Unit (no chain)
bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- src/pages/SwapPage.test.tsx -t "SEC-A02"

# E2E (needs deploy env + LocalTerra LCD)
bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/wrap-swap.spec.ts -g "SEC-A02|wrap mapper paused" --project=e2e-tx

# On-chain wrap-mapper pause cycle (SEC-B06 / GitLab #396) — needs deploy-local full seed
make smoke-wrap-mapper-pause
# or full acceptance bundle:
make verify-issue-396
```

## Related

- Pair pause on `/trade`: [`TradePage.test.tsx`](../frontend-dapp/src/pages/TradePage.test.tsx) (GitLab #87 / #199)
- Trading blacklist CTA: [`blacklist.ts`](../frontend-dapp/src/services/terraclassic/blacklist.ts)
- Native wrap routing: [`router.test.ts`](../frontend-dapp/src/services/terraclassic/router.test.ts)
