# Agent playbook: wrap-mapper split fees (GitLab #516)

Use when changing **wrap/unwrap quotes**, wrap-mapper `Config` parsing, fee notes, ops health probes, or the ≈2% unwrap all-in product goal.

**Issue:** [GitLab **#516**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/516)  
**Upstream:** [ustr-cmm#9](https://gitlab.com/PlasticDigits2/ustr-cmm/-/work_items/9) (fee split + retune). Ops window: [ustr-cmm#13](https://gitlab.com/PlasticDigits2/ustr-cmm/-/work_items/13).  
**Supersedes** the InstantWithdraw **gross-up** follow-up in [#512](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/512) as the prescribed fix for “make unwrap 2%”. Tax disclosure (**W10**/**W11**) stays.

## Problem

After migrate, wrap-mapper `Config` is **`fee_wrap_bps` / `fee_unwrap_bps`** (drops `fee_bps`; no contract dual-read). Product target on columbus-5: **200 / 51**. Unwrap You Receive = post-**unwrap**-fee then 1.5% InstantWithdraw burn tax ≈ **2% all-in** (10 000 → **9 800**), not the old 200+tax **9 653**.

Until migrate, live mapper may still return `{ fee_bps: 200 }`. UI must show that truthful stack — **do not** simulate 51 bps or gross-up to fake 2%.

## Invariants (W12–W15)

| ID | Rule |
|----|------|
| **W12** | Wrap / wrap-input / pool provide auto-wrap use **`fee_wrap_bps` only**. Unwrap / `unwrap_output` You Receive and fee notes use **`fee_unwrap_bps`**. Never swap the fields. **W8**/**W9** still apply (no burn tax on wrap; unwrap = fee then tax; `routerMinReceiveBase` is post-fee pre-tax / **R3**). |
| **W13** | Fail closed on missing, partial, or invalid fee fields (no quote / “fee unavailable”). Never treat missing as 0%. Transitional `{ fee_bps }` → both sides **only** when **both** split fields are absent. |
| **W14** | Do **not** hardcode 200 or 51 in quote math. Always query LCD config. In-memory + React Query cache **≤ 30s** so a gov `set_fees` cannot leave the UI on a stale unwrap fee for long. |
| **W15** | ≈2% unwrap all-in is **asymmetric fees + tax**, not InstantWithdraw gross-up. Retune `fee_unwrap_bps` when `burn_tax_rate` changes (formula below). If tax ≥ ~2%, escalate — cannot hit 2% without subsidy. Do not change ust1-window `fee_bps` (different contract). |

## Retune rule (mirror upstream)

```text
fee_unwrap_bps = round(10000 - 9800 / (1 - burn_tax_rate))
```

Example: `0.015` → **51**. Goal: `receive / A ≈ 0.98`, prefer ≤2% when rounding. Helper: `retuneUnwrapFeeBps` in [`wrapMapper.ts`](../frontend-dapp/src/services/terraclassic/wrapMapper.ts) — **docs/ops only**; UI never substitutes this for on-chain config.

Integer check: 10 000 @ 51 bps → 9 949 after fee; `floor(9949 × 0.015) = 149` → **9 800**.

## Code map

| Path | Role |
|------|------|
| [`wrapMapper.ts`](../frontend-dapp/src/services/terraclassic/wrapMapper.ts) | `parseWrapMapperConfig`, `wrapMapperFeeBps(kind)`, `queryWrapMapperFeeBps(kind)`, 30s cache |
| [`router.ts`](../frontend-dapp/src/services/terraclassic/router.ts) | `netCw20AfterNativeWrap` → wrap fee; `netNativeAfterUnwrap` → unwrap fee then tax |
| [`WrapPage.tsx`](../frontend-dapp/src/pages/WrapPage.tsx) / [`SwapPage.tsx`](../frontend-dapp/src/pages/SwapPage.tsx) | Mode-specific fee note + exchange warning |
| [`poolProvideCounterpart.ts`](../frontend-dapp/src/utils/poolProvideCounterpart.ts) / [`PoolPage.tsx`](../frontend-dapp/src/pages/PoolPage.tsx) | Provide auto-fill: **wrap** fee only |
| [`check-ust1-wrap-ops-health.sh`](../scripts/check-ust1-wrap-ops-health.sh) | Display split fees; WARN if still single `fee_bps` |

Indexer does **not** expose mapper fees — no indexer dependency (mixed old indexer / new UI is N/A).

## Related playbooks

- [`AGENTS_WRAP_UNWRAP_BURN_TAX.md`](./AGENTS_WRAP_UNWRAP_BURN_TAX.md) — W8–W11 tax math (gross-up no longer the 2% fix)
- [`AGENTS_MAINNET_WRAP_ENABLEMENT.md`](./AGENTS_MAINNET_WRAP_ENABLEMENT.md) — W1–W7 Coolify / query-on-chain
- [`AGENTS_ROUTER_MINIMUM_RECEIVE.md`](./AGENTS_ROUTER_MINIMUM_RECEIVE.md) — R3 `routerMinReceiveBase`
- [`AGENTS_UST1_WRAP_PRODUCTION_OPS.md`](./AGENTS_UST1_WRAP_PRODUCTION_OPS.md) — health / registry
- [`NATIVE_TOKEN_WRAPPING.md`](../NATIVE_TOKEN_WRAPPING.md) — architecture
- QA: [`docs/qa-templates/wrap-unwrap-test-pass.md`](../docs/qa-templates/wrap-unwrap-test-pass.md)

## Production checklist (Coolify)

Ship this frontend **before or in the same window** as wrap-mapper store + migrate + `set_fees` 200/51 ([ustr-cmm#13](https://gitlab.com/PlasticDigits2/ustr-cmm/-/work_items/13)). After Coolify rebuild:

1. LCD `config` shows `fee_wrap_bps=200`, `fee_unwrap_bps=51` (not `fee_bps` only).
2. `/wrap` wrap 10 000 → **9 800** CW20; unwrap 10 000 → **≈9 800** native (not 9 653).
3. Fee notes stay honest (“fee; You Receive after burn tax”) — never “2% flat”.
4. DEX 2-of-3 must **not** sign mapper migrate / `SetFees` (wrap-stack governance).

## Router wasm (#523)

DEX router `dex_common::wrap_mapper::ConfigResponse` dual-reads mapper `Config` with the same **W13** rules as the dApp. `unwrap_output` **R3** uses `fee_unwrap_bps` (legacy `fee_bps` only when both split fields are absent). Store + migrate router wasm in the same window as wrap-mapper migrate ([ustr-cmm#13](https://gitlab.com/PlasticDigits2/ustr-cmm/-/work_items/13)). `make verify-issue-523`.

## LocalTerra instantiate (GitLab #539)

`scripts/deploy-dex-local.sh` sends wrap-mapper `InstantiateMsg` as **`fee_wrap_bps` / `fee_unwrap_bps`** (LocalTerra default **50 / 50**, overridable via `WRAP_MAPPER_FEE_BPS`). Older `wrap_mapper.wasm` that still requires `fee_bps` is retried automatically. Rebuild mapper wasm from [ustr-cmm](https://gitlab.com/PlasticDigits2/ustr-cmm) when artifacts are missing. Do not hardcode 200/51.

```bash
make verify-issue-539
```

Records `#533` e2e-tx **P4–P8** when LocalTerra is up (`frontend-dapp/e2e/pool-one-sided-533-tx.spec.ts`).

## Verification

```bash
make verify-issue-516
```
