# Agent playbook: listed-pair autoregister + manager role tax skip (GitLab #633)

Use when changing community-tax `RegisterListedPair` callers, factory `CreatePair` reply, AutoLP pair bind, Manage catch-up, or manager-role tax skip.

Parent template [#592](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/592) ([`AGENTS_COMMUNITY_TAX_CW20.md`](./AGENTS_COMMUNITY_TAX_CW20.md) **T592-7**, **T592-9**). Create/Manage [#593](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/593) ([`AGENTS_FRONTEND_CREATE_TOKEN.md`](./AGENTS_FRONTEND_CREATE_TOKEN.md)). ExemptionDirectory extra wallets [#609](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/609) ([`AGENTS_COMMUNITY_TAX_EXEMPT.md`](./AGENTS_COMMUNITY_TAX_EXEMPT.md)). AutoLP pair [#610](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/610) ([`AGENTS_COMMUNITY_TAX_AUTOLP.md`](./AGENTS_COMMUNITY_TAX_AUTOLP.md)). Migrate-adopt [#626](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/626) (**M626-10** — do **not** register Terraport/GDEX).

Product (2026-08-25): every CL8Y factory pair that holds a community-tax CW20 must be registered. The **manager** wallet skips buy / sell / transfer tax. Catch-up lives on Manage (highest-LP unregistered factory pair). Do not ship autoregister without the manager skip.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#633**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/633) | Autoregister + Manage catch-up + manager role skip |
| [`tax.rs`](../smartcontracts/contracts/community-tax-token/src/tax.rs) `is_manager_exempt` | Role + directory skip |
| [`factory/src/tax_register.rs`](../smartcontracts/contracts/factory/src/tax_register.rs) | cw2-gated CreatePair register |
| [`community-tax-autolp` `pair.rs`](../smartcontracts/contracts/community-tax-autolp/src/pair.rs) | Bind → token register |
| [`communityTaxRegisterPair.ts`](../frontend-dapp/src/utils/communityTaxRegisterPair.ts) | dApp B1 + highest-LP picker |
| [`docs/contracts-terraclassic.md` § Classification](../docs/contracts-terraclassic.md#classification-t592-7) | T592-7 table |

## Invariants **R633-1–R633-8**

1. **R633-1 — manager role skip.** `addr == config.manager || MANAGER_EXEMPT` skips Buy / Sell / Transfer (pair-direct and official-router hop `trader`). `TaxPreview` matches execute. Pair inbound stays 1:1. Extra-debit Max is 0 when `IsProtocolExempt.manager` is true. The manager is **not** protocol-exempt (cooldown / `trading_enabled` / user-side `max_wallet` stay on — **E609-2** / **T592-11**). Extra wallets still need ExemptionDirectory. `remove_exempt` cannot opt the manager into tax.
2. **R633-2 — factory CreatePair autoregister.** After pair persist, execute `register_listed_pair` only on assets whose cw2 name is `crates.io:cl8y-community-tax-token`. Honest/honest → no extra execute. Tax/honest → tax side only. Tax/tax → both. Fail-closed if a tax-side register reverts (token `already` is success). Requires factory migrate. Do not call unknown CW20 executes.
3. **R633-3 — AutoLP bind registers.** Instantiate or `UpdateConfig { pair }` also executes `register_listed_pair` on `cfg.token` for the factory-returned pair. Same lookup as **M610-1**. Re-bind is idempotent. `SkimToLp` is still never called from token `Transfer`/`Send` (**T592-10** / **M610-8**).
4. **R633-4 — Manage catch-up.** Manager wallet + tax template (`VITE_COMMUNITY_TAX_CODE_ID`): highly visible alert iff this token has ≥1 factory pair and ≥1 of those is unregistered. Hidden when no factory pair, all registered, wrong code id, or disconnected/non-manager. Still required at 0 bps. One button registers the **highest-LP** unregistered factory pair (hub USD TVL when both sides price; else tax-token reserve, then other reserve; tie = lower pair address). Factory `Pair` verify required — never Terraport/GDEX. After success, leftover pairs retarget the same button.
5. **R633-5 — dApp Create Pair follow-up.** After a successful `/create` tx, register each tax-pin asset. Honest-only pairs skip. Register failure is a hard error that points at Manage — do not toast “listed” on failure. Sequential txs work **before** factory migrate.
6. **R633-6 — no Terraport / GDEX / adopt register.** Adopt/migrate does not register external DEX pairs (**M626-10**). Existing CL8Y pairs are caught by Manage / permissionless register, not by the migrate tx. Factory lookup stays the gate (**T592-9**). Adopt leftovers (which venues exist, F6/Refresh copy, post-refresh register on `/token/migrate`) are [#634](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/634) / **M634** — do not stretch Manage-only copy to cover migrate inventory.
7. **R633-7 — retail copy / no Swap dump.** Alert uses human words (no `LISTED_PAIRS` / `VITE_*` / `RegisterListedPair`). Stay on Manage after register (optional `/pool` link). Do not inject the token into Swap defaults (**C593-11** / **#596** hybrid stays on). CTA is LCD `GetConfig.manager` gated — no `?pair=` / `?manager=` / `?payee=`.
8. **R633-8 — TransferTax residual.** Unregistered + TransferTax provide still FoT-desyncs pair reserves if register is omitted (P2 / **H-01**). Official Create Pair + factory B2 close this for new factory pairs. Do **not** add pair FoT math. Columbus-5 factory is **11629** (cw2 1.10.0). Token pin **11630** / AutoLP pin **11633** — 0 instances of 11611 / 11619 / 11626 / 11630, so no CMM migrate.

Cross-token Manage line: attested catalog `GET /api/v1/community-tokens?manager=` only (**O601-4**). Link to the other token’s Manage — no extra register buttons.

## Verify

```bash
make verify-issue-633
# LocalTerra live (fail if chain missing):
VERIFY633_REQUIRE_CHAIN=1 make verify-issue-633
# or directly:
./scripts/qa/localterra-633-autoregister.sh
cd smartcontracts && cargo test -p cl8y-community-tax-token -p cl8y-community-tax-autolp -p cl8y-dex-factory
make verify-issue-609
make verify-issue-610
```

LocalTerra (`localterra-633-autoregister.sh`, **R633** live): seed tax/EMBER stays registered; factory `CreatePair` tax/UST1 autoregisters **without** a hand-rolled `register_listed_pair`; honest/honest `CreatePair` must not revert; test1 is `IsProtocolExempt.manager` without ExemptionDirectory and manager `Send+Swap` is Honest; a third wallet extra-debits. Fresh `make deploy-local` instantiates the #633 factory wasm so B2 is live — no factory migrate needed on a new LocalTerra. Columbus-5 factory is **11629**; leftover record + store script stay on [#635](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/635) / [`scripts/upgrade-635-autoregister.sh`](../scripts/upgrade-635-autoregister.sh). Do not treat LocalTerra store ids as columbus-5 evidence.

Sibling migrate inventory: [`AGENTS_FRONTEND_TOKEN_MIGRATE.md`](./AGENTS_FRONTEND_TOKEN_MIGRATE.md) **M634** / [`localterra-634-migrate-inventory.sh`](../scripts/qa/localterra-634-migrate-inventory.sh).

## Do not

- Add pair/router FoT math (**H-01**).
- `RegisterListedPair` Terraport / GDEX / non-factory addrs.
- Factory-whitelist 8654 or launcher / AutoLP code ids.
- Blindly execute `register_listed_pair` on both CreatePair assets.
- Make the manager protocol-exempt.
- Turn hybrid off or dump the token into Swap defaults.
- Add an unregister API.
