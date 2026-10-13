# Browser case lifecycle and bounded request diagnostics

For [#1489](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1489) and
[the #1483 explanation checklist](../issue-1483/README.md), acceptance still
requires independently attested exact deployed source, original useful-result
clocks, original expiry, exact raw DOM adoption and fixed request limits.

The original actual checks on merged source
`8500370c2d4475accca879064ad61fe186ccfd18` retained FAIL verdicts. Discovery
recorded eight useful initial/25-second samples within 750 ms, then denied reads
at its 600-read cap before four 185-second samples could complete. Its counter
reached 699 attempted reads. Explanation recorded 15 useful cases before its
100-read cap denied later requests; its counter reached 144 attempts and its
later timeout overwrote the earlier specific ReadBudget class. These counters
include denied attempts; actual admitted reads remained capped. They do not
establish a production latency failure without the test's network denial.

The [discovery tool](../../../scripts/qa/browser-route-discovery-1489.cjs) now
clears each amount only after recording the complete original 3.5-second
adoption observation. This stops unintended positive-amount polling between
cases. Four pages, twelve samples, 0/25/185-second phase deadlines, the 750 ms
input-to-useful-DOM limit and 600-read cap are unchanged.

The [explanation tool](../../../scripts/qa/browser-route-attribution-1483.cjs)
retains its existing single page per viewport and navigation between entries.
Navigation already unloads previous entries; no multi-page leak was established.
It preserves the first specific failure class when a denied read induces a
later timeout. Eighteen cases, the 750 ms original input clock, 100 read/quote
limits and 180-second elapsed limit are unchanged. Actual full-corpus budget fit
remains unchecked until a new independent deployed first/retest completes.

Both tools report only fixed request families with attempted and budget-admitted
counts. No actor, arbitrary URL, provider prose or raw response enters these
diagnostics. [Shared lifecycle and budget controls](../../../scripts/qa/test-browser-case-lifecycle.cjs)
cover observation-before-clear ordering, failed observations, exact cap refusal,
first-failure retention and bounded private-path-free families. Their
[helper](../../../scripts/qa/browser-case-lifecycle.cjs) is checked against the
expected committed source before live execution and included in stable-source
verifier fingerprints.

- [x] Original failures retained; their induced unavailability is explicit.
- [x] Helper first/retest controls passed without raising caps or changing clocks.
- [ ] Corrected discovery actual deployed first/retest.
- [ ] Explanation actual complete 18-case first/retest within its original cap.
- [ ] Production settlement and physical signing acceptance remain separate.

See the [#1489 checklist](README.md),
[RD1489](../../../skills/AGENTS_ROUTE_DISCOVERY.md) and
[RC1483](../../../skills/AGENTS_ROUTE_COST_ATTRIBUTION.md).

Combined implementation and review: [PR #1494](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1494).
