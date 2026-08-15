# Agent playbook: Swap wrap safety CTA copy (SEC-A02)

Use when changing **wrap mapper pause**, **on-chain wrap rate limit**, **wrap rate-limit inline alert**, or **swap submit button** gating on `/` ([GitLab **#389**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/389), launch checklist SEC-A02; inline alert [**#463**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/463) / SEC-I05 F-04).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [`SwapPage.tsx`](../frontend-dapp/src/pages/SwapPage.tsx) | Submit CTA precedence: wrap pause → blacklist → amount → rate limit; inline `swap-wrap-rate-limit-banner` when rate limit exceeded |
| [`marketDataServiceCopy.ts`](../frontend-dapp/src/utils/marketDataServiceCopy.ts) | `WRAP_RATE_LIMIT_EXCEEDED_MESSAGE` — inline alert copy (SEC-I05 / #463) |
| [`wrapMapper.ts`](../frontend-dapp/src/services/terraclassic/wrapMapper.ts) | `queryWrapMapperConfig`, `queryPausedState`, `checkRateLimitExceeded`, `queryRateLimit` |
| [`WrapRateLimitStatus.tsx`](../frontend-dapp/src/components/wrap/WrapRateLimitStatus.tsx) | Available / max + reset countdown on Swap + `/wrap` |
| [`wrapRateLimit.ts`](../frontend-dapp/src/utils/wrapRateLimit.ts) | Parse CosmWasm Timestamp nanos; expire window → full capacity |
| [`SwapPage.test.tsx`](../frontend-dapp/src/pages/SwapPage.test.tsx) | Vitest: exact copy + `toBeDisabled()` per state (isolated mocks) |
| [`wrap-swap.spec.ts`](../frontend-dapp/e2e/wrap-swap.spec.ts) | Playwright: LCD route mocks via [`wrap-mapper-lcd-mock.ts`](../frontend-dapp/e2e/helpers/wrap-mapper-lcd-mock.ts) |
| [docs/frontend.md § Swap wrap safety CTA](../docs/frontend.md#swap-wrap-safety-cta-sec-a02) | Product copy table |
| [docs/testing.md § Swap wrap safety CTA](../docs/testing.md#swap-wrap-safety-cta-sec-a02-gitlab-389) | Verification commands |
| [docs/testing.md § Wrap-mapper pause smoke (SEC-B06)](../docs/testing.md#wrap-mapper-pause-smoke-sec-b06-gitlab-396) | On-chain pause cycle (`make smoke-wrap-mapper-pause`) |
| [`smoke-wrap-mapper-pause.sh`](../scripts/smoke-wrap-mapper-pause.sh) | LocalTerra wrap/unwrap rejection + restore transcript |

## Invariants (SEC-A02)

| State | Submit label | Disabled |
|-------|--------------|----------|
| Env treasury ≠ on-chain mapper `config.treasury` (#507 / W2) | **Wrap treasury misconfigured** | yes |
| Mapper config / pause / rate-limit LCD unavailable (#507 fail-closed) | **Wrap config unavailable** | yes |
| Wrap mapper `config.paused === true` | **Wrapping is Temporarily Paused** | yes |
| Wrap amount exceeds mapper `rate_limit` window | **Rate Limit Exceeded** | yes |
| Wrap rate limit exceeded (inline alert, SEC-I05 F-04 / #463) | `swap-wrap-rate-limit-banner` with `WRAP_RATE_LIMIT_EXCEEDED_MESSAGE` | visible below form (not only CTA label) |
| Route pair `is_paused === true` (L6 / SEC-B05) | **Pair is paused** | yes |

- **Treasury mismatch and config-unavailable win over pause / rate limit** (`SwapPage` `buttonText` chain).
- **Pause wins over rate limit** when both would apply.
- **Wrap-mapper pause wins over pair pause** on native wrap-only paths.
- Never show **Wrap (1:1)** when mapper config has not loaded (`wrapUnwrapFeeNote(null)` → fee unavailable).
- Tests must cover **each state in isolation** — do not assert pause and rate limit in one combined regex ([`wrap-swap.spec.ts`](../frontend-dapp/e2e/wrap-swap.spec.ts) E12 + dedicated describe).
- Rate limit here is **on-chain wrap mapper quota**, not indexer HTTP **429** (indexer limits: [`AGENTS_INDEXER_API_LCD_SECURITY.md`](./AGENTS_INDEXER_API_LCD_SECURITY.md)).

## Verification

```bash
# Unit (no chain)
bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- src/pages/SwapPage.test.tsx -t "SEC-A02"
bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- src/pages/SwapPage.test.tsx -t "rate-limit alert"
bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- src/pages/SwapPage.test.tsx src/pages/PoolPage.test.tsx -t "SEC-B05"

# E2E (needs deploy env + LocalTerra LCD)
bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/wrap-swap.spec.ts -g "SEC-A02|wrap mapper paused" --project=e2e-tx

# On-chain wrap-mapper pause cycle (SEC-B06 / GitLab #396) — needs deploy-local full seed
make smoke-wrap-mapper-pause
# or full acceptance bundle:
make verify-issue-396
```

## Related

- Wrap **fee display** (`fee_wrap_bps` / `fee_unwrap_bps`, cLUNC/cUSTC) is separate from pause/rate-limit CTAs — [`AGENTS_MAINNET_WRAP_ENABLEMENT.md`](./AGENTS_MAINNET_WRAP_ENABLEMENT.md) (#507), [`AGENTS_WRAP_MAPPER_SPLIT_FEES.md`](./AGENTS_WRAP_MAPPER_SPLIT_FEES.md) (#516). `wrapMapper.ts` exports `queryWrapMapperConfig` for on-chain fees.
- Pair pause on `/` and `/pool`: [`SwapPage.test.tsx`](../frontend-dapp/src/pages/SwapPage.test.tsx), [`PoolPage.test.tsx`](../frontend-dapp/src/pages/PoolPage.test.tsx) (GitLab **#395** / SEC-B05); hook [`usePairPaused.ts`](../frontend-dapp/src/hooks/usePairPaused.ts)
- Pair pause on `/trade`: [`TradePage.test.tsx`](../frontend-dapp/src/pages/TradePage.test.tsx) (GitLab #87 / #199)
- Trading blacklist CTA: [`blacklist.ts`](../frontend-dapp/src/services/terraclassic/blacklist.ts); pool + limits Vitest (**SEC-E01**, GitLab **#425**): [`PoolPage.test.tsx`](../frontend-dapp/src/pages/PoolPage.test.tsx), [`LimitOrdersPage.test.tsx`](../frontend-dapp/src/pages/LimitOrdersPage.test.tsx); [docs/frontend.md § Trading blacklist disabled CTAs](../docs/frontend.md#trading-blacklist-disabled-ctas-sec-e01)
- Native wrap routing: [`router.test.ts`](../frontend-dapp/src/services/terraclassic/router.test.ts)
