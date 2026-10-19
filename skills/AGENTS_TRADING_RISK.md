# Trading risk policy (#1560)

Apply to Swap/Trade notices, ordinary review, funded guidance and mutation-time
risk guards. This policy explicitly supersedes the old ordinary **>5% aggregate
fair-reference warning/consent** in OS1500-8, RC1483-2 and related UI/API guidance.
See [implementation and acceptance](../docs/qa/issue-1560/README.md),
[order economics](AGENTS_ORDER_SLIPPAGE.md) and
[mobile presentation](AGENTS_SWAP_LAYOUT.md).

| ID | Invariant |
| --- | --- |
| TR1560-1 | One shared `evaluateTradingRisk` drives Swap/Trade notices, funded guidance, first-click review and final submission. Warn/review iff signed validated order slippage **>5% OR symmetric fair-reference deviation >10%**. Equality does not trigger. Never sum displayed fees/slippage or substitute worst-hop/legacy labels. |
| TR1560-2 | Checked original Uint128 numerators/denominators govern exact 5/10/30/99 boundaries. Favorable order movement stays signed; favorable reference divergence remains symmetric. Independent total **>30%** requires Expert and **>=99%** is never waivable. Display rounding never supplies a policy predicate. |
| TR1560-3 | Preserve original selected reserve/book replay, attribution reconciliation, operation/amount/direction/generation/policy/tolerance/observation/expiry bindings and reference admission. Missing, forged, contradictory, unsupported or stale evidence is unavailable, never zero/safe or permission to sign. Unknown risk blocks with persistent text. |
| TR1560-4 | Supported zero-hop wrap/unwrap has structurally zero order movement only after `validateWrapperOnlyQuote` revalidates the admitted equal-principal mapping, identity/capability, original clocks and positive floor protocol. Its independently admitted original fair/net comparison still determines total risk. Unsupported wrappers stay unavailable. Never fabricate pool diagnostics. |
| TR1560-5 | Consent binds wallet account, exact asset identities/direction/input, selected operations, manual allocation, policy/tolerance/Expert/deadline, original metadata/output and validated risk evidence. Changed evidence revokes it. Identical fresh original observations preserve it. Pause automatic quote arithmetic refresh/promotion during original unexpired ordinary review in pay and receive sizing; expire or revoke on intent/account/policy edits and resume. Keep independent restriction observations active. Mutation re-evaluates fresh evidence, compares its original identity against the rendered review and rechecks consent; no after-render mutation bypass. |
| TR1560-6 | One concise named order/total/both notice; ordinary warnings use polite status and blocking errors persist. No nested live regions or visible settled `None`; unchanged text does not reannounce. DETAILS retains truthful percentages and costs. No timed error toast or rapid reaction window. |
| TR1560-7 | Preserve 30-minute reviewed local signing, original expiry, offline quoting/preparation/review, delivered-net ranking, independently admitted references, positive hop/post-hook/final floors, gross route taxes/refunds, freeze/pause/blacklist, exact native gas/Max parity and no duplicate broadcast. |
| TR1560-8 | Run necessary bounded relevant risk/UI/types/lint and production-browser checks. Record executed failures, unavailable funded/device/live scope separately; source/synthetic evidence never closes unperformed acceptance. See the [checklist](../docs/qa/issue-1560/README.md). |

Code: [shared evaluator and final guard](../frontend-dapp/src/utils/tradingRisk.ts),
[original replay](../frontend-dapp/src/utils/routeOrderMetrics.ts),
[retained original evidence](../frontend-dapp/src/utils/cw20RouteSolveQuote.ts).
