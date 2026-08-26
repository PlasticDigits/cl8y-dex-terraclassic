# Agent playbook: Create Token desktop density

Audience: third-party agents editing [`CreateTokenPage.tsx`](../frontend-dapp/src/pages/CreateTokenPage.tsx) layout, SKU checkbox grid, or `/token/create` width.

**Issue:** [GitLab **#669**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/669)  
**Invariants:** [`docs/frontend.md` § Create Token](../docs/frontend.md#create-token-community-tax) (**C669-1–C669-8**)  
**Related:** [#593](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/593) create/manage, [#604](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/604) identity, [#605](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/605) SKU init, [#653](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/653) one chrome layer, [#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/489) copy, [#488](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/488) tokens, [#651](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/651) `/tiers` phone (opposite breakpoint). Do **not** fold Manage (`/token/:addr/manage`) or Migrate (`/token/migrate`) into this layout.

## Problem class

On desktop, Create Token was a **520px** centered chimney inside the **1080px** `.app-main` shell. Short fields stacked full-width; Paid features and the CTA sat below the fold on 1280×720. This is **layout only**. Invoice math, identity parsers, SKU hooks, and launcher execute stay **C593 / C604 / C605**.

## Do / don’t

- **Do** use `w-full` on the configured page (`data-testid="create-token-page"`). `.app-main` is the 1080px cap. Pair Name/Symbol/Decimals, Treasury/Manager, and SKU checkboxes at `md+` (`create-token-desktop-grid`).
- **Do** keep a **single** `shell-panel-strong`. Hairline / CSS grid only — no SKU `card-glass` tiles.
- **Do** stack to one column at `<md` (≤767px). `min-w-0` + `break-all` on bech32 inputs. SKU/ack labels `min-h-11`.
- **Do** keep testids stable (`create-token-name`, `create-token-sku-*`, `create-token-ack`, `create-token-free-cta`, `create-token-pay`, helpers). Layout-only ids: `create-token-desktop-grid`, `create-token-identity-row`, `create-token-wallet-row`, `create-token-sku-grid`, `create-token-features-legend`.
- **Don’t** change launcher `CreateToken` / UST1 Receive, invoice payee, SKU prices, or `buildValidatedCreateArgs` field semantics.
- **Don’t** hide ack in a closed `<details>`. Don’t add a screenspace essay or “use two columns” banner.
- **Don’t** widen Swap, Create Pair, Manage, or Migrate to match. Unavailable stub may stay `max-w-[520px]`.
- **Don’t** print 11619 / 8654 / allowlist essays on the card (**M628-7**).
- **Don’t** add `e2e-tx`. Playwright workers stay **5**.
- **Don’t** use `*-neo`. Don’t nest `shell-panel` inside the wizard.

## Invariants

| ID | Meaning |
|----|---------|
| **C669-1** | Desktop 1280 / 1440: configured `create-token-page` width **> 700px** (`w-full`, not `max-w-[520px]`). Form uses the `.app-main` budget; does not stretch past 1080 + padding. |
| **C669-2** | `md+`: Name+Symbol+Decimals share a row; Treasury+Manager share a row; Paid features is a 2-col checkbox grid; Buy/Sell stay paired. Classes live in [`createTokenLayout.ts`](../frontend-dapp/src/utils/createTokenLayout.ts). |
| **C669-3** | 1280×720, 0 SKUs: Paid features legend is in the first viewport; ack/CTA in the first two. Init panels may extend when SKUs are on. |
| **C669-4** | ≤767px: single column; no horizontal `document` scroll at 320 / 375 / 390; bech32 does not overflow the card; tap targets ≥44px (`min-h-11`). Helpers stay `connected wallet` / `not connected wallet`. |
| **C669-5** | Layout only. **C593 / C604 / C605** unchanged: env stub, query payee/manager/treasury ignored, percent 2 dp + 25% combined, SKU uncheck drops hook keys, free CTA vs PayWithAnyToken launcher payee. |
| **C669-6** | One chrome layer (#653). No nested `shell-panel` / SKU `card-glass`. No `*-neo`. Lead sentence + **Migrate here** only — no density banner (#489). |
| **C669-7** | Existing testids stay. Each `create-token-sku-{id}` `onChange` maps to that `sku.id` only. No `tabindex` tricks that skip ack. Grid `z-index` stays below Connect / Legal / WalletConnect (`z-[9999]`). |
| **C669-8** | Swap / Create Pair stay ticket-width. Manage / Migrate density is out of scope. Create Token stays code-id-free on the card. No `e2e-tx`. |

## Canonical code

| File | Role |
|------|------|
| `frontend-dapp/src/utils/createTokenLayout.ts` | Width + `md:` grid class contract |
| `frontend-dapp/src/pages/CreateTokenPage.tsx` | Wizard layout (`create-token-desktop-grid`) |
| `frontend-dapp/src/pages/CreateTokenPage.test.tsx` | Grid / DOM order / SKU panels / query ignore |
| `frontend-dapp/e2e/create-token-602.spec.ts` | #602 smoke + #669 1280 / 1440 / 375 / 390 geometry |

## Regression

```bash
make verify-issue-669
```

Vitest: `CreateTokenPage.test.tsx`. Sibling semantics: `make verify-issue-593` · `make verify-issue-604` · `make verify-issue-605`. Playwright: `e2e/create-token-602.spec.ts` (5 workers) when LocalTerra is up. No `e2e-tx`. Chrome nesting: `python3 scripts/check_chrome_nesting.py` if markup gains `card-glass`.

## Related

- [`AGENTS_FRONTEND_CREATE_TOKEN.md`](./AGENTS_FRONTEND_CREATE_TOKEN.md) — C593 / C604 / C605 execute + invoices
- [`AGENTS_FRONTEND_CHROME_NESTING.md`](./AGENTS_FRONTEND_CHROME_NESTING.md) — one chrome layer (#653)
- [`AGENTS_FRONTEND_DESIGN_SYSTEM.md`](./AGENTS_FRONTEND_DESIGN_SYSTEM.md) — tokens / no `*-neo`
- [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) — no density lecture
- [`AGENTS_FRONTEND_TIERS_PHONE.md`](./AGENTS_FRONTEND_TIERS_PHONE.md) — `/tiers` phone cards (#651); opposite breakpoint
- [`AGENTS_FRONTEND_PAY_INVOICE.md`](./AGENTS_FRONTEND_PAY_INVOICE.md) — Pay card; do not fork onto this page
- Post-merge leftover: [#673](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/673) / `make verify-issue-673` / [`AGENTS_POST_MERGE_OPS_673.md`](./AGENTS_POST_MERGE_OPS_673.md)
