# Agent playbook: launcher Enable Feature + SKU dedupe (GitLab #606)

Use when changing community-tax **Enable Feature**, launcher `enable_feature` / `create_token` SKU lists, Manage Token unlock invoices, or the LocalTerra community-tax smoke SKU step.

Sibling: on-chain template [#592](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/592) ([`AGENTS_COMMUNITY_TAX_CW20.md`](./AGENTS_COMMUNITY_TAX_CW20.md)); Create Token / Manage [#593](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/593) ([`AGENTS_FRONTEND_CREATE_TOKEN.md`](./AGENTS_FRONTEND_CREATE_TOKEN.md)); invoices [#595](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/595); store + smoke [#601](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/601). Audit findings **C-1 / H-2 / L-1** in [`audits/INTERNAL_KIMIK3_1787468843.md`](../audits/INTERNAL_KIMIK3_1787468843.md).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#606**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/606) | Official Enable Feature path + unique SKUs |
| [`docs/contracts-security-audit.md`](../docs/contracts-security-audit.md) | **T606-1–T606-8** matrix row |
| [`community-tax-token/src/invoice.rs`](../smartcontracts/contracts/community-tax-token/src/invoice.rs) | `origin.launcher` payer for EnableFeature only |
| [`community-token-launcher/src/contract.rs`](../smartcontracts/contracts/community-token-launcher/src/contract.rs) | Manager + origin gate; unique SKUs |
| [`communityTaxInvoice.ts`](../frontend-dapp/src/utils/communityTaxInvoice.ts) | Enable Feature payee = env launcher |
| [`localterra-community-tax-smoke.sh`](../scripts/qa/localterra-community-tax-smoke.sh) | Must Send to the **launcher**, not the token |

## Official path (pick one — this is it)

Manager UST1 `Send` → **env launcher** hook `{"enable_feature":{"token":"terra1…","sku":"transfer_tax"}}` → launcher re-Sends the same 50 UST1 to the token → token treats `cw20.sender == origin.launcher` as authorized for **EnableFeature only** → token forwards UST1 to **CMM**.

Direct manager `Send` to the token (`{"enable_feature":{"sku":"…"}}`) still works (CLI). It is **not** the official dApp / QA path. Do not document both as supported retail flows.

Settings Save stays manager `Send` to the **token**. Do not mix Enable Feature into a settings batch (**T592-4**).

## Invariants **T606-1–T606-8**

1. **T606-1 — official Enable Feature.** Manage Token + smoke pay 50 UST1 to the **launcher**. Token accepts `payer == manager` **or** `payer == origin.launcher` for `EnableFeature` only.
2. **T606-2 — manager gate on the launcher.** Launcher queries token `GetConfig.manager` and `GetLauncherOrigin`. `cw20.sender` must be that manager; `origin.launcher` must be **this** launcher. Non-manager → `Unauthorized`; no SKU; victim UST1 untouched.
3. **T606-3 — settings stay manager-only.** `UpdateSettings` still requires `payer == manager`. Origin launcher cannot apply a settings batch.
4. **T606-4 — no arbitrary payer.** Do not loosen invoice auth to random addresses. Only manager or **that token's** `origin.launcher`. A different contract is `Unauthorized`.
5. **T606-5 — unique SKUs.** Create rejects duplicate SKU names **before** multiplying `50 UST1 × len`. Two distinct SKUs = 100 UST1 and both flags on. `Features::from_skus` stays idempotent. dApp `uniqueCommunityTaxSkus` unique-sets before quoting.
6. **T606-6 — exact 50 UST1 to CMM.** Same **T592-4** / **T592-12**. Launcher does **not** also forward to CMM (token does). Already-on / MintControl / wrong token / wrong amount / no-op revert; fee not kept.
7. **T606-7 — QA matches the dApp.** Smoke (and `verify-issue-601` if it calls smoke) must use the launcher hook, not a hidden direct-to-token shortcut that hid C-1 (**L-1**).
8. **T606-8 — columbus-5 migrate.** Launcher instance is on **11622** (2026-08-24) with `token_code_id` **11619** / `autolp_code_id` **11621**. A dApp-only payee change was the no-migrate alternative and was **not** chosen.

## Verify

```bash
make verify-issue-606
# crates only:
cd smartcontracts && cargo test -p cl8y-community-tax-token -p cl8y-community-token-launcher --offline -- --test-threads=1
# inverted PoCs:
cd smartcontracts && cargo test -p cl8y-community-token-launcher --test audit_poc \
  poc_launcher_enable_feature_always_unauthorized \
  poc_launcher_duplicate_sku_double_charge --offline -- --test-threads=1
make verify-issue-593
# smoke (needs LocalTerra + redeploy of these crates):
make verify-issue-601
```

## Do not

- Point Manage Token Enable Feature at the token while also calling launcher “supported”.
- Allow `payer == origin.launcher` for `UpdateSettings`.
- Unique-set on-chain (reject instead) so a crafted `[transfer_tax, transfer_tax]` cannot under/overpay.
- Enable MintControl after instantiate.
- Treat LocalTerra smoke green as proof if SKU unlock still Sends to the token.
