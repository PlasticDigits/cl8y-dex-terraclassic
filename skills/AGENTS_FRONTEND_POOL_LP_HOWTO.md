# Agent playbook: retail LUNC liquidity how-to (v2 LP + maker limits)

Use when adding or editing **user-facing** steps for providing or withdrawing **LUNC** liquidity on dex.cl8y.com ([GitLab **#531**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/531)).

Support already told users they can provide/withdraw v2 LP or place limits, and that there is **no incentive program**. The dApp answer is the opt-in `/pool` how-to — not this engineering page and not `docs/frontend.md` as the only guide.

`#489` (no always-on essays), `#417` (onboarding is Swap/Trade/Limits only), `#147` / `#213` (provide gas + native wrap), and `#366` (IL notice) still apply.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#531**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/531) | Spec: AC, path tests, attack plan |
| [`docs/frontend.md` § Retail LUNC liquidity how-to](../docs/frontend.md#retail-lunc-liquidity-howto) | Invariants **H531-1–H531-10** |
| [`docs/user-lunc-liquidity.md`](../docs/user-lunc-liquidity.md) | Human backup (GitLab); in-app is primary |
| [`poolLpHowtoCopy.ts`](../frontend-dapp/src/utils/poolLpHowtoCopy.ts) | Static retail strings + forbidden-copy check |
| [`poolLpHowto.ts`](../frontend-dapp/src/utils/poolLpHowto.ts) | Section dismiss `cl8y-dex-pool-lp-howto-section-dismissed` ([#547](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/547)) |
| [`PoolLpHowto.tsx`](../frontend-dapp/src/components/pool/PoolLpHowto.tsx) | Dismissible hint + `<details>` on `/pool` (`#lp-howto`) |
| [`LegalFooterNotice.tsx`](../frontend-dapp/src/components/legal/LegalFooterNotice.tsx) | Footer **Add liquidity** → `/pool#lp-howto` |
| [`PortfolioLpOverviewSection.tsx`](../frontend-dapp/src/components/portfolio/PortfolioLpOverviewSection.tsx) | **How to add liquidity** next to Manage on Pool |
| [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) | Opt-in how-to only — no permanent lectures |
| [`AGENTS_FRONTEND_TRADE_ONBOARDING_IA.md`](./AGENTS_FRONTEND_TRADE_ONBOARDING_IA.md) | First-visit strip stays Swap · Trade · Limits |
| [`PoolPage.tsx`](../frontend-dapp/src/pages/PoolPage.tsx) | Retail one-sided cards + Advanced two-sided, IL |
| [`navItems.ts`](../frontend-dapp/src/components/common/navItems.ts) | Pool folds into **More** below 1200px |

## Invariants (H531-1–H531-10)

1. **H531-1** — Same-origin `/pool#lp-howto` names Pool provide/withdraw and optional limit maker.
2. **H531-2** — Wrapped LUNC; pick native LUNC as **Token** on Add to auto-wrap **or** `/wrap`; bank LUNC for gas.
3. **H531-3** — Retail Add is one token + pair + amount. Two-sided is Advanced (empty pools). Off-ratio Advanced still donates; retail zap does not.
4. **H531-4** — No incentive program. No APR / points / farm chrome.
5. **H531-5** — Withdraw on `/pool`; LP tokens are the share.
6. **H531-6** — Limits are maker escrow, not LP. Link `/trade` or `/limits` only.
7. **H531-7** — No always-on lecture on Swap/Trade/Limits/Wrap. `/pool` how-to (hint **and** `<details>`) is dismissible (`cl8y-dex-pool-lp-howto-section-dismissed`); `#lp-howto` restores ([#547](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/547)).
8. **H531-8** — How-to is on `/pool` so More / mobile Pool still find it.
9. **H531-9** — IL, pause, blacklist, gas, ratio, clickwrap, NFA unchanged.
10. **H531-10** — Static React (no `innerHTML`). In-app links only. Mention **Create Pair** LUNC creation fee; creating a pair is not required to LP an existing pool. Unwrap is not free.

## Rules of thumb

1. Edit copy in `poolLpHowtoCopy.ts` — keep `forbiddenPoolLpHowtoCopyHits()` empty.
2. Do **not** extend `TradeOnboardingStrip` with a Pool lecture.
3. Do **not** invent incentives or conflate LP, limits, `/ust1`, and `/wrap` as the destination.
4. Do **not** change pool math, wrap fees, or treasury in this issue.
5. How-to must stay **in-flow** (`relative`) — no `position:fixed` overlay on Provide, wallet, or clickwrap.
6. Pair-creation fee belongs in the how-to **and** on `/create` (live amount). Existing-pool LP does not pay that fee.

## Verify

```bash
make verify-issue-531
make test-frontend
# scoped:
#   PoolPage + PoolLpHowto + poolLpHowtoCopy + LegalFooterNotice
# Playwright (5 workers, e2e-smoke):
#   frontend-dapp/e2e/pool-lp-howto-531.spec.ts
```

Issue: [GitLab **#531**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/531) (AC1–AC10, C1–C6, Playwright P1–P8, attack A1–A10).

## Related

- [`AGENTS_FRONTEND_POOL_TABLE.md`](./AGENTS_FRONTEND_POOL_TABLE.md) — `/pool` table + how-to section dismiss ([#547](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/547))
- Copy / no lectures: [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) (`#489`)
- Onboarding strip: [`AGENTS_FRONTEND_TRADE_ONBOARDING_IA.md`](./AGENTS_FRONTEND_TRADE_ONBOARDING_IA.md) (`#417`)
- Provide/withdraw preview: [`AGENTS_FRONTEND_POOL_PROVIDE_WITHDRAW_PREVIEW.md`](./AGENTS_FRONTEND_POOL_PROVIDE_WITHDRAW_PREVIEW.md) (`#480`)
- Wrap enablement: [`AGENTS_MAINNET_WRAP_ENABLEMENT.md`](./AGENTS_MAINNET_WRAP_ENABLEMENT.md) (`#507`)
- One-sided default: [`AGENTS_FRONTEND_POOL_ONE_SIDED.md`](./AGENTS_FRONTEND_POOL_ONE_SIDED.md) (`#533`)
- Zap execution floors: [`AGENTS_FRONTEND_POOL_ZAP_FLOORS.md`](./AGENTS_FRONTEND_POOL_ZAP_FLOORS.md) (`#559`)
