# API rolling readiness

The image now waits for useful cached native execution before a rolling promotion.
Its `/health/serving` gate latches startup admission in process memory only. After
that proof it checks cheap liveness, preserving proxy routing during temporary
quote/provider loss. The latch resets on every process start and is never a
quote authority; no observations, floors or expiry are renewed. `/ready-api`
remains dynamic and returns503 when cached quotes or database readiness are lost.
`/health` and `/ready` retain their liveness and indexing meanings. A standby may
answer `/ready`503 while its independently scheduled API producers make
`/ready-api`200. Existing quote freshness, trust, economics and signing rules apply.

Set the runtime-only `API_READY_SCENARIOS` environment variable before deploying
this image, including local Docker deployments. It is a JSON array of one or two
objects containing `token_in`, `token_out`, `amount_in` and optional `max_spread`.
Use exact lowercase checksummed Terra CW20 contracts or the engine's supported
native denoms (`uluna`, `uusd`); at least one side must be native and sides differ.
`amount_in` is a canonical positive raw u128 integer string. Spread defaults to
`0.01` and must use `0.` followed by one to six digits, positive and no greater
than`0.30`. The gate uses the existing `quote_relative_v1` policy and bounded hybrid
search. Configured scenarios must require actual pool/router execution; direct
wrapper-only arithmetic and indicative/topology results cannot pass.

Example shape (replace the placeholder with a reviewed checksummed identity;
this placeholder deliberately does not pass validation):

```json
[{"token_in":"uusd","token_out":"<reviewed-CW20-contract>","amount_in":"1000000","max_spread":"0.01"}]
```

Production profiles must reproduce the reviewed native probe's exact identities,
direction, raw amount and spread. Generic deployments choose their own explicit
profile; this feature introduces no default financial identity. All modes require
a valid explicit profile to pass. Missing/invalid profiles return503 while the
server's liveness remains available, allowing a failed replacement to leave the
old instance serving.

The endpoint performs at most one bounded schema read and two local solves.
It reuses the engine's generation/cache eligibility checks, then checks the
public executable shape and original positive floors. Thirty-minute reviewed
signing validity remains unchanged; the gate does not invent a twenty-second
signing deadline. Readiness cannot trigger chain/provider reads. The response
contains only `ready`, a fixed `reason` (or null), and `scenario_count`; it is
`Cache-Control: no-store`. One no-queue permit limits concurrent work, with a500ms
handler budget and a200ms database read budget under the ordinary global governor.

The image healthcheck calls `/health/serving` and has a180-second startup allowance,10-second interval and
12 retries. Coolify must use its proven standalone start-new → health → stop-old
path. Current production settings were inspected read-only: StandaloneDocker,
no host port mappings/custom IP/consistent name, with Dockerfile health enabled.
Before startup admission, health failure must preserve the old process. Actual
Traefik Docker routing removes unhealthy backends even though the process remains
alive; therefore dynamic quote readiness cannot be a perpetual Docker healthcheck.
Coolify4.4.6 records unhealthy status; its restart limiter reacts to actual crash
restart counts, not quote readiness. No external autoheal deployment is asserted.
Cancellation after rolling starts
is not an independently safe promotion barrier. Source/settings inspection alone
is not an actual proxy/rollout proof.

Verification checklist:

- [x] Focused profile/executable-contract unit controls pass (2/2, 0.00s).
- [x] Controlled owned HTTP rehearsal (6.48s): cold503 does not contact LCD; production scheduled
  producers warm explicit native quotes while an old advisory leader is held.
- [x] `/ready` stays503 while `/ready-api` passes; repeated readiness/quote GETs
  add zero provider calls after scheduled producers pause.
- [x] Real dedicated sessions hand over the production advisory lock; cached
  native requests continue. Synthetic lifecycle progress in this fixture is
  distinguished from the full production-poller handoff.
- [x] After source observation loss, dynamic readiness and actual quotes fail
  closed while the admitted startup health stays200; a fresh gate resets and
  refuses the same missing data.
- [x] Wrong schema rejects dynamic readiness; missing/invalid scenarios remain fail closed.
- [ ] Owned actual old/new-process67→68 compatibility, proxy health filtering and
  failed-replacement preservation pass before any production change.
- [ ] After promotion, native original-clock/freshness/live checks pass. No
  production acceptance is claimed by this source implementation.

Commands: `make verify-api-rolling-readiness`. Invariants:
[AR1–AR6](../../../skills/AGENTS_API_ROLLING_READINESS.md).

Retained first controls: initial compile passed1m45s; profile/executable contract
units2/2 passed0.00s. The first HTTP fixture failed32.78s with
`database_unavailable`: generic SQLx test setup did not prepare the standalone
schema marker. The fixture now invokes the production `migrate_and_grant` on its
owned child DB; no history/marker is invented. That failure is not relabelled.
A second fixture run failed0.56s because the thread-owned QA cluster lacked its
standard unprivileged NOLOGIN runtime role. Only that local fixture role was
provisioned after container/port identity checks. Final latch units passed2/2 in
0.00s and the controlled HTTP/native/lock/source-loss/reset rehearsal passed in
6.48s. Each execution had a59s outer bound; compilation was separate. The
stronger independent HTTP/state/block-processing/proxy forwarding rehearsal is
separate acceptance; no OS-process/Coolify deployment proof is asserted here.


Read-only release eligibility is implemented by
[`api-rolling-eligibility.py`](../../../scripts/ops/api-rolling-eligibility.py).
`inventory` performs seven bounded GETs and is always `INVENTORY_ONLY`; it never
permits a rollout. The [retained production inventory](eligibility-inventory-20261018.json)
observed the restored old API healthy, with the reviewed rolling settings and no
active deployment. Its three profile checks failed because the explicit runtime
profile was absent. No environment, source pin or deployment changed.

`inspect` requires exact remote main, its archived compiled-input manifest, a
reviewed public profile and a private0600 owned proof. It checks the executable
Docker startup health instruction, not merely an endpoint mentioned in a comment.
The shared pure `application_manifest(root)` / `manifest_from_files(files)` contract
covers all indexer source/migrations, Cargo manifests, Dockerfile and optional
project compiler configuration/build scripts. It excludes env files, test data
and target caches; it refuses symlinks. Final proofs must bind the final frozen
compiled source, including later changes. `revision_manifest(revision)` is strict
by default; `exact_main=False` skips only the remote-main comparison so read-only
status can inspect an already dispatched release after main advances.

Owned proof contract: `schema=api_rolling_eligibility_proof_v1`, `overall=PASS`,
`source_manifest` equals the full compiled-input descriptor, and `controls` contains
each exact key in `PROOF_CONTROLS` with boolean true. `measurements` contains integer
(not boolean) `proxy_requests>=10`, `native_quote_successes>=4`, `http_failures=0`,
`request_lcd_calls=0` and `parsed_blocks_after_handoff>=1`. The exact scope is
`owned independent HTTP/state, production block indexing and proxy forwarding; not OS-process or actual Coolify deployment`.
These observations do not claim an actual Coolify rollout or continuous future
native availability. Nine focused offline controls pass; no production mutation
or extra application test run is involved in this eligibility implementation.
