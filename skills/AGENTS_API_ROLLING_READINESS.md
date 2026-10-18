# API rolling readiness

Apply to API startup, standby/leader overlap, container HEALTHCHECK or rolling
promotion. Preserve [NR1505](AGENTS_NATIVE_QUOTE_RECOVERY.md),
[LQ1452](AGENTS_LOCAL_SWAP_QUOTES.md) and [DB1396](AGENTS_INDEXER_DATABASE_ISOLATION.md).

| ID | Invariant |
| --- | --- |
| AR1 | `/health` remains cheap liveness. `/ready` remains indexing/leadership readiness. `/ready-api` dynamically certifies the configured bounded API-usefulness sample; it never claims all markets or future provider availability. |
| AR2 | `API_READY_SCENARIOS` explicitly declares one or two native exact-input scenarios. Bind checksummed contract identities and canonical positive raw u128 amounts, never symbols. Missing/malformed/unknown/actor fields fail closed with503; there is no permissive deployment default. |
| AR3 | Readiness consumes the existing local solver and its current-generation executable-cache validator. Preserve original observations, references, protection policy, positive per-hop/final floors and30-minute signing expiry. It cannot poll LCD, refresh observations, create provider work or renew a context. |
| AR4 | One no-queue permit, a500ms handler deadline and a200ms O(1) schema read bound work. Both reviewed67 and current68 serving capabilities are allowed through the canonical database classifier; unknown markers fail. Global HTTP admission still applies. Export only ready, a fixed reason and scenario_count, with no-store. |
| AR5 | Image HEALTHCHECK uses `/health/serving`: before first admission it requires the complete dynamic proof, then a process-owned atomic latch keeps cheap liveness. The latch resets on process start and never stores/authorizes/renews a quote. Dynamic `/ready-api` and actual quotes still fail closed on later observation loss. This prevents Traefik from unrouting the sole API on transient quote loss. Never use a file latch. Keep old serving until initial proof passes; do not rely on cancellation as a post-start stop-old barrier. |
| AR6 | No future maintenance may take the serving API down. Prove actual proxy health filtering, old-preservation on failed health, standby quote warmup and handoff before production. A schema/HTTP fixture, source inspection or skipped live check cannot establish a production rollout. |

Run `make verify-api-rolling-readiness` with an isolated private QA env. Compilation
is separate; each executed test process has a59-second outer bound. The controlled
HTTP/lock rehearsal uses synthetic source observations and production scheduled
API producers. Its lifecycle progress after real advisory-lock exchange is an
explicit fixture; complete old/new-process and proxy promotion evidence is separate.
See the [checklist and configuration](../docs/qa/api-rolling-readiness/README.md).

For release eligibility, use the GET-only
[`api-rolling-eligibility.py`](../scripts/ops/api-rolling-eligibility.py) and its
[manifest/proof contract](../docs/qa/api-rolling-readiness/README.md). An inventory
receipt is never deployment permission. Exact remote-main/source proof is required
before mutations; historical source verification is allowed only for read-only
status of an already dispatched release. Never include credentials, raw env values,
logs or actor identifiers in the public inventory receipt.
