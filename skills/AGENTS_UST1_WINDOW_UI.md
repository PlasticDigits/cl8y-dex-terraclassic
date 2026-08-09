# Agent playbook: UST1 oracle window UI (GitLab #506)

Use when changing **`/ust1`**, ust1-window CW20 Send client, UST1/vFDUSD metadata, or Coolify env for the always-on oracle mint/redeem path (parent [#502](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/502)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [`docs/runbooks/ust1-window-ui.md`](../docs/runbooks/ust1-window-ui.md) | Invariants **U1–U8**, Coolify addresses, smoke checklist |
| [`docs/frontend.md`](../docs/frontend.md) routes table | `/ust1` vs `/mint` |
| [`Ust1Page.tsx`](../frontend-dapp/src/pages/Ust1Page.tsx) | Deposit / Withdraw UI |
| [`ust1Window.ts`](../frontend-dapp/src/services/terraclassic/ust1Window.ts) | `effective_swap` + CW20 Send |
| [`ust1WindowMath.ts`](../frontend-dapp/src/utils/ust1WindowMath.ts) | INV-SWAP-001/002 integer quotes |
| [`ust1WindowGates.ts`](../frontend-dapp/src/utils/ust1WindowGates.ts) | Pause / stale / limit gates |
| Upstream | [ust1-window](https://gitlab.com/PlasticDigits/ust1-window) `effective_swap`, `Cw20HookMsg::{Deposit,Withdraw}` |

## Rules of thumb

1. **Do not** overload soft-launch **`/mint`** for UST1 — separate route + label **UST1** (**U2** / **U6**).
2. **Do not** call the AMM router for mint/redeem — only CW20 `Send` to the window (**U3**).
3. **Do not** invent FX rates — read `effective_swap.oracle.rate` + `fee_bps` and use `ust1WindowMath` (**U4**).
4. Block submit when window paused, oracle paused/stale, or UST1 notional exceeds per-tx / rolling remaining (**U4**).
5. Hide nav unless `isUst1WindowEnabled()` (**U1**); page shows unavailable when env incomplete.
6. Prod Coolify must bake columbus-5 addresses from the runbook table — never LocalTerra defaults (**U7**).
7. New gas shape: `send` → `deposit` / `withdraw` → **`UST1_WINDOW_SEND_GAS_LIMIT`** + retail inventory fixture (**U8** / #475).
8. Keep retail copy short — oracle mint/redeem ≠ AMM; link Swap/Trade for secondary markets ([`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md)).

## Quick commands

```bash
make verify-issue-506
make lint-frontend
make test-frontend
# Playwright CTA gates (LCD mocked; Vite bakes VITE_UST1_* via playwright.config):
bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/ust1-window.spec.ts --project=e2e-smoke
```

## Related

- Soft-launch faucet anti-pattern: [`AGENTS_SOFT_LAUNCH_FAUCET.md`](./AGENTS_SOFT_LAUNCH_FAUCET.md)
- Shell nav: [`AGENTS_FRONTEND_SHELL_NAV.md`](./AGENTS_FRONTEND_SHELL_NAV.md)
- Gas: [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md)
- Design / copy: [`AGENTS_FRONTEND_DESIGN_SYSTEM.md`](./AGENTS_FRONTEND_DESIGN_SYSTEM.md), [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md)
- Prod Vite / Coolify: [`AGENTS_FRONTEND_PRODUCTION_BUILD.md`](./AGENTS_FRONTEND_PRODUCTION_BUILD.md)
- Phase 5 ops (oracle age, pause, inventory): [`AGENTS_UST1_WRAP_PRODUCTION_OPS.md`](./AGENTS_UST1_WRAP_PRODUCTION_OPS.md) ([#503](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/503))
