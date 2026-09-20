# ADR 0006: Indexer `/health` git SHA and protected-main auto-deploy

## Status

Proposed ([#1276](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1276))

Design record for implement. This ADR does **not** flip Coolify, deploy, scrape Coolify logs, publish app UUIDs or tokens, change CAC `/health`, or expand CAC `COOLIFY_APP_MAP`. Enabling the indexer Coolify protected-branch auto-deploy checkbox is operator leftover under [agent-control #297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297) (deploy policy). In-repo work stamps SHA and documents the flag; it cannot close the checkbox.

Playbook: [`skills/AGENTS_INDEXER_HEALTH_GIT_SHA.md`](../../skills/AGENTS_INDEXER_HEALTH_GIT_SHA.md) (**H1276-1–H1276-8**). Overview: [`architecture.md`](../architecture.md#indexer-production-attest). Invariant: [`indexer-invariants.md`](../indexer-invariants.md) **Health git SHA (#1276)**. Rollback: [`runbooks/rollback-decision.md`](../runbooks/rollback-decision.md) § Auto-deploy era.

ADR **0005** is reserved by the #1269 hop-fee design branch (`docs/adr/0005-protocol-fee-multihop-hops.md`). This ticket is **0006**.

## Outcome

1. **Attest.** Unauthenticated `GET /health` stays liveness. JSON is `{"status":"ok"}` plus optional `"git_sha"` when a baked env value parses as lowercase hex length 7–40. Operators compare that field to `git rev-parse HEAD` on the serving merge (prefix OK). They do **not** scrape Coolify `SOURCE SHA` log lines. Regex-only hex on live `/health` is **bake presence**, not “follows `main`.” Leftover-complete requires the Coolify checkbox **on** (operator / #297; never inferred from HTTP) **and** tip-match after an indexer-touching land (`VERIFY1276_EXPECT_SHA`).
2. **Bake.** Production image is [`docker/indexer/Dockerfile`](../../docker/indexer/Dockerfile) (not Nixpacks). Re-declare `ARG GIT_SHA` / `ARG SOURCE_COMMIT` and export them as runtime `ENV` on the **`runtime` stage after `COPY --from=builder`** (after apt/`useradd`/COPY/USER/`API_BIND`; **before** `HEALTHCHECK`). Do **not** place ARG immediately after `FROM debian:bookworm-slim AS runtime` — that busts the `apt-get` layer on every auto-deploy commit. ARG does not cross `FROM`; stamping only the builder leaves the serving process with empty/unset vars. The image still `COPY indexer/` only and does **not** run `git` in either stage. Empty ARG is valid for local `cargo run`. Live leftover bake: set `GIT_SHA` **only** when the value is already hex; leave it unset/empty when the Coolify UI token is `HEAD` or a branch name — **never** copy that token into `GIT_SHA`. `SOURCE_COMMIT` (include-source-commit) is then the candidate.
3. **Auto-deploy (documented intent, operator leftover).** The indexer Coolify app for this Forgejo path should follow protected `main` like the frontend app. CAC grouped drain maps **one** UUID per `owner/repo` and is **not** the indexer redeploy path. Dual-app skew (Vite vs Rust rebuild) is expected. Schema-on-boot stays `sqlx::migrate!()`; rollback after a landed migration without `down.sql` is **forward-fix only**.

Implement may merge slices 1–2 while the Coolify checkbox is still off. Closing #1276 leftover glance waits on operator evidence, not on implement agents. Implement **must not** flip Coolify.

## Context

Protected-branch land already rebuilds the **frontend** Coolify application. The **indexer** application for the same git source has auto-deploy off, so indexer schema/API landings stay off `indexer.dex.cl8y.com` until a manual deploy. `health()` returns only `{"status":"ok"}`. Docker `HEALTHCHECK` and most scripts only require HTTP 200.

Today’s invariant row says generic `GET /health` stays `{"status":"ok"}` **only**. [`indexer/tests/api_fee_discount_health.rs`](../../indexer/tests/api_fee_discount_health.rs) `generic_health_unchanged` asserts exact object equality. An additive field is compatible with HEALTHCHECK **after** that invariant and test change. `Config::from_env` calls `dotenvy::dotenv()`; CI and Coolify-like shells often already have `SOURCE_COMMIT`. Ok-only exact-object tests are flaky unless they `remove_var` **both** keys before the request. The handler must read env **per request** (not at router build).

Coolify is known to store the literal `HEAD` token in commit fields. Mapping `git_commit_sha=HEAD` (or a branch name) into runtime/build-arg `GIT_SHA` makes select reject and **not** fall through to `SOURCE_COMMIT` — leftover HTTP stays omitted. Include-source-commit injects **`SOURCE_COMMIT`** as a hex build-arg; that path only works when `GIT_SHA` is unset or empty after trim.

The Dockerfile is **multi-stage**: builder `COPY indexer/` then a separate `debian:bookworm-slim` runtime. It has no commit ARG/ENV today and cannot `git rev-parse` at build. Dockerfile `ARG GIT_SHA=` + `ENV GIT_SHA=${GIT_SHA}` materializes **`GIT_SHA=""`** at runtime (set-but-empty), not unset. A parser that treats “first set value” without a non-empty trim check would skip `SOURCE_COMMIT` and omit `git_sha` on the leftover path. ARG immediately after `FROM runtime` would also bust `apt-get` on every commit SHA change.

The binary always runs `sqlx::migrate!()` before bind ([`indexer/src/main.rs`](../../indexer/src/main.rs)). Almost all migrations have no `down.sql` (three files under [`indexer/migrations/revert/`](../../indexer/migrations/revert/)). Post-merge playbooks still treat indexer as a gated Coolify redeploy after migrate, separate from the frontend rebuild. Auto-deploy without a forward-fix/rollback rule and a skew window is not supportable.

## Non-goals

- CAC public `/health` SHA, occupancy, or SOURCE stamp ([agent-control #348](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/348) / leftover [#352](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/352)).
- CAC `drain_deploy_url` / `app_uuid_for_path` two-UUID map. If CAC must deploy two apps per path, that is a **separate** agent-control issue. Do not block indexer auto-deploy on it.
- `GET /api/v1/health/fee-discount` (LCD probe only — **H1276-4**).
- A second public `/status`, Prometheus `/metrics`, or DB/LCD on generic `/health` (**H1276-1**).
- Fetching Coolify from the indexer process. Scraping Coolify logs. Publishing Coolify tokens, app UUIDs, hosts, or `/status` JSON.
- Nixpacks. Production stays `docker/indexer/Dockerfile`.
- Numeric overflow on volume_aggregator ([#1277](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1277)) — out of scope, not approved.
- Frontend stale Vite chunks ([#706](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/706)) — complementary UX.
- Rewriting historical coupled leftover **M573** / **M590** as if indexer auto-deploy applied retroactively. [`docs/runbooks/mainnet-soft-launch.md`](../runbooks/mainnet-soft-launch.md) “rebuild frontend + restart indexer together” stays the **#573** / [`AGENTS_POST_MERGE_STACK.md`](../../skills/AGENTS_POST_MERGE_STACK.md) note.
- Flipping Coolify UI from this repo, HMAC/`autonomy.rs` self-approval, or a founder card for this ordinary design.
- Inferring the auto-deploy checkbox from HTTP. A stale hex from a one-off manual bake can satisfy regex-only `git_sha` while auto-deploy is still off.

## Decision

### Field name

**`git_sha`.** Not `commit`. Matches `.qa-deploy-stamp` naming. Omit the key entirely when the parser rejects the env (JSON `{"status":"ok"}` with no `git_sha`).

### Env / build-arg (mandatory on the **runtime** stage after `COPY --from=builder`)

| Name | Role |
|------|------|
| **`GIT_SHA`** | Optional Docker `ARG` + runtime `ENV`. Set **only** when the operator already has a hex. Leave unset/empty when Coolify shows `HEAD` or a branch. Never copy that UI token into `GIT_SHA`. |
| **`SOURCE_COMMIT`** | Alias `ARG` + runtime `ENV` so Coolify **include-source-commit** injection works without mapping the UI token. This is the leftover candidate when `GIT_SHA` is unset/empty. |

**Select (H1276-2):** use `GIT_SHA` only if it is **present and non-empty after ASCII trim**; otherwise use `SOURCE_COMMIT`. Empty and **whitespace-only** `GIT_SHA` are omitted at select time — Dockerfile `ARG GIT_SHA=` + `ENV GIT_SHA=${GIT_SHA}` yields `GIT_SHA=""`, which **must** fall through. `GIT_SHA="   "` + valid `SOURCE_COMMIT` **must** fall through (trim), distinct from a whitespace-only **selected** candidate (parser omit).

**No fallthrough** only after a **non-empty** rejected value (`HEAD`, `=`, secret prefix, non-hex after the parser below). A rejected non-empty `GIT_SHA` does **not** mix with a leftover hex in `SOURCE_COMMIT`.

Required leftover case: `GIT_SHA=""` + `SOURCE_COMMIT=<hex>` → `git_sha` **present**. Keep `GIT_SHA=HEAD` + `SOURCE_COMMIT=<hex>` → **omit**. That is why leftover bake must **not** wire Coolify `HEAD` into `GIT_SHA`.

Local `cargo run` / tests: both unset or both empty after trim → omit field.

Implement helpers take `&str` / `Option<&str>` (no process-env dependency in unit tests):

- `select_commit_env(git_sha: Option<&str>, source_commit: Option<&str>) -> Option<&str>`
- `parse_git_sha(raw: &str) -> Option<String>`

The handler reads `std::env` **per request** (`GIT_SHA`, then `SOURCE_COMMIT` via the helpers). Do **not** cache `git_sha` on `AppState` at router build. Do **not** call `dotenvy` from `health()`. Helpers stay pure `&str`.

Dockerfile (implement slice 1). Pin on **`AS runtime` after `COPY --from=builder`**, not the builder and not immediately after `FROM`. Do not copy the frontend Vite ARG-on-builder pattern — Vite bakes at compile time; this binary reads `ENV` at serve time. Do not run `git` in either stage:

```dockerfile
FROM debian:bookworm-slim AS runtime
# apt-get + useradd stay ABOVE commit ARG so auto-deploy SHAs do not bust the apt layer
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates libssl3 curl \
  && rm -rf /var/lib/apt/lists/* \
  && useradd --system --uid 10001 --create-home indexer

COPY --from=builder /build/target/release/cl8y-dex-indexer /usr/local/bin/cl8y-dex-indexer

USER indexer
WORKDIR /app

ENV API_BIND=0.0.0.0 \
    API_PORT=3001 \
    RUST_LOG=info

# ARG/ENV after COPY --from=builder (and USER / WORKDIR / API_BIND), before HEALTHCHECK
ARG GIT_SHA=
ARG SOURCE_COMMIT=
ENV GIT_SHA=${GIT_SHA} \
    SOURCE_COMMIT=${SOURCE_COMMIT}

EXPOSE 3001
HEALTHCHECK --interval=30s --timeout=5s --start-period=40s --retries=3 \
  CMD curl -fsS "http://127.0.0.1:${API_PORT}/health" >/dev/null || exit 1
CMD ["cl8y-dex-indexer"]
```

Coolify leftover bake (operator / #297; implement must not flip UI):

1. Turn **include-source-commit** on so `SOURCE_COMMIT` is a 40-char hex build-arg.
2. Set `GIT_SHA` **only** when the value is already hex. If the UI token is `HEAD` or a branch, leave `GIT_SHA` **unset/empty** — do not copy that token into the build-arg or runtime env.
3. Include-source-commit **alone** is enough **because** empty `GIT_SHA` falls through to `SOURCE_COMMIT`. An extra hex `GIT_SHA` is optional, not “prefer both.” Copying `HEAD` into `GIT_SHA` **and** enabling include-source-commit still omits `git_sha` (**H1276-2**).
4. Optional watch paths (when the UI supports them): `/indexer/**` and `/docker/indexer/**` so frontend-only lands skip the Rust rebuild. If watch paths are unavailable, extra indexer rebuilds on frontend-only `main` lands are accepted **cost**, not a correctness hole.

### Parser (uppercase → normalize; else omit)

Pure function, no I/O. Input: the **selected** candidate string from `select_commit_env`.

1. Trim ASCII whitespace. Empty after trim → omit (select should already have skipped this).
2. If the trimmed value still contains whitespace → omit.
3. If it contains `=` → omit.
4. If it equals `HEAD` case-insensitively → omit.
5. If it starts with a secret prefix → omit: `Bearer `, `bearer `, `sk-`, `ghp_`, `gho_`, `ghu_`, `ghs_`, `glpat-`, `N|` (Sanctum-shaped).
6. Lowercase ASCII. If it does not match `^[0-9a-f]{7,40}$` → omit.
7. Else emit that lowercase hex as `git_sha`.

**Uppercase hex is normalized** (not omitted). `HEAD` / empty / secret-shaped / non-hex is omitted, never echoed.

`/health` stays fast: no Postgres, no LCD, no Coolify client. `status` is always `"ok"` when the process is serving (existing behavior). Docker HEALTHCHECK remains `curl -fsS …/health` HTTP 200.

### Auto-deploy policy (so the leftover checkbox is supportable)

| Rule | Meaning |
|------|---------|
| **Expand-only while auto-deploy is on** | New sqlx migrations that ride auto-deploy must be additive (nullable columns, new tables, new indexes). Breaking rewrites require the operator to **turn auto-deploy off**, apply by [`rollback-decision.md`](../runbooks/rollback-decision.md), then re-enable. |
| **Forward-fix if schema moved** | After boot migrate with **no** paired `down.sql`, do **not** roll the previous binary — schema is ahead and startup will fail. Snapshot Postgres before risky migrations. Ship a hotfix image. |
| **Binary rollback if schema did not move** | Crash/logic bug with unchanged `_sqlx_migrations`: restore the previous Coolify indexer image. Confirm `git_sha` on `/health` matches the restored digest’s commit (prefix OK). |
| **Dual-app skew** | Frontend (Vite) and indexer (Rust) are two apps on one repo. After `main` land they may differ for minutes to tens of minutes. Additive JSON is the default contract. A frontend that **requires** a new indexer field must land **after** live `/health` `git_sha` includes the indexer commit (or auto-deploy is temporarily off and the pair is coupled). Historical **#573** / **M573** “rebuild frontend + restart indexer together” stays that stack’s leftover — do **not** rewrite [`AGENTS_POST_MERGE_STACK.md`](../../skills/AGENTS_POST_MERGE_STACK.md) or **M590** as if auto-deploy applied retroactively. Going forward, couple only for breaking indexer contracts. |
| **CAC drain** | Still one UUID per Forgejo path. Auto-deploy on the indexer app is independent. Do not use drain as the indexer redeploy path. |

## Component / state / interface changes

| Layer | Change |
|-------|--------|
| API | `health()` in [`indexer/src/api/mod.rs`](../../indexer/src/api/mod.rs): per-request `select_commit_env` + `parse_git_sha`. No new route. No `dotenvy` in the handler. |
| Parser | Lib-testable helpers as above — do not inline ad-hoc regex only in the handler; unit tests pass `&str` / `Option<&str>`. |
| Image | `docker/indexer/Dockerfile` **runtime** ARG/ENV **after** `COPY --from=builder`, before `HEALTHCHECK`. HEALTHCHECK unchanged (HTTP 200). |
| Tests | See Tests. `generic_health_unchanged` becomes “ok-only when env unset.” Env-mutating integration tests are `#[serial]` and `remove_var` both keys before ok-only requests. |
| Invariant | Observability unhappy-path no longer “ok-only”; new **Health git SHA (#1276)** row. |
| Runbooks | Indexer auto-deploy era + SHA glance; mainnet-soft-launch Coolify indexer bake-args on the **runtime** stage after COPY. Do not rewrite M573. |
| Coolify | Operator leftover only. Not in git. |
| CAC | Unchanged. |
| dApp | No required chrome. Clients that `JSON.parse` `/health` must tolerate an extra key (already true if they read `status` only). |

## Affected invariants

| ID | Effect |
|----|--------|
| **H1276-1** | Generic `GET /health` is liveness: HTTP 200, `"status":"ok"`, no DB/LCD. Additive `git_sha` only. |
| **H1276-2** | Field name is `git_sha`. Select `GIT_SHA` only when present **and non-empty after trim** (whitespace-only `GIT_SHA` falls through); else `SOURCE_COMMIT`. No fallthrough after a **non-empty** rejected value. Parser is hex 7–40, lowercase; uppercase normalized; `HEAD` / whitespace / `=` / secret prefixes omitted. Never wire Coolify `HEAD`/branch into `GIT_SHA`. |
| **H1276-3** | Dockerfile **must** re-declare `GIT_SHA` + `SOURCE_COMMIT` ARG/ENV on the **runtime** stage **after** `COPY --from=builder` (after apt/`useradd`; **not** immediately after `FROM runtime`) and **before** `HEALTHCHECK`. Image does not run `git` in either stage. Nixpacks is not the prod path. |
| **H1276-4** | `GET /api/v1/health/fee-discount` unchanged. |
| **H1276-5** | No Coolify UUID, token, host, occupancy, or inventory in `/health` or issue comments. |
| **H1276-6** | Auto-deploy is the indexer Coolify protected-branch flag (operator / #297). CAC drain is not the indexer redeploy path. CAC `/health` stays SHA-free. Do not infer the checkbox from HTTP. |
| **H1276-7** | While auto-deploy is on: expand-only migrations; schema-ahead → forward-fix; dual-app skew is expected. |
| **H1276-8** | #1277, CAC map shape, #706, Nixpacks, and a second `/status` stay out of this ticket. |

Existing Observability exception for fee-discount LCD probe is unchanged. Generic health is **no longer** “ok-only.” Do not reuse hub-wrap **H11–H16** or audit **H11** (LCD fanout) for this liveness rule — those IDs are taken.

## Alternatives

| Option | Why not |
|--------|---------|
| Field name `commit` | Collides with colloquial Coolify `HEAD` tokens; `git_sha` matches in-repo stamp files. |
| Omit uppercase instead of normalize | Operators paste mixed-case hex; normalize is the same commit. Invalid non-hex still omitted. |
| First **set** var wins, including empty `GIT_SHA=""` | Leftover include-source-commit injects `SOURCE_COMMIT` while Dockerfile still exports empty `GIT_SHA`. Live `/health` would omit `git_sha`. Rejected. |
| “Prefer both” by copying Coolify `git_commit_sha` into `GIT_SHA` | That UI field is often `HEAD`. Select then rejects and does **not** fall through to `SOURCE_COMMIT`. Set `GIT_SHA` only when already hex. |
| Close leftover on regex-only live `git_sha` | A stale hex from a one-off manual bake would PASS while auto-deploy is still off. Regex-only is bake presence. Leftover-complete is checkbox **and** tip-match. |
| Optional Dockerfile stamp | Live leftover AC cannot pass; Coolify `HEAD` stays invisible. Bake path is **required**. |
| ARG/ENV only on the builder (frontend Vite pattern) | ARG does not cross `FROM`. The serving process would have no `GIT_SHA` / `SOURCE_COMMIT`. |
| ARG immediately after `FROM runtime` | Busts `apt-get` on every auto-deploy commit. Place after `COPY --from=builder`. |
| Run `git` in the image | Build context is `indexer/` only; no `.git`. Forbidden. |
| Nixpacks | Prod is the Dockerfile. Do not add a second bake path. |
| Cache `git_sha` on `AppState` | Ok-only tests and leftover bake-arg updates would see stale env. Read per request. |
| Put SHA on fee-discount health | Different probe; keep LCD-only (**H1276-4**). |
| Put SHA on CAC `/health` | Different process (#348 / #352). |
| Close Coolify checkbox in the same implement slice | #297 operator action; ticket would be unfinishable for implement. |
| Path-filter in-repo (custom CI webhook) | Coolify watch paths are leftover UI; do not invent a second deploy controller here. |
| Dual UUID CAC map in this repo | Wrong repo; separate agent-control issue. |
| Readiness `/health` with DB/LCD | Rejected under **H1276-1**; DoS and fee-discount already cover LCD. |
| Rewrite M573/M590 “together” wording | Historical coupled leftover for those stacks; auto-deploy is the **new** default, not a retroactive rewrite. |

## Complexity added / removed

**Added:** parser + select (empty/`GIT_SHA` whitespace trim fallthrough) + two Docker ARG/ENV on the runtime stage after COPY; one optional JSON key; auto-deploy rollback/skew rules; operator leftover checklist; `VERIFY1276_REQUIRE_LIVE` leftover probe with optional `VERIFY1276_EXPECT_SHA` tip-match.

**Removed:** “scrape Coolify logs to learn the serving SHA”; implicit “indexer is always a manual Coolify click after every `main` land” as the **only** documented path (manual remains available when auto-deploy is off or disabled for a breaking migrate).

Net: small public surface in exchange for attest + a supportable auto-deploy leftover.

## Migration

No sqlx migration for this ticket. `sqlx::migrate!()` on boot is unchanged and is why auto-deploy needs the expand-only / forward-fix rule.

Additive `/health` JSON: old clients that require exact `{"status":"ok"}` **outside this repo** may break; in-repo tests are updated in slice 1. Docker HEALTHCHECK does not parse JSON.

## Observability

- Attest: `GET https://indexer.dex.cl8y.com/health` → parse JSON with **jq** (not grep). `status=ok` plus hex `git_sha`. After an indexer-touching land, compare to `git rev-parse HEAD` (prefix OK) via `VERIFY1276_EXPECT_SHA`.
- Tracing: optional debug log that SHA was omitted (reason enum: unset / rejected), **never** log the rejected raw value if it looked secret-shaped (log `rejected` only).
- Do **not** scrape Coolify logs. Do **not** add `/metrics`.
- `make verify-issue-1276` live leftover: see Tests. Do not assert the Coolify checkbox from HTTP.

## Failure modes

| Failure | Behavior |
|---------|----------|
| Env unset (local cargo, tests) | `{"status":"ok"}` — 200. |
| `GIT_SHA=""` + `SOURCE_COMMIT=<hex>` | `git_sha` present (empty falls through). |
| `GIT_SHA="   "` + `SOURCE_COMMIT=<hex>` | `git_sha` present (trim fallthrough). |
| Whitespace-only **selected** candidate | Omit — 200. |
| Coolify baked `HEAD` in the **selected** var | Omit `git_sha` — 200. Leftover glance **fails** until bake-arg is a real hex. |
| Operator maps `git_commit_sha=HEAD` into `GIT_SHA` | Omit — 200. Non-empty reject; `SOURCE_COMMIT` unused. Leftover HTTP stays omitted. |
| `GIT_SHA=HEAD` + `SOURCE_COMMIT=<hex>` | Omit — 200. Non-empty reject; no fallthrough. |
| Secret-shaped env | Omit — 200. No echo. |
| ARG/ENV only on builder | Serving process has no stamp; live leftover **FAIL** (`git_sha` omitted). Implement must pin runtime stage. |
| ARG immediately after `FROM runtime` | `apt-get` layer busts on every commit. Implement must pin after `COPY --from=builder`. |
| Live hex without `VERIFY1276_EXPECT_SHA` | Named probe can PASS (bake presence). A stale manual bake still PASSes. Not leftover-complete. |
| Auto-deploy rebuild on frontend-only land (no watch paths) | Extra Rust image build + process restart; migrate no-ops if schema unchanged. Cost only. |
| Auto-deploy land with new expand-only migration | Boot migrate succeeds; previous image **cannot** start on the new schema → forward-fix if the new binary is bad. |
| Auto-deploy land with breaking migration while flag on | High risk. Policy: do not ship breaking migrations with auto-deploy on. |
| Vite at tip T, indexer still T-n | Skew window. Additive APIs OK. Breaking consume-after-expand. |
| Implement agent tries Coolify UI | Forbidden (#297). Document leftover only. |
| Injected `/health` wait on LCD | Forbidden. Fee-discount stays the slow probe. |
| Ok-only test without `remove_var` both keys | Flaky exact `{"status":"ok"}` when CI/Coolify-like `SOURCE_COMMIT` or `dotenvy` is set. |

## Ordered implementation slices

| Slice | Who | Deliverable | Blocks |
|-------|-----|-------------|--------|
| **0 — this design** | design_author | ADR 0006, architecture pointer, invariant amendment, runbook policy, skill | Slice 1 |
| **1 — code + tests** | implement | `select_commit_env` + `parse_git_sha`, per-request `health()`, Dockerfile **runtime** ARG/ENV **after** `COPY --from=builder`, unit tests (`&str` / `Option<&str>`), `#[serial]` env-mutating integration tests (`remove_var` both keys on ok-only), `scripts/qa/verify-issue-1276.sh` (local; live Coolify SKIP unless `VERIFY1276_REQUIRE_LIVE=1` / `VERIFY1276_IID=1276`; jq parse; optional `VERIFY1276_EXPECT_SHA` prefix-match) | Slice 3 leftover glance |
| **2 — remaining docs in the code MR** | implement | `indexer/.env.example` optional `GIT_SHA` / `SOURCE_COMMIT` (hex only; never document copying `HEAD` into `GIT_SHA`); Makefile `verify-issue-1276` wiring; AGENTS.md **only if** that file already lists 1276 children. Do **not** rewrite M573/M590. `docs/testing.md` already has the verify row (do not treat “together” wording as this ticket’s deliverable). | none |
| **3 — operator leftover** | founder/operator (#297) | Coolify indexer: protected-branch auto-deploy **on**; include-source-commit with `GIT_SHA` left unset/empty unless already hex; optional watch paths. Live leftover glance: HTTP probe **plus** `VERIFY1276_EXPECT_SHA` tip-match after an indexer-touching land. Checkbox is operator-attested, not inferred from HTTP. No log scrape. No UUID in comments. | Closes leftover AC only |

Slice 1 **must not** wait on slice 3. Slice 3 **must not** be attempted by implement agents.

No product-issue dependencies. Do not sequence on #1277.

## Tests (slice 1)

Parser unit tests take `&str` / `Option<&str>` and **must not** read process env.

| Case | Expected |
|------|----------|
| Both env unset | 200, exact `{"status":"ok"}` (update `generic_health_unchanged`) |
| Valid 40-char lowercase | 200, `status=ok`, `git_sha` equal |
| Valid 7-char prefix | 200, `git_sha` equal |
| Uppercase 40-char hex | 200, `git_sha` lowercased |
| `HEAD` / `head` | 200, no `git_sha` |
| Empty / whitespace-only (**selected** candidate) | 200, no `git_sha` |
| Internal whitespace / contains `=` | 200, no `git_sha` |
| `N\|…` / `Bearer …` / `sk-…` / `glpat-…` | 200, no `git_sha` |
| `GIT_SHA=""` + `SOURCE_COMMIT=<hex>` | **present** (empty falls through) |
| `GIT_SHA="   "` + `SOURCE_COMMIT=<hex>` | **present** (trim fallthrough) |
| `GIT_SHA` unset, `SOURCE_COMMIT` valid hex | present |
| `GIT_SHA=HEAD` and `SOURCE_COMMIT=<hex>` | omit (non-empty reject, no fallthrough) |
| Fee-discount route | unchanged 200/fail-closed |
| Security | `/health` 200, no extra auth headers; body has no UUID/token/host inventory |

Integration tests that mutate process env (`std::env::set_var` / `remove_var`) **must** be `#[serial]` (same as `generic_health_unchanged`). Parser unit tests stay env-free.

**Ok-only exact object:** `generic_health_unchanged` (and any sibling that asserts exact `{"status":"ok"}`) **must** call `remove_var("GIT_SHA")` and `remove_var("SOURCE_COMMIT")` **before the request**. `Config::from_env` calls `dotenvy::dotenv()`; CI/Coolify-like shells often export `SOURCE_COMMIT`. The handler reads env **per request**, so stripping after app build and before `GET /health` is what makes the assertion stable.

### `make verify-issue-1276` live leftover probe

Local cargo/lib + API tests always run. Live Coolify is **not** required for the code MR.

When `VERIFY1276_REQUIRE_LIVE=1` **or** `VERIFY1276_IID=1276`:

1. `GET https://indexer.dex.cl8y.com/health` must be HTTP **200**.
2. Parse the body with **jq** (not grep): `status` and `git_sha` as JSON fields.
3. Require `status=ok` and `git_sha` matching `^[0-9a-f]{7,40}$` (lowercase; if the field is mixed-case hex, compare after ASCII lowercasing).
4. Missing or omitted `git_sha` is **FAIL**, not SKIP. `{"status":"ok"}` alone is FAIL.
5. If **`VERIFY1276_EXPECT_SHA`** is set (leftover glance after a land: full or short hex of that merge, `git rev-parse HEAD` or prefix): trim, lowercase, require it is hex 7–40, then **FAIL** unless one of `git_sha` / expect is a **prefix of the other** (case-insensitive). A stale hex that is valid but not a prefix of that merge **FAIL**s.
6. Unreachable host is **FAIL** under this flag (do not SKIP).
7. Without the live flag: unreachable host is **SKIP**. Local tests can still PASS while live `/health` stays `{"status":"ok"}` until leftover bake.
8. Do **not** scrape Coolify. Do **not** try to assert the auto-deploy checkbox from HTTP.

Without `VERIFY1276_EXPECT_SHA`, a passing live probe is **bake presence only**. That is not leftover-complete.

## Rollout

1. Merge slice 1–2 to `main` (frontend auto-deploy may ship the dApp first; indexer still manual until leftover). Implement **must not** flip Coolify.
2. Operator leftover: include-source-commit; `GIT_SHA` only if already hex (never copy `HEAD`); auto-deploy checkbox **on**; optional watch paths.
3. Glance after an indexer-touching land: `VERIFY1276_REQUIRE_LIVE=1 VERIFY1276_EXPECT_SHA=$(git rev-parse HEAD) make verify-issue-1276` (short hex OK). Live `git_sha` must prefix-match that merge. Regex-only without `EXPECT_SHA` does not close leftover.
4. Post-merge playbooks that required a coupled restart for a **named historical stack** stay those issues’ leftovers (M573, M590). New default after leftover: each app follows `main`; couple only for breaking contracts.

## Rollback

- **Health field:** revert the handler (clients tolerate missing `git_sha`). HEALTHCHECK unchanged.
- **Dockerfile ARG:** removing ARG is compatible with empty env (omit field).
- **Indexer binary after auto-deploy:** see Decision table. Schema-ahead → forward-fix. Attest rollback with `/health` `git_sha`, not Coolify log scrape.
- **Auto-deploy flag:** operator may turn it **off** (#297) without a code revert.

## Integration completion criteria

**Implement merge (slices 1–2) is complete when:**

- Parser + select + per-request `/health` + Dockerfile **runtime** ARG/ENV after `COPY --from=builder` + tests above pass locally (`make verify-issue-1276` without live Coolify).
- Invariant row **Health git SHA (#1276)** matches shipped JSON.
- Fee-discount tests still pass.
- No Coolify secret/UUID in the MR.
- Implement **must not** flip Coolify.

**#1276 leftover is complete when (operator, not implement) — both, not regex-only:**

1. Indexer Coolify protected-branch auto-deploy is **on** (operator / #297; documented; no UUID pasted). **Do not infer this from HTTP.**
2. After an indexer-touching `main` land: live `GET /health` includes hex `git_sha` that **prefix-matches** that merge (`VERIFY1276_REQUIRE_LIVE=1` + `VERIFY1276_EXPECT_SHA=<full or short hex>`). Not `HEAD`. Regex-only bake presence is **not** enough — a stale one-off manual hex would PASS it while auto-deploy is still off.
3. Glance used public `/health` + jq only (no Coolify log scrape).

Keywords on the issue (“architecture”, “deploy”) are **not** approval to flip Coolify or to write `DESIGN: APPROVE`.
