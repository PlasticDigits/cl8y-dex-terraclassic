# Agent playbook: post-merge !415–!417 tax swarm / e2e-tx / Layer B tax-on leftovers (GitLab #625)

Audience: third-party agents verifying the integrated tip after [!415](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/merge_requests/415) (#621), [!416](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/merge_requests/416) (#623), and [!417](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/merge_requests/417) (#622) landed on `main`. Child `make verify-issue-621` / `622` / `623` already passed on the merge tip for docs + unit (and #621 live `--dry-run` on a dirty factory). GitLab CI on !415–!417 failed with `ci_quota_exceeded` (not a code defect).

These leftovers were **not** proven on a clean #620 seed volume: tax-on seed-path buy from a non-treasury wallet, Playwright `e2e-tx` P0 LCD extra-debit / buy net, live swarm soak, and OE-1 `pool_only`.

**Issue:** [GitLab **#625**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/625)  
**Parents (closed unless a merged invariant is wrong):** [#621](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/621) (**S621-1–S621-8**), [#622](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/622) (**E622-1–E622-8**), [#623](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/623) (**C623-1–C623-8**)  
**Invariants:** [`docs/qa-invariants.md`](../docs/qa-invariants.md) **Q13** (**M625-1–M625-8**)  
**Verify:** `make verify-issue-625`

Do **not** reopen #621 / #622 / #623 / #620 unless a merged invariant is wrong. Fresh #620 seed volume leftovers stay on [#624](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/624).

Child playbooks: [`AGENTS_LOCALNET_SWARM_TAX.md`](./AGENTS_LOCALNET_SWARM_TAX.md), [`AGENTS_E2E_COMMUNITY_TAX_TX.md`](./AGENTS_E2E_COMMUNITY_TAX_TX.md), [`AGENTS_CW20_CODE_ID_TAX_ON.md`](./AGENTS_CW20_CODE_ID_TAX_ON.md). Seed: [`AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED.md`](./AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED.md). OE-1 gems: [`AGENTS_LOCALNET_TRADING_SWARM.md`](./AGENTS_LOCALNET_TRADING_SWARM.md).

## Invariants (M625-1–M625-8)

| ID | Rule |
|----|------|
| **M625-1** | Local regression is `make verify-issue-625`, which runs children **621, 622, 623** plus leftover live (tax-on seed buy, Playwright P0, swarm soak, **293** `pool_only`). A child FAIL fails the stack. Fresh-volume leftover SKIP unless `VERIFY625_FRESH=1`. Live leftover SKIP unless LocalTerra + seed pins (FAIL when `VERIFY625_REQUIRE_LIVE=1`). |
| **M625-2** | Seed token **treasury ≠ test1** (e2e / swarm trader). CMM stand-in stays **test1** (**L620-7**). `layer-b-tax-on.sh` seed-path **buy** uses `pick_trader` (non-treasury / non-exempt). Manager-directory skip (#609) is 1:1 — do not assert outbound split against the manager or treasury. Seed pins win by default; `LAYER_B_TAX_ON_FORCE_EPHEMERAL=1` skips them so a #624 volume still proves instantiate + buy-from-trader. |
| **M625-3** | `VERIFY_ISSUE_622_CHAIN=1 make verify-issue-622` P0 sell extra-debit + buy net + provide/limit 1:1 on the pinned tax/EMBER pair. Missing pins fail closed (**E622-2**). Attach sell + buy screenshots. Gem `firstDualCwPair` prefers EMBER/CORAL and skips the pinned tax market. Dedicated Playwright Vite (`PLAYWRIGHT_WEB_PORT`, default **3173**) must be in indexer `CORS_ORIGINS` or `/pool` shows a market-data outage and provide cannot find `pool-pairs-table`. `e2e-start-indexer.sh` merges that origin and restarts when it was missing. |
| **M625-4** | Short `make swarm-local` / `make swarm-launch` soak: `tax_listed` extra-debit + buy split + router `trader` + `tax_hybrid_skip`. Gem workers never offer the tax token. `SWARM_TAX_WORKERS=0` is exclude-only (no tax volume). Python `tax-0` starts **before** gem workers and warms up `hybrid` then `sell` so a short leftover soak is not RNG / sequence-storm flaky. |
| **M625-5** | `make verify-issue-293` stays **OE-1 gem `pool_only`**. Do not add tax/EMBER to hub symmetry. |
| **M625-6** | Prefer a **fresh** #620 seed (`VERIFY625_FRESH=1` / `VERIFY624_FRESH=1`) so leftover #623 ephemeral tax pairs cannot steal `pairs[0]`. Tax-on seed pins are `VITE_TOKEN_COMMUNITY_TAX_*`, not factory page 0. |
| **M625-7** | Do **not** reopen #621 / #622 / #623 / #620 for ops/QA. Do not merge tax-on math into `layer-b-lt.sh` (**C623-1**). Do not turn hybrid off (**#596** / **E622-7**). Do not `test.skip` the e2e-tx spec (**E622-2**). Never whitelist columbus-5 **11611** / **11619** / ALPHA **8654**. Do not implement pair/router FoT math (**H-01**). |
| **M625-8** | This playbook + **Q13** + child skills stay crosslinked. GitLab CI quota is not a substitute for local verify. |

## Seed-path buy (leftover #1)

`layer-b-tax-on.sh` extra-debit **sell** already uses `pick_trader` (test2 when treasury is test1). The **buy** used to Send EMBER from `test1`. When `test1` is treasury, buy tax returns to the same wallet so `user credit >= pair debit` even with `buy_bps=500`. If ExemptionDirectory is on, manager-exempt buy is 1:1 (**#609**).

Buy from the same non-treasury trader as sell (`BUY_USER=$TRADER`). Ephemeral path keeps treasury=`test2` and trader=`test1` — still green.

A leftover tax-on **hostile** probe leaves AutoLP `skim_min_return=1e15`. The next `LAYER_B_TAX_ON=1` floor-success skim then reverts (`expected at least 1000000000000000`). `layer-b-tax-on.sh` clears with `skim_min_return: 0` before skim and restores after the hostile probe (**C623-8** / **M610-3**).

```bash
LAYER_B_TAX_ON=1 make verify-issue-623
jq '{source,buy_user,trader,pair_direct_buy}' cw20-codeid-audits/harness/layer-b-tax-on.json
# leftover: prove ephemeral even when #624 seed pins are present
LAYER_B_TAX_ON_FORCE_EPHEMERAL=1 \
  LAYER_B_TAX_ON_JSON=cw20-codeid-audits/harness/layer-b-tax-on-ephemeral.json \
  ./cw20-codeid-audits/scripts/layer-b-tax-on.sh
jq '{source,buy_user,trader}' cw20-codeid-audits/harness/layer-b-tax-on-ephemeral.json
```

## Playwright P0 (leftover #2)

Needs indexer + pinned tax/EMBER (not leftover ephemeral pairs). Provide opens `/pool`, which is indexer-catalog + CORS:

```bash
PLAYWRIGHT_WEB_PORT=3173 bash scripts/e2e-start-indexer.sh
VERIFY_ISSUE_622_CHAIN=1 make verify-issue-622
```

Sell user debit must equal `TaxPreview.debit` (not 1:1). That only holds when the e2e wallet is **not** the token treasury. If `/pool` shows **Market data service unavailable**, the Playwright Vite Origin is missing from `CORS_ORIGINS` (`scripts/lib/indexer-cors-playwright.sh`). If Swap is replaced by **Slippage is too high** (~30% vs hub), enable Expert Mode — seed tax/EMBER often has no tight USD mark. Provide uses `ADD_LIQUIDITY_GAS_LIMIT` **1M** (tax `TransferFrom` OOGs at 650k). Retail place is `send` → batch `n=1`; `PLACE_LIMIT_ORDER_BATCH_BASE_GAS_LIMIT` is **1M** (400k + 180k OOGs tax `Send` at 580k). Map send-inner `place_limit_order` to **1.2M**, not `SWAP_GAS_LIMIT`. Place Send stays honest; wallet/pair deltas are `declared - maker_fee` (not sell extra-debit). Cancel uses `CANCEL_LIMIT_ORDER_GAS_LIMIT` **1M** (tax refund OOGs at 450k). Cancel refund is buy-taxed unless ExemptionDirectory skip — pair returns remaining 1:1; user credit is net. Do not assert wallet restore to the pre-place balance.

## Swarm soak (leftover #3)

```bash
make swarm-launch          # or make swarm-local
# tax-0 starts first; warmup logs tax_listed + tax_hybrid_skip + tax_debit=
rg -n 'tax_listed|tax_debit|tax_hybrid_skip' scripts/bots/run/logs/tax-0.log
make swarm-stop
SWARM_TAX_WORKERS=0 make swarm-launch && make swarm-stop   # exclude-only
make verify-issue-293
```

## Do / don’t

- **Do** run `make verify-issue-625` from a git worktree after pulling `main`.
- **Do** `VERIFY625_FRESH=1` once on a tip that still has leftover #623 tax instances.
- **Do** keep `#601` smoke and generic B-lt tax-off.
- **Don’t** `AddWhitelistedCodeId` 11611 / 11619 / 8654 from LocalTerra evidence.
- **Don’t** assert buy split against manager / treasury.
- **Don’t** add tax/EMBER to OE-1 `pool_only` hubs.

## Regression

```bash
make verify-issue-625
# docs + children 621/622/623 source only (no live leftovers):
VERIFY625_SKIP_CHILDREN=1 VERIFY625_SKIP_CHAIN=1 make verify-issue-625
# fail if live leftovers are still unset:
VERIFY625_REQUIRE_LIVE=1 make verify-issue-625
# fresh volume (reset + deploy-local) then leftovers (seed + ephemeral):
VERIFY625_FRESH=1 VERIFY625_REQUIRE_LIVE=1 make verify-issue-625
```
