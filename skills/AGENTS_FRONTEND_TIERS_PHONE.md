# Agent playbook: `/tiers` phone-width layout

Audience: third-party agents editing [`TiersPage.tsx`](../frontend-dapp/src/pages/TiersPage.tsx), fee-discount register chrome, or How it works copy.

**Issue:** [GitLab **#651**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/651)  
**Invariants:** [`docs/frontend.md` § Tiers Page](../docs/frontend.md#tiers-page) (**T651-1–T651-8**); registry **I15** in [`docs/reference/fee-discount-tiers.md`](../docs/reference/fee-discount-tiers.md)  
**Related:** [#476](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/476) Hold labels / I12, [#384](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/384) register gas, [#514](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/514) I13 limit-place, [#537](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/537) pair chrome, [#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/489) copy, [#488](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/488) design tokens. Do **not** fold this into picker viewport work ([#632](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/632)).

## Problem class

On ≤390px, a five-slot horizontal `TierRow` plus a reserved empty `w-28` Register column squeezed **Tier N**, **Hold {n} CL8Y**, and **fee discount** into one-word-per-line wrap. QA **11.1.4** failed on production phones. This is **layout only**.

## Do / don’t

- **Do** keep **Hold {n} CL8Y** as one `whitespace-nowrap` phrase (`data-testid="tier-hold-{id}"`). Live `formatTokenAmountAbbrev` is the source (`Hold 7.5K CL8Y`, not a second ladder).
- **Do** render the Register slot **only** when a button is shown. Full-width `min-h-11` on `<md`; `data-testid="register-tier-{id}"`.
- **Do** stack How it works on `<md` (`tiers-how-it-works-mobile`). Keep the five-column table at `md+`.
- **Do** filter `governance_only` before listing. Register sends `tier_id` only — never parse the hold label.
- **Don’t** change `Register` / `Deregister` msgs, gas (#384), or `GetTiers` / `GetRegistration`.
- **Don’t** invent discounts from `get_discount` or pair `fee_bps` on this page (I4 / I14).
- **Don’t** raise tier 9 swap `discount_bps` (I13). **Limit place*** stays 0.
- **Don’t** reserve `w-28` when disconnected. **Don’t** ship a ghost disabled Register.
- **Don’t** use `truncate` / ellipsis on the hold magnitude. **Don’t** add `*-neo` or fee-trivia banners.
- **Don’t** add `e2e-tx` for this layout. Playwright workers stay **5**.
- **Don’t** duplicate the numeric ladder — link [`docs/reference/fee-discount-tiers.md`](../docs/reference/fee-discount-tiers.md).

## Invariants

| ID | Meaning |
|----|---------|
| **T651-1** | Phone cards: **Tier N** one line; **Hold {n} CL8Y** one phrase; no magnitude ellipsis. |
| **T651-2** | No empty Register column when the button is absent. |
| **T651-3** | Fee cluster phrases stay intact (`whitespace-nowrap`). |
| **T651-4** | Register ≥44px on coarse/narrow; `register-tier-{id}` matches `onRegister(tier_id)`. |
| **T651-5** | Registered row has no Register; 0 / 255 stay off the self-register list. |
| **T651-6** | How it works stacks on phone; Limit place* stays (I13). |
| **T651-7** | Display-only; text-only LCD fields; pending Register disables every button. |
| **T651-8** | Design-system chrome; no `*-neo`; not a `visualViewport` picker fix. |

## Canonical code

| File | Role |
|------|------|
| `frontend-dapp/src/pages/TiersPage.tsx` | `TierRow` + How it works mobile/desktop |
| `frontend-dapp/src/pages/TiersPage.test.tsx` | Disconnected / register / governance / fee labels |
| `frontend-dapp/e2e/fee-tiers.spec.ts` | Desktop smoke + 390 / 375 geometry |
| `frontend-dapp/src/utils/formatAmount.ts` | `formatTokenAmountAbbrev` |
| `frontend-dapp/src/utils/limitOrderFeeSummary.ts` | I4 / I13 helpers — do not fork |

## Regression

```bash
make verify-issue-651
```

Vitest: `TiersPage.test.tsx` plus existing `formatAmount` / `limitOrderFeeSummary` / `feeDiscountUiCopy` suites. Playwright: `e2e/fee-tiers.spec.ts` (5 workers) when LocalTerra is up. No `e2e-tx`.

## Related

- [`AGENTS_FEE_DISCOUNT_TIERS.md`](./AGENTS_FEE_DISCOUNT_TIERS.md) — ladder + I12–I15
- [`AGENTS_FRONTEND_PAIR_FEE_DISCOUNT.md`](./AGENTS_FRONTEND_PAIR_FEE_DISCOUNT.md) — pair-scoped chrome, not `/tiers`
- [`AGENTS_FRONTEND_DESIGN_SYSTEM.md`](./AGENTS_FRONTEND_DESIGN_SYSTEM.md) — tokens / no `*-neo`
- [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) — no fee-trivia banners
- [`AGENTS_FRONTEND_PORTAL_LISTBOX_VIEWPORT.md`](./AGENTS_FRONTEND_PORTAL_LISTBOX_VIEWPORT.md) — do not mix with #632
