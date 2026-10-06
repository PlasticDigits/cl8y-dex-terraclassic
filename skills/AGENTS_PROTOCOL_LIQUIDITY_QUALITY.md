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
  numeric totals must disclose lower-bound quality. Zero requires validated current
  reserves or fresh complete empty-factory enumeration. No server smoothing/last-good peg. The frontend retains the last qualified
  complete public total as display-only history, following the explicit 2026-10-06
  user instruction; do not erase it at TTL expiry or during an outage.
- Frontend retention preserves original as-of and stores only value, qualified
  quality and time, scoped to expected chain/factory/indexer. Reject malformed,
  future or contradictory cached records. New qualified decreases and zero replace
  older totals; partial never overwrites a complete total. Suppress 24h delta when
  current quality is unqualified. Explain last verified time only in optional info;
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
  solely `legacy_unqualified: 1`. Keep canonical totals null; repaired,
  unreconciled, zero and missing values remain gaps. Estimates never feed deltas.
- Reserve work is bounded: ≤8 jobs, 4s pair deadline, 2s height probe, ≤1000 orders
  per side, no overlapping cadence cycles. Failed jobs retain old amounts and time.
  Oversized contract bodies must not poison LCD-wide health.
  Reserve/book publication stays atomic so an old book cannot be labeled fresh.
- Run `make verify-issue-1380` and relevant 569/689/655/664/1263 regressions. An agent
  may report overall PASS with explicit unavailable skips under the QA policy; never
  convert an executed assertion failure or unperformed live check into passed evidence.
