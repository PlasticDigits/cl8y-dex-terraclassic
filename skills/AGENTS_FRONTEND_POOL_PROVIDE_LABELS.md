# Agent playbook: `/pool` Manage provide labels + wrap default

Audience: third-party agents changing Advanced two-sided **Provide Liquidity** field labels, wrap checkboxes, or `/pool` amount `aria-label`s.

**Issue:** [GitLab **#661**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/661)  
**Invariants:** [`docs/frontend.md` § Pool page — provide liquidity](../docs/frontend.md#pool-page--provide-liquidity-ui-invariants) (**P661-1–P661-12**)  
**Related:** [#547](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/547) Manage expand, [#533](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/533) one-sided (out of scope), [#630](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/630) product tickers, [#480](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/480) counterpart auto-fill, [#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/489) no slot names / essays.

## Problem class

Manage → Provide still said **Asset A / Asset B**. Retail cannot tell which token they deposit. Auto-wrap defaulted **off**, so wrap-pair LPs were steered to CW20 (or “exceeds wallet”) instead of bank LUNC/USTC + wrap in one multi-msg.

## Do / don’t

- **Do** label each provide amount with `{Name} ({SYMBOL})` or `{SYMBOL}` for the **selected** input (native when wrap is on, pair CW20 when off).
- **Do** keep tickers in a `normal-case` node (`.label-glass` is `uppercase` and would paint `cLUNC` as `CLUNC`).
- **Do** default wrap **on** at mount when `getNativeEquivalent(pair leg)` is set. Session-only; no `localStorage`.
- **Do** keep execute / React Query ids as `uluna` / `uusd` / CW20 address (**N630-3**).
- **Don’t** print `Asset A` / `Asset B` / `ASSET A` in the provide form, aria, or tests.
- **Don’t** add a wrap checkbox to retail one-sided (**Z533-1**). Native pick already implies wrap.
- **Don’t** flip withdraw `receiveWrapped` (stays default **receive wrapped**).
- **Don’t** send native coins into the pair contract. Wrap path is wrap_deposit + CW20 provide.
- **Don’t** persist wrap preference or force wrap on non-wrap pairs (`?wrap=1` is forbidden).

## Invariants

| ID | Meaning |
|----|---------|
| **P661-1** | Provide visible labels are `{Name} ({SYMBOL})` or `{SYMBOL}`. No Asset A/B copy (including uppercase CSS). |
| **P661-2** | Amount `aria-label` is `{product ticker} amount` (`LUNC amount`, `cLUNC amount`, `UST1 amount`). |
| **P661-3** | Known wrap / native / listed CW20 tickers follow **#630**. Never `uluna` / `uusd` as the visible label. |
| **P661-4** | Name === symbol (or missing / spoofed / HTML) collapses to symbol only. Never `UST1 (UST1)`. |
| **P661-5** | cLUNC / cUSTC leg: auto-wrap checkbox **checked on first paint**. Uncheck is session-only. |
| **P661-6** | Wrap on → native balance + Max/50% + wrap execute + native label. Wrap off → CW20 path + `#147` gas gate + CW20 label. |
| **P661-7** | Pair with no native equivalent: no wrap checkbox; provide stays CW20/CW20; labels still name/symbol. |
| **P661-8** | Dual wrap pair: both checkboxes default on independently; user can uncheck one side. |
| **P661-9** | Withdraw expected receive / pre-sign stay product tickers. `receiveWrapped` default unchanged. |
| **P661-10** | Pause, freeze, blacklist, wrap treasury mismatch, IL, pre-sign, counterpart auto-fill, empty-pool no auto-fill still hold. |
| **P661-11** | Retail one-sided cards unchanged (no wrap checkbox, no Asset A/B). |
| **P661-12** | Indexer `name`/`symbol` render as text. Registry allowlist wins. No `dangerouslySetInnerHTML`. No N+1 LCD on default `/pool` paint (A8). |

## Canonical code

| File | Role |
|------|------|
| `frontend-dapp/src/utils/tokenDisplay.ts` | `formatPoolAssetFieldLabel`, `usablePoolAssetName`, `poolProvideAmountAriaLabel` |
| `frontend-dapp/src/utils/poolProvideWrapDefault.ts` | `provideWrapDefaultOn` — wrap default iff `getNativeEquivalent` |
| `frontend-dapp/src/hooks/useTokenDisplayInfo.ts` | Registry `name` + sanitized indexer name |
| `frontend-dapp/src/components/pool/PoolAdvancedManage.tsx` | Labels, wrap defaults, selected-input pre-sign amounts |

## Regression

```bash
make verify-issue-661
```

Vitest: `tokenDisplay.test.ts`, `poolProvideWrapDefault.test.ts`, `useTokenDisplayInfo.test.tsx`, `PoolPage.test.tsx`. Static grep: `PoolAdvancedManage.tsx` has no `Asset A` / `Asset B`.

## Related

- [`AGENTS_FRONTEND_POOL_PROVIDE_WITHDRAW_PREVIEW.md`](./AGENTS_FRONTEND_POOL_PROVIDE_WITHDRAW_PREVIEW.md) — counterpart auto-fill (#480)
- [`AGENTS_FRONTEND_POOL_ONE_SIDED.md`](./AGENTS_FRONTEND_POOL_ONE_SIDED.md) — retail zap; no wrap checkbox
- [`AGENTS_FRONTEND_POOL_TABLE.md`](./AGENTS_FRONTEND_POOL_TABLE.md) — Manage expand / A8
- [`AGENTS_FRONTEND_NATIVE_TICKERS.md`](./AGENTS_FRONTEND_NATIVE_TICKERS.md) — LUNC / USTC / cLUNC / cUSTC
- [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) — no Asset A essays
- [`AGENTS_FRONTEND_POOL_SIGNING_CONFIRMATION.md`](./AGENTS_FRONTEND_POOL_SIGNING_CONFIRMATION.md) — pre-sign amount lines
