# Agent playbook: UTC-day evidence accounting (#1205)

**Privacy/cursor/budget guidance superseded by #1397:** use
[AGENTS_EVIDENCE_EXPORT_PRIVACY.md](AGENTS_EVIDENCE_EXPORT_PRIVACY.md) and
`make verify-issue-1397`. Schema 2 emits no actor identifiers or hashes and
explicitly retains public-chain transaction linkage. It does not provide anonymity.

Audience: agents adding ingest, new HTTP routes, or sharing protocol activity with external researchers.

**Issue:** [Forgejo **#1205**](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1205)

**Endpoint:** `GET /api/v1/evidence/daily?day=YYYY-MM-DD` (+ optional `surface=`, `limit`, `cursor`)

**Verify:** `make verify-issue-1205`

## Invariants (E1205-1–E1205-8)

| ID | Rule |
|----|------|
| **E1205-1** | One UTC calendar day per request (`[00:00Z, +1d)`). No multi-day range. |
| **E1205-2** | Surfaces: `swap` \| `wrap` \| `limit` \| `lp` only. A single `surface=swap`, comma `surface=swap,lp`, and repeated `surface=swap&surface=lp` all parse. L10: limit fills are **not** `surface=swap`. |
| **E1205-3** | **Superseded by E1397-1–E1397-3:** omit actor identifiers and digests entirely. The old unsalted `actor_hash` was dictionary-reversible and did not hide public transaction linkage; never restore it. |
| **E1205-4** | Wrap = `protocol_fee_events` `source` `wrap` \| `unwrap` only (fee amount, not principal). |
| **E1205-5** | Postgres-only request path — not on `lcd_heavy_router`; use the dedicated admission/DB/HTTP budgets in E1397-6–E1397-8, not a global governor alone. Keyset pagination (`limit` 1–1000, default 500). Schema-2 cursors are day/surface scoped, bounded `e2.` values per E1397-5; old `e1.` values return 400. Cursor sort is `(block_height, tx_hash, surface, kind, ordinal, row_id)`. `row_id` is the source primary key (not in JSON) so two pair swaps in one tx that share `swap_index` are not dropped on the next page. Every UNION arm aliases the same columns — a `surface=` filter that omits swap must still decode. |
| **E1205-6** | No `format=csv`, no `?redact=0`, no API keys. Pair/token contracts stay; gems included (not L639 listing-safe). |
| **E1205-7** | Do not change `/gt/events`, trader tapes, or DeFiLlama daily aggregates for redaction. |
| **E1205-8** | OpenAPI tag **Evidence**; skill + `docs/indexer-invariants.md` + `make verify-issue-1205`. |

## Related

- Unredacted incident SQL: [`docs/runbooks/suspicious-activity-queries.md`](../docs/runbooks/suspicious-activity-queries.md)
- Docs-only OpenAPI pack: [#1204](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1204) — no new business routes there
- DeFiLlama UTC aggregates: [#631](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/631)

## Don’t

- Add `wrap_events` ingest or wrap principal on this ticket.
- Reuse GeckoTerminal `maker` or 400-over-cap semantics on this route.
- Point suspicious-activity runbook readers at this export for wallet-level triage (use leaderboard / SQL).
