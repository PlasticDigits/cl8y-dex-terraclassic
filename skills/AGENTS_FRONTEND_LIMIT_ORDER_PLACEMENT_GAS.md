# Agent playbook: Limit order placement gas presets (Advanced)

Use when changing **Placement gas (book walk)** on `/trade` or `/limit`: Low / Medium / High / Custom presets, helper copy, or the on-chain `max_adjust_steps` integer wired into `place_limit_order`.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/limit-orders.md § dApp: retail form](../docs/limit-orders.md#dapp-retail-form-wires-invariants) | **#204** bullet — preset mapping, default Medium → 32, UI clamp 256 |
| [docs/frontend.md § Trade page — limit place success affordances](../docs/frontend.md#trade-page-limit-place-success-affordances) | **Place another** resets to **Medium (32 steps)** |
| [`limitOrderExpiry.ts`](../frontend-dapp/src/utils/limitOrderExpiry.ts) | `LIMIT_ORDER_MAX_ADJUST_STEPS_PRESET_VALUES`, `resolveLimitOrderMaxAdjustStepsPresetTier`, `clampLimitOrderMaxAdjustSteps` |
| [`LimitOrderAdvancedLimitSettings.tsx`](../frontend-dapp/src/components/trade/LimitOrderAdvancedLimitSettings.tsx) | Shared Advanced UI on trade ticket + standalone limit page |
| [`useLimitOrderForm.ts`](../frontend-dapp/src/hooks/useLimitOrderForm.ts) | `maxSteps` form state (broadcast integer); default `LIMIT_ORDER_MAX_ADJUST_STEPS_DEFAULT` |
| [`pair.ts`](../frontend-dapp/src/services/terraclassic/pair.ts) | `placeLimitOrder` / `placeLimitOrderWithAllowance` — passes `max_adjust_steps` unchanged |
| [docs/limit-orders.md § Messages (CosmWasm)](../docs/limit-orders.md#messages-cosmwasm) | On-chain field semantics + hard cap |

## Preset mapping (invariants — GitLab #204)

| UI preset | On-chain `max_adjust_steps` | When to use |
|-----------|----------------------------|-------------|
| **Low** | 16 | Less placement gas; thin books |
| **Medium** | 32 | **Default** — balanced for most pairs |
| **High** | 128 | More gas; deep/busy books when placement fails |
| **Custom** | User integer **1…256** | Advanced; numeric input only in Custom mode |

- **Do not** surface `max_adjust_steps` or raw preset integers as primary button labels.
- **Custom mode:** component may force Custom UI while value still matches a preset integer until the user edits steps or picks another tier.
- **Broadcast:** form `maxSteps` is the single source of truth; presets only map to integers before submit.
- **Reset:** fresh form and **Place another** → Medium (32).

## Rules of thumb

1. **Keep mapping in `limitOrderExpiry.ts`** — UI must not hard-code 16/32/128 in multiple places.
2. **Changing preset values** requires updating Vitest (`limitOrderExpiry.test.ts`, `LimitOrderAdvancedLimitSettings.test.tsx`) and this skill + `docs/limit-orders.md`.
3. **Gas semantics** for the two-tx place sequence (allowance + hook) stay in [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md); this skill is only the **book-walk step cap** traders pick in Advanced.
4. **Both surfaces** must stay shared via `LimitOrderAdvancedLimitSettings` — do not fork trade vs `/limit` copy.

## Related

- Limit expiry presets (24h / 7d / No expiry): [`LimitOrderExpiryField.tsx`](../frontend-dapp/src/components/trade/LimitOrderExpiryField.tsx) — [GitLab **#156**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/156)
- Terra Classic gas / two-tx place fees: [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md) — [GitLab **#132**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/132)
- Deep order book (context for when High helps): [`AGENTS_FRONTEND_DEEP_ORDER_BOOK.md`](./AGENTS_FRONTEND_DEEP_ORDER_BOOK.md)
