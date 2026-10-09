# Protocol liquidity coverage quality (#1380)

Use for protocol TVL valuation, reserve cadence, overview, liquidity history,
percentage baselines, migration repair and their frontend consumers.
Issue: [#1380](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1380).
Implementation: [PR #1382](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1382).
Invariants: [Q1380](../docs/indexer-invariants.md#protocol-liquidity-quality-1380).
Review/run instructions and skip policy: [QA](../docs/qa/issue-1380/README.md).
Evidence/PR: [verification](../docs/qa/issue-1380/verification.md).
Related pricing/layout: [Protocol stats](AGENTS_FRONTEND_PROTOCOL_STATS.md).

- Census full factory inventory before filtering reserves. Supported identities
  are configured contract/denom pins; symbols never add a pricing handle. Unsupported
  pairs do not block supported-catalog completeness. A failed first enumeration
  leaving populated rows is still unknown inventory without completion provenance. Missing supported inputs do.
- Do not turn omitted-all into current zero. Canonical total is nullable; partial
  numeric totals retain lower-bound quality internally. Zero requires validated current
  reserves or fresh complete empty-factory enumeration. No server smoothing/last-good peg. The frontend retains the last qualified
  complete public total as display-only history, following the explicit 2026-10-06
  user instruction; do not erase it at TTL expiry or during an outage.
- Frontend retention preserves original as-of and stores only value, qualified
  quality and time, scoped to expected chain/factory/indexer. Reject malformed,
  future or contradictory cached records. New qualified decreases and zero replace
  older totals; partial never overwrites a complete total. Before a complete total
  exists, retain the last qualified partial amount and its original
  time through expiry/errors/reload; newer lower partial values replace it.
  Suppress 24h delta when
  current quality is unqualified. Retail headlines show only an ordinary USD amount;
  no `≥` prefix, quality/age text or info control. Observation time and partial
  quality stay internal, following the explicit 2026-10-07 user direction;
  never relabel retained data fresh or feed it into server baselines/history.
- Lock before input reads and use atomic generation publication. Do not separately
  write headline, stamps, history or grains. Reject older publications. Preserve
  original as-of and qualify cached responses; unrelated volume work is no heartbeat.
- Both supported legs require available prices. Approved 2× is only for a genuinely
  one-sided catalog pool, not a temporary quote outage. Humanize decimals, reject
  overflow, exclude book escrow, retain existing hub source floors and solver TTL.
- Source slots and all UTC grains keep terminal attempt quality. Do not skip an
  invalid close or select a healthy earlier sample as a baseline. Match identity,
  not counts. Preserve monthly history before pruning ≥35-day source samples.
- Repair only fixed documented public UTC windows, preserving original source and
  derived rows. Historical provenance absent means unknown, never today's value.
  Legacy values are unqualified, and a skip is not reconstructed chain evidence.
  A strictly positive preserved legacy amount may be a separate
  `reported_liquidity_usd` historical estimate only with unknown quality and
  solely `legacy_unqualified: 1`. Keep canonical totals null. The retail chart
  displays valid canonical amounts (including partial observations), falling back
  to valid reported amounts, all in the same normal color. Omit rows without any
  valid amount; no hatching, gray quality categories or empty outlined boxes.
  Keep each observation's original date and show only date and amount in tooltips.
  Reported amounts never feed deltas or overwrite server canonical totals.
  These presentation rules follow the explicit 2026-10-07 user direction and
  supersede previous frontend quality-gap/estimate styling instructions.
- Reserve work is bounded: ≤8 pool jobs and ≤4 book jobs, 4s pair deadline,
  4s height probe ([#1453](AGENTS_LIQUIDITY_FEED.md) supersedes the former
  2s head limit), ≤1000 orders
  per side, no overlapping cadence cycles. Failed jobs retain old amounts and time.
  Oversized contract bodies must not poison LCD-wide health.
  Hybrid reserve/book publication stays atomic so an old book cannot be labeled
  fresh. Under [LF1453](AGENTS_LIQUIDITY_FEED.md), validated pool-only observations
  publish independently and survive unrelated book failures.
- Run `make verify-issue-1380` and relevant 569/689/655/664/1263 regressions. An agent
  may report overall PASS with explicit unavailable skips under the QA policy; never
  convert an executed assertion failure or unperformed live check into passed evidence.

For independent pool observation and separate compatible hybrid generations,
apply [LF1453](AGENTS_LIQUIDITY_FEED.md) and run `make verify-issue-1453`.
A successful pool read survives unrelated book failures; the atomic hybrid reserve,
fee and book snapshot remains separate. Failed observations retain original time.
This refines reserve ingestion while preserving Q1380 valuation/history semantics.
