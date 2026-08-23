# Agent playbook: community tax CW20 (GitLab #592)

Use when changing the **community tax token**, **launcher**, or **AutoLP** sister, or when listing that template on the factory.

This is the **only** tax-token exception to **H-01**. It is DEX-safe because inbound credits to pair / router / escrow / AutoLP stay **1:1**. Do **not** add pair/router FoT balance-delta math. Factory `AddWhitelistedCodeId` is a **separate ops step** after `#589` REPORT **GO**.

Sibling surfaces: dApp create/manage [#593](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/593); indexer catalog [#594](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/594); pay-with-any-token [#595](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/595) (**I595-14** — launcher/token still accept UST1 `Send` only).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#592**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/592) | On-chain design |
| [`docs/contracts-terraclassic.md` § Community tax CW20](../docs/contracts-terraclassic.md#community-tax-cw20-gitlab-592) | Message shapes + classification |
| [`docs/contracts-security-audit.md`](../docs/contracts-security-audit.md) | Invariants **T592-1–T592-12** |
| [`docs/runbooks/cw20-whitelist-policy.md`](../docs/runbooks/cw20-whitelist-policy.md) | Narrow whitelist exception |
| [`cw20-codeid-audits/codeids/community-tax-token/REPORT.md`](../cw20-codeid-audits/codeids/community-tax-token/REPORT.md) | #589 template (NO-GO until LCD store) |
| [`AGENTS_HOOK_CW20_OPS.md`](./AGENTS_HOOK_CW20_OPS.md) | **H-01** / do not add FoT math |
| [`AGENTS_CW20_CODE_ID_AUDIT.md`](./AGENTS_CW20_CODE_ID_AUDIT.md) | Listing intake |
| [`AGENTS_FRONTEND_PAY_INVOICE.md`](./AGENTS_FRONTEND_PAY_INVOICE.md) | Off-chain any-token pay |

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
11. **T592-11 — launch guards.** `trading_enabled=false` blocks **both** buy and sell. Sell to a listed pair bypasses `max_wallet`.
12. **T592-12 — UST1-only invoice.** #595 routes any token off-chain. Token/launcher do not accept the tax token as fee.

## Launcher UST1 hook

`community-token-launcher` `Receive` accepts UST1 only. Exact invoice: `50 UST1 × SKU count` on create; `50 UST1` on later `EnableFeature`.

| Hook | JSON |
|------|------|
| `create_token` | Newtype `CreateTokenMsg` — `{"create_token":{name,symbol,decimals,initial_balances,manager,treasury,buy_bps,sell_bps,max_*,features,…}}` |
| `enable_feature` | `{"enable_feature":{"token":"terra1…","sku":"transfer_tax"}}` |

Do not send a settings batch to the launcher. AutoLP instantiate+bind in the same create tx is **out of v1** (pay SKU, then bind).

## Option A (v1)

Token-only extra-debit sell + outbound buy. `SetPairHooks` stays governance-only. Pair/router swap math **unchanged**.

## Verify

```bash
make verify-issue-592
cd smartcontracts && cargo test -p cl8y-community-tax-token -p cl8y-community-token-launcher -p cl8y-community-tax-autolp
```

Do **not** `AddWhitelistedCodeId` until `cw20-codeid-audits/codeids/<id>/REPORT.md` is **GO**. Do not whitelist ALPHA **8654**.

## Do not

- Credit the pair `amount * (1 - sell_bps)` (inbound FoT).
- Call `SkimToLp` from taxed `Transfer`/`Send`.
- Charge settings fees on `Mint` or `SkimToLp`.
- Enable MintControl after instantiate.
- Treat this template as a license to whitelist other tax tokens.
