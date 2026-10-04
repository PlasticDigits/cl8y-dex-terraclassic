# #1256: editable Swap receive on native and client routes

Issue and criteria: [#1256](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1256). Implementation invariants: [R1256-1–R1256-6](../../frontend.md#swap-reverse-quotes), [Q713-4](../../frontend.md#swap-url-sync), [Q711-6](../../frontend.md#swap-query-params). Agent playbook: [AGENTS_FRONTEND_SWAP_REVERSE_QUOTES.md](../../../skills/AGENTS_FRONTEND_SWAP_REVERSE_QUOTES.md).

The receive field sizes a quote across direct CW20, mapper-only wrap/unwrap, native routes, and client multi-hop. Submit continues to offer-in execution, verified by the forward quote and existing safety guards. Indexer-only without client operations silently ignores output as the reverse source; `/route/solve` remains offer-in.

Implementation evidence: [verification.md](verification.md).

## Run and record

From the reviewed checkout, record `git rev-parse HEAD` and run the following. Retain each command's exit status and evidence in the PR/review. After corrective edits, rerun affected tests and `make verify-issue-1256`.

```bash
make verify-issue-1256
make verify-issue-713
make verify-issue-678
make verify-issue-512
python3 scripts/check_chrome_nesting.py
make lint-frontend
```

The new verifier runs focused RTL, reverse service/math, documentation links, chrome nesting, and a mocked-LCD browser spec with five workers. The browser spec uses `E2E_SWAP_RECEIVE_1256=1` to bake isolated synthetic contract pins and `PLAYWRIGHT_SKIP_CHAIN=1`; it verifies interaction without claiming live-chain execution. Existing #713 browser steps may require a valid deployed factory; provision/probe before declaring that live-chain check unavailable.

When a check cannot be completed, the reviewing agent may mark **SKIP** with the precise missing capability/prerequisite and remaining scope, and still issue **PASS with explicit skips** if completed checks pass and no executed check fails. Never check off unexecuted acceptance criteria or relabel failed assertions as unavailable. Unchecked boxes below are a reusable reviewer checklist, not execution evidence. Record actual PASS/SKIP/FAIL separately in the PR.

The verifier supports `VERIFY_ISSUE_1256_SKIP_FRONTEND_REASON` and `VERIFY_ISSUE_1256_SKIP_BROWSER_REASON` only for truly unavailable execution capabilities; set a nonempty explanation. They report SKIP separately. They are not application runtime flags and do not alter wallet, tax, or impact safety checks.

## Acceptance and review checklist

- [ ] **AC1 / T1 / T11:** Default native LUNC↔listed wrap hub: receive is a decimal input; typing a legal amount reverse-quotes pay after debounce and URL replaces `exactAmount` / `exactField=output`. Check `SwapPage.queryParams.test.tsx` and the browser spec. Rapid typing uses the final amount without history spam.
- [ ] **AC2 / T2 / A5:** Direct factory CW20 reverse remains pair `reverseSimulateSwap`; inspect/assert submit offer raw equals reverse pay raw, never the typed ask. Run the RTL direct-pair cases.
- [ ] **AC3 / T3–T4 / A3–A4:** Mapper-only wrap and unwrap use shared integer inverse math. Run `router.reverse.test.ts`: wrap mint covers typed CW20; unwrap typed native is post-fee post-tax; forward inverse differs by at most one raw unit. Include fee/tax cap and rounding fixtures; wrap input never burn-taxed (**W8**/**W9**).
- [ ] **AC4 / T5–T6:** Client multi-hop and native routes reverse through the same client operations, gross wrap input / unwrap ask appropriately, then submit offer-in. Inspect existing forward execution for unchanged hop floors and post-fee pre-tax `routerMinReceiveBase`.
- [ ] **AC5 / T10:** No client operations: output is silently ignored as reverse source. Assert no reverse LCD or indexer exact-out/brute-force request, and safe forward fallback. No lecture banner.
- [ ] **AC6 / T7–T9 / A1 / A6:** Clear omits exact amount/field; flip preserves reverse direction; Max/25/50/75 become pay-sided. Run RTL and #678 acquire guards: quote-only, insufficient balance, impact/Expert, blacklist, pause, and freeze still block execution.
- [ ] **AC7:** Community-tax receive targets net; `min_return` / `executeAmountOut` stay pre-tax. Run `communityTaxNetOut.test.ts` and tax-aware RTL/service fixtures; inspect extra-debit accounting occurs once.
- [ ] **AC8 / T12:** `make verify-issue-1256`, `make verify-issue-713`, `make verify-issue-678`, `make verify-issue-512`, chrome nesting, and frontend lint pass. Record each command independently and rerun after fixes.
- [ ] **AC9:** Review diff: no extra chrome layer/lecture banner, indexer endpoint/schema, contract migration, nginx/Coolify change, or amount `localStorage` persistence. UST1 remains AMM. Documentation and agent crosslinks resolve.
- [ ] **A2 / A7:** Overlong (25+ chars), nonpositive, nondecimal, hostile URL amounts are ignored safely. Run parser/RTL cases; no raw HTML insertion; ignored `slippage` / `expertMode` / `recipient` / `pool_only` / `hybrid_optimize` URL keys stay ignored.

## Optional live/manual check

Start with `make has-localterra`; if needed, provision with `make setup-cloud-localterra`, then `make dev`. Use a currently listed wrap hub for the selected chain.

1. Open `/`, type a small legal receive amount for LUNC↔wrapped hub. Pay updates, receive remains editable, and the URL contains `exactField=output`.
2. Flip, clear, then use Max/fractions. Observe output direction on flip, omitted URL amount after clear, and input direction for balance buttons.
3. Repeat direct wrap, direct unwrap, and a listed CW20/CW20 pair. Compare forward quote against the typed net receive; unwrap fee/tax copy stays truthful.
4. Remain disconnected: Connect is required, typing never broadcasts. With a test wallet, inspect the offer and min-receive payload; only authorize a transaction if the test account and live-chain scope permit it.
5. Try a hostile or overlong amount URL. It is ignored and existing impact/pause/freeze/blacklist guards still apply.

Record this as live/manual evidence only when executed. A missing wallet/device/chain can be SKIP with reason under the review policy above. This feature requires no production operator action or migration.
