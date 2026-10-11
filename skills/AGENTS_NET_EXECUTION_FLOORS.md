# Net execution floors and bounded hooks (#1398)

Use this playbook when changing pair/router slippage, hook settlement, forward or
reverse quotes, hook admission/recovery, or community-tax AutoLP settlement. It addresses
[#1398](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1398).

## Invariants

| ID | Required behavior |
| --- | --- |
| NE1398-1 | Keep the existing L9 market-spread and hybrid book-degradation checks. `max_spread` and `belief_price` alone are not a post-hook receive guarantee. A swap with registered hooks requires an explicit positive `min_return`; compare it against ask output after all recognized ask-token hook deductions. |
| NE1398-2 | Apply the same floor to direct, declared hybrid, greedy and routed pair execution. Router per-hop `min_return` must be caller supplied; final `minimum_receive` does not substitute. Official Swap/Trade direct signing uses a fresh post-hook pool/hybrid quote and a positive floor; reverse Swap keeps the stronger target/fresh floor. Zero/dust output blocks signing. Never invent a weaker floor or substitute pre-hook leg fields. |
| NE1398-3 | Forward `HybridSimulation.return_amount` is post-hook ask output, with per-hook integer-floor rounding matching execution. Leg fields and protocol commission retain their pre-hook meanings. `AfterSwap.return_asset.amount` and swap `return_amount` remain pre-hook; swap `net_return_amount` is the delivered pair amount. |
| NE1398-4 | Unknown, malformed or unqueryable hook pricing fails explicitly in quote and execute. Do not interpret unavailable config as zero fees. Non-deducting supported LP hooks do not reduce ask quotes; forward pricing is not proof their callback will succeed. |
| NE1398-5 | Reverse pricing must never target a pre-hook output while claiming net sufficiency. Unsupported hook-aware inverse pricing fails explicitly; no gross quote fallback or unbounded search. Hook-free reverse minimality and simulation bounds remain intact. |
| NE1398-6 | Every hook-list admission path rejects more than eight entries and duplicate validated addresses before persisting configuration or emitting updates. Preserve accepted order. This limits dispatch count, not arbitrary callback gas. |
| NE1398-7 | Invalid legacy stored lists fail closed before hook queries/callbacks. Governance repairs them with a full bounded replacement through factory `SetPairHooks` → pair `UpdateHooks`; execution and migration never silently truncate or deduplicate policies. |
| NE1398-8 | A recognized hook's ask deduction cannot exceed the return, and combined deductions cannot overdraw it. Checked wide arithmetic and identical per-hook rounding apply to quotes and settlement. Preserve L7 protocol commission semantics and I2 funding from this swap. |
| NE1398-9 | Community-tax AutoLP anchors swap and minted-LP minima to manager-approved reserves per share. Permissionless skim never refreshes that anchor from mutable spot. Missing/stale policy, missing provide leg, insufficient minted LP or ratio failure reverts swap and provide atomically; the reentrancy lock remains held through provide completion. |
| NE1398-10 | Complete runnable verification and retest after corrections. Record unavailable checks as SKIP with observed reasons; overall PASS WITH SKIPS is permitted only with no unresolved executed failures. Source evidence never establishes deployed code or production acceptance. |

## Agent review

For a legacy token-owned AutoLP sister, token **1.4.0** adds an explicit
manager-authorized same-pair refresh with exact reserve/share approval. Follow
[LP1429 recovery](AGENTS_AUTOLP_POLICY_RECOVERY.md) and its
[separate inventory/migration/readback checklist](../docs/qa/issue-1429/README.md);
unchanged settings remain a no-op and skim never refreshes policy.

1. Read the [QA checklist](../docs/qa/issue-1398/README.md) and
   [integrator contract](../docs/integrators.md#net-execution-floors-and-hook-pricing-1398).
2. Run `make verify-issue-1398`. Review direct/hybrid/router net floors, hook
   rounding, unknown pricing, bounded admission and legacy replacement tests.
3. Inspect AutoLP shortage/ratio failure transaction snapshots: no successful insufficient-LP
   provision and no persistent reserve, escrow or treasury changes.
4. Retest corrected behavior. Save revision, UTC timestamp, exact command,
   outcome and test counts. Check off only verified acceptance criteria.
5. Provision available dependencies before recording unavailable chain/device
   checks. SKIP is allowed and does not create an implementation hold, but it
   does not mark omitted deployment criteria complete.

Sources: [settlement helper](../smartcontracts/packages/dex-common/src/hook_settlement.rs),
[hook bounds](../smartcontracts/packages/dex-common/src/hook.rs),
[pair](../smartcontracts/contracts/pair/src/contract.rs),
[router](../smartcontracts/contracts/router/src/contract.rs),
[factory](../smartcontracts/contracts/factory/src/contract.rs),
[AutoLP](../smartcontracts/contracts/community-tax-autolp/src/contract.rs),
[AutoLP policy](../smartcontracts/contracts/community-tax-autolp/src/lp_guard.rs).
Related: [L9 market spread](./AGENTS_MAX_SPREAD_HYBRID.md),
[L8 hybrid quoting](./AGENTS_HYBRID_QUOTING.md),
[L7 hook commission](./AGENTS_HOOK_COMMISSION.md),
[community-tax AutoLP](./AGENTS_COMMUNITY_TAX_AUTOLP.md).

Pair 1.21.0 retains the reviewed router reverse-probe accounting. Keep
[the reverse-query budget playbook](./AGENTS_REVERSE_QUERY_BUDGET.md) in sync with
reviewed version changes; future unknown versions keep the conservative budget.

Remaining observation-only quote coverage, explicit external wrapper floors and
bounded composed inverse follow [QC1459](AGENTS_LOCAL_QUOTE_COVERAGE.md) and the
[agent implementation checklist](../docs/qa/issue-1459/README.md).

Current retail quote-relative execution follows [TQ1473](AGENTS_TRADING_QUOTE_OUTCOMES.md).
The selected UI tolerance anchors upward-rounded positive net hop/final floors;
checked pinned L9 economics derive the separate signed market-spread ceiling.
The existing L9 formula/material-book guards remain, with legacy API requests
retaining direct UI-to-L9 admission. Read [the exact policy and compatibility](../docs/route-solver.md#quote-relative-execution-protection)
and run `make verify-issue-1473`. No request-time simulation or weaker receive floor.
