# Agent playbook: community tax router hops tax the original trader (GitLab #607)

Use when changing **community tax** classification, Swap/Trade copy for `code_id` **11611**, extra-debit Max, or anything that could tax (or skip tax on) a **router** hop.

This is the **C-2 design close**. Chosen option: **improved 2 — tax the original trader on official-router hops**. Do **not** revert to option 1 (disclose-only Honest hops) or implement option 3 (un-exempt the router) unless a new issue reopens H-01 or drops hybrid for this template.

Issue **#607 is an implement close**. Token classify changes. Pair/router **swap math** stay unchanged (**H-01**). Official router wasm already passes `Swap.trader` (fee-discount path) — do not teach the router to size extra-debit.

Live columbus-5 **11611** still runs pre-option-2 wasm until CMM store + migrate. In-repo crate + dApp match this playbook. Solver ranking that prepares for option-2 wasm (without treating unmigrated 11611 as option 2) is **[#615](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/615)** — [`AGENTS_INDEXER_TAX_AWARE_ROUTING.md`](./AGENTS_INDEXER_TAX_AWARE_ROUTING.md).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#607**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/607) | Design + implement (improved option 2) |
| Audit C-2 | [`audits/INTERNAL_KIMIK3_1787468843.md`](../audits/INTERNAL_KIMIK3_1787468843.md) |
| [`AGENTS_COMMUNITY_TAX_CW20.md`](./AGENTS_COMMUNITY_TAX_CW20.md) | **T592-13** + inbound 1:1 (**T592-1**) |
| [`AGENTS_FRONTEND_CREATE_TOKEN.md`](./AGENTS_FRONTEND_CREATE_TOKEN.md) | **C593-14** sell/buy copy |
| [`AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md`](./AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md) | Official Swap keeps hybrid (**#596**) |
| [`AGENTS_HYBRID_QUOTING.md`](./AGENTS_HYBRID_QUOTING.md) | Quote = execute; router when `ops.length >= 2` |
| [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) | Short hints; no architecture essays |
| [`tax.rs` `classify`](../smartcontracts/contracts/community-tax-token/src/tax.rs) | Sell on pair-direct **or** official-router `Send+Swap`; buy on pair→EOA **or** router→user |
| [`swapRouting.ts`](../frontend-dapp/src/services/terraclassic/swapRouting.ts) | `swapOpsRequireRouter` ⇔ `ops.length >= 2` |
| [`taxPreviewMaxSpend.ts`](../frontend-dapp/src/utils/taxPreviewMaxSpend.ts) | Extra-debit Max on every listed-pair sell + route hints |

## Invariants **R607-1–R607-8**

1. **R607-1 — improved option 2.** Buy/sell bps apply to **every listed-pair swap**, including official multi-hop, hybrid-on-router, permissionless 1-op `execute_swap_operations`, and invoice wrap-routes. Pair-direct `Send+Swap` extra-debits `from`. Official-router `Send+Swap` extra-debits authenticated `Cw20HookMsg::Swap.trader`. Write this as **T592-13**.
2. **R607-2 — H-01 held.** Do not add pair/router FoT balance-delta math. Do not teach the router to size extra-debit. Inbound to pair / router / escrow / AutoLP stays 1:1 (**T592-1**). Router wasm already sets `trader` for fee discount — reuse it; do not add a spoofable free-form field on pair-direct.
3. **R607-3 — still taxed (pair-direct).** Official **single-hop** pair `Send+Swap` (and Trade pair-direct) still extra-debits sell / splits buy. Extra-debit Max (**C593-9**) applies.
4. **R607-4 — router hops taxed.** Official multi-hop, hybrid ops that execute on the router, permissionless 1-op `execute_swap_operations`, and invoice wrap-routes via the router collect buy/sell:
   - **Sell:** `from == config.router` + `Send+Swap` → extra-debit `trader` (not the router’s `amount`). Missing / protocol-exempt / self `trader` → `RouterTraderRequired` (fail closed).
   - **Buy:** pair→router stays 1:1; official router→non-exempt is outbound split (same primitive as pair→EOA). TransferTax SKU is not a substitute for buy tax on that hop.
5. **R607-5 — hybrid stays on.** Do **not** force pair-only execute for `code_id` 11611. That is option 3 and it does not stop off-dApp router use. **#596** is unchanged. Tax applies **because** classify honors the router’s `trader`, not because hybrid is off.
6. **R607-6 — disclose, don’t lecture.** Swap/Trade: sell → `Sell tax extra`; receive tax token → `Buy tax applies`. Create/Manage: `Buy/sell tax applies on every listed-pair swap.` No always-on architecture banner. Do not say hops skip tax.
7. **R607-7 — extra-debit Max matches execute.** Cap Max for extra-debit on **both** pair-direct and router-hop sells. User Sends `amount` to the router 1:1, then the hop extra-debits leftover `tax` — 100% Max self-DoS.
8. **R607-8 — PoC is inverted.** `poc_router_exemption_full_tax_bypass` proves: no-`trader` router sell fail-closes; authenticated trader is extra-debited; pair-direct ignores spoofed `trader`; pair→router 1:1; router→user is buy split.

## Why this is “improved” option 2

- **No spoof:** `Swap.trader` is honored **only** when `from` is the stamped `config.router`. Pair-direct extra-debits `from` and ignores a victim `trader`. Protocol-exempt trader is rejected.
- **No router migrate for trader passing:** columbus-5 router already sets `trader: Some(sender)` on every hop (fee-discount). Token migrate only.
- **Buy-leg specified:** pair→router 1:1 (**T592-1**); tax is withheld on router→user, not by shrinking inbound.
- **Hop does not brick:** extra-debit never hits the router’s `amount` (no `InsufficientForSellTax` on the hop itself).
- **Option 3 still rejected:** un-exempting the router makes hops fail unless the router holds extra-debit.

## #603 importer

[#603](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/603) may promise “bps on every official Swap” **after** the token instance is on this wasm (new launch or 11611 migrate). Importers of **unmigrated** 11611 must still disclose Honest hops.

## 11611 pin vs option-2 ranking (#615)

Live columbus-5 **code_id 11611** stays Honest hops (**R607-4**) until CMM migrate. The indexer must **not** skip UST1→TAX→USTR on that pin.

**Flip after migrate / new crate** (ops, not a silent default):

```bash
# indexer/.env — list the option-2 code id and/or wasm data_hash
COMMUNITY_TAX_OPTION2_CODE_IDS=11611
# COMMUNITY_TAX_OPTION2_DATA_HASHES=<lowercase hex>
```

Then `GET /route/solve` drops middle-hop TAX sells and applies buy-split net on multi-hop `token_out` (**R615-3** / **R615-5**). Pair/router wasm still must actually tax hops — the env flag only changes **ranking**. See [`AGENTS_INDEXER_TAX_AWARE_ROUTING.md`](./AGENTS_INDEXER_TAX_AWARE_ROUTING.md).

## Verify

```bash
make verify-issue-607
make verify-issue-615
make verify-issue-612
```

Also: `make verify-issue-592` (T592 crates + docs) · `make verify-issue-593` (Create Token + extra-debit). Post-merge Enable Feature LocalTerra: [#612](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/612) ([`AGENTS_POST_MERGE_OPS_612.md`](./AGENTS_POST_MERGE_OPS_612.md)). Do not run the stale option-1 disclose checklist on this ticket.

## Do not

- Honor `Swap.trader` when `from` is a user EOA (option-2 spoof / extra-debit a victim).
- Extra-debit the router’s hop `amount` (option 3 failure mode).
- Remove `PROTOCOL_EXEMPT` on the router (option 3) without accepting a pair-only tax token.
- Claim hops skip buy/sell tax (option 1 copy).
- Add pair/router FoT math to “make bps mean bps” on multi-hop.
