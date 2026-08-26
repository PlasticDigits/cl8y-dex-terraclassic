# Testing

**Master verification checklist:** [GitLab **#337**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/337) — executable Local/QA scenarios and **REG-00** / **LR-00** regression gates referenced below.

## Philosophy

CL8Y DEX tests focus on real contract behavior — no blockchain mocks. Unit tests exercise pure logic, integration tests deploy to a simulated chain environment, and E2E tests drive the actual frontend against LocalTerra.

## P2 testing epic (GitLab #199)

Consolidated coverage for production-review P2 gaps ([`TEST_GAP_MATRIX.md`](./reviews/20260409T030009Z/TEST_GAP_MATRIX.md)). Agent playbook: [`skills/AGENTS_TESTING_P2_EPIC.md`](../skills/AGENTS_TESTING_P2_EPIC.md).

| Area | Issue | Primary automated test | Notes |
|------|-------|------------------------|-------|
| Indexer hybrid attrs on `swap_events` | [#82](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/82) | [`indexer/tests/swap_events_hybrid_columns.rs`](../indexer/tests/swap_events_hybrid_columns.rs) | `book_return_amount`, `limit_book_offer_consumed`, `effective_fee_bps` |
| Book-leg fee discount | [#83](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/83) | `limit_order_tests::hybrid_book_fill_uses_taker_discounted_effective_fee_bps` | Same `effective_fee_bps` as pool path |
| Frontend hybrid message shape | [#84](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/84) | [`pair.test.ts`](../frontend-dapp/src/services/terraclassic/__tests__/pair.test.ts), [`router.hybrid.test.ts`](../frontend-dapp/src/services/terraclassic/router.hybrid.test.ts) | Direct pair + router `execute_swap_operations` |
| Pause blocks swap + limits | [#87](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/87) | `pause_blocks_swap_and_place_cancel_refunds_escrow`; [`TradePage.test.tsx`](../frontend-dapp/src/pages/TradePage.test.tsx) | L6 — see [`contracts-security-audit.md`](./contracts-security-audit.md) |
| Post-deploy smoke | [#86](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/86), [#368](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/368) | [`make smoke-pool-swap`](../Makefile); wired in [`make start-qa`](../scripts/qa/start-qa.sh) (`QA_SKIP_SMOKE=1` to skip) | LCD `pool` + optional `hybrid_simulation`; pair from `.qa-deploy-stamp` |
| Wrap-mapper pause on-chain | [#396](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/396) | [`make smoke-wrap-mapper-pause`](../Makefile); [`make verify-issue-396`](../Makefile) | Treasury `wrap_deposit` + LUNC-C `send` unwrap; governance `set_paused` on wrap-mapper (SEC-B06) |
| IBC-hooks deploy gate | [#407](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/407) | [`make verify-issue-407`](../Makefile); [`make verify-no-ibc-hooks-in-contracts`](../Makefile) | SEC-D02 — chain version + contract IBC entry-point record in launch runbook |
| Pre-deploy test evidence | [#444](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/444) | [`make verify-issue-444`](../Makefile) | SEC-H08 — paste `test-contracts`, `test-indexer-integration`, `test-frontend`, and pool smoke output on release issue (CI pipeline link satisfies CI-built deploys) |
| Stubs / mocks catalog | [#105](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/105) | Policy below + issue #105 | LCD stub vs AMM-sim orderbook |
| Charts integration (indexer HTTP) | [#104](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/104), [#230](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/230) | [`ChartsPage.integration.test.tsx`](../frontend-dapp/src/pages/ChartsPage.integration.test.tsx) | Reference job `frontend-charts-integration` → `make test-charts-integration`; **stubbed** `lightweight-charts` — not canvas |
| Price chart real `lightweight-charts` (Vitest) | [#211](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/211), [#229](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/229), [#230](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/230) | `*.charts.test.{ts,tsx}` via `npm run test:charts` | Reference job `frontend-charts-vitest` → `make test-frontend-charts`; large-candle + real visible-range autoscale ([#229](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/229)) |
| Price chart candle parsing + stale pair race | [#226](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/226) | [`priceChartCandles.test.ts`](../frontend-dapp/src/components/charts/__tests__/priceChartCandles.test.ts), [`PriceChart.test.tsx`](../frontend-dapp/src/components/charts/__tests__/PriceChart.test.tsx) | Default `npm run test:run`; no Postgres |
| Parked refund `reason` discriminator | [#504](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/504) | [`make verify-issue-504`](../Makefile); skill [`AGENTS_EXPIRED_LIMIT_PARK_REASON.md`](../skills/AGENTS_EXPIRED_LIMIT_PARK_REASON.md) | Invariant **L22** — wire `expired` / `dust_filled` / `force_cleaned` / `blacklisted`; optional `VERIFY504_LCD=1` after deploy |
| LP ticker digits + factory upgrade | [#518](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/518) | [`make verify-issue-518`](../Makefile); [`scripts/upgrade-518-lp-symbol.sh`](../scripts/upgrade-518-lp-symbol.sh); skill [`AGENTS_LP_SYMBOL_DIGITS.md`](../skills/AGENTS_LP_SYMBOL_DIGITS.md) | Invariant **F3** — keep `0-9`, strip non-alnum; classic LP reverts until `lp_token_code_id` upgrade |
| Limit placement discount shift | [#514](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/514) | [`make verify-issue-514`](../Makefile); [`scripts/upgrade-514-limit-discount.sh`](../scripts/upgrade-514-limit-discount.sh); skill [`AGENTS_FEE_DISCOUNT_TIERS.md`](../skills/AGENTS_FEE_DISCOUNT_TIERS.md) | Invariant **I13** — placement uses `limit_discount_bps`; swap/take stays on `discount_bps`; tier 9 place = 0 |
| Factory discount-registry snapshot | [#536](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/536) | [`make verify-issue-536`](../Makefile); skill [`AGENTS_FACTORY_DISCOUNT_REGISTRY.md`](../skills/AGENTS_FACTORY_DISCOUNT_REGISTRY.md) | Invariant **F5** / **I14** — `CreatePair` copies factory pointer; All/Batch persist it; `GetDiscountRegistry` is on pair `QueryMsg` |
| Listed CW20 `code_id` pin (F6) | [#582](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/582) | [`make verify-issue-582`](../Makefile); skill [`AGENTS_CW20_CODE_ID_PIN.md`](../skills/AGENTS_CW20_CODE_ID_PIN.md) | Pair pin + write-path whitelist re-check; `asset_code_id_pin_tests::*` |
| CW20 code-id audit harness | [#589](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/589), [#581](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/581) | [`make verify-issue-589`](../Makefile); [`make verify-issue-581`](../Makefile); skill [`AGENTS_CW20_CODE_ID_AUDIT.md`](../skills/AGENTS_CW20_CODE_ID_AUDIT.md); [`cw20-codeid-audits/PROCEDURE.md`](../cw20-codeid-audits/PROCEDURE.md) | LCD pin + decomp + catalogue + Layer A/B; `LAYER_B_LT=1` runs full Transfer/TransferFrom/Send/provide/swap/limit on pinned wasm; 8266 GO/NO-GO is `codeids/8266/REPORT.md`; issuer/Everybody/minter are residuals not veto; optimizer rebuild appendix only |
| F6 upgrade script + freeze runbook | [#584](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/584) | [`make verify-issue-584`](../Makefile); [`scripts/upgrade-582-code-id-pin.sh`](../scripts/upgrade-582-code-id-pin.sh); [`docs/runbooks/cw20-code-id-ops.md`](./runbooks/cw20-code-id-ops.md) | Factory 1.9.0 **before** pair 1.15.0; `UpdateConfig { pair_code_id }`; `limit: 30` + `GetPairCount`; LCD ContractInfo probe + whitelist retries; refresh parses `has_more`. Columbus-5 **RAN 2026-08-21**. 8266 REPORT **GO**; factory listed **8266** 2026-08-22 (height 30060600). ALPHA **8654** stays off. |
| F6 freeze on dApp + `route/solve` | [#585](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/585) | [`make verify-issue-585`](../Makefile); skill [`AGENTS_FRONTEND_CODE_ID_FREEZE.md`](../skills/AGENTS_FRONTEND_CODE_ID_FREEZE.md) | Indexer excludes frozen hops; pair `code_id_frozen`; humanized execute errors; banners on Swap/Trade/Pool/Charts. Does **not** un-gate exits or add FoT math. Postgres for indexer integration; no LocalTerra. |
| Retail hybrid always-on (no opt-out) | [#596](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/596) | [`make verify-issue-596`](../Makefile); skill [`AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md`](../skills/AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md) | Swap + Trade Market always GET `/route/solve`; no hybrid checkbox. Optional `VERIFY_ISSUE_596_CHAIN=1` Playwright. |
| Pay with any token (invoice) | [#595](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/595) | [`make verify-issue-595`](../Makefile); skill [`AGENTS_FRONTEND_PAY_INVOICE.md`](../skills/AGENTS_FRONTEND_PAY_INVOICE.md) | Shared `quotePayInvoice` + `PayWithAnyToken`; exact invoice Send; wrap+2hop+invoice gas. `#593` / `#597` must import the card. |
| Community tax CW20 (DEX-safe) | [#592](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/592) | [`make verify-issue-592`](../Makefile); skill [`AGENTS_COMMUNITY_TAX_CW20.md`](../skills/AGENTS_COMMUNITY_TAX_CW20.md) | Invariants **T592-1–T592-13** — inbound 1:1, extra-debit sell, outbound buy, router hops tax the original trader, 50 UST1 SKU + batch; factory whitelist is `#589` GO / [#601](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/601) |
| LaunchGuards cooldown / max_wallet liveness | [#608](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/608) | [`make verify-issue-608`](../Makefile); skill [`AGENTS_COMMUNITY_TAX_LAUNCH_GUARDS.md`](../skills/AGENTS_COMMUNITY_TAX_LAUNCH_GUARDS.md) | **H608-1–H608-8** — per-wallet cooldown (not pair-wide); `max_wallet` skips protocol/`to` so provide stays live; sell bypass **T592-11**; H-5 pause residual unchanged |
| Launcher Enable Feature + SKU dedupe | [#606](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/606) | [`make verify-issue-606`](../Makefile); skill [`AGENTS_COMMUNITY_TAX_ENABLE_FEATURE.md`](../skills/AGENTS_COMMUNITY_TAX_ENABLE_FEATURE.md) | **T606-1–T606-8** — official manager→launcher→token unlock; unique SKUs; smoke uses the launcher hook. Columbus-5 launcher is **11622**; new tokens instantiate listed **11619** |
| ExemptionDirectory full tax skip | [#609](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/609) | [`make verify-issue-609`](../Makefile); skill [`AGENTS_COMMUNITY_TAX_EXEMPT.md`](../skills/AGENTS_COMMUNITY_TAX_EXEMPT.md) | Invariants **E609-1–E609-7** — manager directory skips buy/sell/transfer tax; launch guards stay on; extra-debit Max fail-closed |
| Autoregister + manager role skip | [#633](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/633) | [`make verify-issue-633`](../Makefile); [`localterra-633-autoregister.sh`](../scripts/qa/localterra-633-autoregister.sh); skill [`AGENTS_COMMUNITY_TAX_AUTOREGISTER.md`](../skills/AGENTS_COMMUNITY_TAX_AUTOREGISTER.md) | **R633-1–R633-8** — `config.manager` tax skip; factory/dApp/AutoLP register; Manage highest-LP catch-up; no Terraport/GDEX. LocalTerra live when chain+tax pins are up (`VERIFY633_REQUIRE_CHAIN=1`). Columbus-5 migrates: [#635](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/635) / [`scripts/upgrade-635-autoregister.sh`](../scripts/upgrade-635-autoregister.sh) |
| AutoLP factory pair + skim floor | [#610](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/610) | [`make verify-issue-610`](../Makefile); skill [`AGENTS_COMMUNITY_TAX_AUTOLP.md`](../skills/AGENTS_COMMUNITY_TAX_AUTOLP.md) | **M610-1–M610-8** — pair must be factory-listed with this tax token; skim `max_spread` 100 bps (cap 200); inverted fake-pair PoC |
| Create Token + manager console | [#593](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/593) | [`make verify-issue-593`](../Makefile); skill [`AGENTS_FRONTEND_CREATE_TOKEN.md`](../skills/AGENTS_FRONTEND_CREATE_TOKEN.md) | Invariants **C593-1–C593-14** — env gate, PayWithAnyToken, 50 UST1 invoices, manager read-only, extra-debit Max, listed-pair tax copy, combined instantiate caps |
| Migrate Token adopt + LP gate | [#626](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/626) | [`make verify-issue-626`](../Makefile); skill [`AGENTS_FRONTEND_TOKEN_MIGRATE.md`](../skills/AGENTS_FRONTEND_TOKEN_MIGRATE.md) | **M626-1–M626-12** — free `/token/migrate`; migrate allowlist env (default includes 8654); never factory-list 8654; `GetMigrateOrigin` attest; Terraport LP table |
| Columbus-5 CW20 code 3 investigation | [#627](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/627) | [`make verify-issue-627`](../Makefile); skill [`AGENTS_CW20_CODE_ID_3.md`](../skills/AGENTS_CW20_CODE_ID_3.md); [`codeids/3/REPORT.md`](../cw20-codeid-audits/codeids/3/REPORT.md) | **C627-1–C627-8** — adopt **NO-GO** (CanonicalAddr / `AdoptLegacyLayout`); factory list **NO-GO** (`interface_version_7`, B13 ≥34 900); do not append env or `AddWhitelistedCodeId 3` |
| Migrate pair inventory | [#634](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/634) | [`make verify-issue-634`](../Makefile); [`localterra-634-migrate-inventory.sh`](../scripts/qa/localterra-634-migrate-inventory.sh); skill [`AGENTS_FRONTEND_TOKEN_MIGRATE.md`](../skills/AGENTS_FRONTEND_TOKEN_MIGRATE.md) | **M634-1–M634-8** — CL8Y vs Terraport/GDEX venue list; register only factory-verified CL8Y after Refresh; no Refresh execute. LocalTerra live when chain+tax pins are up (`VERIFY634_REQUIRE_CHAIN=1`). Columbus-5 Open/ALPHA: [#636](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/636) |
| Create Token identity + wallet helpers | [#604](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/604) | [`make verify-issue-604`](../Makefile); skill [`AGENTS_FRONTEND_CREATE_TOKEN.md`](../skills/AGENTS_FRONTEND_CREATE_TOKEN.md) | **C604-1–C604-3** — decimals 6–18, alphanumeric name/symbol, connected-wallet helpers; 11611 gap until launcher rotate |
| Create Token SKU init + percent taxes | [#605](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/605) | [`make verify-issue-605`](../Makefile); skill [`AGENTS_FRONTEND_CREATE_TOKEN.md`](../skills/AGENTS_FRONTEND_CREATE_TOKEN.md) | **C605-1–C605-4** — percent 2 dp, SKU init payloads, AutoLP instantiate+bind or refuse, VariableRates gate |
| Community tax router hop tax | [#607](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/607) | [`make verify-issue-607`](../Makefile); skill [`AGENTS_COMMUNITY_TAX_ROUTER.md`](../skills/AGENTS_COMMUNITY_TAX_ROUTER.md) | **T592-13** / **C593-14** / **R607-1–R607-8** — C-2 improved option 2; official-router hops extra-debit authenticated `trader`; pair/router swap math unchanged |
| Tax-aware route/solve ranking | [#615](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/615) | [`make verify-issue-615`](../Makefile); skill [`AGENTS_INDEXER_TAX_AWARE_ROUTING.md`](../skills/AGENTS_INDEXER_TAX_AWARE_ROUTING.md) | **R615-1–R615-8** — net rank; middle-hop skip on option-2 only; 11611 Honest until env/hash flip; You Receive net; cache isolates tax identity |
| Community tax indexer catalog | [#594](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/594) | [`make verify-issue-594`](../Makefile); skill [`AGENTS_INDEXER_COMMUNITY_TOKENS.md`](../skills/AGENTS_INDEXER_COMMUNITY_TOKENS.md) | Invariants **I594-1–I594-10** — configured envelope, attested default list, launcher-only create, no request-path LCD |
| Community tax store + LocalTerra smoke | [#601](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/601) | [`make verify-issue-601`](../Makefile); [`localterra-community-tax-smoke.sh`](../scripts/qa/localterra-community-tax-smoke.sh); skill [`AGENTS_COMMUNITY_TAX_CW20.md`](../skills/AGENTS_COMMUNITY_TAX_CW20.md) | **O601-1–O601-7** — 11611 + 11619 REPORT GO + A-lcd/B-lt; factory `[6036, 8266, 10184, 11611, 11619]`; free-profile create; extra-debit/outbound; 50 UST1 invoices. Needs LocalTerra (`VERIFY601_REQUIRE_CHAIN=1` default) |
| Named tax-on Layer B (keep B-lt tax-off) | [#623](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/623) | [`make verify-issue-623`](../Makefile); [`layer-b-tax-on.sh`](../cw20-codeid-audits/scripts/layer-b-tax-on.sh); skill [`AGENTS_CW20_CODE_ID_TAX_ON.md`](../skills/AGENTS_CW20_CODE_ID_TAX_ON.md) | **C623-1–C623-8** — B-lt stays 1:1 / no register; named suite after `RegisterListedPair` (sell extra-debit, buy split, router `trader`, limit 1:1, AutoLP floor). `LAYER_B_TAX_ON=1` without chain **FAIL**s. Invoices stay `#601`. Never whitelist 8654 / a LocalTerra id from tax-on evidence |
| LocalTerra community-tax seed | [#620](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/620) | [`make verify-issue-620`](../Makefile); [`deploy-community-tax-local.sh`](../scripts/lib/deploy-community-tax-local.sh); skill [`AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED.md`](../skills/AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED.md) | **L620-1–L620-8** — default `make deploy-local` lists a tax/EMBER pair + AutoLP; Transfer funding (no Mint on the QA token); indexer catalog env; `DEPLOY_SKIP_COMMUNITY_TAX=1` is gems-only. `#601` smoke stays ephemeral |
| Tax-aware localnet swarm | [#621](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/621) | [`make verify-issue-621`](../Makefile); skill [`AGENTS_LOCALNET_SWARM_TAX.md`](../skills/AGENTS_LOCALNET_SWARM_TAX.md) | **S621-1–S621-8** — gem workers exclude the tax token; `tax_listed` / `--worker tax` size sells with TaxPreview; pair-direct does not set `trader`; hybrid skip `tax_hybrid_skip`; `SWARM_TAX_WORKERS=0` is exclude-only |
| Community-tax Playwright e2e-tx | [#622](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/622) | [`make verify-issue-622`](../Makefile); [`community-tax-tx.spec.ts`](../frontend-dapp/e2e/community-tax-tx.spec.ts); skill [`AGENTS_E2E_COMMUNITY_TAX_TX.md`](../skills/AGENTS_E2E_COMMUNITY_TAX_TX.md) | **E622-1–E622-8** — sell extra-debit + buy You Receive net + provide/limit 1:1 on the seed tax/EMBER pair; fail-closed if pins missing; smoke columbus-5 bake stays for `/token/create` only. Optional `VERIFY_ISSUE_622_CHAIN=1` Playwright |
| Wrap+≥2hop LUNC↔USTR gas + Swap Network fee | [#587](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/587) | [`make verify-issue-587`](../Makefile); skill [`AGENTS_TERRACLASSIC_GAS.md`](../skills/AGENTS_TERRACLASSIC_GAS.md) | Combo overhead 400k on wrap+router N≥2; `SendHookGasDecodeError`; Swap `swap-network-fee` LUNC-only. E2E `wrap-swap.spec.ts` E7/E8 needs LocalTerra. |
| Unwrap+≥2hop USTR→USTC gas | [#599](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/599) | [`make verify-issue-599`](../Makefile); skill [`AGENTS_TERRACLASSIC_GAS.md`](../skills/AGENTS_TERRACLASSIC_GAS.md) | `UNWRAP_ROUTER_COMBO_OVERHEAD_GAS` 400k on router `unwrap_output` N≥2; USTR→USTC **3,110,000**; direct unwrap stays 800k. E2E `wrap-swap.spec.ts` E9 needs LocalTerra. Post-merge chain QA: [#600](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/600). |
| Post-migrate inherit + dApp query | [#538](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/538) | [`make verify-issue-538`](../Makefile); [`scripts/qa/localterra-create-pair-inherit.sh`](../scripts/qa/localterra-create-pair-inherit.sh) | **F538-1–F538-3** — LocalTerra `create_pair` inherit without follow-up Set; dApp `GetDiscountRegistry` smart-query-first. Needs LocalTerra for the live inherit rung |
| Pair-scoped fee-tier chrome | [#537](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/537) | [`make verify-issue-537`](../Makefile); skill [`AGENTS_FRONTEND_PAIR_FEE_DISCOUNT.md`](../skills/AGENTS_FRONTEND_PAIR_FEE_DISCOUNT.md) | Invariant **I14** — strikethrough / maker place discount only when pair `DISCOUNT_REGISTRY` matches `VITE_FEE_DISCOUNT_ADDRESS`; no LocalTerra |
| Compact token identity (Pool / Trade / Charts) | [#541](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/541) | [`make verify-issue-541`](../Makefile); skill [`AGENTS_FRONTEND_TOKEN_IDENTITY.md`](../skills/AGENTS_FRONTEND_TOKEN_IDENTITY.md) | Invariants **T541-1–T541-8** — copy + explorer for both legs + pair; native copy-only; invert-stable payloads; Playwright smoke (5 workers, no e2e-tx) |
| Native LUNC / USTC picker labels | [#630](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/630) | [`make verify-issue-630`](../Makefile); skill [`AGENTS_FRONTEND_NATIVE_TICKERS.md`](../skills/AGENTS_FRONTEND_NATIVE_TICKERS.md) | Invariants **N630-1–N630-8** — visible **LUNC** / **USTC** (never `uluna` / `uusd`); registry beats indexer spoof; ids + copy stay denoms; indexer native upsert + repair |
| `/tiers` phone-width layout | [#651](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/651) | [`make verify-issue-651`](../Makefile); skill [`AGENTS_FRONTEND_TIERS_PHONE.md`](../skills/AGENTS_FRONTEND_TIERS_PHONE.md) | Invariants **T651-1–T651-8** / **I15** — intact **Hold {n} CL8Y** + fee phrases; no empty Register column; How it works stacks on `<md`. Playwright `e2e/fee-tiers.spec.ts` (5 workers) when LocalTerra is up; no `e2e-tx` |
| Keplr in-app / visualViewport picker | [#632](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/632) | [`make verify-issue-632`](../Makefile); skill [`AGENTS_FRONTEND_PORTAL_LISTBOX_VIEWPORT.md`](../skills/AGENTS_FRONTEND_PORTAL_LISTBOX_VIEWPORT.md) | **V632-1–V632-8** — `visualViewport` + tab/in-app/finger insets; coarse/narrow browse-without-IME; #498 CLS + new clearance E2E; Playwright 5 workers, no e2e-tx |
| DEX hub USD (cUSTC / UST1 / USTR) | [#556](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/556) | [`make verify-issue-556`](../Makefile); skill [`AGENTS_INDEXER_HUB_USD.md`](../skills/AGENTS_INDEXER_HUB_USD.md) | Invariants **H1–H10** — largest-liquidity DEX marks replace `$1` / `2.5×` ingest pegs; Protocol DEX card; CEX catalog stays 3 tickers |
| Protocol UST1 window mint/redeem fees | [#614](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/614) | [`make verify-issue-614`](../Makefile); skill [`AGENTS_INDEXER_UST1_WINDOW_FEES.md`](../skills/AGENTS_INDEXER_UST1_WINDOW_FEES.md) | **I614-1–I614-8** / **PFee-13** — indexer `UST1_WINDOW_ADDRESS`; `deposit`→`ust1_mint` / `withdraw`→`ust1_redeem`; 11618 `fee_amount`+`fee_asset`; 11566 bps-only fail closed; never `× fee_bps`; hub UST1 USD. Optional `VERIFY614_REQUIRE_LIVE=1` |
| Protocol DEX hub wrap + LUNC column | [#570](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/570) | [`make verify-issue-570`](../Makefile); skill [`AGENTS_FRONTEND_PROTOCOL_HUB.md`](../skills/AGENTS_FRONTEND_PROTOCOL_HUB.md) | Invariants **H11–H16** — cUSTC/cLUNC wrap `AddressRow`; LUNC/USD CEX wrap 1:1 on hub card; CEX tabs unchanged |
| Candle USD clock + idle marks | [#568](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/568) | [`make verify-issue-568`](../Makefile); skill [`AGENTS_INDEXER_CANDLE_USD_MARK.md`](../skills/AGENTS_INDEXER_CANDLE_USD_MARK.md) | Invariants **C568-1–C568-8** — no as-of-now hub rewrite; idle mark-to-market bars (`trade_count=0`); USTC/LUNC as-of repair |
| Human tape Amount in/out/Price | [#557](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/557) | [`make verify-issue-557`](../Makefile); skill [`AGENTS_FRONTEND_TAPE_AMOUNTS.md`](../skills/AGENTS_FRONTEND_TAPE_AMOUNTS.md) | Invariants **T557-1–T557-11** — human amounts + symbols; Price stays quote-per-base; invert reciprocates Price only; JSON/CSV raw amounts unchanged |
| Portfolio / trader P&L USD from hub | [#560](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/560) | [`make verify-issue-560`](../Makefile); skill [`AGENTS_FRONTEND_HUB_PNL.md`](../skills/AGENTS_FRONTEND_HUB_PNL.md) | Invariants **P560-1–P560-6** — header realized P&L uses `GET /api/v1/hub-prices`; Best/Worst is **—**; missing asset rows kept |
| Trade desktop layout (no drag-resize) | [#561](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/561) | [`make verify-issue-561`](../Makefile); skill [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](../skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md) | Invariants **L561-1–L561-12** — CSS grid book \| chart \| ticket + independent tape; hide sides; no `PanelResizeHandle`; Playwright P10–P13 |
| Hide soft-launch gems on production | [#562](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/562) | [`make verify-issue-562`](../Makefile); skill [`AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md`](../skills/AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md) | Invariants **P562-1–P562-8** — LocalTerra P1 still lists EMBER (`e2e/retail-test-tokens-562.spec.ts`) |
| DeFiLlama UTC-day API + adapter copies | [#631](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/631) | [`make verify-issue-631`](../Makefile); skill [`AGENTS_DEFILLAMA.md`](../skills/AGENTS_DEFILLAMA.md); [`docs/DEFILLAMA.md`](./DEFILLAMA.md) | **L631-1–L631-9** — daily GET rollup (gem/hybrid/fee exclusions); TVL is on-chain `Pool {}` only; upstream Llama PRs are operator follow-up |
| Trade ticket heading + Buy/Sell colors | [#563](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/563) | [`make verify-issue-563`](../Makefile); skill [`AGENTS_FRONTEND_TRADE_TICKET_HEADING.md`](../skills/AGENTS_FRONTEND_TRADE_TICKET_HEADING.md) | Invariants **T563-1–T563-8** — full **Buy {base}**, no compact wallet chip |
| Charts pair 24h Stats + TWAP human scale | [#564](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/564) / [#565](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/565) | [`make verify-issue-564`](../Makefile) · [`make verify-issue-565`](../Makefile); skill [`AGENTS_FRONTEND_CHARTS_PAIR_STATS.md`](../skills/AGENTS_FRONTEND_CHARTS_PAIR_STATS.md) | **S564-1–S564-11** + **P565-1–P565-7** — Vol (USD) primary; no `formatNum(raw)` |
| Station + Cosmostation WalletConnect | [#566](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/566) | [`make verify-issue-566`](../Makefile); skill [`AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md`](../skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md) | **WC-M10** — mobile Open/Copy; Leap stays out |
| Keplr + Ledger Nano signing | [#567](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/567) | [`make verify-issue-567`](../Makefile); skill [`AGENTS_FRONTEND_KEPLR_LEDGER.md`](../skills/AGENTS_FRONTEND_KEPLR_LEDGER.md) | **K567-1–K567-8** — amino on Ledger; sign-stall ≠ broadcast timeout |
| Keplr CW20 recognition pack | [#629](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/629) | [`make verify-issue-629`](../Makefile); skill [`AGENTS_KEPLR_CW20_REGISTRY.md`](../skills/AGENTS_KEPLR_CW20_REGISTRY.md) | **K629-1–K629-8** — `cosmos/columbus` JSON + logos; USTR already live; no invented price fields. No LocalTerra |
| Listing venue catalog | [#639](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/639) | [`make verify-issue-639`](../Makefile); skill [`AGENTS_LISTINGS.md`](../skills/AGENTS_LISTINGS.md); [`docs/listings/README.md`](./listings/README.md) | **L639-1–L639-8** — catalog + go/no-go; Keplr pin lockstep; form drafts pin `/cg` `/cmc`; skip Coinhall/DexScreener. No new indexer API. No LocalTerra |
| Cosmostation CW20 pack | [#640](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/640) | [`make verify-issue-640`](../Makefile); skill [`AGENTS_COSMOSTATION.md`](../skills/AGENTS_COSMOSTATION.md) | **C640-1–C640-8** — append `chain/terra/cw20_2.json` + `asset/` PNGs; Keplr pin lockstep; no gems. Upstream **archived** — pack-only. No LocalTerra |
| Hexxagon CW20 pack | [#641](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/641) | [`make verify-issue-641`](../Makefile); skill [`AGENTS_HEXXAGON.md`](../skills/AGENTS_HEXXAGON.md) | **H641-1–H641-8** — append `terra.js`; USTR already live; no gems. Live PR [hexxagon-io/chain-registry#68](https://github.com/hexxagon-io/chain-registry/pull/68). No LocalTerra |
| Post-merge Coolify + indexer stack | [#573](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/573) | [`make verify-issue-573`](../Makefile); skill [`AGENTS_POST_MERGE_STACK.md`](../skills/AGENTS_POST_MERGE_STACK.md) | **M573-1–M573-8** / **Q6** — children 557–567 + Coolify env + LocalTerra EMBER P1 |
| Post-merge fees / wrap gas / 8266 A-lcd | [#590](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/590) | [`make verify-issue-590`](../Makefile); skill [`AGENTS_POST_MERGE_OPS_590.md`](../skills/AGENTS_POST_MERGE_OPS_590.md) | **M590-1–M590-8** / **Q7** — children 586, 587, 589; L7 once; unwrap not burn tax; `LAYER_B_LT=1` executes LCD wasm; 8266 REPORT GO + columbus-5 listed; do not whitelist a LocalTerra store id; ALPHA **8654** stays off |
| Post-merge !400 unwrap E9 + columbus-5 | [#600](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/600) | [`make verify-issue-600`](../Makefile); skill [`AGENTS_POST_MERGE_OPS_600.md`](../skills/AGENTS_POST_MERGE_OPS_600.md) | **M600-1–M600-8** / **Q8** — children 599, 587; LocalTerra E9 `gas_used < gas_wanted`; wrap combo still one-tx; direct unwrap 800k; columbus-5 hash via `VERIFY600_COLUMBUS_TX`; do not reopen #599 unless 3.11M OOGs |
| Post-merge !402 Coolify + Create Token QA | [#602](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/602) | [`make verify-issue-602`](../Makefile); skill [`AGENTS_POST_MERGE_OPS_602.md`](../skills/AGENTS_POST_MERGE_OPS_602.md) | **M602-1–M602-8** / **Q9** — children 593, 594; Coolify 11611 + 11614 launcher (not unused 11612); LocalTerra smoke + `/create` copy-address only; do not reopen #593 / #594 unless **C593** / **I594** is wrong |
| Post-merge !407/!408 Enable Feature QA | [#612](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/612) | [`make verify-issue-612`](../Makefile); skill [`AGENTS_POST_MERGE_OPS_612.md`](../skills/AGENTS_POST_MERGE_OPS_612.md) | **M612-1–M612-8** / **Q10** — children 606, 607; columbus-5 launcher **11622** + `token_code_id` **11619**; LocalTerra `sku_unlock_via_launcher` + paid create + second SKU; do not reopen #606 / #607 unless **T606** / **T592-13** is wrong; option-1 disclose moved to [#616](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/616) |
| Post-merge !409–!413 option-2 / wrap / window / AutoLP / ranking | [#616](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/616) | [`make verify-issue-616`](../Makefile); skill [`AGENTS_POST_MERGE_OPS_616.md`](../skills/AGENTS_POST_MERGE_OPS_616.md) | **M616-1–M616-8** / **Q11** — children 607, 610, 613, 614, 615; columbus-5 11622 + 11619/11621; option-2 copy; do not reopen #607 / #610 / #613 / #614 / #615; window mint/redeem ingest is [`AGENTS_INDEXER_UST1_WINDOW_FEES.md`](../skills/AGENTS_INDEXER_UST1_WINDOW_FEES.md) |
| Post-merge !414 LocalTerra community-tax seed leftovers | [#624](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/624) | [`make verify-issue-624`](../Makefile); skill [`AGENTS_POST_MERGE_OPS_624.md`](../skills/AGENTS_POST_MERGE_OPS_624.md) | **M624-1–M624-8** / **Q12** — child 620 + leftover fresh volume / indexer catalog / Transfer / stamp skip; children 601, 592, 610, 594; swarm `--dry-run` is not `fundBotWallets`; do not reopen #620 / #592 / #601 / #610 / #594 |
| Post-merge !415–!417 tax swarm / e2e-tx / Layer B leftovers | [#625](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/625) | [`make verify-issue-625`](../Makefile); skill [`AGENTS_POST_MERGE_OPS_625.md`](../skills/AGENTS_POST_MERGE_OPS_625.md) | **M625-1–M625-8** / **Q13** — children 621, 622, 623 + leftover tax-on seed buy (non-treasury) + `LAYER_B_TAX_ON_FORCE_EPHEMERAL=1` instantiate path, Playwright P0 extra-debit / provide (indexer CORS includes `:3173`), swarm soak, OE-1 `pool_only`; seed treasury ≠ test1; do not reopen #621 / #622 / #623 / #620 |
| Post-merge !418 community-tax migrate leftovers | [#628](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/628) | [`make verify-issue-628`](../Makefile); [`localterra-628-migrate-leftover.sh`](../scripts/qa/localterra-628-migrate-leftover.sh); skill [`AGENTS_POST_MERGE_OPS_628.md`](../skills/AGENTS_POST_MERGE_OPS_628.md) | **M628-1–M628-8** / **Q14** — children 626, 592, 593, 594; adopt pin ≠ 11619 (11626 store / 11630 live); never factory-list 8654 or code 3; Coolify/indexer single-id; live 6036 cw2 `crates.io:terraswap-token` (page-go / chain-revert); LocalTerra P3/P7/P11; Create Token stays code-id-free; do not reopen #626 / implement #627 |

**Post-deploy smoke (#86 / #368):**

```bash
make smoke-pool-swap
# or manually after deploy:
# source scripts/lib/smoke-deploy-env.sh && ./scripts/smoke-pool-swap.sh
```

`make start-qa` runs smoke after `qa-verify-deploy` (skip with `QA_SKIP_SMOKE=1`). `scripts/lib/smoke-deploy-env.sh` resolves `PAIR_ADDR` from `.qa-deploy-stamp` and `OFFER_TOKEN` from the pair `pool` query — no hardcoded testnet addresses.

See also [`docs/deployment-guide.md`](./deployment-guide.md) and [`docs/runbooks/launch-checklist.md`](./runbooks/launch-checklist.md) (Phase 5 go/no-go gate — `make verify-issue-391`).

## Test Types

### Indexer (Rust)

- **Unit tests (`cargo test --lib`):** parser stress tests, candle OHLC merge invariants, position clamping, oracle `f64` conversion, CG ticker shape validation — **no database required**.
- **Integration tests (`cargo test --tests`):** require PostgreSQL (set `TEST_DATABASE_URL` or use the default URL with valid credentials). They assert API allowlists, caps, CORS, rate limiting (429), sanitized 500 responses, and sanitized **502** LCD bodies ([#239](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/239); [`indexer/tests/security.rs`](../indexer/tests/security.rs), [`skills/AGENTS_INDEXER_API_LCD_SECURITY.md`](../skills/AGENTS_INDEXER_API_LCD_SECURITY.md)).

```bash
cd indexer
cargo test --lib          # fast, no Postgres
cargo test --tests        # needs Postgres + migrations
```

Run those on the **host** (after `make setup-indexer-postgres` when needed). Do **not** `docker run -v indexer:/work rust:… cargo test` to reach compose Postgres — that leaves **root-owned** `indexer/target` lock files (`target/debug/.cargo-build-lock`) and breaks later host cargo / rust-analyzer. See [AGENTS.md § Rust / Docker gotchas](../AGENTS.md) and [`scripts/lib/docker-indexer-bind-mount.sh`](../scripts/lib/docker-indexer-bind-mount.sh).

#### Local Postgres setup (agents)

**Agent playbook:** [`skills/AGENTS_LOCAL_POSTGRES_DEV.md`](../skills/AGENTS_LOCAL_POSTGRES_DEV.md) — default user `cl8y_legal`, superuser bootstrap via `setup-postgres-dev-databases.sh`, `make reset` when an old Docker volume still has `postgres:postgres`, and what `deploy-dex-local` writes to `indexer/.env`.

**Stack prerequisite (invariant PG-1):** The indexer and integration tests authenticate as **`cl8y_legal`**. Compose creates that role on a fresh volume (`POSTGRES_USER=cl8y_legal`). External or legacy Postgres that only ships **`postgres:postgres`** must either run [`scripts/setup-postgres-dev-databases.sh`](../scripts/setup-postgres-dev-databases.sh) (auto-creates `cl8y_legal` when `POSTGRES_SUPERUSER` is reachable) or provision the role manually — see [GitLab **#245**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/245) QA note and the skill § Stack prerequisite.

#### Shared Postgres and test parallelism

Integration tests call [`tests/common/mod.rs`](../indexer/tests/common/mod.rs) helpers that **truncate and re-seed** the same database. With default Cargo/Rust test parallelism, multiple integration test **binaries** and multiple **tests per binary** can run concurrently against that DB, which can surface as duplicate unique keys (e.g. on `assets.denom`) or foreign-key violations—not application bugs.

`seed_db` / `clean_db` take an exclusive **file lock** (`/tmp/cl8y-dex-indexer-test.seed.lock`, override with `TEST_DB_LOCK_FILE`) so parallel `cargo test` processes on one host do not interleave truncate/insert (GitLab **#210** orderbook verification). Prefer serialized execution anyway:

When using a **single** shared test database (typical local or QA host), prefer serialized execution:

```bash
cd indexer
export TEST_DATABASE_URL="${TEST_DATABASE_URL:-postgres://cl8y_legal:cl8y_legal@127.0.0.1:5432/dex_indexer_test}"
cargo test --tests -j 1 -- --test-threads=1
```

- **`-j 1`** — run one integration test crate at a time (reduces cross-crate contention).
- **`--test-threads=1`** — run tests inside each binary one at a time (reduces intra-crate contention).

Start Postgres (`docker compose up -d postgres`) and run `./scripts/setup-postgres-dev-databases.sh` (or `make deploy-local`) so `dex_indexer_test` exists before the first run.

**Cursor Cloud Agent (no wasm deploy):** `make setup-indexer-postgres` then `make test-indexer-integration` — see [`AGENTS.md`](../AGENTS.md) § Indexer integration tests (Postgres-only) and [GitLab **#335**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/335).

See [Indexer invariants](./indexer-invariants.md) for the full matrix and the same note under **Running tests**.

**Stubs, mocks, and test stand-ins:** intentional test doubles are cataloged in [GitLab issue #105](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/105) and summarized under [P2 testing epic (#199)](./testing.md#p2-testing-epic-gitlab-199). Key indexer spots: `indexer/tests/common/lcd_mock.rs` (LCD HTTP stub only) vs `indexer/src/api/orderbook_sim.rs` (**production** AMM curve-walk for CG/CMC — not the on-chain FIFO book; see **#210**). Agent playbook: [`skills/AGENTS_TESTING_P2_EPIC.md`](../skills/AGENTS_TESTING_P2_EPIC.md).

### Unit Tests (Rust)

Test individual contract functions in isolation using `cosmwasm_std::testing` helpers.

```bash
cd smartcontracts
cargo test
```

### Unit Tests (Frontend)

Test React components and hooks with Vitest and jsdom. **CosmWasm / LCD I/O** is typically **stubbed at the service layer** so unit tests stay fast and deterministic. That does **not** replace integration coverage for features that depend on indexer HTTP or chart data: use the **integration** Vitest config (below) or dedicated issues (e.g. GitLab [**#104**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/104) for charts).

```bash
make test-frontend        # single run (nvm via scripts/with-node.sh)
# or:
bash scripts/with-node.sh --cwd frontend-dapp -- npm run test
bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run
```

Config: `vitest.config.ts`

**Cosmes fork patch verification ([GitLab #367](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/367)):** `make test-frontend` runs [`cosmesPatch127.test.ts`](../frontend-dapp/src/services/terraclassic/__tests__/cosmesPatch127.test.ts), which (1) SHA-256-hashes `patches/@goblinhunt+cosmes+*.patch` against committed [`patches/.cosmes-patch-sha256`](../frontend-dapp/patches/.cosmes-patch-sha256) and (2) asserts patched fee-guard symbols exist in `node_modules/@goblinhunt/cosmes/dist/...` after `postinstall` / `patch-package`. Requires a normal `npm ci` (not `--ignore-scripts`). Operator docs: [`docs/frontend.md` § Forked cosmes](./frontend.md#cosmes-fork-patches).

**Regression:** Trade/Charts **price chart** empty-candle UX and `getPairStats` fallback are covered in `src/components/charts/__tests__/PriceChart.test.tsx` (see GitLab [**#113**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/113) and [Trade page — price chart invariants](./frontend.md#trade-page--price-chart-invariants)).

#### Real `lightweight-charts` in Vitest (GitLab #211)

TradingView **[lightweight-charts](https://github.com/tradingview/lightweight-charts)** (open-source canvas library — **not** the hosted TradingView widget) has two Vitest layers:

| Layer | Config / command | What runs |
|-------|------------------|-----------|
| **Fast stub** (default) | `vitest.config.ts` → `npm run test:run` | `lightweightChartsJsdomMock.ts` — React/indexer wiring, `createChart` / `applyOptions` / `addSeries` option capture, `setData` payloads; canvas **contract** + lifecycle in `PriceChartLightweightCanvas.test.tsx` ([#227](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/227), [#225](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/225); stub fidelity epic [#105](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/105)) |
| **Real library** | `vitest.config.charts.ts` → `npm run test:charts` | Imports actual `lightweight-charts`; Node `canvas` shim in `src/test/chartsSetup.ts`; files matching `*.charts.test.{ts,tsx}` (includes post-layout sizing [#225](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/225)) |

```bash
make test-frontend-charts   # from repo root
# or:
bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:charts
```

**Automation:** reference job **`frontend-charts-vitest`** → `make test-frontend-charts` (`npm run test:charts` with Node `canvas` OS deps: `libcairo`, etc.) — isolated from the fast `frontend` unit target so native binding failures do not block 600+ jsdom tests ([#230](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/230), [#211](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/211)). Treat as **required** in release checklists (not optional/skip silently).

**Local `canvas` deps (Ubuntu/Debian):** if `npm run test:charts` fails loading the native module, install the same packages as the reference workflow spec:

```bash
sudo apt-get install -y build-essential libcairo2-dev libgif-dev libjpeg-dev libpango1.0-dev librsvg2-dev
```

**Large-candle ceiling ([#229](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/229)):** Real-library tests always cover **500** and **1500** candles (per-test timeouts up to 25s). A **2000**-candle soak runs **only when `CI` is set** (`it.runIf(process.env.CI)`). Do not add 50k-row cases to CI — local-only if ever needed. Default suite timeout remains **15s** in `vitest.config.charts.ts`; autoscale regressions use the chart’s real `getVisibleLogicalRange()` after `setVisibleLogicalRange`, not a synthetic `original()` alone. Harness: [`chartRealLibraryHarness.ts`](../frontend-dapp/src/test/chartRealLibraryHarness.ts).

**Agent playbook:** [`skills/AGENTS_FRONTEND_PRICE_CHART.md`](../skills/AGENTS_FRONTEND_PRICE_CHART.md). Chart invariants: [Trade page — price chart invariants](./frontend.md#trade-page--price-chart-invariants).

**Do not** load `lightweightChartsJsdomMock.ts` in the charts config. Pure helpers (`priceChartCandles`, `priceChartIndicators`, `priceChartPriceScale`) stay in default unit tests only.

### Integration Tests (Frontend)

Longer-running tests are kept out of the default `npm run test:run` suite. **Charts + indexer HTTP** coverage uses `vitest.config.integration.ts`: tests call a real indexer (`VITE_INDEXER_URL`, default `http://127.0.0.1:3001`) with PostgreSQL migrations applied. They are **not** skipped when the stack is down — the run fails so automation catches broken wiring. E2E and other flows may still use LocalTerra where documented.

**Charts test layers (GitLab #230):**

| Layer | Command / reference job | Validates | Does **not** validate |
|-------|-------------------------|-----------|------------------------|
| Unit (jsdom stub) | `npm run test:run` / `frontend` | React wiring, stub contract ([#227](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/227)) | Real canvas / library |
| Real library Vitest | `npm run test:charts` / `frontend-charts-vitest` | `lightweight-charts` init, `setData`, autoscale, large candles ([#211](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/211), [#229](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/229)) | Indexer HTTP |
| Indexer HTTP integration | `npm run test:integration` / `frontend-charts-integration` | Live candles API → ChartsPage shell ([#104](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/104)) | Canvas render (stubbed) |
| Playwright | `npm run test:e2e` / `e2e` | Full browser + LocalTerra tx path | Chart pixel perf at 50k candles |

**Charts integration (local)**

**Primary path:** from repo root with Postgres and the indexer API on `:3001` running (host Postgres or QA stack):

```bash
make test-charts-integration
```

This runs [`scripts/test-charts-integration.sh`](../scripts/test-charts-integration.sh): ensures the target database exists, applies `sqlx migrate run`, seeds fixtures idempotently, verifies indexer `/health`, then `npm run test:integration` via `scripts/with-node.sh`. If host `sqlx migrate` times out, the script retries on the compose Postgres network by mounting **`indexer/migrations` only** (not `indexer/`), so Docker cannot create root-owned `indexer/target` lock files. Limit-order pool ref tests ([GitLab **#166**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/166)) also need LocalTerra LCD reachable (defaults `http://localhost:1317`; override `VITE_TERRA_LCD_URL` / `VITE_TERRA_RPC_URL` — same as [`frontend-dapp/.env.example`](../frontend-dapp/.env.example)).

**Fixture invariants**

| Invariant | Where |
|-----------|--------|
| Charts pair address `terra1paircontractabc` | [`chartsIntegrationConstants.ts`](../frontend-dapp/src/test/chartsIntegrationConstants.ts) ↔ [`seed-charts-integration.sql`](../indexer/scripts/seed-charts-integration.sql) |
| Fixture candle `open_time` inside indexer default API window (90-day lookback when `from`/`to` omitted) | Seed SQL refreshes to current UTC hour on each run; see [`docs/indexer-invariants.md`](indexer-invariants.md) |
| Limit-order pool-ref pair (EMBER/CORAL) | Resolved from factory via LCD when LocalTerra is up; see [`limitOrderIntegrationConstants.ts`](../frontend-dapp/src/test/limitOrderIntegrationConstants.ts) and GitLab **#166** |

Override the database with `CHARTS_INT_DATABASE_URL` (defaults to `DATABASE_URL` / `dex_indexer` from [`scripts/lib/postgres-dev.env`](../scripts/lib/postgres-dev.env)); override indexer URL with `VITE_INDEXER_URL` (default `http://127.0.0.1:3001`). **Charts** tests (5) need Postgres + indexer only. **Limit-order** tests (2) run when the script resolves the factory pair into `VITE_LIMIT_ORDER_INTEGRATION_*`; otherwise Vitest skips them — full **7/7** needs LocalTerra LCD (`http://127.0.0.1:1317`) after `make deploy-local`. See GitLab [**#205**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/205) and [`skills/AGENTS_TESTING_P2_EPIC.md`](../skills/AGENTS_TESTING_P2_EPIC.md).

**Manual steps** (when debugging individual phases):

1. Start PostgreSQL (for example `docker compose up -d postgres` from the repo root).
2. Create a database (once): `CREATE DATABASE cl8y_charts_int;` (name can match your `DATABASE_URL`).
3. Run migrations and seed minimal pair + candles:

   ```bash
   export DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/cl8y_charts_int
   cd indexer && sqlx migrate run && psql "$DATABASE_URL" -f scripts/seed-charts-integration.sql
   ```

4. Start the indexer API (same `DATABASE_URL` plus required env from `indexer/.env.example`: at minimum `FACTORY_ADDRESS`, **`CORS_ORIGINS`** (for browser integration tests / local Vite, include both `http://localhost:5173` and `http://127.0.0.1:5173` — [GitLab **#131**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/131)), `LCD_URLS`).

5. Run Vitest integration:

   ```bash
   cd frontend-dapp
   VITE_INDEXER_URL=http://127.0.0.1:3001 npm run test:integration
   ```

**Manual rollback SQL** (not run by `sqlx migrate`): paired `.down.sql` for selected migrations lives under [`indexer/migrations/revert/`](../indexer/migrations/revert/) — e.g. limit-order lifecycle columns ([GitLab **#142**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/142)) ship beside [`20260509160000_limit_order_placement_lifecycle.sql`](../indexer/migrations/20260509160000_limit_order_placement_lifecycle.sql). CosmWasm contract migration rollback is separate — [wasm admin migration runbook § Rollback and limitations](./runbooks/wasm-admin-migration.md#rollback-and-limitations-sec-h05) (**SEC-H05**, [#443](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/443)).

**Note:** Default Vitest stubs `lightweight-charts` under jsdom via `src/test/lightweightChartsJsdomMock.ts` (including `LineSeries` for MA/RSI lines). The stub records `createChart` / `applyOptions` / `addSeries` (pane index, `autoscaleInfoProvider`) for fast **contract** tests ([#227](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/227)); helpers: `lwChartTestDouble.getLastCreateChartOptions()`, `getLastApplyOptions()`, `getCandlestickAutoscaleProvider()`, `addSeriesCalls`. **Real-library** chart init, `setData`, indicators, volume fallback, USD autoscale (including visible-range zoom via real `getVisibleLogicalRange()` — [#229](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/229)), and large-candle perf guards run in `npm run test:charts` ([#211](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/211)). Pure clamp math stays in `priceChartPriceScale.test.ts` ([#151](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/151)). Pixel-level zoom/pan and layout regressions remain Playwright / manual QA.

Config: `vitest.config.integration.ts`

### E2E Tests (Playwright)

Full browser tests against the running dApp + LocalTerra. **Strict on-chain policy** ([GitLab **#201**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/201), [**#103**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/103)): default strict automation/local tx specs **fail** when LCD, funds, routes, or pairs are missing — no silent `test.skip` for those gaps.

```bash
make test-e2e-tx              # one command: LocalTerra + deploy + strict tx project
# Or from repo root with nvm (scripts/with-node.sh — see .nvmrc):
bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:e2e
bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:e2e:tx
bash scripts/with-node.sh --cwd frontend-dapp -- env PLAYWRIGHT_SKIP_CHAIN=1 npm run test:e2e:smoke
bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:e2e:ui
```

After `nvm use` in a shell, you may `cd frontend-dapp` and run the same `npm run test:e2e*` scripts directly.

Config: `playwright.config.ts` (`e2e-smoke`, `e2e-tx`, `e2e-indexer-outage` projects). Agent playbook: [`skills/AGENTS_E2E_STRICT_CHAIN.md`](../skills/AGENTS_E2E_STRICT_CHAIN.md).

#### Price chart Playwright smoke (GitLab #228) {#price-chart-playwright-smoke-gitlab-228}

Browser regression for **lightweight-charts** canvas presence and **fullscreen** aria toggles — complements Vitest ([#211](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/211)) and layout-only trade specs ([#146](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/146)).

| Layer | Spec / helper | What it proves |
|-------|----------------|----------------|
| **Strict automation / local** | `e2e/price-chart-smoke.spec.ts` (`e2e-smoke` project) | `/charts` and `/trade` mount `price-chart-lightweight-canvas` + child `canvas`; interval `1h`→`1d` keeps canvas; read-only chart without wallet |
| **Fullscreen (no indexer)** | Same file, mocked Fullscreen API via `e2e/helpers/price-chart.ts` | `aria-label` **Expand…** / **Exit…**, `aria-pressed`, denied enter does not remove button |
| **UI-only skip** | `PLAYWRIGHT_SKIP_CHAIN=1` | Entire spec **skipped** (trade workspace + indexer required for toolbar and canvas) |
| **Outage regression** | `*-indexer-outage.spec.ts` (separate job) | Unchanged — `trade-chart-unavailable` when indexer stopped ([#165](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/165), [#219](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/219)); swap banner ([#241](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/241)) |

```bash
# After LocalTerra + deploy-dex-local.sh + indexer (see scripts/e2e-start-indexer.sh):
bash scripts/e2e-start-indexer.sh
bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/price-chart-smoke.spec.ts --project=e2e-smoke
```

Reference job **`e2e`** (local: Postgres + `deploy-dex-local.sh` + `bash scripts/e2e-start-indexer.sh` + `make test-e2e`) starts the stack, then runs full Playwright ([#228](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/228)).

Chart invariants: [Trade page — price chart invariants](./frontend.md#trade-page--price-chart-invariants). Agent: [`skills/AGENTS_FRONTEND_PRICE_CHART.md`](../skills/AGENTS_FRONTEND_PRICE_CHART.md).

**Manual QA crosswalk** ([`QA_TEMPLATE.md`](../QA_TEMPLATE.md) §5.1 — GitLab #228 verification):

| QA row | Automated? | Where |
|--------|------------|-------|
| 5.1.1 Chart loads (`/charts`) | Yes | `e2e/price-chart-smoke.spec.ts` — `price-chart-lightweight-canvas` + child `canvas` |
| 5.1.5 / 5.1.7 Interval (1h / 1d) | Partial | Same spec — `/trade` interval click 1h→1d keeps canvas |
| 5.1.10 Loading state | Partial | Mobile viewport test — canvas **or** “Loading chart…” |
| 5.1.11 Error / outage | Yes (regression) | `e2e/*-indexer-outage.spec.ts` (separate job; not `price-chart-smoke`) |
| 5.1.12 Zoom / scroll | **Manual** | [#211](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/211) — no pixel assertions in Playwright |
| Fullscreen aria | Yes | `price-chart-smoke.spec.ts` + `e2e/helpers/price-chart.ts` (mocked Fullscreen API) |

#### Frontend E2E — indexer outage {#frontend-e2e-indexer-outage}

Market-data-down Playwright specs live in project **`e2e-indexer-outage`** (`**/*-indexer-outage.spec.ts`). They require the indexer HTTP API to be **stopped** while LocalTerra/Vite remain up, with **`E2E_INDEXER_OUTAGE=1`**. Default `npm run test:e2e` and the strict **`e2e`** reference job **exclude** this project — avoids flaking the strict chain suite ([#201](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/201)).

| Layer | Target |
|-------|--------|
| **Automation** | `make test-e2e-indexer-outage` / [`scripts/test-e2e-indexer-outage.sh`](../scripts/test-e2e-indexer-outage.sh) (workflow job name **`frontend-e2e-indexer-outage`** in [`.github/workflows/test.yml`](../.github/workflows/test.yml) is a portable spec only — this repo does not run GitHub Actions) |
| **Local one-command** | `make test-e2e-indexer-outage` → [`scripts/test-e2e-indexer-outage.sh`](../scripts/test-e2e-indexer-outage.sh) |
| **Manual** | Stop indexer on `:3001`, set pair from deploy, then run specs |

```bash
# Preferred local path (starts indexer, verifies /api/v1/overview, stops, runs specs):
make test-e2e-indexer-outage

# Manual (after deploy + indexer was running on :3001):
cd frontend-dapp
export E2E_TRADE_PAIR="$(bash ../scripts/lib/e2e-trade-pair-from-deploy.sh)"
E2E_INDEXER_OUTAGE=1 npm run test:e2e:indexer-outage
```

**Env vars:** `E2E_INDEXER_OUTAGE=1` (required for specs to run); `VITE_E2E_INDEXER_OUTAGE=1` (set automatically — fast indexer transport failure in the browser); `E2E_TRADE_PAIR` (optional — defaults to first deploy pair via `.qa-deploy-stamp` / factory LCD); `VITE_INDEXER_URL` (default `http://127.0.0.1:3001`).

After sanity on `:3001`, Playwright runs with **`OUTAGE_E2E_INDEXER_URL`** (default `http://127.0.0.1:39991`, nothing listening) so a shared host cannot auto-restart the QA indexer on `:3001` and produce a false green. Local and reference-job paths use the same script: [`scripts/test-e2e-indexer-outage.sh`](../scripts/test-e2e-indexer-outage.sh).

**Local QA stack:** If an indexer is already listening on `:3001` (e.g. `make qa-start`), `test-e2e-indexer-outage.sh` reuses it for the sanity check, then stops **every** process bound to that port before Playwright. Restart afterward with `bash scripts/e2e-start-indexer.sh` or `make qa-start` (indexer only) if other work needs the API.

Vitest covers Charts/Trader/Pool/**Limits**/ **Swap** outage banners with mocked transport errors (`npm run test:run`; `/limits`: [`LimitOrdersPage.test.tsx`](../frontend-dapp/src/pages/LimitOrdersPage.test.tsx), GitLab **#218**; `/`: [`SwapPage.test.tsx`](../frontend-dapp/src/pages/SwapPage.test.tsx), GitLab **#241**). Product invariants: [docs/frontend.md § Market data loading & outage](./frontend.md#market-data-loading-outage); agent: [`skills/AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md`](../skills/AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md). Tracking: [GitLab **#219**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/219).

### Swap wrap safety CTA (SEC-A02, GitLab #389) {#swap-wrap-safety-cta-sec-a02-gitlab-389}

Launch checklist **SEC-A02** requires retail copy **and** a disabled submit CTA for wrap-mapper pause and on-chain wrap rate limit on `/`.

| Check | Command |
|-------|---------|
| Vitest — pause copy + disabled | `bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- src/pages/SwapPage.test.tsx -t "SEC-A02"` |
| Vitest — rate limit copy + disabled | same (both cases in one `describe`) |
| Vitest — rate limit inline alert (SEC-I05 F-04 / #463) | `bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- src/pages/SwapPage.test.tsx -t "rate-limit alert"` |
| Playwright — isolated LCD mocks | `bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/wrap-swap.spec.ts -g "SEC-A02|wrap mapper paused" --project=e2e-tx` |
| Playwright — rate limit inline banner (#463) | same (`E12` asserts `swap-wrap-rate-limit-banner` + retry copy) |

### Pair pause disabled CTAs (SEC-B05, GitLab #395) {#pair-pause-disabled-ctas-sec-b05-gitlab-395}

Launch checklist **SEC-B05** requires LCD `is_paused` gating on `/` and `/pool` to match on-chain **L6** policy (swap + LP provide/withdraw disabled while paused).

| Check | Command |
|-------|---------|
| Vitest — swap + pool pause CTAs | `bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- src/pages/SwapPage.test.tsx src/pages/PoolPage.test.tsx -t "SEC-B05"` |
| Vitest — trade/limit path (regression) | `bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- src/pages/TradePage.test.tsx -t "pair is paused"` |

Product invariants: [docs/frontend.md § Pair pause disabled CTAs](./frontend.md#pair-pause-disabled-ctas-sec-b05); agent: [`skills/AGENTS_FRONTEND_SWAP_SAFETY_CTA.md`](../skills/AGENTS_FRONTEND_SWAP_SAFETY_CTA.md).

### Trading blacklist disabled CTAs (SEC-E01, GitLab #425) {#trading-blacklist-disabled-ctas-sec-e01-gitlab-425}

Launch checklist **SEC-E01** requires factory trading blacklist to disable write CTAs on `/pool` (provide + withdraw) and `/limits` (place + cancel), with Vitest coverage for wallet, token, and pair dimensions (Swap and `/trade` limit ticket already covered by [#388](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/388)).

| Check | Command |
|-------|---------|
| Vitest — pool + limits blacklist CTAs | `bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- src/pages/PoolPage.test.tsx src/pages/LimitOrdersPage.test.tsx -t "SEC-E01"` |
| Vitest — swap + trade blacklist (regression) | `bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- src/pages/SwapPage.test.tsx src/pages/TradePage.test.tsx -t "trading blacklist"` |

Product invariants: [docs/frontend.md § Trading blacklist disabled CTAs](./frontend.md#trading-blacklist-disabled-ctas-sec-e01); security model: [§ Trading blacklist](./security-model.md#trading-blacklist-compliance--incident-response). Shared mocks: [`tradingBlacklistMocks.ts`](../frontend-dapp/src/test/tradingBlacklistMocks.ts).

### Indexer HTTP 429 calm retry copy (SEC-E04, GitLab #426) {#indexer-http-429-calm-retry-copy-sec-e04-gitlab-426}

Launch checklist **SEC-E04** requires a frontend test that mocks **HTTP 429** from the indexer API (or LCD quote path) and asserts **calm retry guidance** with no raw HTTP status, URL, or stack trace in the displayed message. Distinct from on-chain wrap-mapper rate limits ([SEC-A02](#swap-wrap-safety-cta-sec-a02-gitlab-389)).

| Check | Command |
|-------|---------|
| Vitest — `isIndexerRateLimitError` classification | `bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- src/utils/__tests__/indexerErrors.test.ts -t "429"` |
| Vitest — calm retry copy mapping | `bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- src/utils/__tests__/humanizeUserFacingError.test.ts -t "429"` |
| Vitest — swap page mocks indexer 429 | `bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- src/pages/SwapPage.test.tsx -t "SEC-E04"` |

Copy source: [`INDEXER_RATE_LIMIT_RETRY_MESSAGE`](../frontend-dapp/src/utils/marketDataServiceCopy.ts); classifier: [`indexerErrors.ts`](../frontend-dapp/src/utils/indexerErrors.ts). User FAQ: [docs/user-incident-faq.md § Rate limits](./user-incident-faq.md#rate-limits). Agent: [`skills/AGENTS_USER_INCIDENT_FAQ.md`](../skills/AGENTS_USER_INCIDENT_FAQ.md), [`skills/AGENTS_INDEXER_API_LCD_SECURITY.md`](../skills/AGENTS_INDEXER_API_LCD_SECURITY.md).

Mocks: [`wrap-mapper-lcd-mock.ts`](../frontend-dapp/e2e/helpers/wrap-mapper-lcd-mock.ts). Agent playbook: [`skills/AGENTS_FRONTEND_SWAP_SAFETY_CTA.md`](../skills/AGENTS_FRONTEND_SWAP_SAFETY_CTA.md). Product copy: [docs/frontend.md § Swap wrap safety CTA](./frontend.md#swap-wrap-safety-cta-sec-a02).

### Wrap-mapper pause on-chain smoke (SEC-B06, GitLab #396) {#wrap-mapper-pause-smoke-sec-b06-gitlab-396}

Launch checklist **SEC-B06** requires **contract-level** proof that wrap-mapper `SetPaused` blocks wrap and unwrap, and that unpausing restores both — not only the frontend LCD-mocked CTA ([SEC-A02](#swap-wrap-safety-cta-sec-a02-gitlab-389) above).

The wrap-mapper wasm is deployed on LocalTerra from [`ustr-cmm`](https://gitlab.com/PlasticDigits/ustr-cmm) during `make deploy-local` (full seed). It is **not** an untestable external-only dependency on localnet.

| Check | Command |
|-------|---------|
| On-chain pause cycle (wrap reject → unwrap reject → unpause → wrap OK → unwrap OK) | `make smoke-wrap-mapper-pause` (after `make deploy-local`) |
| Full #396 acceptance (smoke + SEC-A02 Vitest) | `make verify-issue-396` |

Script: [`scripts/smoke-wrap-mapper-pause.sh`](../scripts/smoke-wrap-mapper-pause.sh); env resolver: [`scripts/lib/smoke-wrap-env.sh`](../scripts/lib/smoke-wrap-env.sh). Manual QA template: [wrap-unwrap-test-pass.md § Paused State](./qa-templates/wrap-unwrap-test-pass.md). Audit invariant **W1**: [contracts-security-audit.md](./contracts-security-audit.md).

**Header / tablet compact nav:** `e2e/navigation.spec.ts` asserts no horizontal overlap for the **Swap + More** row at 773×743, **1024–1098px** (follow-up cram band), and other tablet widths; the full primary row at 1280px; desktop **Swap → Pool → Trade** tab transitions without reload at 1440px ([GitLab **#182**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/182)); and that **“Terra Classic ecosystem”** does not appear in the shell (header brand is logo + title only). Invariants: [docs/frontend.md § Responsive shell & header navigation](./frontend.md#responsive-header-navigation) ([GitLab **#136**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/136)); shell nav playbook [`skills/AGENTS_FRONTEND_SHELL_NAV.md`](../skills/AGENTS_FRONTEND_SHELL_NAV.md).

**Connected wallet chip — network label:** same spec file — desktop **`Local`** short label on the trigger at 1280px, mobile LUNC without visible network text, connected chip vs **More** non-overlap at 773px ([GitLab **#186**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/186); [docs/frontend.md § Connected wallet chip — network & mobile](./frontend.md#connected-wallet-chip-network-mobile)).

**Local stack for strict on-chain tests (default `e2e` automation path):**

LocalTerra must be **terrad v4 / SDK 0.53** with a fresh volume after digest bumps — see [`docs/localterra-sdk53.md`](./localterra-sdk53.md) ([GitLab **#292**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/292)).

1. `docker compose up -d localterra` (or `make reset && make start && make wait-healthy` after image bump)
2. From repo root: `bash scripts/deploy-dex-local.sh` (writes `frontend-dapp/.env.local`, deploys contracts, seeds CW20 balances on the dev account `terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v`).
3. `make test-e2e` or `bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:e2e`

**Cloud Agent VM:** global setup runs deploy/e2e shell scripts that need `docker exec` when host `:1317` hangs — wrap as `sg docker -c 'CI=1 make test-e2e'` (see [`AGENTS.md`](../AGENTS.md) and [`docs/localterra-sdk53.md`](./localterra-sdk53.md) **LT11** / **LT12**). Install Playwright Chromium via `./node_modules/.bin/playwright install chromium` (not bare `npx playwright`). Tx project stays at **1 worker**; smoke uses **5 workers**.

Before tests, **`e2e/global-setup.ts`** waits for the LCD and runs **`scripts/e2e-provision-dev-wallet.sh`**, which **mints factory CW20s** to the dev wallet when any listed token balance is below **`E2E_DEV_MIN_CW20_U128`** (default `10000000000000` raw units), then **`scripts/e2e-seed-hybrid-book.sh`**, which idempotently places a **resting bid** on the first dual-CW20 pair when the bid book head is empty (GitLab [**#193**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/193)). **Invariant:** pair `OrderBookHead` returns a bare **`u64`** on LCD (`{"data":13}`), not `{ head_order_id }`; the seed script must treat an existing head as success so global setup can re-run ([GitLab **#138**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/138)). Native gas denoms **uluna** / **uusd** are expected from genesis ([`docker/init-chain.sh`](../docker/init-chain.sh) — **11M LUNC** on SDK 0.53 LocalTerra; GitLab [**#372**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/372)), not from the provision script.

**Single-file pool tx run (documented in `frontend-dapp/e2e/README.md`):**

```bash
bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/pool-tx.spec.ts --project=e2e-tx
```

**Hybrid swap E2E (strict tx path, GitLab [#193](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/193)):** `bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/hybrid-swap.spec.ts --project=e2e-tx` — requires global-setup seeding. See [`frontend-dapp/e2e/README.md`](../frontend-dapp/e2e/README.md) and [`skills/AGENTS_E2E_HYBRID_SWAP.md`](../skills/AGENTS_E2E_HYBRID_SWAP.md).

**Multihop hybrid + page smoke E2E (GitLab [#422](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/422)):**

| Spec | Project | Acceptance |
|------|---------|------------|
| `e2e/multihop-hybrid-tx.spec.ts` | `e2e-tx` | Router multihop CORAL→IRON (≥2 hops; hybrid ask-book on hop 0); tx `limit_order_fill`; return within slippage; screenshot attachment |
| `e2e/trader-page.spec.ts` | `e2e-smoke` | `/trader/{wallet}` positions section loads (rows or empty); no console errors |
| `e2e/protocol-page.spec.ts` | `e2e-smoke` | `/protocol` factory + router `AddressRow` audit copy ([#378](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/378)) |
| `e2e/blacklist-swap.spec.ts` | `e2e-smoke` | Factory `blacklist_check` LCD mock blocks Swap CTA ([#388](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/388)) |

```bash
bash scripts/e2e-start-indexer.sh
bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/multihop-hybrid-tx.spec.ts e2e/trader-page.spec.ts e2e/protocol-page.spec.ts e2e/blacklist-swap.spec.ts
# Full suite (smoke @ 5 workers, then tx @ 1):
sg docker -c 'CI=1 make test-e2e'
```

Playbook: [`skills/AGENTS_TESTING_MULTIHOP_HYBRID.md`](../skills/AGENTS_TESTING_MULTIHOP_HYBRID.md). Blacklist mock helper: [`e2e/helpers/blacklist-lcd-mock.ts`](../frontend-dapp/e2e/helpers/blacklist-lcd-mock.ts).

**Swap route display vs on-chain ops (SEC-E07, GitLab [#428](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/428)):** `bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/swap-route-alignment-tx.spec.ts --project=e2e-tx` — direct dual-CW20 (1 wasm hop) and multihop CORAL→IRON (≥2 hops); UI `swap-route-summary` symbols must match tx `offer_asset`/`ask_asset` sequence. Playbook: [`skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md`](../skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md).

**Limit order tx E2E (strict place + cancel, GitLab [#195](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/195)):** `bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/limit-orders-tx.spec.ts --project=e2e-tx` — first **unpaused** dual-CW20 pair via LCD `is_paused`. See [`frontend-dapp/e2e/README.md`](../frontend-dapp/e2e/README.md) and [`skills/AGENTS_E2E_LIMIT_ORDERS_TX.md`](../skills/AGENTS_E2E_LIMIT_ORDERS_TX.md).

**Community-tax pair tx E2E (GitLab [#622](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/622)):** `bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/community-tax-tx.spec.ts --project=e2e-tx` — LocalTerra **QATax/EMBER** seed pair (`VITE_PAIR_COMMUNITY_TAX_EMBER`). Sell extra-debit + buy You Receive net + provide/limit 1:1. Missing tax pins **fail** (no `test.skip`). Gem specs stay on EMBER/CORAL. Playbook: [`skills/AGENTS_E2E_COMMUNITY_TAX_TX.md`](../skills/AGENTS_E2E_COMMUNITY_TAX_TX.md).

**Claim all parked tx E2E (GitLab [#259](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/259)):** `bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/limit-orders-claim-all-tx.spec.ts --project=e2e-tx` — expiry-park harness + batch claim confirm gas copy. Requires indexer + [`scripts/e2e-seed-expired-parked-claim-all.sh`](../scripts/e2e-seed-expired-parked-claim-all.sh). See [`skills/AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md`](../skills/AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md).

**Optional chain (skip instead of fail):** set `PLAYWRIGHT_SKIP_CHAIN=1` (or legacy `REQUIRE_LOCALTERRA=0`) for local UI-only runs (`npm run test:e2e:smoke`). **Do not** set this in release automation checklists. Default is strict (unset).

**GitLab [#138](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/138) verification (risk surfacing + E2E blockers):** after LocalTerra + `deploy-dex-local.sh` + indexer on `VITE_INDEXER_URL` (see [`docs/frontend.md` § Risk surfacing](./frontend.md#legal-risk-surfacing)):

```bash
cd frontend-dapp && npm ci && npm run test:unit
bash scripts/e2e-seed-hybrid-book.sh && bash scripts/e2e-seed-hybrid-book.sh   # second run must skip
bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/navigation.spec.ts -g "NFA footer"
bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/navigation.spec.ts -g "navigates to Pool"
```

Fixes: **`bd763be`** (`cosmesPatch127.test.ts`, `e2e-seed-hybrid-book.sh` bare `u64` head), **`f58cce5`** (`Outlet key={pathname}` shell tab nav). Agent playbooks: [`skills/AGENTS_FRONTEND_RISK_DISCLAIMERS.md`](../skills/AGENTS_FRONTEND_RISK_DISCLAIMERS.md), [`skills/AGENTS_FRONTEND_SHELL_NAV.md`](../skills/AGENTS_FRONTEND_SHELL_NAV.md).

### Trading swarm for UI load (localnet)

The [`@cl8y-dex/localnet-trading-swarm`](../packages/localnet-trading-swarm) package drives **five** concurrent bot wallets against LocalTerra to stress the dApp (swap, hybrid, router multi-hop, limit orders, LP flows) and optional indexer-backed views. It is **not** run in CI by default; manual QA runs it after `deploy-dex-local.sh`.

```bash
# From repo root (requires LocalTerra + frontend-dapp/.env.local)
./scripts/localnet-trading-swarm.sh
# Optional: validate wiring without txs
./scripts/localnet-trading-swarm.sh -- --dry-run
# Optional: JSON stats on SIGINT (mean inter-tx gap per bot ~20s target)
./scripts/localnet-trading-swarm.sh -- --stats
```

Contract message shapes align with [`docs/contracts-terraclassic.md`](./contracts-terraclassic.md), [`docs/limit-orders.md`](./limit-orders.md), and frontend Terra services. Full invariants: [`packages/localnet-trading-swarm/README.md`](../packages/localnet-trading-swarm/README.md); agent playbook: [`skills/AGENTS_LOCALNET_TRADING_SWARM.md`](../skills/AGENTS_LOCALNET_TRADING_SWARM.md). Issue: [GitLab #119](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/119).

**Python QA swarm (`make swarm-launch`):** separate from the TypeScript package — **25** swap workers + **5** limit makers + **3** `provide_liquidity` workers + **1** tax-aware worker unless `SWARM_TAX_WORKERS=0` ([#621](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/621)). Gem swap/LP workers exclude the listed tax token. Tax-aware playbook: [`skills/AGENTS_LOCALNET_SWARM_TAX.md`](../skills/AGENTS_LOCALNET_SWARM_TAX.md). All Python bots broadcast **`--from test1`** (same mnemonic as the Simulated Wallet / Playwright dev wallet — GitLab [#372](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/372)). Genesis and deploy seeds were raised **10×** so a full QA day with swarm volume does not exhaust LUNC gas; after genesis changes run **`make reset`** before redeploy. `launch-swarm.sh` runs `bootstrap-swarm-liquidity` once so OE-1 swap pairs (EMBER/CORAL, TOPAZ/ONYX, ONYX/CORAL) stay deep after swap-only volume ([#293](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/293)). Unit tests: `make test-swarm-liquidity`. Funding headroom check (needs fresh reset + deploy): `make verify-localterra-funding-headroom`. Live OE-1 quote check (needs LocalTerra + indexer): `make verify-issue-293` — **acceptance** is `pool_only=true` direct-pool reciprocal (≤5%), `route/solve` **slippage enrichment** (`slippage_percent` + `spot_amount_out` present and math-consistent; at least one EMBER→CORAL direction **>30%** so retail Expert Mode guard is exercisable), and global best-execution route asymmetry traced (multi-hop vs direct is expected on LocalTerra, not a Swap decimal bug). **Prerequisite:** indexer DB must not retain stale duplicate quote assets from an earlier deploy — if `make verify-issue-293` reports missing slippage fields, rerun `make setup-cloud-localterra --fresh`.

### Fee Discount Contract Tests

Canonical tier numbers: [`docs/reference/fee-discount-tiers.md`](reference/fee-discount-tiers.md) (GitLab [#198](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/198)). Drift check: `make check-fee-discount-tier-docs`. Agent playbook: [`skills/AGENTS_FEE_DISCOUNT_TIERS.md`](../skills/AGENTS_FEE_DISCOUNT_TIERS.md). **Registry outage observability** ([#365](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/365), [#375](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/375)): integrator guidance in [`docs/integrators.md` § Fee-discount registry outage](./integrators.md#fee-discount-registry-outage); regression ladder `make verify-issue-365` (contract P5 test + indexer health API + frontend warning util — Postgres bootstrap via `make setup-indexer-postgres` when `indexer/.env` is missing; no LocalTerra). **Pair-scoped fee chrome** ([#537](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/537), **I14**): do not strikethrough a CL8Y discount unless the pair `DISCOUNT_REGISTRY` matches `VITE_FEE_DISCOUNT_ADDRESS`; `make verify-issue-537`. **Post-migrate inherit** ([#538](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/538)): LocalTerra `create_pair` without follow-up Set + dApp smart-query-first; `make verify-issue-538`.

The fee-discount contract has unit tests covering:

- **Tier management:** `AddTier`, `UpdateTier`, `RemoveTier` — validates governance-only access, duplicate tier rejection, and bps bounds (≤10000)
- **Registration:** `Register` for self-registration (EOA-only enforcement), `RegisterWallet` for governance-controlled registration, rejection of contracts attempting self-registration
- **Deregistration:** `Deregister` (self), `DeregisterWallet` (governance), lazy deregistration triggered by insufficient balance
- **Discount queries:** `GetDiscount` returns correct bps for registered traders, returns 0 for unregistered traders, fires deregistration when CL8Y balance is below threshold
- **Trusted routers:** `AddTrustedRouter`, `RemoveTrustedRouter`, `IsTrustedRouter` query
- **Governance tiers:** Tier 0 and Tier 255 cannot be self-registered, only governance can assign them
- **Config updates:** `UpdateConfig` governance-only access

### Integration Tests (Contracts)

The integration test harness in `smartcontracts/tests/` deploys the full contract suite (Factory, Pair, Router, Fee Discount) to a simulated chain and tests:

- End-to-end swap with discount: register a tier (from `STANDARD_PRODUCTION_TIERS` / canonical doc), execute swap, verify reduced commission
- Swap without registration: verify full fee applied
- Balance drop: transfer CL8Y away, swap, verify discount revoked and deregistration fired
- Router trusted forwarding: swap via Router passes trader address correctly
- Factory `SetDiscountRegistryAll`: succeeds when `PAIR_COUNT` ≤ 10; rejects with `DiscountRegistryAllTooManyPairs` when larger ([GitLab #242](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/242), `test_set_discount_registry_all_rejects_when_pair_count_exceeds_cap`)
- Factory `SetDiscountRegistryBatch`: verify cursor `next_start_after` + `has_more` advance until complete ([GitLab #123](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/123), [#242](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/242) — `test_set_discount_registry_batch_covers_many_pairs` for 25 pairs / limit 10)
- Factory `CreatePair` inherits `config.discount_registry` after All/Batch; single-pair `SetDiscountRegistry` does not change the factory pointer; `GetDiscountRegistry` returns stored `Option<Addr>` ([GitLab #536](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/536), `discount_registry_inherit_tests`, `make verify-issue-536`). Live LocalTerra create-pair without a follow-up Set: `make verify-issue-538` ([#538](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/538))
- Blacklist (Tier 255): verify wallet receives zero discount

**Hybrid / limit book (L8 regression):** [`limit_order_tests.rs`](../smartcontracts/tests/src/limit_order_tests.rs) — single-hop hybrid sim vs execute, two-hop router with hybrid on the first leg (`router_two_hop_first_leg_hybrid_matches_simulate`), and **3-hop router with hybrid on ≥2 legs** (`router_three_hop_two_legs_hybrid_matches_simulate`, [GitLab **#192**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/192)). Agent playbook: [`skills/AGENTS_TESTING_MULTIHOP_HYBRID.md`](../skills/AGENTS_TESTING_MULTIHOP_HYBRID.md).

**Indexer route solve (hybrid merge):** integration tests in [`indexer/tests/api_route_solve.rs`](../indexer/tests/api_route_solve.rs) — POST `hybrid_by_hop` merge + LCD mock, GET default hybrid + `hybrid_optimize` on 2- and **3-hop** paths ([GitLab **#192**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/192), default hybrid GET [**#191**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/191)). **Route slippage (#293):** `route_solve_slippage_percent_enrichment_293` uses fixture `seed_route_slippage_293` (token B = `USTC-C` quote asset) and asserts `slippage_percent` / `spot_amount_out` on `GET /api/v1/route/solve` with `pool_only=true`; live OE-1 parity: `bash scripts/qa/verify-issue-293.sh`. Invariants: [indexer-invariants.md](./indexer-invariants.md).

## Coverage

### Frontend (Vitest)

```bash
cd frontend-dapp
npx vitest run --coverage
```

Coverage reports are generated in `frontend-dapp/coverage/` in text, JSON, and HTML formats (configured via `vitest.config.ts`).

### Smart contracts (Rust / LLVM)

Instrumented line coverage for the CosmWasm workspace uses [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov):

```bash
cargo install cargo-llvm-cov
cd smartcontracts
cargo llvm-cov test --workspace --lcov --output-path lcov.info
# Optional HTML report:
cargo llvm-cov report --html --output-dir target/llvm-cov-html
```

Or from the repo root: `make coverage-contracts` (writes `smartcontracts/lcov.info`).

Use coverage to find **untested business logic**, not as a vanity metric — see [contracts-security-audit.md](./contracts-security-audit.md) for invariant-to-test mapping.

## CI {#ci}

**Invariants ([GitLab #234](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/234), [#380](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/380)):**

| Concept | Canonical wording |
|--------|-------------------|
| Hosted GitLab CI | [`.gitlab-ci.yml`](../.gitlab-ci.yml) — `security` stage (gitleaks, cargo/npm audit, indexer log-secret grep [#433](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/433)) + `test` stage (contracts + indexer-lib + Postgres-backed indexer-integration + frontend unit/lint + build gate, [#421](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/421)) + `build` stage (QA wasm/indexer artifacts) |
| Where full checks run | **GitLab** runs security + Phase-1 functional gates (contracts/indexer-lib/frontend unit + build) plus the Phase-2a Postgres-backed indexer-integration suite on default branch + change-gated MRs; **local / QA host** for the remaining heavy row (Playwright E2E) still pending the final Phase 2 of [#421](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/421) |
| [`.github/workflows/*.yml`](../.github/workflows/) | **Reference spec only** (job names, services, step order) — not executed |
| Job names (`e2e`, `frontend-charts-integration`, …) | **Labels** mapping to Make/scripts below |
| “CI green on main” | **GitLab pipeline green** (security + Phase-1/2a functional test + artifact jobs) **and** local automation checklist for the Phase-2 row not yet hosted (Playwright E2E) |
| Supply-chain local mirror | `make audit-smartcontracts`, `make audit-indexer`, `make audit-frontend`, `make gitleaks-detect` — see [supply-chain-security.md](./supply-chain-security.md) |

**Agents:** For GitLab-hosted gates use [docs/supply-chain-security.md](./supply-chain-security.md) and [`skills/AGENTS_SUPPLY_CHAIN_SECURITY.md`](../skills/AGENTS_SUPPLY_CHAIN_SECURITY.md). For the full portable checklist (contracts, frontend, indexer, E2E) use [`.github/workflows/README.md`](../.github/workflows/README.md) and the relevant `skills/AGENTS_*.md` playbook.

### GitLab CI jobs (hosted)

| Job | Local command |
|-----|---------------|
| `gitleaks` | `make gitleaks-detect` |
| `lint-indexer-log-secrets` | `make lint-indexer-log-secrets` (SEC-F13 — grep `indexer/src/` tracing args; unit tests in [`startup.rs`](../indexer/src/startup.rs)) |
| `cargo-audit-smartcontracts` | `make audit-smartcontracts` |
| `cargo-audit-indexer` | `make audit-indexer` |
| `npm-audit-frontend` | `make audit-frontend` |
| `test-contracts` | `make test-contracts` (`cd smartcontracts && cargo test`) |
| `test-indexer-lib` | `cd indexer && cargo test --lib` |
| `test-indexer-integration` | `make test-indexer-integration` (`cd indexer && cargo test --tests -j1 -- --test-threads=1`; a `postgres:16` service replaces `setup-indexer-postgres` — the suite migrates the empty DB itself). Host cargo only — do not bind-mount `indexer/` into Docker (`make test-indexer-target-ownership`) |
| `test-frontend` | `make lint-frontend` + `make test-frontend` (`npm run lint` + `npm run test:run`) |
| `test-frontend-build` | `make build-frontend` (`npm run build` — tsc -b + vite; the build gate that catches tsc-only breaks lint/vitest miss) |
| `qa-wasm-artifacts` | `make build-optimized` (needs Docker; DinD TLS in CI) |
| `qa-indexer-binary` | `cd indexer && cargo build --release` |

Gitleaks abuse check: `make verify-gitleaks` (fixture must fail, clean tree must pass).

### Reference job → local command

| Reference job (`test.yml`) | Local command |
|--------------------------|---------------|
| `docs-fee-discount-tiers` | `make check-fee-discount-tier-docs` |
| `docs-launch-go-no-go` | `make check-launch-go-no-go-docs` / `make verify-issue-391` ([#391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391)) |
| `docs-governance-emergency-rehearsal` | `make check-governance-emergency-rehearsal-docs` / `make verify-issue-397` ([#397](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/397)) |
| `docs-blacklist-decision` | `make check-blacklist-decision-docs` / `make verify-issue-400` ([#400](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/400)) |
| `docs-suspicious-activity-queries` | `make check-suspicious-activity-queries-docs` / `make verify-issue-437` ([#437](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/437)) |
| `docs-anomaly-signals` | `make check-anomaly-signals-docs` / `make verify-issue-435` ([#435](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/435)) |
| `docs-incident-template` | `make check-incident-template-docs` / `make verify-issue-439` ([#439](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/439)) |
| `docs-pool-triage` | `make check-pool-triage-docs` / `make verify-issue-436` ([#436](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/436)) |
| `docs-ibc-hooks-deploy` | `make check-ibc-hooks-deploy-docs` / `make verify-issue-407` ([#407](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/407)) |
| `docs-wasm-migration-rollback` | `make check-wasm-migration-rollback-docs` / `make verify-issue-443` ([#443](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/443)) |
| `contracts-terra` | `make lint-contracts` && `make test-contracts` (optional LCOV: `make coverage-contracts`) |
| `localnet-trading-swarm` | `cd packages/localnet-trading-swarm && npm ci && npx tsc -p tsconfig.json && npm run test:run` |
| `frontend` | `bash scripts/with-node.sh --cwd frontend-dapp -- npx tsc --noEmit` && `make lint-frontend` && `make test-frontend` |
| `frontend-charts-vitest` | `make test-frontend-charts` ([#230](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/230), [#211](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/211)) |
| `frontend-charts-integration` | `make test-charts-integration` ([#205](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/205)) |
| `indexer` | Postgres + `cd indexer && cargo fmt --check && cargo clippy -- -D warnings && cargo test` (integration: [§ Shared Postgres](#shared-postgres-and-test-parallelism)) |
| `e2e` | `make wait-localterra` → `bash scripts/deploy-dex-local.sh` → `make qa-verify-deploy` → `bash scripts/e2e-start-indexer.sh` → `make test-e2e` |
| `frontend-e2e-indexer-outage` | `make test-e2e-indexer-outage` ([#219](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/219)) |

**Wasm (release):** reference job in [`contracts-wasm-optimizer.yml`](../.github/workflows/contracts-wasm-optimizer.yml) → `make build-optimized`. Fast `cargo wasm` in `test.yml` is dev-only; see [deployment guide § Build Optimized WASM](./deployment-guide.md#1-build-optimized-wasm).

The reference workflow [`.github/workflows/test.yml`](../.github/workflows/test.yml) also documents step order for: contract `cargo fmt` / `clippy` / `llvm-cov`, indexer Postgres service container, and Playwright browser install. Full mapping: [`.github/workflows/README.md`](../.github/workflows/README.md).

**Agent playbooks:** [`skills/AGENTS_E2E_STRICT_CHAIN.md`](../skills/AGENTS_E2E_STRICT_CHAIN.md), [`skills/AGENTS_E2E_INDEXER_OUTAGE.md`](../skills/AGENTS_E2E_INDEXER_OUTAGE.md), [`skills/AGENTS_LOCAL_POSTGRES_DEV.md`](../skills/AGENTS_LOCAL_POSTGRES_DEV.md), [`skills/AGENTS_TESTING_P2_EPIC.md`](../skills/AGENTS_TESTING_P2_EPIC.md), [`skills/AGENTS_SUPPLY_CHAIN_SECURITY.md`](../skills/AGENTS_SUPPLY_CHAIN_SECURITY.md).

## Writing Tests

- Place unit tests next to source files: `MyComponent.test.tsx`
- Place integration tests next to source files: `MyComponent.integration.test.tsx`
- Place E2E tests in `frontend-dapp/e2e/`
- Use `renderWithProviders()` from `src/test/helpers.tsx` for component tests
- Use the dev-wallet fixture from `e2e/fixtures/dev-wallet.ts` for E2E tests
