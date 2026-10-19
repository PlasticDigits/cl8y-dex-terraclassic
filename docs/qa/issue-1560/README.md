# Trading risk warning and review (#1560)

Issue: [#1560](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1560).
Companion: [#1561](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1561).
Contract: [TR1560](../../../skills/AGENTS_TRADING_RISK.md),
[OS1500](../../../skills/AGENTS_ORDER_SLIPPAGE.md),
[API/order economics](../../route-solver.md#expected-order-slippage-1500).

`evaluateTradingRisk` retains original execution-validated response evidence and
independently replays selected original order economics before applying exact
integer predicates. Ordinary warning and consent are `order >5 OR total >10`.
Total is `100*abs(fair-net)/max(fair,net)`; order is signed movement/baseline.
Equality stays below the ordinary thresholds. Fees do not add to either rule.
Original total >30 Expert and >=99 nonwaivable controls remain symmetric.
No worst-hop, server display label, Number conversion or rounded percentage
establishes a safe risk class. A validated supported direct wrapper has no market
order movement; original mapping/floor/reference/freshness validation still applies.

Swap and Trade use one notice reason and the same policy for funded guidance,
first click and mutation. Consent includes exact account/assets/input, selected
operations/manual split, tolerance/Expert/deadline, original execution metadata
and risk evidence. Final mutation reruns validation, requires the same evidence
identity and checks fresh consent. Identical original observation refreshes retain
review; edits, generation/promotion, policy/account changes and expiry revoke it.
Automatic arithmetic refresh and route promotion pause while ordinary review owns the original unexpired observation in pay and receive sizing. Original expiry and intent/account/tolerance/policy edits revoke review and resume normal quote refresh; independent negative restriction checks remain active. This prevents a ten-second background refresh from imposing a rapid acknowledgement window. Original network/chain execution validation and all signing floors remain enforced.

## Adjacent noise audit

- Active old aggregate >5 checks existed in Swap notice/CTA/mutation, Trade
  notice/review/submission and funded guidance. All now consume the shared policy.
- Active Swap polite status nested child alerts; parent live region is removed,
  each meaningful status is announced locally, and the reserved neutral slot is
  empty rather than a visible checked `None`. Stable text is unchanged by polling.
- Ordinary price warnings use a polite `status`; real transaction/gate failures
  retain persistent textual alerts and priority. Both flows retain
  `toastOnError:false`; generic unrelated `ToastContext`'s timer is unchanged.
- `slippageSeverityClass` (>1 amber) and
  `INDEXER_WALLET_AMOUNT_MISMATCH_WARN_PCT` (0.1%) have no active production
  callers in this revision. They did not cause this reported warning and were
  not weakened. Selected protection >5 is a separate active settings disclosure.

## Acceptance checklist

- [ ] AC1: Both real Swap/Trade shared evaluator/ordinary review use exact strict 5/10.
- [ ] AC2: 3.34/1.71/6.91 is quiet; independently triggered order/total and both are named once.
- [ ] AC3: Favorable/unfavorable >30 Expert and >=99 nonwaivable guards pass.
- [ ] AC4: Missing/forged/stale/unsupported evidence unavailable; expiry/RPC invariants retained.
- [ ] AC5: Noise audit and one meaningful accessible transition; failures remain persistent.
- [ ] AC6: Account/edit/promotion/unchanged-refresh/expiry/tolerance/recovery consent and mutation agree.
- [ ] AC7: Superseded guidance, relevant tests/types/lint and production browser checks pass.
- [ ] Live/funded execution families, decimal/amount/direction variants and physical assistive devices.

Unit/synthetic checks establish only their tested scope. The final issue comment
and deployment receipts record checked criteria and unavailable acceptance;
unperformed scenarios remain unchecked. Run each test process with a <=55-second
timeout under the task's 60-second limit. Keep production serving while validating.
