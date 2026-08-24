# Agent playbook: community tax CW20 (GitLab #592)

Use when changing the **community tax token**, **launcher**, or **AutoLP** sister, or when listing that template on the factory.

This is the **only** tax-token exception to **H-01**. It is DEX-safe because inbound credits to pair / router / escrow / AutoLP stay **1:1**. Do **not** add pair/router FoT balance-delta math. Factory `AddWhitelistedCodeId` is a **separate ops step** after `#589` REPORT **GO**.

LocalTerra reused QA market: [`AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED.md`](./AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED.md) (**L620-1–L620-8**, [#620](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/620); `make verify-issue-620`). Retail dApp txs on that pair: [`AGENTS_E2E_COMMUNITY_TAX_TX.md`](./AGENTS_E2E_COMMUNITY_TAX_TX.md) (**E622-1–E622-8**, [#622](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/622); `make verify-issue-622`). Sibling surfaces: dApp create/manage [#593](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/593) ([`AGENTS_FRONTEND_CREATE_TOKEN.md`](./AGENTS_FRONTEND_CREATE_TOKEN.md)); identity [#604](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/604); SKU init + percent [#605](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/605); Enable Feature path + SKU dedupe [#606](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/606) ([`AGENTS_COMMUNITY_TAX_ENABLE_FEATURE.md`](./AGENTS_COMMUNITY_TAX_ENABLE_FEATURE.md)); indexer catalog [#594](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/594) ([`AGENTS_INDEXER_COMMUNITY_TOKENS.md`](./AGENTS_INDEXER_COMMUNITY_TOKENS.md)); pay-with-any-token [#595](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/595) (**I595-14** — launcher/token still accept UST1 `Send` only); post-merge Coolify + LocalTerra [#602](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/602) ([`AGENTS_POST_MERGE_OPS_602.md`](./AGENTS_POST_MERGE_OPS_602.md)); Enable Feature post-merge QA [#612](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/612) ([`AGENTS_POST_MERGE_OPS_612.md`](./AGENTS_POST_MERGE_OPS_612.md)); router hop tax [#607](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/607) ([`AGENTS_COMMUNITY_TAX_ROUTER.md`](./AGENTS_COMMUNITY_TAX_ROUTER.md)); LaunchGuards liveness [#608](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/608) ([`AGENTS_COMMUNITY_TAX_LAUNCH_GUARDS.md`](./AGENTS_COMMUNITY_TAX_LAUNCH_GUARDS.md) **H608-1–H608-8**); ExemptionDirectory buy/sell/transfer skip [#609](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/609) ([`AGENTS_COMMUNITY_TAX_EXEMPT.md`](./AGENTS_COMMUNITY_TAX_EXEMPT.md)); AutoLP factory pair + skim floor [#610](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/610) ([`AGENTS_COMMUNITY_TAX_AUTOLP.md`](./AGENTS_COMMUNITY_TAX_AUTOLP.md) **M610-1–M610-8**).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#592**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/592) | On-chain design |
| [`docs/contracts-terraclassic.md` § Community tax CW20](../docs/contracts-terraclassic.md#community-tax-cw20-gitlab-592) | Message shapes + classification |
| [`docs/contracts-security-audit.md`](../docs/contracts-security-audit.md) | Invariants **T592-1–T592-13** + **O601-1–O601-7** |
| [`AGENTS_COMMUNITY_TAX_ROUTER.md`](./AGENTS_COMMUNITY_TAX_ROUTER.md) | **T592-13** / **R607** — router hops tax the original trader (#607 improved option 2) |
| [`docs/runbooks/cw20-whitelist-policy.md`](../docs/runbooks/cw20-whitelist-policy.md) | Narrow whitelist exception |
| [`cw20-codeid-audits/codeids/11611/REPORT.md`](../cw20-codeid-audits/codeids/11611/REPORT.md) | #589 intake (**GO**; columbus-5 listed). Stub: [`community-tax-token/REPORT.md`](../cw20-codeid-audits/codeids/community-tax-token/REPORT.md) |
| [`AGENTS_HOOK_CW20_OPS.md`](./AGENTS_HOOK_CW20_OPS.md) | **H-01** / do not add FoT math |
| [`AGENTS_CW20_CODE_ID_AUDIT.md`](./AGENTS_CW20_CODE_ID_AUDIT.md) | Listing intake |
| [`AGENTS_FRONTEND_PAY_INVOICE.md`](./AGENTS_FRONTEND_PAY_INVOICE.md) | Off-chain any-token pay |
| [`deployments/mainnet-ust1-wrap/REGISTRY.md`](../deployments/mainnet-ust1-wrap/REGISTRY.md) | Columbus-5 listed **11611** + **11619** / launcher `terra126pr5…ahzwze` (**11622**) |

## Columbus-5 pins (#601)

| Role | Value |
|------|--------|
| Token code_id (listed) | **11611** · `data_hash` `9D33BF2539A9A5B2F13FD4B321CDBD0B0FD86D936D5D6BD6681955FA30210EC2` |
| Token rotate (listed) | **11619** · `63CB21D1…BAFA20` — `#589` REPORT **GO**; factory-listed 2026-08-24 |
| Launcher (canonical) | `terra126pr5323xkhwas7y03azv48sqr2fy3fxxg0sxu8xhmjdxr8v5tzqahzwze` (code **11622**, wasm admin DEX 2-of-3 `terra1zlmv2…hep7`) |
| AutoLP code_id (launcher config) | **11621** (not factory-whitelisted) |
| AutoLP superseded store | **11613** · not listed |
| Token wasm admin / CMM | `terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2` (stamped on create; not launcher `ContractInfo.admin`) |
| Factory whitelist | `[6036, 8266, 10184, 11611, 11619]` |

Post-merge rotate (#611 / #612 / [#616](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/616), 2026-08-24): stored **11619** / **11620** / **11621** / **11622**; listed **11619**; launcher **11614 → 11620 → 11622**; `UpdateConfig` set `token_code_id` **11619** / `autolp_code_id` **11621** (tx [`DAC86F27…6ED3`](https://finder.terraclassic.community/columbus-5/tx/DAC86F27B4E95FC83461B733453A9EF1028BC8421F2FE3AE022B0A14DADF6ED3) height **30086058**). Do not whitelist launcher **11612** / **11614** / **11620** / **11622**, AutoLP **11613** / **11621**, or ALPHA **8654**. Keep **11611** listed until Refresh.

`VITE_COMMUNITY_TAX_CODE_ID` / `COMMUNITY_TAX_CODE_ID` = **11619** (catalog is single-id; leave 11611 and new creates stay unattested). `COMMUNITY_TAX_OPTION2_CODE_IDS=11619`. `VITE_COMMUNITY_TOKEN_LAUNCHER` / `COMMUNITY_TOKEN_LAUNCHER` = launcher. Unset → dApp page unavailable / indexer `configured: false`.

## Ops / LocalTerra invariants **O601-1–O601-7** ([#601](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/601))

1. **O601-1 — pinned A-lcd / B-lt.** `CODE_ID=11611 LAYER_B_LT=1 make verify-issue-589` (or `CODE_ID=11619`) executes the LCD wasm (not mintable analogue). A-lcd retries community-tax `InstantiateMsg` when cw20-base init fails, then retries without SKU-gated `launch_guards` / `max_*` headroom (11619+ #605). `balance_at` is A29 N/A.
2. **O601-2 — whitelist after GO only.** Factory lists **11611** and **11619**. Do **not** whitelist launcher **11612** / **11614** / **11620** / **11622**, AutoLP **11613** / **11621**, a LocalTerra store id, or ALPHA **8654**.
3. **O601-3 — free-profile launcher create.** `features == []` uses `ExecuteMsg::CreateToken` (CW20 cannot `Send` 0). Paid SKUs stay on UST1 `Send`. Stamps token `admin: cmm_governance` and `GetLauncherOrigin`. Canonical columbus-5 launcher is `terra126pr5…ahzwze` (code **11622**; store was **11614** / **11620**). **11612** (`terra1af9xm…`) predates this execute and is not migratable (CMM treasury wasm admin); leave unused.
4. **O601-4 — catalog filter.** Rogue `--admin` instantiate has `GetLauncherOrigin.launcher == null`. dApp (#593) / indexer (#594) must require CMM admin + origin.
5. **O601-5 — listed-pair tax.** After `RegisterListedPair`, sell extra-debit + buy outbound split match `TaxPreview` (max-button). Provide `TransferFrom` stays 1:1. Layer B-lt does **not** register the pair (1:1 harness).
6. **O601-6 — invoices on chain.** SKU unlock and settings batch are each exactly 50 UST1. Smoke SKU unlock must Send to the **launcher** (same as the dApp; **T606-7**). MintControl `RevokeMint` is one-way.
7. **O601-7 — DEX residuals stay out.** Hybrid / live zap / AutoLP `SkimToLp` vs a live router are **not** #601 close gates (issue body §5). Router multi-hop tax is **T592-13** (improved option 2, [#607](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/607)) — not an open classify bug.

Smoke: [`scripts/qa/localterra-community-tax-smoke.sh`](../scripts/qa/localterra-community-tax-smoke.sh). Gate: `make verify-issue-601`.

## Invariants **T592-1–T592-13**

1. **T592-1 — inbound 1:1.** Transfers **to** pair, router, this token, AutoLP, or other protocol-exempt addresses credit exactly `amount`. Classic inbound FoT still fails `fee_on_transfer_creates_reserve_imbalance`.
2. **T592-2 — sell extra-debit.** `Send` to a registered listed pair with `Cw20HookMsg::Swap` debits `amount + tax` economically and credits the pair `amount`. Pair-direct: extra-debit `from`. Official-router hop: debit router `amount`, extra-debit authenticated `Swap.trader`.
3. **T592-3 — buy outbound split.** `Transfer`/`Send` **from** a registered listed pair **or** the official router **to a non-protocol-exempt `to`** debits `amount`; trader + sinks = `amount`. Pair→router stays 1:1.
4. **T592-4 — invoices.** SKU unlock and settings batch are each **exactly 50 UST1** (`50000000`). Forwarded to CMM treasury. Wrong token / wrong amount / no-op / unactivated SKU → revert, fee not kept. Do not mix `EnableFeature` into a settings batch. Official post-create SKU unlock is manager → **launcher** → token: `origin.launcher` is authorized for `EnableFeature` only (**T606-1–T606-4**, [#606](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/606)). `UpdateSettings` stays manager-only.
5. **T592-5 — CMM wasm admin.** Launcher `Instantiate { admin: cmm_governance }`. Manager cannot migrate or `UpdateAdmin`. Rogue instantiate with another admin is not catalog-promoted (`GetLauncherOrigin`).
6. **T592-6 — MintControl instantiate-only.** `RevokeMint` is one-way and requires a settings invoice. `Mint` itself is not invoiced.
7. **T592-7 — classification.** Provide (`TransferFrom`) and limit `PlaceLimitOrder*` `Send` are 1:1. Pair→EOA `Transfer` (swap receive / withdraw / limit refund) uses **buy tax** — same CosmWasm primitive; documented, not a pair wasm change. Official router hops tax the original trader (**T592-13**). Manager-directory wallets (including hop `trader`) skip **buy, sell, and transfer** tax (**#609** / **E609-1**); launch guards still use the economic kind (**E609-2** / **T592-11**).
8. **T592-8 — no rug APIs.** No reflection, rebase, pause, or blacklist manager methods.
9. **T592-9 — protocol exemptions.** `RegisterListedPair` requires factory `Pair` lookup. Manager cannot remove protocol entries. The paid manager directory is a **full tax skip** for Transfer / Buy / Sell, not a transfer-only list, and does not grant listed-pair status.
10. **T592-10 — AutoLP skim.** `SkimToLp` is permissionless and is never called from token `Transfer`/`Send` or pair `AfterSwap`. Pair must be factory-listed with this token; skim has a floor (`max_spread` default 100 bps, cap 200 bps). Full **M610-1–M610-8** in [`AGENTS_COMMUNITY_TAX_AUTOLP.md`](./AGENTS_COMMUNITY_TAX_AUTOLP.md) ([#610](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/610)).
11. **T592-11 — launch guards.** `trading_enabled=false` blocks **both** buy and sell (**H-5** residual). Cooldown is **per user wallet** — do not check or record listed pairs / protocol-exempt addresses. `max_wallet` skips protocol / listed pair as `to` (provide + sell-to-pair bypass). User Buy / Transfer still capped. Full **H608-1–H608-8** in [`AGENTS_COMMUNITY_TAX_LAUNCH_GUARDS.md`](./AGENTS_COMMUNITY_TAX_LAUNCH_GUARDS.md) ([#608](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/608)).
12. **T592-12 — UST1-only invoice.** #595 routes any token off-chain. Token/launcher do not accept the tax token as fee.
13. **T592-13 — router hops tax the original trader (#607 improved option 2).** Buy/sell apply to every listed-pair swap, including official multi-hop, hybrid-on-router, permissionless 1-op `execute_swap_operations`, and invoice wrap-routes. Pair-direct `Send+Swap` extra-debits `from`. Official-router `Send+Swap` extra-debits authenticated `Swap.trader` (honored only when `from == config.router`; pair-direct ignores spoofed `trader`; missing trader fail-closes). Pair→router stays 1:1; official router→non-exempt is buy outbound split. Pair/router swap math unchanged (**H-01**). Disclose on Swap/Trade/Create (**C593-14** / [`AGENTS_COMMUNITY_TAX_ROUTER.md`](./AGENTS_COMMUNITY_TAX_ROUTER.md)). Live **11611** needs store + migrate before this holds on-chain.

## Launcher UST1 hook

`community-token-launcher` `Receive` accepts UST1 only. Exact invoice: `50 UST1 × unique SKU count` on create (duplicate names **reject**, **T606-5**); `50 UST1` on later `EnableFeature`. **O601-3:** zero-SKU free profile is `ExecuteMsg::CreateToken` (no UST1).

| Path | JSON |
|------|------|
| Execute `create_token` (0 SKUs) | `{"create_token":{name,symbol,…,features:[]}}` |
| UST1 hook `create_token` | Newtype `CreateTokenMsg` — `{"create_token":{name,symbol,decimals,initial_balances,manager,treasury,buy_bps,sell_bps,max_*,features,…}}` |
| `enable_feature` | `{"enable_feature":{"token":"terra1…","sku":"transfer_tax"}}` |

Do not send a settings batch to the launcher. **#605 H-1:** AutoLP instantiate+bind **is** wired on create (and on Enable Feature) when `autolp_code_id` is set. Unset → `AutolpCodeNotSet`, invoice not kept. Token `BindAutolp` is launcher-only. **#605 M-1:** `variable_rates` is a real gate — instantiate `max_*` cannot exceed current rates without the SKU; settings `buy_bps` / `sell_bps` require it. Columbus-5 **11611** does not gain #604 identity / #605 `initial_exempt` / this gate until `token_code_id` rotates after #589 GO.

## Option A (v1)

Token-only extra-debit sell + outbound buy. `SetPairHooks` stays governance-only. Pair/router swap math **unchanged**.

## Verify

```bash
make verify-issue-592
make verify-issue-604
make verify-issue-605
make verify-issue-606
make verify-issue-607
make verify-issue-608
make verify-issue-609
make verify-issue-610
make verify-issue-601
make verify-issue-602
make verify-issue-612
make verify-issue-616
make verify-issue-620
CODE_ID=11611 LAYER_B_LT=1 make verify-issue-589
cd smartcontracts && cargo test -p cl8y-community-tax-token -p cl8y-community-token-launcher -p cl8y-community-tax-autolp
```

Do **not** `AddWhitelistedCodeId` until `cw20-codeid-audits/codeids/<id>/REPORT.md` is **GO**. Columbus-5 **11611** and **11619** are listed. Do not whitelist ALPHA **8654**. Do not whitelist launcher **11612** / **11614** / **11620** / **11622** or AutoLP **11613** / **11621**.

## Do not

- Credit the pair `amount * (1 - sell_bps)` (inbound FoT).
- Call `SkimToLp` from taxed `Transfer`/`Send`.
- Charge settings fees on `Mint` or `SkimToLp`.
- Enable MintControl after instantiate.
- Treat this template as a license to whitelist other tax tokens.
