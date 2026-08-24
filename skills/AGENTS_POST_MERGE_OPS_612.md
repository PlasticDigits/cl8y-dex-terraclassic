# Agent playbook: post-merge !407–!408 Enable Feature migrate + LocalTerra QA (GitLab #612)

Audience: third-party agents verifying **Enable Feature** after [!407](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/merge_requests/407) / [!408](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/merge_requests/408) landed on `main`. Automated `make verify-issue-606` / `make verify-issue-607` already passed on the merge commit. Columbus-5 store/rotate shared [#611](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/611).

**Issue:** [GitLab **#612**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/612)  
**Parents (closed):** [#606](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/606) / [#607](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/607)  
**Invariants:** [`docs/qa-invariants.md`](../docs/qa-invariants.md) **Q10** (**M612-1–M612-8**)  
**Verify:** `make verify-issue-612`

Do **not** reopen #606 / #607 unless a merged invariant is wrong (**T606** / **T592-13** / **C593-14**). Do **not** run the stale option-1 disclose checklist (`Route skips buy/sell tax`) — that copy moved to [#616](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/616) after [!409](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/merge_requests/409).

## Pins (do not use the issue-body 11614-only table)

The #612 issue body was written when the canonical launcher was still **11614**. The shared #611 rotate already migrated it.

| Env | Value |
|-----|--------|
| `VITE_COMMUNITY_TAX_CODE_ID` / `COMMUNITY_TAX_CODE_ID` | **11619** (catalog is single-id; keep **11611** factory-listed until Refresh) |
| `COMMUNITY_TAX_OPTION2_CODE_IDS` | **11619** |
| `VITE_COMMUNITY_TOKEN_LAUNCHER` / `COMMUNITY_TOKEN_LAUNCHER` | `terra126pr5323xkhwas7y03azv48sqr2fy3fxxg0sxu8xhmjdxr8v5tzqahzwze` (code **11622**) |

| Record | Tx |
|--------|----|
| Token store **11619** | listed 2026-08-24 height **30085794** (`B659D914…40CF`) |
| Launcher migrate 11614 → **11620** | [`97C0FCA9…EE8C`](https://finder.terraclassic.community/columbus-5/tx/97C0FCA93DFADD4BE4250935C7EFAF1CAB0A20C6FB64B2D8B774A4A8BF63EE8C) height **30085550** |
| Store launcher **11622** | [`FA30A5B1…2355`](https://finder.terraclassic.community/columbus-5/tx/FA30A5B1CD87E4D984123EE17BA05D7CFE6F852D1A93EFAB293AB7CBE0B12355) pin `8E56AE0F…8E90` |
| Migrate `terra126pr5…` 11620 → **11622** | [`F2166AB0…AAB2`](https://finder.terraclassic.community/columbus-5/tx/F2166AB0C09B4E7989AB10DC8DCC4D5855B4E3F91C7E4F8C6D5B8F780947AAB2) height **30086055** |
| `UpdateConfig` token **11619** / autolp **11621** | [`DAC86F27…6ED3`](https://finder.terraclassic.community/columbus-5/tx/DAC86F27B4E95FC83461B733453A9EF1028BC8421F2FE3AE022B0A14DADF6ED3) height **30086058** |
| Unused 11612 instance | `terra1af9xm63mev4hnf4z0nmmcsnd9f4lpac2vs205rmaeg3kdqlqudhq894lyz` |

Registry: [`deployments/mainnet-ust1-wrap/REGISTRY.md`](../deployments/mainnet-ust1-wrap/REGISTRY.md). Trace: [`deployments/mainnet-soft-launch/deploy-trace.md`](../deployments/mainnet-soft-launch/deploy-trace.md). Child playbooks: [`AGENTS_COMMUNITY_TAX_ENABLE_FEATURE.md`](./AGENTS_COMMUNITY_TAX_ENABLE_FEATURE.md) (**T606**), [`AGENTS_COMMUNITY_TAX_CW20.md`](./AGENTS_COMMUNITY_TAX_CW20.md) (**O601**), [`AGENTS_COMMUNITY_TAX_ROUTER.md`](./AGENTS_COMMUNITY_TAX_ROUTER.md) (**R607** / **T592-13**), [`AGENTS_FRONTEND_CREATE_TOKEN.md`](./AGENTS_FRONTEND_CREATE_TOKEN.md) (**C593-4** / **C593-14**), [`AGENTS_POST_MERGE_OPS_602.md`](./AGENTS_POST_MERGE_OPS_602.md) (**Q9**).

## Invariants (M612-1–M612-8)

| ID | Rule |
|----|------|
| **M612-1** | Local regression is `make verify-issue-612`, which runs children **606** and **607**. A child FAIL fails the stack. Live Coolify SKIP only when `VERIFY612_SKIP_LIVE=1`. LocalTerra / `verify-issue-601` SKIP only when the chain is down (unless `VERIFY612_REQUIRE_CHAIN=1`). |
| **M612-2** | Columbus-5 launcher `terra126pr5…` is code **11622** with `GetConfig.token_code_id` **11619** and `autolp_code_id` **11621**. Do **not** treat a 11614-only store as the Enable Feature fix. Do **not** whitelist **11612** / **11614** / **11620** / **11621** / **11622** / ALPHA **8654**. Keep **11611** listed until Refresh. |
| **M612-3** | Coolify frontend bakes `VITE_COMMUNITY_TAX_CODE_ID=11619` and launcher `terra126pr5…`. Bundle has a single `communityTaxHint` (no TS2451 duplicate). Do **not** bake unused **11612**. |
| **M612-4** | LocalTerra smoke (`localterra-community-tax-smoke.sh` via `verify-issue-601`) reports `sku_unlock_via_launcher: true`. Instantiates **current** token+launcher artifacts (11619 pin `63CB21D1…`), not 11611 — post-#605 launcher sends `initial_exempt`, which 11611 rejects. Free-profile create then Enable Feature `transfer_tax` goes manager → **launcher** → token. CMM receives 50 UST1; launcher/token do not keep the fee. |
| **M612-5** | Same smoke reports `paid_create_one_sku: true` and `sku_second_unlock_via_launcher: true`. Paid create with one SKU, then Enable Feature a second SKU (not MintControl). |
| **M612-6** | Manual Simulated Wallet (or Playwright chrome) covers Manage **Enable feature** (launcher payee, Minting omitted). Do **not** run the stale option-1 disclose checklist. Current Swap/Trade copy is `Sell tax extra` / `Buy tax applies` / `Buy/sell tax applies on every listed-pair swap.` — live option-2 copy QA lives on [#616](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/616). |
| **M612-7** | Do **not** reopen #606 / #607 for ops/QA. File a new ticket if **T606** / **T592-13** / **C593-14** is wrong. Do not implement option 2/3 classify changes on this ticket (waived / moved). |
| **M612-8** | This playbook + **Q10** + child skills stay crosslinked. GitLab CI quota is not a substitute for local verify. |

## Do / don’t

- **Do** run `make verify-issue-612` from a git worktree after pulling `main`.
- **Do** `make setup-cloud-localterra` when LocalTerra smoke SKIP and **M612-4** / **M612-5** are still open. Then `VERIFY612_REQUIRE_CHAIN=1 make verify-issue-612`.
- **Do** keep Coolify on **11619** + `terra126pr5…` (code **11622**). Rebuild the frontend (Vite bakes at build time).
- **Don’t** migrate or point product env at unused **11612** `terra1af9xm…`.
- **Don’t** whitelist launcher / AutoLP / ALPHA **8654**.
- **Don’t** treat green `make verify-issue-606` (crates) as LocalTerra Enable Feature clearance.
- **Don’t** run `Route skips buy/sell tax` as a pass criterion — that is stale option 1.

## Regression

```bash
make verify-issue-612
# docs + children only (no live Coolify / LocalTerra):
VERIFY612_SKIP_LIVE=1 VERIFY612_SKIP_CHAIN=1 make verify-issue-612
# after LocalTerra:
make setup-cloud-localterra
VERIFY612_REQUIRE_CHAIN=1 make verify-issue-612
```
