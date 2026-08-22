# Agent playbook: post-merge !394–!396 ops (GitLab #590)

Audience: third-party agents verifying the stacked **indexer fees / Swap gas / CW20 audit** merge after GitLab CI was skipped, or shipping Coolify + LocalTerra QA so protocol fees, wrap+≥2hop gas, and 8266 NO-GO land as one cut.

**Issue:** [GitLab **#590**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/590)  
**Invariants:** [`docs/qa-invariants.md`](../docs/qa-invariants.md) **Q7** (**M590-1–M590-8**)  
**Verify:** `make verify-issue-590`

Stacked MRs on `main`: **!394** (#586), **!395** (#587), **!396** (#589). Do not treat green `make verify-issue-589` (multi-test only) as 8266 clearance.

## Invariants (M590-1–M590-8)

| ID | Rule |
|----|------|
| **M590-1** | Local regression is `make verify-issue-590`, which runs child verifies **586, 587, 589**. A child FAIL fails the stack. SKIP (no LocalTerra) is allowed only for A-lcd/B-lt and wrap-swap E7/E8 unless `VERIFY590_REQUIRE_CHAIN=1`. |
| **M590-2** | Coolify indexer migrate `20260821120000_protocol_fees.sql` + `WRAP_MAPPER_ADDRESS` + dApp `/protocol` **Protocol fees** and Swap **Network fee (est.) ~X LUNC** ship together. Local verify does **not** replace Coolify. Empty mapper omits wrap/unwrap (not fake `$0`). |
| **M590-3** | Hybrid treasury fee = pool `commission_amount` (`swap_amm`) + `limit_order_fills.commission_amount` (`book_take`) **once** — never also swap `book_commission_amount` (**L7** / **PFee-5**). |
| **M590-4** | Limit place → `limit_place` from `maker_fee_amount`. Unwrap → wrap-mapper `fee_amount` only. InstantWithdraw burn tax / `tax_amount` are **not** protocol fees. |
| **M590-5** | `CI=1` Playwright `e2e/wrap-swap.spec.ts --project=e2e-tx` **E7** LUNC→USTR (or JADE/RUBY stand-in) and **E8** reverse; `gas_used < gas_wanted` (no OOG). **1 worker** (shared LocalTerra account). 0 USTC still allows a LUNC-funded swap; AMM Fee row stays pool bps. |
| **M590-6** | `LAYER_B_LT=1` **must not PASS as a stub**. It runs [`layer-a-lcd.sh`](../cw20-codeid-audits/scripts/layer-a-lcd.sh) (store + instantiate + Transfer/TransferFrom 1:1 of pinned `token.wasm`) then [`layer-b-lt.sh`](../cw20-codeid-audits/scripts/layer-b-lt.sh) (whitelist **local** store id, `CreatePair` vs EMBER, provide, round-trip Send swap, limit escrow). Columbus-5 templates need LocalTerra (`requires_terra`). |
| **M590-7** | **#581 / 8266 stays NO-GO** until [`codeids/8266/REPORT.md`](../cw20-codeid-audits/codeids/8266/REPORT.md) is **GO**. No `AddWhitelistedCodeId 8266` on columbus-5. LocalTerra may whitelist a **locally stored copy** of the LCD bytes for the harness only. |
| **M590-8** | This playbook + **Q7** + child skills stay crosslinked. Do not wait for GitHub Actions; GitLab CI may be quota-blocked — local `make verify-issue-*` is the gate. |

## Do / don’t

- **Do** run `make verify-issue-590` from a git worktree after pulling `main`.
- **Do** `apt install wabt` before `CODE_ID=8266 make verify-issue-589` (decomp is **C2**).
- **Do** provision LocalTerra (`make setup-cloud-localterra`) when A-lcd/B-lt or E7/E8 SKIP and the issue asks for those rungs.
- **Don’t** treat `make verify-issue-589` without `LAYER_B_LT=1` as 8266 execution. LAYER_B_LT=1 must not PASS as a stub.
- **Don’t** `AddWhitelistedCodeId 8266` on mainnet from a LocalTerra store code id.
- **Don’t** count InstantWithdraw burn tax or swap `book_commission_amount` as treasury fees.
- **Don’t** close #590 while **M590-5** / **M590-6** remain unexecuted if the issue still has those checkboxes open — run with `VERIFY590_REQUIRE_CHAIN=1`.

## Regression

```bash
make setup-indexer-postgres
make verify-issue-590
# docs + classification only:
VERIFY590_SKIP_CHILDREN=1 make verify-issue-590
# after A-lcd/B-lt exist:
CODE_ID=8266 LAYER_B_LT=1 make verify-issue-589
VERIFY590_REQUIRE_CHAIN=1 make verify-issue-590
```

Child playbooks: [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) (**PFee-1–PFee-12**), [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md) (#587), [`AGENTS_CW20_CODE_ID_AUDIT.md`](./AGENTS_CW20_CODE_ID_AUDIT.md) (**C589-1–C589-9**), [`AGENTS_HOOK_COMMISSION.md`](./AGENTS_HOOK_COMMISSION.md) (**L7**).

Coolify env: [`docs/runbooks/mainnet-soft-launch.md`](../docs/runbooks/mainnet-soft-launch.md). Whitelist policy: [`docs/runbooks/cw20-whitelist-policy.md`](../docs/runbooks/cw20-whitelist-policy.md).
