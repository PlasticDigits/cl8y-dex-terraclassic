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
| [`pair.ts`](../frontend-dapp/src/services/terraclassic/pair.ts) | `placeLimitOrderWithAllowance` — passes `max_adjust_steps` and optional **`hint_after_order_id`** on batch item ([#261](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/261)) |
| [`limitBookInsertHint.ts`](../frontend-dapp/src/utils/limitBookInsertHint.ts) | **`resolveLimitInsertHintAfter`** from deep-book pages ([#261](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/261)) |
| [docs/limit-orders.md § Messages (CosmWasm)](../docs/limit-orders.md#messages-cosmwasm) | On-chain field semantics + hard cap + **`hint_after_order_id`** ([#256](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/256)) |

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

## Verification (GitLab #204)

| Check | Command / surface |
|-------|-------------------|
| Preset → hook `max_adjust_steps` on LocalTerra | `bash scripts/qa/verify-glab-204-limit-gas-presets.sh` |
| LCD decode helper | [`e2e/helpers/lcd.ts`](../frontend-dapp/e2e/helpers/lcd.ts) — `txJsonPlaceLimitMaxAdjustSteps` |
| E2E spec | [`e2e/limit-orders-gas-presets-tx.spec.ts`](../frontend-dapp/e2e/limit-orders-gas-presets-tx.spec.ts) |
| UI smoke (standalone) | `/limits` → Advanced → **Low / Medium / High / Custom** (shared component with `/trade`) |

Cross-link: [`AGENTS_E2E_LIMIT_ORDERS_TX.md`](./AGENTS_E2E_LIMIT_ORDERS_TX.md).

## Rules of thumb

1. **Keep mapping in `limitOrderExpiry.ts`** — UI must not hard-code 16/32/128 in multiple places.
2. **Changing preset values** requires updating Vitest (`limitOrderExpiry.test.ts`, `LimitOrderAdvancedLimitSettings.test.tsx`) and this skill + `docs/limit-orders.md`.
3. **Gas semantics** for the two-tx place sequence (allowance + hook) stay in [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md); this skill is only the **book-walk step cap** traders pick in Advanced.
4. **Both surfaces** must stay shared via `LimitOrderAdvancedLimitSettings` — do not fork trade vs `/limit` copy.
5. **`hint_after_order_id` (GitLab #256 / #261):** when the dApp/indexer knows the predecessor order id (deep book, prior rung), pass it on `UpdateLimitOrderPrice` and **single-rung batch placement** (`placeLimitOrderWithAllowance` → batch item field) to avoid head walks. On-chain verifies hint; stale/missing anchors fall back to head walk; **near-miss** hints on valid anchors walk from the hint toward head/tail ([#265](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/265)). Resolver: [`limitBookInsertHint.ts`](../frontend-dapp/src/utils/limitBookInsertHint.ts). See [limit-orders.md § hint_after](../docs/limit-orders.md#place--cancel-limit-gitlab-206), [integrators.md § Batch placement insert hints](../docs/integrators.md#batch-placement-insert-hints-gitlab-261), invariant **L14** in [contracts-security-audit.md](../docs/contracts-security-audit.md).

## Related

- Limit expiry presets (24h / 7d / No expiry): [`LimitOrderExpiryField.tsx`](../frontend-dapp/src/components/trade/LimitOrderExpiryField.tsx) — [GitLab **#156**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/156)
- Terra Classic gas / two-tx place fees: [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md) — [GitLab **#132**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/132)
- Deep order book (context for when High helps): [`AGENTS_FRONTEND_DEEP_ORDER_BOOK.md`](./AGENTS_FRONTEND_DEEP_ORDER_BOOK.md)
