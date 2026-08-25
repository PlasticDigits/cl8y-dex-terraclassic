# Agent playbook: post-merge !418 community-tax migrate leftovers (GitLab #628)

Audience: third-party agents verifying the integrated tip after [!418](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/merge_requests/418) landed on `main` ([#626](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/626)). Child `make verify-issue-626` was recorded 8/8 on the source branch. GitLab CI on !418 failed with `ci_quota_exceeded` (not a code defect).

These leftovers were **not** proven on the merge tip: columbus-5 adopt-capable wasm (11619 lacks `adopt.rs`), factory list of that **new** id, Coolify/indexer pin, live 6036 cw2, LocalTerra P3 / P7 / P11, and Create Token copy that listed 6036 / 10184 / 8266 while omitting allowlisted 8654.

**Issue:** [GitLab **#628**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/628)  
**Parent (closed unless a merged invariant is wrong):** [#626](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/626) (**M626-1–M626-12**)  
**Invariants:** [`docs/qa-invariants.md`](../docs/qa-invariants.md) **Q14** (**M628-1–M628-8**)  
**Verify:** `make verify-issue-628`

Do **not** reopen #626 unless a merged invariant is wrong. Design parent [#603](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/603) stays open. ALPHA wrap vs drop [#558](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/558) stays open. Columbus-5 **code 3** stays on [#627](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/627) — do **not** append `3` to the migrate allowlist or factory whitelist from this ticket. Pair inventory leftovers live on [#634](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/634). Autoregister pin leftovers live on [#635](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/635).

Parent adopt playbook: [`AGENTS_FRONTEND_TOKEN_MIGRATE.md`](./AGENTS_FRONTEND_TOKEN_MIGRATE.md). Template: [`AGENTS_COMMUNITY_TAX_CW20.md`](./AGENTS_COMMUNITY_TAX_CW20.md). Create Token: [`AGENTS_FRONTEND_CREATE_TOKEN.md`](./AGENTS_FRONTEND_CREATE_TOKEN.md). Catalog: [`AGENTS_INDEXER_COMMUNITY_TOKENS.md`](./AGENTS_INDEXER_COMMUNITY_TOKENS.md). F6 pin: [`AGENTS_CW20_CODE_ID_PIN.md`](./AGENTS_CW20_CODE_ID_PIN.md). Listing intake: [`AGENTS_CW20_CODE_ID_AUDIT.md`](./AGENTS_CW20_CODE_ID_AUDIT.md).

## Invariants (M628-1–M628-8)

| ID | Rule |
|----|------|
| **M628-1** | Local regression is `make verify-issue-628`, which runs children **626, 592, 593, 594** plus leftover live probes. A child FAIL fails the stack. Columbus-5 / Coolify leftover SKIP unless LCD or `dex.cl8y.com` answers (FAIL when `VERIFY628_REQUIRE_LIVE=1` or `VERIFY628_IID=628`). LocalTerra P3/P7/P11 SKIP unless the chain + tax pins are up (FAIL when `VERIFY628_REQUIRE_LIVE=1` / `VERIFY628_IID=628` / `VERIFY628_REQUIRE_CHAIN=1`). |
| **M628-2** | Retail adopt target is the **current listed tax pin**, not 11619. #628 stored + listed **11626** (`adopt.rs`, REPORT GO). #635 same-crate bump **11630** is the live factory / launcher / Coolify pin (`GetWhitelistedCodeIds` **`[6036, 8266, 10184, 11630]`**). Do **not** treat 11619 as the adopt target. Optional CMM `MigrateMsg {}` of 11619 instances is N/A while instance count is 0. |
| **M628-3** | Factory-list only a GO tax pin. Never `AddWhitelistedCodeId 8654` or columbus-5 **code 3**. Do not list launcher / AutoLP / unused factory stores (**11612** / **11614** / **11620** / **11621** / **11622** / **11628** / **11629** / **11631** / **11632** / **11633**). 11619 may be removed once instance count is 0 (RAN). Keep 6036 / 8266 / 10184 listed. |
| **M628-4** | Coolify `VITE_COMMUNITY_TAX_CODE_ID` and indexer `COMMUNITY_TAX_CODE_ID` match the current listed tax pin (**11630** after #635; **11626** was the #628 store). `VITE_COMMUNITY_MIGRATE_CODE_IDS` default **6036,10184,8266,8654**. Catalog is **single-id**. Do not bake 11619 after the rotate. |
| **M628-5** | Confirm LCD cw2 on a live 6036 instance before the first 6036 adopt. `crates.io:cw20-base` is adopt-go (**S3**). `terraswap-token` / `crates.io:terraswap-token` stays page-go / chain-revert — do **not** append that name from this ticket (layout proof is a follow-up crate change). Either recorded outcome PASSes the leftover probe. |
| **M628-6** | LocalTerra leftover: adopt a factory-listed mintable gem (**P3**) — balances + `total_supply` unchanged, inbound Transfer to a CL8Y pair 1:1, admin CMM. **P7:** pair pinned to the old source id fail-closes until `RefreshPairAssetCodeIds`; after Refresh, extra-debit sell works. Do not Refresh a pair whose other asset is unlisted. **P11:** after adopt, `/token/:addr/manage` shows tax SKUs (code_id matches the env pin). Script: [`localterra-628-migrate-leftover.sh`](../scripts/qa/localterra-628-migrate-leftover.sh). |
| **M628-7** | Create Token retail copy stays code-id-free (**#489** / **C593**). Lead is “Already have a token? Migrate here”. Do **not** print 6036 / 10184 / 8266 / 8654 on the card. 8654 is a normal migrate-allowlist entry in `VITE_COMMUNITY_MIGRATE_CODE_IDS` / `communityTaxMigrate.ts` / docs — never factory-list language. Do not reopen #626 / implement #627 / RegisterListedPair a Terraport/GDEX pair / turn hybrid off / add pair-router FoT math. |
| **M628-8** | This playbook + **Q14** + child skills stay crosslinked. GitLab CI quota is not a substitute for local verify. |

## Columbus-5 leftover (already RAN)

| Step | Status |
|------|--------|
| Store optimized `cl8y-community-tax-token` with `adopt.rs` | **11626** 2026-08-24 · pin `A7244C93…D9DA1C` · REPORT GO |
| `AddWhitelistedCodeId` of the new id | **11626** listed; **11619** removed (0 instances) |
| Same-crate bump | **11630** stored + listed 2026-08-25 ([#635](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/635)) — this is the Coolify/indexer bake |
| CMM migrate of 11619 instances | **N/A** (0 instances) |

Trace: [`deployments/mainnet-soft-launch/deploy-trace.md`](../deployments/mainnet-soft-launch/deploy-trace.md) § Community tax adopt pin. Token REPORT: [`cw20-codeid-audits/codeids/11626/REPORT.md`](../cw20-codeid-audits/codeids/11626/REPORT.md). Current pin REPORT: [`cw20-codeid-audits/codeids/11630/REPORT.md`](../cw20-codeid-audits/codeids/11630/REPORT.md).

## LocalTerra leftover

`make verify-issue-626` is crates + Vitest + docs. It does **not** prove P3 / P7 / P11 on a live volume. Sibling [`localterra-634-migrate-inventory.sh`](../scripts/qa/localterra-634-migrate-inventory.sh) proves inventory + Refresh + factory-only register. #628 leftover adds balances / inbound Transfer / F6 fail-close / extra-debit after Refresh / Manage SKUs.

```bash
# existing volume with tax pins:
make has-localterra
./scripts/qa/localterra-628-migrate-leftover.sh
# or:
VERIFY628_REQUIRE_CHAIN=1 make verify-issue-628
```

Do **not** `AddWhitelistedCodeId` columbus-5 11611 / 11619 / 11626 / 11630 from LocalTerra evidence. Local store ids are ephemeral.

## Create Token leftover

The merge-tip complaint was “Create Token still says 6036 / 10184 / 8266 and omits 8654.” Retail Create Token must **not** list those ids (**#489**). The fix is the **Migrate here** link plus the migrate allowlist (including 8654) in env/docs — not a code-id essay on `/token/create`.

## Do / don’t

- **Do** run `make verify-issue-628` from a git worktree after pulling `main`.
- **Do** probe columbus-5 6036 cw2 before the first 6036 adopt.
- **Do** keep Coolify + indexer on the current listed tax pin (11630).
- **Don’t** reopen #626 / implement #627 / factory-list 8654 or code 3.
- **Don’t** `RegisterListedPair` a Terraport / GDEX pair.
- **Don’t** turn hybrid off (#596) or add pair/router FoT math (H-01).
- **Don’t** treat 11619 as the retail adopt target.

## Regression

```bash
make verify-issue-628
# docs + children only (no live leftovers):
VERIFY628_SKIP_LIVE=1 VERIFY628_SKIP_CHAIN=1 make verify-issue-628
# fail if columbus-5 / Coolify / LocalTerra leftovers cannot run:
VERIFY628_REQUIRE_LIVE=1 make verify-issue-628
# LocalTerra leftover only (fail if chain missing):
VERIFY628_REQUIRE_CHAIN=1 VERIFY628_SKIP_LIVE=1 make verify-issue-628
```
