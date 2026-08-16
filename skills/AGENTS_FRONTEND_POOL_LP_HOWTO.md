# Agent playbook: retail LUNC liquidity how-to (v2 LP + maker limits)

Use when adding or editing **user-facing** steps for providing or withdrawing **LUNC** liquidity on dex.cl8y.com ([GitLab **#531**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/531)).

Issue **#531 is open**. Support already told users they can provide/withdraw v2 LP or place limits, and that there is **no incentive program**. They still cannot find the steps on the dApp. `#489` (no always-on essays), `#417` (onboarding is Swap/Trade/Limits only), `#147` / `#213` (provide gas + native wrap), and `#366` (IL notice) still apply.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#531**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/531) | Full spec: current code, guardrails, AC, path tests, attack plan |
| [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) | Opt-in how-to only — no permanent lectures |
| [`AGENTS_FRONTEND_TRADE_ONBOARDING_IA.md`](./AGENTS_FRONTEND_TRADE_ONBOARDING_IA.md) | First-visit strip (no Pool today) |
| [`AGENTS_FRONTEND_POOL_PROVIDE_WITHDRAW_PREVIEW.md`](./AGENTS_FRONTEND_POOL_PROVIDE_WITHDRAW_PREVIEW.md) | Auto-fill + withdraw preview (`#480`) |
| [`AGENTS_MAINNET_WRAP_ENABLEMENT.md`](./AGENTS_MAINNET_WRAP_ENABLEMENT.md) | `/wrap` + cLUNC (`#507`) |
| [`PoolPage.tsx`](../frontend-dapp/src/pages/PoolPage.tsx) | Provide / withdraw, native LUNC checkbox, GitLab IL link |
| [`TradeOnboardingStrip.tsx`](../frontend-dapp/src/components/common/TradeOnboardingStrip.tsx) | Swap · Trade · Limits only |
| [`navItems.ts`](../frontend-dapp/src/components/common/navItems.ts) | Pool folds into **More** below 1200px |
| [`docs/frontend.md`](../docs/frontend.md) § Pool provide | Engineering invariants — **not** the retail answer |
| [`docs/user-incident-faq.md`](../docs/user-incident-faq.md) | Incident FAQ only |

## Current gap

Mechanics work. There is **no retail how-to** on dex.cl8y.com. `/pool` links GitLab `frontend.md`. Onboarding never mentions Pool or Wrap. Users hear “provide LUNC” as one action; the product is two:

1. **v2 AMM LP** — both sides of a pair; native LUNC via **Use native LUNC (auto-wrap)** or `/wrap` first; withdraw via LP tokens.
2. **Maker limits** — escrow on `/trade` or `/limits`. Not pool shares. Not a farm.

## Target

One short retail how-to the dApp can open in one tap, plus one discoverability hook (dismissible `/pool` line, onboarding Pool link, or Portfolio/footer “How to add liquidity”). Do **not** add APR/points/rewards.

How-to must state: both tokens required; wrap or auto-wrap for native LUNC; bank LUNC still needed for gas; withdraw on `/pool`; no incentive program; limits are optional and **not** LP.

## Rules of thumb

1. **#489** — no always-on essay on Swap/Trade/Limits/Wrap. Entry is `<details>`, first-visit dismissible, or a dedicated help page.
2. **Do not invent incentives.** One sentence: no LP/maker program currently.
3. **Do not conflate** v2 LP, limit escrow, `/ust1` mint, and `/wrap` as the destination. Wrap is a **step** for native LUNC.
4. **Keep safety gates** — IL (#366), NFA (#138), clickwrap (#517), pause/blacklist, gas (#147), ratio donate warning, pre-sign (#462). Docs never replace blocking errors.
5. **No pool-math / wrap-fee / treasury changes** in this issue.
6. **Copy** — Pool / Provide / Withdraw / Limit + token symbols. No `token0` / `CW20 Send` / “AMM invariant”.
7. **More / mobile** — “open Pool” must work when Pool is under header **More** (< 1200px) and on the mobile tab bar.
8. **Engineering `docs/frontend.md` stays** the invariant source; point agents at the retail doc so they do not paste protocol essays onto the page.

## Verify

Issue: [GitLab **#531**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/531) (AC1–AC10, C1–C6, Playwright P1–P8, attack A1–A10). After implement: `make verify-issue-531` (add the script in the same MR).

```bash
make test-frontend
# scoped: PoolPage + TradeOnboardingStrip (if extended)
# Playwright: /pool how-to + tablet More + phone bottom nav
```

## Related

- Copy / no lectures: [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) (`#489`)
- Onboarding strip: [`AGENTS_FRONTEND_TRADE_ONBOARDING_IA.md`](./AGENTS_FRONTEND_TRADE_ONBOARDING_IA.md) (`#417`)
- Provide/withdraw preview: [`AGENTS_FRONTEND_POOL_PROVIDE_WITHDRAW_PREVIEW.md`](./AGENTS_FRONTEND_POOL_PROVIDE_WITHDRAW_PREVIEW.md) (`#480`)
- Wrap enablement: [`AGENTS_MAINNET_WRAP_ENABLEMENT.md`](./AGENTS_MAINNET_WRAP_ENABLEMENT.md) (`#507`)
- Open-limits Cancel (separate): [`AGENTS_FRONTEND_LIMIT_CANCEL_OPEN.md`](./AGENTS_FRONTEND_LIMIT_CANCEL_OPEN.md) (`#530`)
