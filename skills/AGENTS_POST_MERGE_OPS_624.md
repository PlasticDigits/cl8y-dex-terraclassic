# Agent playbook: post-merge !414 LocalTerra community-tax seed leftovers (GitLab #624)

Audience: third-party agents verifying the integrated tip after [!414](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/merge_requests/414) landed on `main` ([#620](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/620)). Child `make verify-issue-620` already passed on the merge commit (8/8 including a live LCD probe of an **existing** volume). GitLab CI on !414 failed with `ci_quota_exceeded` (not a code defect).

These leftovers were **not** proven on the merge tip: fresh volume deploy, indexer catalog ingest, Transfer provision, swarm funding (not `--dry-run` alone), stamp skip, and children **601 / 592 / 610 / 594**.

**Issue:** [GitLab **#624**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/624)  
**Parent (closed unless a merged invariant is wrong):** [#620](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/620) (**L620-1–L620-8**)  
**Invariants:** [`docs/qa-invariants.md`](../docs/qa-invariants.md) **Q12** (**M624-1–M624-8**)  
**Verify:** `make verify-issue-624`

Do **not** reopen #620 / #592 / #601 / #610 / #594 unless a merged invariant is wrong. Siblings stay their own tickets: tax-aware swarm [#621](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/621), Playwright `e2e-tx` [#622](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/622), named tax-on Layer B [#623](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/623). Live leftovers from !415 / !416 / !417 live on [#625](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/625) ([`AGENTS_POST_MERGE_OPS_625.md`](./AGENTS_POST_MERGE_OPS_625.md), `make verify-issue-625`).

Parent seed playbook: [`AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED.md`](./AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED.md). Catalog: [`AGENTS_INDEXER_COMMUNITY_TOKENS.md`](./AGENTS_INDEXER_COMMUNITY_TOKENS.md) (**I594**). Ranking: [`AGENTS_INDEXER_TAX_AWARE_ROUTING.md`](./AGENTS_INDEXER_TAX_AWARE_ROUTING.md) (**R615**). Swarm funding: [`AGENTS_LOCALNET_TRADING_SWARM.md`](./AGENTS_LOCALNET_TRADING_SWARM.md).

## Invariants (M624-1–M624-8)

| ID | Rule |
|----|------|
| **M624-1** | Local regression is `make verify-issue-624`, which runs child **620** plus leftover live probes and children **601, 592, 610, 594**. A child FAIL fails the stack. Fresh-volume leftover SKIP unless `VERIFY624_FRESH=1`. Indexer leftover SKIP unless the indexer is up (FAIL when `VERIFY624_REQUIRE_LIVE=1`). LocalTerra SKIP only when the chain is down (unless `VERIFY624_REQUIRE_CHAIN=1`). |
| **M624-2** | Fresh `make reset && make start && make deploy-local` writes **local** `VITE_COMMUNITY_TAX_*` / SmokeUST1 `VITE_UST1_TOKEN_ADDRESS` / indexer `COMMUNITY_TAX_*` / `CMM_GOVERNANCE_ADDR=test1` / `COMMUNITY_TAX_OPTION2_CODE_IDS=<local id>`. LCD factory pair + `RegisterListedPair` + AutoLP `pair` match env. Reserves ≥ swarm floor (**10M** raw / side). `.qa-deploy-stamp` `git_sha` matches `HEAD`. Never `AddWhitelistedCodeId` columbus-5 **11611** / **11619**, launcher **11612** / **11614** / **11620** / **11622**, AutoLP **11613** / **11621**, or ALPHA **8654**. |
| **M624-3** | After ingest/probe, `GET /api/v1/community-tokens` is `configured: true` and lists the QA token as `attested_cmm`. Do **not** pin columbus-5 **11619** against local instances. `GET /api/v1/tokens/{tax}` may embed `community_tax`. `route/solve` sees `buy_tax_bps` / `sell_tax_bps` (**R615**). |
| **M624-4** | `bash scripts/e2e-provision-dev-wallet.sh` **Transfer**s the QA tax token from `test1` (fail-closed). Wrap CW20s skip. TCL8Y still **Mint**s. `DEPLOY_SKIP_COMMUNITY_TAX=1` stays gems-only. Playwright’s e2e wallet **is** `test1`: Transfer `test1→test1` cannot raise the balance — the seed `initial_balances` must already meet the floor. |
| **M624-5** | Swarm `fundBotWallets` never **Mint**s the tax token. `--dry-run` **skips** `fundBotWallets` (it is not a funding proof) and logs `kind: "swarm_funding_plan"` (classify only). Live funding logs `kind: "swarm_funding_action"` with `fundingKind: "transfer"` for the QA token. Same fork as `fundingExecuteMsg` / `classify_cw20_funding_kind`. |
| **M624-6** | Re-run without `--fresh`: `deploy_up_to_date` stamp skip / safe no-op (no second tax pair). Phase 4d itself **always** stores + paid-creates if the whole `deploy-dex-local.sh` runs — do not invoke that script again on a live volume to “prove” skip. |
| **M624-7** | Do **not** reopen #620 / #592 / #601 / #610 / #594 for ops/QA. Do not enable `MintControl` or fall back to Mint when Transfer fails. Do not implement pair/router FoT math (**H-01**). `#601` smoke stays ephemeral. |
| **M624-8** | This playbook + **Q12** + child skills stay crosslinked. GitLab CI quota is not a substitute for local verify. |

## Fresh volume leftover

`make verify-issue-620` on an **existing** LocalTerra volume is not leftover #1. Prove a clean store + seed:

```bash
make reset && make start && make wait-healthy && make deploy-local
# stamp git_sha must match HEAD
grep -E '^(git_sha|community_tax_)' .qa-deploy-stamp
grep VITE_COMMUNITY VITE_UST1 frontend-dapp/.env.local
grep COMMUNITY_TAX indexer/.env
```

`make reset` only runs `docker compose down -v`. Start compose before deploy (`make start`). `VERIFY624_FRESH=1` does reset + start + `make deploy-local` (optimizer) + indexer restart.

Then `VERIFY624_FRESH=1 make verify-issue-624` (the harness runs reset + deploy itself) or probe the leftovers on an already-fresh tip.

## Indexer leftover

Deploy-written env is required (`COMMUNITY_TAX_CODE_ID` = **local** store id, `CMM_GOVERNANCE_ADDR=test1`). Restart the indexer so it reads the new `.env`:

```bash
# after deploy-local
bash scripts/e2e-start-indexer.sh   # or make indexer-dev
curl -sS http://127.0.0.1:3001/api/v1/community-tokens | jq '{configured,code_id,items:[.items[]|{address,attested_cmm,buy_bps,sell_bps}]}'
```

Do not set `COMMUNITY_TAX_CODE_ID=11619` while instances are the local store.

## Swarm `--dry-run` is not funding

`--dry-run` validates env, prints `swarm_funding_plan`, and **does not** call `fundBotWallets`. Treat a dry-run classify log as leftover **documentation**, not leftover #5. Live funding (provision script or swarm without `--dry-run`) is the Transfer proof.

## Do / don’t

- **Do** run `make verify-issue-624` from a git worktree after pulling `main`.
- **Do** `VERIFY624_FRESH=1` once on a tip that has never had `make reset && make deploy-local` (shared compose project `cl8y-dex-terraclassic`).
- **Do** keep `#601` smoke ephemeral — it is not the reused QA tax market.
- **Don’t** `AddWhitelistedCodeId` 11611 / 11619 / launcher / AutoLP / ALPHA **8654** from LocalTerra evidence.
- **Don’t** enable `MintControl` on the QA token so funding can Mint.
- **Don’t** re-run `scripts/deploy-dex-local.sh` on a live volume to test skip (Phase 4d is not idempotent).
- **Don’t** treat swarm `--dry-run` as a `fundBotWallets` proof.

## Regression

```bash
make verify-issue-624
# docs + child 620 + source only (no live leftovers / children 601…):
VERIFY624_SKIP_CHILDREN=1 VERIFY624_SKIP_CHAIN=1 make verify-issue-624
# fail if indexer / provision leftovers are still unset:
VERIFY624_REQUIRE_LIVE=1 make verify-issue-624
# fresh volume (reset + deploy-local) then leftovers:
VERIFY624_FRESH=1 make verify-issue-624
```
