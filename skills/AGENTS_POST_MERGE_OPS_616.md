# Agent playbook: post-merge !409–!413 option-2 wasm, wrap/window fees, AutoLP, tax ranking (GitLab #616)

Audience: third-party agents verifying the integrated tip after [!409](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/merge_requests/409)–[!413](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/merge_requests/413) landed on `main`. Child `make verify-issue-{607,610,613,614,615}` already passed on the merge commits. Columbus-5 store/rotate shared [#611](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/611). Enable Feature migrate remainder is [#612](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/612).

**Issue:** [GitLab **#616**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/616)  
**Parents (closed unless noted):** [#607](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/607) / [#610](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/610) / [#613](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/613) / [#614](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/614) / [#615](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/615). Window mint/redeem ingest playbook: [`AGENTS_INDEXER_UST1_WINDOW_FEES.md`](./AGENTS_INDEXER_UST1_WINDOW_FEES.md) (**I614**).  
**Invariants:** [`docs/qa-invariants.md`](../docs/qa-invariants.md) **Q11** (**M616-1–M616-8**)  
**Verify:** `make verify-issue-616`

Do **not** reopen #607 / #610 / #613 / #615 unless a merged invariant is wrong (**R607** / **M610** / **I613** / **R615**). Do **not** run the stale option-1 disclose checklist (`Route skips buy/sell tax`).

## Pins

| Env | Value |
|-----|--------|
| `VITE_COMMUNITY_TAX_CODE_ID` / `COMMUNITY_TAX_CODE_ID` | **11619** (catalog is single-id; keep **11611** factory-listed until Refresh) |
| `COMMUNITY_TAX_OPTION2_CODE_IDS` | **11619** after a live 11619 instance exists — **not** `11611` until those instances migrate (**R615-5**) |
| `VITE_COMMUNITY_TOKEN_LAUNCHER` / `COMMUNITY_TOKEN_LAUNCHER` | `terra126pr5323xkhwas7y03azv48sqr2fy3fxxg0sxu8xhmjdxr8v5tzqahzwze` (code **11622**) |
| Indexer `UST1_WINDOW_ADDRESS` | `terra1zxwpzpzpleatqn39r00grau4yt29sld8pw78s7ktvjafnj5nsaxq0h3rh2` (window wasm **11618**). Vite `VITE_UST1_WINDOW_ADDRESS` is **not** enough. |
| Wrap mapper | `terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2` |

| Record | Tx |
|--------|----|
| Token store **11619** listed | height **30085794** (`B659D914…40CF`) |
| Launcher migrate 11620 → **11622** | [`F2166AB0…AAB2`](https://finder.terraclassic.community/columbus-5/tx/F2166AB0C09B4E7989AB10DC8DCC4D5855B4E3F91C7E4F8C6D5B8F780947AAB2) height **30086055** |
| `UpdateConfig` token **11619** / autolp **11621** | [`DAC86F27…6ED3`](https://finder.terraclassic.community/columbus-5/tx/DAC86F27B4E95FC83461B733453A9EF1028BC8421F2FE3AE022B0A14DADF6ED3) height **30086058** |
| Window migrate 11566 → **11618** | [`009BD391…747D`](https://finder.terraclassic.community/columbus-5/tx/009BD391) (ust1-window#33 `fee_amount`) |

Registry: [`deployments/mainnet-ust1-wrap/REGISTRY.md`](../deployments/mainnet-ust1-wrap/REGISTRY.md). Trace: [`deployments/mainnet-soft-launch/deploy-trace.md`](../deployments/mainnet-soft-launch/deploy-trace.md). Child playbooks: [`AGENTS_COMMUNITY_TAX_ROUTER.md`](./AGENTS_COMMUNITY_TAX_ROUTER.md) (**R607**), [`AGENTS_COMMUNITY_TAX_AUTOLP.md`](./AGENTS_COMMUNITY_TAX_AUTOLP.md) (**M610**), [`AGENTS_INDEXER_WRAP_FEE_INGEST.md`](./AGENTS_INDEXER_WRAP_FEE_INGEST.md) (**I613**), [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) (**PFee-13** / [#614](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/614)), [`AGENTS_INDEXER_TAX_AWARE_ROUTING.md`](./AGENTS_INDEXER_TAX_AWARE_ROUTING.md) (**R615**), [`AGENTS_POST_MERGE_OPS_612.md`](./AGENTS_POST_MERGE_OPS_612.md) (**Q10**).

## Invariants (M616-1–M616-8)

| ID | Rule |
|----|------|
| **M616-1** | Local regression is `make verify-issue-616`, which runs children **607, 610, 613, 614, 615**. A child FAIL fails the stack. Live Coolify leftover probes (window pin / wrap `event_count` / option-2 env) SKIP unless `VERIFY616_REQUIRE_LIVE_LEFTOVERS=1`. LocalTerra SKIP only when the chain is down (unless `VERIFY616_REQUIRE_CHAIN=1`). |
| **M616-2** | Columbus-5 launcher `terra126pr5…` is code **11622** with `GetConfig.token_code_id` **11619** and `autolp_code_id` **11621**. Factory whitelist includes **11611** and **11619**. Do **not** whitelist **11612** / **11613** / **11614** / **11620** / **11621** / **11622** / ALPHA **8654**. Zero 11611/11613/11619/11621 instances is OK until Refresh — do not CMM-migrate nothing. |
| **M616-3** | Coolify frontend bakes `VITE_COMMUNITY_TAX_CODE_ID=11619` + launcher `terra126pr5…`. Indexer catalog `code_id=11619`. Indexer `UST1_WINDOW_ADDRESS` is pinned (`ust1_window_configured: true`). Live wrap/unwrap and `ust1_mint` / `ust1_redeem` `event_count` increment on `indexer.dex.cl8y.com` (2026-08-25). Vite-only window bake is not enough. |
| **M616-4** | Swap/Trade pair-direct **and** multi-hop (`usesRouter`) show `Sell tax extra` / `Buy tax applies`. Create/Manage: `Buy/sell tax applies on every listed-pair swap.` Coolify bundle must not ship `Route skips buy/sell tax`. Extra-debit Max applies on router-hop sells (**R607-7**). |
| **M616-5** | AutoLP skim against a factory-listed tax pair always sets a spread floor (default 100 bps, cap 200). Floor revert keeps tax on AutoLP (**M610-3** / **M610-4**). Do not whitelist AutoLP. LocalTerra skim-vs-real-pair is optional when the chain is down; crate **610** still must stay green. |
| **M616-6** | Ranking: TAX→UST1 vs TAX→USTR / UST1→TAX→USTR. Unmigrated **11611** stays Honest hops until `COMMUNITY_TAX_OPTION2_CODE_IDS` / `DATA_HASHES` lists that id/hash (**R615-5**). New crate **11619** may be listed in option-2 env after an instance exists. Do not infer wrap or window fees from `amount × bps`. |
| **M616-7** | Do **not** reopen #607 / #610 / #613 / #614 / #615 for ops/QA. Live window deposit+withdraw increment `ust1_mint` / `ust1_redeem` on `/protocol`. File a new ticket if a merged invariant is wrong. Do not implement pair/router FoT math (**H-01**) or turn hybrid off (#596). |
| **M616-8** | This playbook + **Q11** + child skills stay crosslinked. GitLab CI quota is not a substitute for local verify. |

## Coolify leftovers (do not treat green children as live ingest)

Window pin landed (`ust1_window_configured: true`). Live wrap/unwrap and window mint/redeem `event_count` increment as of 2026-08-25. `make verify-issue-616` still records leftover probes as SKIP unless `VERIFY616_REQUIRE_LIVE_LEFTOVERS=1`.

```
# already pinned on indexer — keep on rebuild
UST1_WINDOW_ADDRESS=terra1zxwpzpzpleatqn39r00grau4yt29sld8pw78s7ktvjafnj5nsaxq0h3rh2
COMMUNITY_TAX_CODE_ID=11619
COMMUNITY_TOKEN_LAUNCHER=terra126pr5323xkhwas7y03azv48sqr2fy3fxxg0sxu8xhmjdxr8v5tzqahzwze
# after a live 11619 instance (do not add 11611 until migrate)
COMMUNITY_TAX_OPTION2_CODE_IDS=11619
```

Then one captured wrap (uusd and/or uluna) so wrap `event_count ≥ 1`, and one window deposit + withdraw so Source shows **UST1 mint** / **UST1 redeem**. InstantWithdraw `tax_amount` is not stored.

## Do / don’t

- **Do** run `make verify-issue-616` from a git worktree after pulling `main`.
- **Do** `make setup-cloud-localterra` when LocalTerra SKIP and **M616-4** / **M616-5** execute rungs are still open. Then `VERIFY616_REQUIRE_CHAIN=1 make verify-issue-616`.
- **Do** keep 11611 listed until Refresh. Do not `RemoveWhitelistedCodeId 11611`.
- **Don’t** treat unmigrated 11611 as option 2 for ranking.
- **Don’t** whitelist launcher / AutoLP / ALPHA **8654**.
- **Don’t** infer wrap or window fees from `amount × bps`.
- **Don’t** run `Route skips buy/sell tax` as a pass criterion.

## Regression

```bash
make verify-issue-616
# docs + children only (no live Coolify leftovers / LocalTerra):
VERIFY616_SKIP_LIVE=1 VERIFY616_SKIP_CHAIN=1 make verify-issue-616
# fail if Coolify leftovers are still unset:
VERIFY616_REQUIRE_LIVE_LEFTOVERS=1 make verify-issue-616
# after LocalTerra:
make setup-cloud-localterra
VERIFY616_REQUIRE_CHAIN=1 make verify-issue-616
```
