# Agent playbook: wrap/unwrap burn tax incidence (GitLab #512)

Use when changing **native wrap/unwrap quotes**, InstantWithdraw tax math, wrap fee notes, or exchange-deposit unwrap warnings.

**Issue:** [GitLab **#512**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/512)

## Problem (mainnet evidence)

Unwrapping cLUNC → LUNC via wrap-mapper → treasury `InstantWithdraw` → `BankMsg::Send` is **burn-taxed**. With `fee_bps = 200` and `burn_tax_rate = 0.015`:

| Step | Amount (10 000 LUNC CW20) |
|------|---------------------------|
| Mapper fee (2%) | −200 → withdraw **9 800** |
| Burn tax on InstantWithdraw (1.5% of 9 800) | −147 → user **9 653** |
| Effective haircut | **~3.47%** (not the documented 2% fee alone) |

Tx: `C282C337B3F3E4AC7ECC95B92E82DCD1484C2E6FA766A07DBFAFAC49F6B280A0`.

Wrap-mapper `fee_bps` was raised to **200** so the fee residual *can* cover tax (ustr-cmm deployment policy), but **current contracts do not gross-up** InstantWithdraw — tax incidence falls on the user. On-chain gross-up lives in **ustr-cmm** (follow-up migrate), not this dApp alone.

## Classic tax rules (do not confuse)

| Transfer | Taxed? |
|----------|--------|
| `MsgExecuteContract` with funds (user → treasury `wrap_deposit`) | **No** |
| Contract `BankMsg::Send` (treasury → user InstantWithdraw) | **Yes** |

Formula (columbus-5 ComputeTax): `tax = min(floor(amount × burn_tax_rate), cap)`.

LCD: `GET ${LCD}/terra/tax/v1beta1/params` → `params.burn_tax_rate` (e.g. `"0.015"`). Cap: `/terra/treasury/v1beta1/tax_caps/{denom}`. Do **not** rely on unimplemented `/terra/tax/v1beta1/tax_rate`.

## Invariants (W8–W11)

| ID | Rule |
|----|------|
| **W8** | Direct **wrap** / wrap-input mint quotes use **mapper `fee_bps` only** — never burn-tax `wrap_deposit` gross. Display for 10 000 @ 200 bps = **9 800**, not ~9 751. |
| **W9** | Direct **unwrap** / `unwrap_output` **You Receive** = post-fee then post–burn-tax. `routerMinReceiveBase` stays post-fee pre-tax for router `minimum_receive` (**R3**). |
| **W10** | Unwrap fee line discloses burn tax on payout (single line; no permanent essay — still **W7**). |
| **W11** | Unwrap UI warns: withdraw to **own wallet** first — exchanges often ignore contract-initiated deposits (even with memo). |

## Code map

| Path | Role |
|------|------|
| [`nativeTransferTax.ts`](../frontend-dapp/src/utils/nativeTransferTax.ts) | `burn_tax_rate` fetch + multiply tax math |
| [`router.ts`](../frontend-dapp/src/services/terraclassic/router.ts) | `netCw20AfterNativeWrap`, `netNativeAfterUnwrap`, `simulateNativeSwap` |
| [`wrapMapper.ts`](../frontend-dapp/src/services/terraclassic/wrapMapper.ts) | `wrapUnwrapFeeNote(kind, feeBps, burnTaxRate?)` |
| [`WrapPage.tsx`](../frontend-dapp/src/pages/WrapPage.tsx) / [`SwapPage.tsx`](../frontend-dapp/src/pages/SwapPage.tsx) | Fee note + exchange-deposit warning |
| [`poolProvideCounterpart.ts`](../frontend-dapp/src/utils/poolProvideCounterpart.ts) | Provide auto-fill: fee-only wrap net |

## Related playbooks

- [`AGENTS_NATIVE_WRAP_TAX.md`](./AGENTS_NATIVE_WRAP_TAX.md) — history of #342; superseded for wrap mint by **W8**
- [`AGENTS_MAINNET_WRAP_ENABLEMENT.md`](./AGENTS_MAINNET_WRAP_ENABLEMENT.md) — W1–W7 + Coolify env (#507)
- [`AGENTS_ROUTER_MINIMUM_RECEIVE.md`](./AGENTS_ROUTER_MINIMUM_RECEIVE.md) — R3 post-fee floor
- [`NATIVE_TOKEN_WRAPPING.md`](../NATIVE_TOKEN_WRAPPING.md) — architecture

## Verification

```bash
make verify-issue-512
# or:
bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
  src/utils/__tests__/nativeTransferTax.test.ts \
  src/services/terraclassic/router.test.ts \
  src/services/terraclassic/__tests__/wrapMapper.test.ts \
  src/utils/__tests__/poolProvideCounterpart.test.ts \
  src/pages/WrapPage.test.tsx
```

## Follow-up (not in this dApp MR)

On-chain **gross-up** InstantWithdraw in ustr-cmm wrap-mapper/treasury so recipient nets `gross − fee` and tax is paid from fee residual — then bump dependency + migrate mainnet wrap-mapper. Until then, accurate UI quotes + disclosure are the shipped mitigation.
