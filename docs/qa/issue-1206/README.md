# Anonymous UTC-day digest (#1206)

Issue/body/comments: [#1206](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1206).
Agent invariants: [AD1206](../../../skills/AGENTS_ACTIVITY_DIGEST.md).
Implementation: [handler](../../../indexer/src/api/evidence_digest.rs),
[queries](../../../indexer/src/db/queries/evidence_digest.rs),
[HTTP/Postgres tests](../../../indexer/tests/api_evidence_digest.rs).

`GET /api/v1/evidence/digest?day=2026-06-10` is a protocol-grain anonymous aggregate.
It includes indexed gem markets and is **not L639 listing-safe**. Wrap and window
counts are fee-event counts, never principal. No actors, transaction linkage,
per-pair census or unique-trader count. Asset contracts may appear only under
`fees.by_token`; symbols are deliberately omitted because catalog text is untrusted.
This is separate from `/evidence/daily` (chain-linkable tape), `/defillama/daily`
(listing accounting) and `/protocol/fees` (trailing materialized fee mix).

The original issue predates #1209/#1210: `FeeSource::ALL` now contains **ten**
paths. This implementation preserves its first seven in order and appends
pair_creation, sku_unlock and settings_fee. All ten participate in fee totals
and token mix, with no fabricated activity surfaces for the appended sources.
Source/token USD is null if any contributing activity is unpriced; the headline
still sums all priced rows. Such unknown shares need not sum to 100%.

| Criteria | Evidence |
| --- | --- |
| AC1, AC6 | Canonical current ten-source response, original seven prefix; idle zeros |
| AC2–AC5, AC7 | Isolated DB fixture: hybrid parent, book fill, LP, limit, wrap/window and adjacent-day spillover |
| AC8, AC14 | Actor/transaction stringify guards and SQL dependency unit guard |
| AC9 | Missing/malformed/future/injected day; duplicate/unknown/filter/CSV parameters return sanitized 400 |
| AC10 | Empty 200, today incomplete, past complete |
| AC11 | Real router OpenAPI path; no mutation of existing fee/listing handlers |
| AC12 | Bound timestamps/source array, sanitized closed-pool 500 |
| AC13 | Shared evidence admission plus global peer governor, outside LCD-heavy routes |
| AC15 | Plain digit strings, unpriced source null, priced headline preserved |

Run `make verify-issue-1206` after compiling the focused targets. The script
separates compilation from actual tests and caps each test invocation at 59s.
Use the dedicated local QA capability (`indexer/.env.qa`) and a disposable child DB.
Reset only that owned child through the existing harness after a failed run;
never reset shared or production infrastructure. Executed failures remain FAIL.
Live API/OpenAPI, production query timing and deployment identity need separate
receipts. An unavailable deployment is not PASS and does not permit issue closure.

The bounded-size fixture inserts 10,000 parent swaps and ten fee assets. It
requires one small digest object, eight token rows plus other, partial-source/token
nulls and an intact priced headline. Midnight fixtures distinguish prior 23:30Z
from selected-day 00:30Z and exclude the next midnight exactly.

The [dated live release](../issues-1206-1512/live-release-20261018.md) and
[14-request API receipt](../issues-1206-1512/live-api-20261018.json) now establish
exact deployed identity, digest/OpenAPI and observed socket timing separately
from the seeded source/SQL checks. Required body/comment acceptance is complete.
