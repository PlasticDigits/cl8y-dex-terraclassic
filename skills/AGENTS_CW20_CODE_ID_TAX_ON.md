# Agent playbook: named tax-on Layer B suite (GitLab #623)

Use when proving **community-tax** DEX paths after `RegisterListedPair`, or when tempted to “just extend B-lt” with extra-debit math.

This is the **tax-on** harness. Generic honest-CW20 Layer B-lt stays **tax-off** ([`AGENTS_CW20_CODE_ID_AUDIT.md`](./AGENTS_CW20_CODE_ID_AUDIT.md) **C589-5**). Invoice / SKU / MintControl stay on `#601` smoke ([`AGENTS_COMMUNITY_TAX_CW20.md`](./AGENTS_COMMUNITY_TAX_CW20.md) **O601-3–O601-6**).

## Invariants (C623)

| ID | Rule |
|----|------|
| **C623-1** | [`layer-b-lt.sh`](../cw20-codeid-audits/scripts/layer-b-lt.sh) stays 1:1. Do not RegisterListedPair in generic B-lt. **Do not** change asserts to `debit == amount + tax` or special-case `if community-tax`. |
| **C623-2** | [`layer-b-tax-on.sh`](../cw20-codeid-audits/scripts/layer-b-tax-on.sh) is not a stub PASS. `LAYER_B_TAX_ON=1` without LocalTerra **FAIL**s (same shape as **C589-7**). `make verify-issue-623` is always-on. |
| **C623-3** | JSON artifact records pair-direct sell/buy, router `trader`, spoof-trader negative, missing-trader fail-closed, limit 1:1, AutoLP floor + fake-pair reject. Stub `executed: true` without txs is a harness bug. |
| **C623-4** | Known-bad ALPHA **8654** / FoT mutants stay **red** on generic B-lt. Tax-on PASS is **not** a listing gate for other templates. |
| **C623-5** | Whitelist **local store id only**. Never columbus-5 `11611` / `11612` / `11613` / `11614` / `11619` / `11620` / `11621` / `11622` / `8654` from this evidence. |
| **C623-6** | Pair credit on sell stays `amount` (**H-01** / **T592-1**). No pair/router FoT math. |
| **C623-7** | Official-router ≥2hop extra-debits authenticated `Swap.trader`; pair→router 1:1; pair-direct ignores spoofed `trader`; missing router `trader` fail-closes (**T592-13** / **R607**). |
| **C623-8** | AutoLP `pair` is factory-listed with this token; `SkimToLp` respects floor (100/200 bps); fake pair rejected; skim is **never** called from token `Transfer`/`Send` (**T592-10** / **M610**). Seed-path leftover: a prior hostile `skim_min_return=1e15` stays on AutoLP — clear with `UpdateConfig { skim_min_return: 0 }` before the floor-success skim, and restore after the hostile probe so `#625` re-runs are not stuck. |

## Operator sequence

```bash
# tax-off 1:1 unchanged (do not register):
CODE_ID=11619 LAYER_B_LT=1 make verify-issue-589

# named tax-on (LocalTerra required):
LAYER_B_TAX_ON=1 make verify-issue-623

# invoices / SKU / MintControl stay here:
make verify-issue-601
```

Inputs (first match): seed pins `VITE_TOKEN_COMMUNITY_TAX_ADDRESS` + pair + AutoLP ([#620](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/620)), **or** ephemeral store/instantiate (nonzero bps, AutoLP on). Shared helpers: [`lib-tax-on.sh`](../cw20-codeid-audits/scripts/lib-tax-on.sh) (also sourced by `#601` smoke). Seed-path **buy** must use `pick_trader` (non-treasury / non-exempt) — leftover live after !416 is [#625](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/625).

## Do not

- Merge tax-on math into `layer-b-lt.sh`.
- Treat a tax-on PASS as permission to whitelist 8654 or a LocalTerra id on columbus-5.
- Call `SkimToLp` from taxed `Transfer`/`Send`.
- Honor `Swap.trader` on pair-direct (victim extra-debit).
- Leave invoice / launcher SKU coverage only in this suite — that remains `#601`.

## Related

- [`AGENTS_CW20_CODE_ID_AUDIT.md`](./AGENTS_CW20_CODE_ID_AUDIT.md) — C589 intake; B-lt tax-off
- [`AGENTS_COMMUNITY_TAX_CW20.md`](./AGENTS_COMMUNITY_TAX_CW20.md) — T592 / O601
- [`AGENTS_COMMUNITY_TAX_ROUTER.md`](./AGENTS_COMMUNITY_TAX_ROUTER.md) — R607 / T592-13
- [`AGENTS_COMMUNITY_TAX_AUTOLP.md`](./AGENTS_COMMUNITY_TAX_AUTOLP.md) — M610
- [`AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED.md`](./AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED.md) — optional seed pins
- [`AGENTS_POST_MERGE_OPS_625.md`](./AGENTS_POST_MERGE_OPS_625.md) — leftover seed-path buy after !415–!417 ([#625](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/625))
- [`cw20-codeid-audits/harness/README.md`](../cw20-codeid-audits/harness/README.md)
