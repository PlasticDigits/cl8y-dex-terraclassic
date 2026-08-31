# Agent playbook: post-merge !402 Coolify + launcher + LocalTerra Create Token (GitLab #602)

Audience: third-party agents verifying **Create Token + catalog** after [!402](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/merge_requests/402) landed on `main` without Coolify bake / columbus-5 free-create / LocalTerra retail QA. Automated `make verify-issue-593` / `make verify-issue-594` already passed on the merge commit.

**Issue:** [GitLab **#602**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/602)  
**Parents (closed):** [#593](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/593) / [#594](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/594) / [#601](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/601)  
**Invariants:** [`docs/qa-invariants.md`](../docs/qa-invariants.md) **Q9** (**M602-1–M602-8**)  
**Verify:** `make verify-issue-602`

Do **not** reopen #593 / #594 / #601 unless a merged invariant is wrong (**C593** / **I594** / **O601**).

On-chain store/listing already landed in [#601](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/601). This ticket is **product env + free-create on the canonical launcher + retail QA**.

## Pins (do not use the issue-body 11612 table)

The #602 issue body still lists unused launcher **11612** (`terra1af9xm…`). That instance predates `ExecuteMsg::CreateToken` and is **not migratable** (CMM treasury wasm admin). [#601](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/601) **re-stored** the in-repo launcher as **11614**.

| Env | Value |
|-----|--------|
| `VITE_COMMUNITY_TAX_CODE_ID` / `COMMUNITY_TAX_CODE_ID` | **11619** (catalog is single-id; 11611 stays factory-listed) |
| `VITE_COMMUNITY_TOKEN_LAUNCHER` / `COMMUNITY_TOKEN_LAUNCHER` | `terra126pr5323xkhwas7y03azv48sqr2fy3fxxg0sxu8xhmjdxr8v5tzqahzwze` (code **11622**; store was **11614**) |
| `CMM_GOVERNANCE_ADDR` | `terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2` |

| Record | Tx |
|--------|----|
| 11614 store (`CreateToken`) | [`33F6A49F…45B8`](https://finder.terraclassic.community/columbus-5/tx/33F6A49F7221A377132D0A2B534A48D5AC64A5CA1F30D20BBE8A34086D3A45B8) height **30072268** |
| Canonical launcher instantiate | [`041E3C43…87FE`](https://finder.terraclassic.community/columbus-5/tx/041E3C4379E88CE073B2EEED0125BEC58BCCAC31711AC15500352581763287FE) height **30072275** |
| Launcher migrate 11614 → **11620** | [`97C0FCA9…EE8C`](https://finder.terraclassic.community/columbus-5/tx/97C0FCA93DFADD4BE4250935C7EFAF1CAB0A20C6FB64B2D8B774A4A8BF63EE8C) height **30085550** ([#611](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/611)) |
| Launcher migrate 11620 → **11622** + `UpdateConfig` 11619/11621 | [`F2166AB0…AAB2`](https://finder.terraclassic.community/columbus-5/tx/F2166AB0C09B4E7989AB10DC8DCC4D5855B4E3F91C7E4F8C6D5B8F780947AAB2) / [`DAC86F27…6ED3`](https://finder.terraclassic.community/columbus-5/tx/DAC86F27B4E95FC83461B733453A9EF1028BC8421F2FE3AE022B0A14DADF6ED3) height **30086055** / **30086058** |
| Unused 11612 instance | `terra1af9xm63mev4hnf4z0nmmcsnd9f4lpac2vs205rmaeg3kdqlqudhq894lyz` |

Registry: [`deployments/mainnet-ust1-wrap/REGISTRY.md`](../deployments/mainnet-ust1-wrap/REGISTRY.md). Trace: [`deployments/mainnet-soft-launch/deploy-trace.md`](../deployments/mainnet-soft-launch/deploy-trace.md). Coolify keys: [`deployments/mainnet-ust1-wrap/coolify.env.example`](../deployments/mainnet-ust1-wrap/coolify.env.example).

Child playbooks: [`AGENTS_FRONTEND_CREATE_TOKEN.md`](./AGENTS_FRONTEND_CREATE_TOKEN.md) (**C593**), [`AGENTS_INDEXER_COMMUNITY_TOKENS.md`](./AGENTS_INDEXER_COMMUNITY_TOKENS.md) (**I594**), [`AGENTS_COMMUNITY_TAX_CW20.md`](./AGENTS_COMMUNITY_TAX_CW20.md) (**O601-3**), [`AGENTS_FRONTEND_CREATE_PAIR_PICKER.md`](./AGENTS_FRONTEND_CREATE_PAIR_PICKER.md) (**C542-11**), [`AGENTS_FRONTEND_PAY_INVOICE.md`](./AGENTS_FRONTEND_PAY_INVOICE.md).

## Invariants (M602-1–M602-8)

| ID | Rule |
|----|------|
| **M602-1** | Local regression is `make verify-issue-602`, which runs children **593** and **594**. A child FAIL fails the stack. Live Coolify SKIP only when `VERIFY602_SKIP_LIVE=1`. LocalTerra smoke SKIP only when the chain is down (unless `VERIFY602_REQUIRE_CHAIN=1`). |
| **M602-2** | **P402-1.** Coolify frontend rebuild bakes `VITE_COMMUNITY_TAX_CODE_ID=11619` and launcher `terra126pr5…` (code **11622**). `https://dex.cl8y.com` More menu shows **Create Token**; `/token/create` is not the unavailable stub. Do **not** bake unused **11612** `terra1af9xm…`. |
| **M602-3** | **P402-2.** Coolify indexer has `COMMUNITY_TAX_CODE_ID`, `COMMUNITY_TOKEN_LAUNCHER`, `CMM_GOVERNANCE_ADDR`. `GET https://indexer.dex.cl8y.com/api/v1/community-tokens` returns `{ configured: true, … }` (empty `items` is OK until a launcher create is ingested). |
| **M602-4** | **P402-3.** Columbus-5 free create lives on **11614** (store + instantiate txs above). **11612** is unused. Do **not** whitelist 11612 / 11613 / 11614 on the factory. |
| **M602-5** | **P402-4.** LocalTerra: in-repo launcher smoke (`localterra-community-tax-smoke.sh`) plus UI — free create, paid SKU via `PayWithAnyToken`, Manage Save flat 50 UST1, non-manager read-only, Unverified admin when admin ≠ CMM. |
| **M602-6** | **P402-5.** After create, Create Token may still copy-address to `/create`. Create Pair honors `/create?a=&b=` prefill ([#713](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/713) **C542-11**). |
| **M602-7** | **P402-6.** Swap/Trade Max on the tax template is extra-debit (not 100% balance). Indexer list shows a token **without** a pair; default list is `attested_cmm` only. |
| **M602-8** | This playbook + **Q9** + child skills stay crosslinked. Do not wait for GitHub Actions; GitLab CI may be quota-blocked — local `make verify-issue-*` is the gate. |

## Do / don’t

- **Do** run `make verify-issue-602` from a git worktree after pulling `main`.
- **Do** `make setup-cloud-localterra` when LocalTerra smoke SKIP and **P402-4** is still open. Then `VERIFY602_REQUIRE_CHAIN=1 make verify-issue-602`.
- **Do** point Coolify at **11614** `terra126pr5…`. Rebuild the frontend (Vite bakes at build time) and restart the indexer.
- **Don’t** migrate or point product env at unused **11612** `terra1af9xm…`.
- **Don’t** whitelist launcher **11612** / **11614** or AutoLP **11613**.
- **Don’t** treat green `make verify-issue-593` (Vitest) as Coolify or LocalTerra retail clearance.
- **Don’t** reopen [#593](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/593) / [#594](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/594) unless **C593** / **I594** is wrong — file a new ticket.

## Regression

```bash
make verify-issue-602
# docs + children only (no live Coolify / LocalTerra):
VERIFY602_SKIP_LIVE=1 VERIFY602_SKIP_CHAIN=1 make verify-issue-602
# after LocalTerra:
make setup-cloud-localterra
VERIFY602_REQUIRE_CHAIN=1 make verify-issue-602
```
