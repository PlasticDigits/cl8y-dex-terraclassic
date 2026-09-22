# QA — indexer HTTP pack (#1204)

Docs + OpenAPI completeness. No new routes.

| Check | Command |
|-------|---------|
| Pack + `ApiDoc` drift | `make verify-issue-1204` |
| Served spec (Postgres) | `cd indexer && cargo test --test security openapi_spec_available -- --test-threads=1` |

Invariants **I1204-1–I1204-8**. Skill [`AGENTS_INDEXER_HTTP_PACK.md`](../../../skills/AGENTS_INDEXER_HTTP_PACK.md). Pack [`docs/indexer-http.md`](../../indexer-http.md).

Live `curl` against a running indexer is optional. Skip when port 3001 is down. Do not loop LCD-heavy routes.
