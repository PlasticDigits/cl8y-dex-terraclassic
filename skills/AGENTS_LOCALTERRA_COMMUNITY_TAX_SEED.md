# Agent playbook: LocalTerra community-tax seed (GitLab #620)

Use when changing **`make deploy-local`** community-tax wiring, LocalTerra funding (Mint vs Transfer), or indexer/dApp env pins for the QA tax market.

This is **stack wiring only**. Do **not** reopen [#592](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/592) / [#601](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/601) / [#610](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/610) / [#615](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/615) unless an on-chain invariant is wrong. Sibling follow-ups: tax-aware swarm [#621](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/621) ([`AGENTS_LOCALNET_SWARM_TAX.md`](./AGENTS_LOCALNET_SWARM_TAX.md)), Playwright `e2e-tx` [#622](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/622), named tax-on Layer B [#623](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/623).

Parent template: [`AGENTS_COMMUNITY_TAX_CW20.md`](./AGENTS_COMMUNITY_TAX_CW20.md) (**T592** / **O601**). AutoLP: [`AGENTS_COMMUNITY_TAX_AUTOLP.md`](./AGENTS_COMMUNITY_TAX_AUTOLP.md) (**M610**). Catalog: [`AGENTS_INDEXER_COMMUNITY_TOKENS.md`](./AGENTS_INDEXER_COMMUNITY_TOKENS.md) (**I594**). Swarm funding: [`AGENTS_LOCALNET_TRADING_SWARM.md`](./AGENTS_LOCALNET_TRADING_SWARM.md).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#620**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/620) | LocalTerra seed + Transfer funding + indexer env |
| [`scripts/lib/deploy-community-tax-local.sh`](../scripts/lib/deploy-community-tax-local.sh) | Store / launcher / paid create / pair / AutoLP |
| [`scripts/lib/cw20-funding-kind.sh`](../scripts/lib/cw20-funding-kind.sh) | Bash classify: skip / transfer / mint |
| [`fundingKind.ts`](../packages/localnet-trading-swarm/src/fundingKind.ts) | TS classify (same fork) |
| [`scripts/e2e-provision-dev-wallet.sh`](../scripts/e2e-provision-dev-wallet.sh) | Factory CW20 top-up |
| [`docs/contracts-security-audit.md`](../docs/contracts-security-audit.md) **L620-1** | Invariant table |
| [`localterra-community-tax-smoke.sh`](../scripts/qa/localterra-community-tax-smoke.sh) | **Ephemeral** #601 smoke — do not replace this seed |

## Invariants **L620-1–L620-8**

1. **L620-1 — local store id only.** `AddWhitelistedCodeId` the LocalTerra token store id. Never whitelist columbus-5 **11611** / **11619**, launcher **11612** / **11614** / **11620** / **11622**, AutoLP **11613** / **11621**, or ALPHA **8654** from this script. If the local store id collides with those numbers, abort.
2. **L620-2 — funding fork.** Wrap CW20s skip Mint. Community-tax (`VITE_TOKEN_COMMUNITY_TAX_ADDRESS` or `GetLauncherOrigin.launcher`) **Transfer** from `test1`. Gems and TCL8Y stay **Mint**. Transfer failure is **fail-closed** — do not add `MintControl` or fall back to Mint.
3. **L620-3 — QA profile.** Default seed token has **no** `MintControl`, no `LaunchGuards` SKU (`trading_enabled` stays live), buy/sell **500 bps**, large `initial_balances` to `test1`. A “safe launch” preset will brick swarm/E2E.
4. **L620-4 — AutoLP listed pair.** Paid create `features: ["auto_v2_lp"]` (50 UST1 to launcher). After `CreatePair` + `RegisterListedPair`, manager `UpdateConfig { pair }` so `GetConfig.pair` is the factory pair (**M610-1**). `SkimToLp` is permissionless and **never** called from token `Transfer`/`Send` (**T592-10**).
5. **L620-5 — env pins.** Default `make deploy-local` writes local `VITE_COMMUNITY_TAX_CODE_ID` / `VITE_COMMUNITY_TOKEN_LAUNCHER` / `VITE_TOKEN_COMMUNITY_TAX_ADDRESS` / `VITE_PAIR_COMMUNITY_TAX_EMBER` / SmokeUST1 `VITE_UST1_TOKEN_ADDRESS`, and indexer `COMMUNITY_TAX_CODE_ID` / `COMMUNITY_TOKEN_LAUNCHER` / `CMM_GOVERNANCE_ADDR=test1` / `COMMUNITY_TAX_OPTION2_CODE_IDS=<local id>`. `DEPLOY_SKIP_COMMUNITY_TAX=1` leaves those unset (gems only). Do not bake columbus-5 **11611**.
6. **L620-6 — swarm floor.** Seed LP ≥ **10M** raw / side (`liquidityGuards.ts` / `BOTS_MIN_RESERVE_PER_SIDE`). Provide stays **1:1** (**T592-1**).
7. **L620-7 — stand-ins.** Local UST1 is mintable invoice gas only — not columbus-5 UST1 / window. CMM stand-in is `test1` (`terra1x46rq…`), not `terra16j5u6…`.
8. **L620-8 — not a FoT license.** Do not add pair/router FoT math (**H-01**). `#601` smoke stays ephemeral and green. QA token is not a license to whitelist other tax wasm.

## Verify

```bash
make verify-issue-620
# Optional live (after make deploy-local):
# grep VITE_COMMUNITY VITE_UST1 frontend-dapp/.env.local
# grep COMMUNITY_TAX indexer/.env
# bash scripts/e2e-provision-dev-wallet.sh
# cd packages/localnet-trading-swarm && npm run start -- --dry-run
```

`make reset` / `--fresh` must recreate the tax market — do not rely on a stale `.qa-deploy-stamp` alone ([#325](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/325)).

## Do not

- `AddWhitelistedCodeId` 11611/11619 from LocalTerra evidence.
- Enable `MintControl` on the default QA token so funding can Mint.
- Ship `trading_enabled=false` on the QA profile.
- Point AutoLP `pair` at a non-factory or wrong-token contract.
- Call `SkimToLp` from taxed `Transfer`/`Send`.
- Set indexer `COMMUNITY_TAX_CODE_ID` to columbus-5 **11619** while instances are the local store.
- Send Enable Feature invoices to the **token** instead of the launcher (**T606-7**).
