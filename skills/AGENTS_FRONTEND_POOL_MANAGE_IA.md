# Agent playbook: `/pool` pair Manage IA (GitLab #660)

Audience: third-party agents changing `/pool` provide, withdraw, or zap chrome.

**Issue:** [GitLab **#660**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/660)  
**Invariants:** [`docs/frontend.md` § Pool Manage IA](../docs/frontend.md#pool-manage-ia) (**M660-1–M660-8**)  
**Related:** [#533](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/533) zap math (**Z533**), [#559](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/559) floors, [#547](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/547) table, [#531](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/531) how-to, [#653](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/653) one chrome layer, [#201](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/201) tab vs submit names, [#462](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/462) pre-sign.

## Problem class

Page-level **Add** / **Withdraw** were one-sided zap but looked like ordinary LP. Two-sided **Provide Liquidity** / **Withdraw Liquidity** lived under **Manage → Advanced**. Retail users could not tell which product they were signing, and empty-pool first deposit looked optional.

## Do / don’t

- **Do** keep one pair-scoped Manage panel: four peer actions — **Provide Liquidity**, **Withdraw Liquidity**, **Zap Add**, **Zap Withdraw**.
- **Do** lazy-mount the selected action form only. Manage expand is one LCD `getPool` / fee config (**P547-9**).
- **Do** pin zap add to the expanded pair (no second pair picker). Pin zap withdraw to this pair’s LP.
- **Do** disable empty-pool zap with `Empty pool. Use Provide Liquidity.`
- **Don’t** restore page-level zap cards (`md:grid-cols-2` Add/Withdraw).
- **Don’t** wrap two-sided in an **Advanced** `<details>` (`pool-card-advanced`).
- **Don’t** nest `shell-panel*` inside Manage (`shell-panel-strong`). Zap forms are `card-glass` wells (**C653-1**).
- **Don’t** re-solve zap math, floors, or pair mint/burn. IA only.

## Canonical code

| File | Role |
|------|------|
| `frontend-dapp/src/pages/PoolPage.tsx` | Search + how-to + table; **no** page-level zap |
| `frontend-dapp/src/components/pool/PoolPairsTable.tsx` | Manage expand; mount pair manage once |
| `frontend-dapp/src/components/pool/PoolAdvancedManage.tsx` | Four-action toolbar; two-sided forms |
| `frontend-dapp/src/components/pool/OneSidedAddCard.tsx` | Pair-implicit zap-in |
| `frontend-dapp/src/components/pool/OneSidedWithdrawCard.tsx` | This-pair LP zap-out |
| `frontend-dapp/e2e/helpers/pool-ui.ts` | `openPoolManage` / `openFirstFactoryManage` |

## Invariants (M660-1–M660-8)

1. **M660-1** — Default `/pool` paint has no `pool-one-sided-add` / `pool-one-sided-withdraw` until Manage is expanded **and** a zap tab is selected.
2. **M660-2** — Factory Manage shows four peer tabs. None require **Advanced**.
3. **M660-3** — No `pool-card-advanced` disclosure. `ONE_SIDED_ADVANCED_LABEL` is unused.
4. **M660-4** — Zap operates on the expanded pair only. Collapse / other row unmounts the form.
5. **M660-5** — One action form at a time. Tabs use `Select …` aria-labels so they do not collide with submit names (**#201**).
6. **M660-6** — Empty pool: zap tabs disabled; sentence points at **Provide Liquidity**. First deposit stays two-sided (`MINIMUM_LIQUIDITY` / **Z533-5**).
7. **M660-7** — Indexer-only rows omit zap tabs. Factory-only zap (**Z533-6**).
8. **M660-8** — Pre-sign **action** matches the selected tab (`Zap Add` / `Zap Withdraw` / `Provide Liquidity` / `Withdraw Liquidity`). How-to + docs do not teach “two-sided is Advanced.”

## Verify

```bash
# Playwright Vite Origin must be in indexer CORS_ORIGINS (default :3173).
PLAYWRIGHT_WEB_PORT=3173 make verify-issue-660
make verify-issue-531
make verify-issue-533
make verify-issue-547
make verify-issue-559
make verify-issue-653
```

Issue: [GitLab **#660**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/660) (M1–M12, T1–T20, A1–A15).

## Related

- Zap math: [`AGENTS_FRONTEND_POOL_ONE_SIDED.md`](./AGENTS_FRONTEND_POOL_ONE_SIDED.md) (`#533`)
- Zap floors: [`AGENTS_FRONTEND_POOL_ZAP_FLOORS.md`](./AGENTS_FRONTEND_POOL_ZAP_FLOORS.md) (`#559`)
- Table: [`AGENTS_FRONTEND_POOL_TABLE.md`](./AGENTS_FRONTEND_POOL_TABLE.md) (`#547`)
- How-to: [`AGENTS_FRONTEND_POOL_LP_HOWTO.md`](./AGENTS_FRONTEND_POOL_LP_HOWTO.md) (`#531`)
- Chrome nesting: [`AGENTS_FRONTEND_CHROME_NESTING.md`](./AGENTS_FRONTEND_CHROME_NESTING.md) (`#653`)
