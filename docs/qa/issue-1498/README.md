# Issue #1498: stable Swap layout

Issue: [#1498](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1498).
Apply [SL1498](../../../skills/AGENTS_SWAP_LAYOUT.md). Run `make verify-issue-1498`
and the related `1480`, `1473`, `1452`, `1483` and `1489` checks on stable source.
The [joint release results](../trading-1497-1500/RESULTS.md) record actual evidence,
retained failures and remaining scope. PASS WITH SKIPS never completes an
unchecked acceptance criterion. Executed failures remain FAIL until fixed/retested.

- [ ] AC1: Header/inputs/summary/status/DETAILS/CTA order; quote changes cannot move inputs.
- [ ] AC2: One Slippage/Fee/actual token route row; equal-height loading replacement; bounded horizontal scroll.
- [ ] AC3: Fixed status envelope; truthful check/None, waits and cross/current failures; keyboard and zoom access.
- [ ] AC4: One persistent DETAILS, one network fee and current checked minimum including anonymous quotes.
- [ ] AC5: Settings/Share/pickers/edits/flip/deadline/keyboard/CTA functional and aligned.
- [ ] AC6: Route/minimum/costs/signing belong to the same validated original cohort; all safeguards preserved.

Source tests include Swap page/acquire/query tests, summary/component tests and
quote/signing safeguards. Production-build geometry uses the owned deterministic
HTTP fixture; it proves rendering under that fixture, not live economics. Live
browser/API/deployment receipts and funded/physical-device checks remain distinct.

For focused geometry, use:

```bash
LOCAL_QUOTE_BROWSER_GEOMETRY_ONLY=1 \
LOCAL_QUOTE_BROWSER_RECEIPT=/tmp/swap-layout-1498.json \
bash scripts/with-node.sh -- node scripts/qa/browser-local-quotes-1452.cjs
```

First/retest final source. Reset only the owned browser/DB/chain fixtures after a
failure. Publish only sanitized bounded receipts; never credentials, wallet/session
identities, private endpoints, raw deployment/provider logs or reporter details.

Merged implementation: [PR #1501](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1501).
Exact deployment and subsequent live receipts: [joint release results](../trading-1497-1500/RESULTS.md#merged-release-and-deployment).
