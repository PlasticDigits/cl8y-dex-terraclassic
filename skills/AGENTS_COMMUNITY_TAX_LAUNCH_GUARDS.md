# Agent playbook: community tax LaunchGuards liveness (GitLab #608)

Use when changing **LaunchGuards** cooldown / `max_wallet` on `cl8y-community-tax-token`, or when writing 11611 migrate notes after a token wasm upgrade.

Parent template: [`AGENTS_COMMUNITY_TAX_CW20.md`](./AGENTS_COMMUNITY_TAX_CW20.md) (**T592-11**). Audit findings **H-3 / H-4** in [`audits/INTERNAL_KIMIK3_1787468843.md`](../audits/INTERNAL_KIMIK3_1787468843.md). **H-5** (`trading_enabled=false` locks pair→EOA) stays documented residual — do **not** carve withdraw/cancel/claim here.

Issue **#608 is implemented** in crate + inverted PoCs. Live columbus-5 **11611** instances still need a **token migrate** after a new store (F6 pin). Catalog was empty at audit; do not treat this MR as the migrate.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#608**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/608) | Pair-wide cooldown + provide brick |
| [`tax.rs`](../smartcontracts/contracts/community-tax-token/src/tax.rs) `apply_launch_guards` / `is_cooldown_subject` | H-3 / H-4 fix |
| [`state.rs`](../smartcontracts/contracts/community-tax-token/src/state.rs) `LAST_TRADE_BLOCK` | Per **user** wallet only |
| [`multitest.rs`](../smartcontracts/contracts/community-tax-token/src/multitest.rs) `launch_guards_*` | Acceptance paths |
| [`audit_poc.rs`](../smartcontracts/contracts/community-token-launcher/tests/audit_poc.rs) `poc_cooldown_bricks_pair` / `poc_max_wallet_bricks_provide` | Inverted H-3 / H-4 |
| [`docs/contracts-security-audit.md`](../docs/contracts-security-audit.md) **H608-1** | Invariant table |
| [`cw20-codeid-audits/codeids/11611/REPORT.md`](../cw20-codeid-audits/codeids/11611/REPORT.md) | D11 / A11 notes |

## Invariants **H608-1–H608-8**

1. **H608-1 — cooldown subjects are user wallets.** `check_cooldown` / `record_trade_blocks` skip listed pairs, router, factory, this token, and AutoLP (`is_protocol_exempt`). Two different wallets can buy or sell in the same block when `cooldown_blocks > 0`. The pair timestamp must never rate-limit a third party.
2. **H608-2 — same wallet still cools down.** The trader on Sell (`from`) and the user recipient on Buy (`to`) are recorded. A second hop from that wallet before `cooldown_blocks` elapse reverts `Cooldown` (anti-snipe). Self-trade / same-wallet two hops stay limited.
3. **H608-3 — cooldown 0 is off.** `cooldown_blocks == 0` skips the map entirely. LaunchGuards SKU off skips cooldown and `max_wallet`.
4. **H608-4 — max_wallet skips protocol `to`.** Do not apply the cap when `to` is a listed pair or other protocol-exempt address. Provide `TransferFrom` must succeed after sells grow `pair_balance > max_wallet`.
5. **H608-5 — max_wallet still caps users.** Ordinary wallets on Buy / Transfer (including TransferTax + max_wallet) cannot receive above the cap.
6. **H608-6 — sell-to-pair bypass stays.** Sell to a listed pair still bypasses `max_wallet` (**T592-11**). Do not “fix” H-4 by applying the cap to exits.
7. **H608-7 — H-5 pause residual.** `trading_enabled=false` still blocks **both** buy and sell (pair→EOA withdraw/cancel/claim stay locked). Out of scope for #608.
8. **H608-8 — same-sender in-block residual (11611 D11).** A second transfer from the **same** wallet in one block may still cooldown. Acceptable anti-snipe; not a pair-wide halt. Manager dust `max_wallet` / cooldown=1 after users buy is product (M-4), not this ticket.

## Rules of thumb

- `LAST_TRADE_BLOCK` is **not** a pair clock. If a new call site writes both `from` and `to` without `is_cooldown_subject`, H-3 returns.
- `is_protocol_exempt` is the skip set for both cooldown and `max_wallet` `to`. Do not invent a second list.
- Manager-exempt wallets are **not** protocol-exempt: they still take cooldown / max_wallet unless a later spec says otherwise.
- Do not add pair/router FoT math (**H-01** / **T592-1**).

## Verify

```bash
make verify-issue-608
# equivalent:
cd smartcontracts && cargo test -p cl8y-community-tax-token --lib
cd smartcontracts && cargo test -p cl8y-community-token-launcher --test audit_poc -- --test-threads=1
```

LocalTerra after a new token store: LaunchGuards token, two wallets swap in one window; provide after pair > cap. That is **ops / migrate**, not this crate gate.

## Do not

- Record or check cooldown on the listed pair / router / factory / token / AutoLP.
- Apply `max_wallet` to provide or other protocol `to`.
- Change `trading_enabled` exit behavior (H-5).
- Treat inverted PoC names as still-broken — `poc_cooldown_bricks_pair` / `poc_max_wallet_bricks_provide` now assert the fix.
