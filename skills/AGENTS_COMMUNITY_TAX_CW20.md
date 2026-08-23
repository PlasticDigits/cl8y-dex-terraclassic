# Agent playbook: community tax CW20 (GitLab #592)

Use when changing the **community tax token**, **launcher**, or **AutoLP** sister, or when listing that template on the factory.

This is the **only** tax-token exception to **H-01**. It is DEX-safe because inbound credits to pair / router / escrow / AutoLP stay **1:1**. Do **not** add pair/router FoT balance-delta math. Factory `AddWhitelistedCodeId` is a **separate ops step** after `#589` REPORT **GO**.

Sibling surfaces: dApp create/manage [#593](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/593) ([`AGENTS_FRONTEND_CREATE_TOKEN.md`](./AGENTS_FRONTEND_CREATE_TOKEN.md)); indexer catalog [#594](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/594) ([`AGENTS_INDEXER_COMMUNITY_TOKENS.md`](./AGENTS_INDEXER_COMMUNITY_TOKENS.md)); pay-with-any-token [#595](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/595) (**I595-14** — launcher/token still accept UST1 `Send` only); post-merge Coolify + LocalTerra [#602](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/602) ([`AGENTS_POST_MERGE_OPS_602.md`](./AGENTS_POST_MERGE_OPS_602.md)); LaunchGuards liveness [#608](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/608) ([`AGENTS_COMMUNITY_TAX_LAUNCH_GUARDS.md`](./AGENTS_COMMUNITY_TAX_LAUNCH_GUARDS.md) **H608-1–H608-8**).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#592**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/592) | On-chain design |
| [`docs/contracts-terraclassic.md` § Community tax CW20](../docs/contracts-terraclassic.md#community-tax-cw20-gitlab-592) | Message shapes + classification |
| [`docs/contracts-security-audit.md`](../docs/contracts-security-audit.md) | Invariants **T592-1–T592-12** + **O601-1–O601-7** |
| [`docs/runbooks/cw20-whitelist-policy.md`](../docs/runbooks/cw20-whitelist-policy.md) | Narrow whitelist exception |
| [`cw20-codeid-audits/codeids/11611/REPORT.md`](../cw20-codeid-audits/codeids/11611/REPORT.md) | #589 intake (**GO**; columbus-5 listed). Stub: [`community-tax-token/REPORT.md`](../cw20-codeid-audits/codeids/community-tax-token/REPORT.md) |
| [`AGENTS_HOOK_CW20_OPS.md`](./AGENTS_HOOK_CW20_OPS.md) | **H-01** / do not add FoT math |
| [`AGENTS_CW20_CODE_ID_AUDIT.md`](./AGENTS_CW20_CODE_ID_AUDIT.md) | Listing intake |
| [`AGENTS_FRONTEND_PAY_INVOICE.md`](./AGENTS_FRONTEND_PAY_INVOICE.md) | Off-chain any-token pay |
| [`deployments/mainnet-ust1-wrap/REGISTRY.md`](../deployments/mainnet-ust1-wrap/REGISTRY.md) | Columbus-5 **11611** / launcher `terra126pr5…ahzwze` (11614) |

## Columbus-5 pins (#601)

| Role | Value |
|------|--------|
| Token code_id (listed) | **11611** · `data_hash` `9D33BF2539A9A5B2F13FD4B321CDBD0B0FD86D936D5D6BD6681955FA30210EC2` |
| Launcher (canonical) | `terra126pr5323xkhwas7y03azv48sqr2fy3fxxg0sxu8xhmjdxr8v5tzqahzwze` (code **11614**, wasm admin DEX 2-of-3 `terra1zlmv2…hep7`) |
| AutoLP code_id | **11613** (stored; not factory-whitelisted) |
| Token wasm admin / CMM | `terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2` (stamped on create; not launcher `ContractInfo.admin`) |
| Factory whitelist | `[6036, 8266, 10184, 11611]` |

`VITE_COMMUNITY_TAX_CODE_ID` / `COMMUNITY_TAX_CODE_ID` = 11611. `VITE_COMMUNITY_TOKEN_LAUNCHER` / `COMMUNITY_TOKEN_LAUNCHER` = launcher. Unset → dApp page unavailable / indexer `configured: false`.

## Ops / LocalTerra invariants **O601-1–O601-7** ([#601](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/601))

1. **O601-1 — pinned A-lcd / B-lt.** `CODE_ID=11611 LAYER_B_LT=1 make verify-issue-589` executes the LCD wasm (not mintable analogue). A-lcd retries community-tax `InstantiateMsg` when cw20-base init fails. `balance_at` is A29 N/A.
2. **O601-2 — whitelist after GO only.** Factory lists **11611**. Do **not** whitelist launcher **11612** / **11614**, AutoLP **11613**, a LocalTerra store id, or ALPHA **8654**.
3. **O601-3 — free-profile launcher create.** `features == []` uses `ExecuteMsg::CreateToken` (CW20 cannot `Send` 0). Paid SKUs stay on UST1 `Send`. Stamps token `admin: cmm_governance` and `GetLauncherOrigin`. Canonical columbus-5 launcher is **11614** (`terra126pr5…ahzwze`). **11612** (`terra1af9xm…`) predates this execute and is not migratable (CMM treasury wasm admin); leave unused.
4. **O601-4 — catalog filter.** Rogue `--admin` instantiate has `GetLauncherOrigin.launcher == null`. dApp (#593) / indexer (#594) must require CMM admin + origin.
5. **O601-5 — listed-pair tax.** After `RegisterListedPair`, sell extra-debit + buy outbound split match `TaxPreview` (max-button). Provide `TransferFrom` stays 1:1. Layer B-lt does **not** register the pair (1:1 harness).
6. **O601-6 — invoices on chain.** SKU unlock and settings batch are each exactly 50 UST1. MintControl `RevokeMint` is one-way.
7. **O601-7 — DEX residuals stay out.** Hybrid / router multi-hop / live zap / AutoLP `SkimToLp` vs a live router are **not** close gates (issue body §5).

Smoke: [`scripts/qa/localterra-community-tax-smoke.sh`](../scripts/qa/localterra-community-tax-smoke.sh). Gate: `make verify-issue-601`.

## Invariants **T592-1–T592-12**

1. **T592-1 — inbound 1:1.** Transfers **to** pair, router, this token, AutoLP, or other protocol-exempt addresses credit exactly `amount`. Classic inbound FoT still fails `fee_on_transfer_creates_reserve_imbalance`.
2. **T592-2 — sell extra-debit.** `Send` to a registered listed pair with `Cw20HookMsg::Swap` debits `amount + tax` and credits the pair `amount`.
3. **T592-3 — buy outbound split.** `Transfer`/`Send` **from** a registered listed pair debits the pair `amount`; trader + sinks = `amount`.
4. **T592-4 — invoices.** SKU unlock and settings batch are each **exactly 50 UST1** (`50000000`). Forwarded to CMM treasury. Wrong token / wrong amount / no-op / unactivated SKU → revert, fee not kept. Do not mix `EnableFeature` into a settings batch.
5. **T592-5 — CMM wasm admin.** Launcher `Instantiate { admin: cmm_governance }`. Manager cannot migrate or `UpdateAdmin`. Rogue instantiate with another admin is not catalog-promoted (`GetLauncherOrigin`).
6. **T592-6 — MintControl instantiate-only.** `RevokeMint` is one-way and requires a settings invoice. `Mint` itself is not invoiced.
7. **T592-7 — classification.** Provide (`TransferFrom`) and limit `PlaceLimitOrder*` `Send` are 1:1. Pair→EOA `Transfer` (swap receive / withdraw / limit refund) uses **buy tax** — same CosmWasm primitive; documented, not a pair wasm change.
8. **T592-8 — no rug APIs.** No reflection, rebase, pause, or blacklist manager methods.
9. **T592-9 — protocol exemptions.** `RegisterListedPair` requires factory `Pair` lookup. Manager cannot remove protocol entries.
10. **T592-10 — AutoLP deferred.** `SkimToLp` is permissionless and is never called from token `Transfer`/`Send` or pair `AfterSwap`.
11. **T592-11 — launch guards.** `trading_enabled=false` blocks **both** buy and sell (**H-5** residual). Cooldown is **per user wallet** — do not check or record listed pairs / protocol-exempt addresses. `max_wallet` skips protocol / listed pair as `to` (provide + sell-to-pair bypass). User Buy / Transfer still capped. Full **H608-1–H608-8** in [`AGENTS_COMMUNITY_TAX_LAUNCH_GUARDS.md`](./AGENTS_COMMUNITY_TAX_LAUNCH_GUARDS.md) ([#608](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/608)).
12. **T592-12 — UST1-only invoice.** #595 routes any token off-chain. Token/launcher do not accept the tax token as fee.

## Launcher UST1 hook

`community-token-launcher` `Receive` accepts UST1 only. Exact invoice: `50 UST1 × SKU count` on create; `50 UST1` on later `EnableFeature`. **O601-3:** zero-SKU free profile is `ExecuteMsg::CreateToken` (no UST1).

| Path | JSON |
|------|------|
| Execute `create_token` (0 SKUs) | `{"create_token":{name,symbol,…,features:[]}}` |
| UST1 hook `create_token` | Newtype `CreateTokenMsg` — `{"create_token":{name,symbol,decimals,initial_balances,manager,treasury,buy_bps,sell_bps,max_*,features,…}}` |
| `enable_feature` | `{"enable_feature":{"token":"terra1…","sku":"transfer_tax"}}` |

Do not send a settings batch to the launcher. AutoLP instantiate+bind in the same create tx is **out of v1** (pay SKU, then bind).

## Option A (v1)

Token-only extra-debit sell + outbound buy. `SetPairHooks` stays governance-only. Pair/router swap math **unchanged**.

## Verify

```bash
make verify-issue-592
make verify-issue-608
make verify-issue-601
make verify-issue-602
CODE_ID=11611 LAYER_B_LT=1 make verify-issue-589
cd smartcontracts && cargo test -p cl8y-community-tax-token -p cl8y-community-token-launcher -p cl8y-community-tax-autolp
```

Do **not** `AddWhitelistedCodeId` until `cw20-codeid-audits/codeids/<id>/REPORT.md` is **GO**. Columbus-5 **11611** is listed (see that REPORT). Do not whitelist ALPHA **8654**. Do not whitelist launcher **11612** / **11614** or AutoLP **11613**.

## Do not

- Credit the pair `amount * (1 - sell_bps)` (inbound FoT).
- Call `SkimToLp` from taxed `Transfer`/`Send`.
- Charge settings fees on `Mint` or `SkimToLp`.
- Enable MintControl after instantiate.
- Treat this template as a license to whitelist other tax tokens.
