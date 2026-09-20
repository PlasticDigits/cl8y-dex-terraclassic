# ADR 0006: Indexer `/health` git SHA and protected-main auto-deploy

## Status

Proposed ([#1276](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1276))

Design record for implement. This ADR does **not** flip Coolify, deploy, scrape Coolify logs, publish app UUIDs or tokens, change CAC `/health`, or expand CAC `COOLIFY_APP_MAP`. Enabling the indexer Coolify protected-branch auto-deploy checkbox is operator leftover under [agent-control #297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297) (deploy policy). In-repo work stamps SHA and documents the flag; it cannot close the checkbox.

Playbook: [`skills/AGENTS_INDEXER_HEALTH_GIT_SHA.md`](../../skills/AGENTS_INDEXER_HEALTH_GIT_SHA.md) (**H1276-1–H1276-8**). Overview: [`architecture.md`](../architecture.md#indexer-production-attest). Invariant: [`indexer-invariants.md`](../indexer-invariants.md) **Health git SHA (#1276)**. Rollback: [`runbooks/rollback-decision.md`](../runbooks/rollback-decision.md) § Auto-deploy era.

ADR **0005** is reserved by the #1269 hop-fee design branch (`docs/adr/0005-protocol-fee-multihop-hops.md`). This ticket is **0006**.

## Outcome

1. **Attest.** Unauthenticated `GET /health` stays liveness. JSON is `{"status":"ok"}` plus optional `"git_sha"` when a baked env value parses as lowercase hex length 7–40. Operators compare that field to `git rev-parse HEAD` on the serving merge (prefix OK). They do **not** scrape Coolify `SOURCE SHA` log lines.
2. **Bake.** Production image is [`docker/indexer/Dockerfile`](../../docker/indexer/Dockerfile) (not Nixpacks). The recipe **must** accept `ARG GIT_SHA` / `ARG SOURCE_COMMIT` and export them as runtime `ENV`. The image still `COPY indexer/` only and does **not** run `git`. Empty ARG is valid for local `cargo run`; live AC after leftover requires a real hex, not the literal token `HEAD`.
3. **Auto-deploy (documented intent, operator leftover).** The indexer Coolify app for this Forgejo path should follow protected `main` like the frontend app. CAC grouped drain maps **one** UUID per `owner/repo` and is **not** the indexer redeploy path. Dual-app skew (Vite vs Rust rebuild) is expected. Schema-on-boot stays `sqlx::migrate!()`; rollback after a landed migration without `down.sql` is **forward-fix only**.

Implement may merge slices 1–2 while the Coolify checkbox is still off. Closing #1276 leftover glance waits on operator evidence, not on implement agents.

## Context

Protected-branch land already rebuilds the **frontend** Coolify application. The **indexer** application for the same git source has auto-deploy off, so indexer schema/API landings stay off `indexer.dex.cl8y.com` until a manual deploy. `health()` returns only `{"status":"ok"}`. Docker `HEALTHCHECK` and most scripts only require HTTP 200.

Today’s invariant row says generic `GET /health` stays `{"status":"ok"}` **only**. [`indexer/tests/api_fee_discount_health.rs`](../../indexer/tests/api_fee_discount_health.rs) `generic_health_unchanged` asserts exact object equality. An additive field is compatible with HEALTHCHECK **after** that invariant and test change.

Coolify is known to store the literal `HEAD` token in commit fields. The Dockerfile copies `indexer/` only, has no commit ARG/ENV, and cannot `git rev-parse` at build. Without a required bake path, live `/health` keeps omitting `git_sha` and leftover AC fails.

The binary always runs `sqlx::migrate!()` before bind ([`indexer/src/main.rs`](../../indexer/src/main.rs)). Almost all migrations have no `down.sql` (three files under [`indexer/migrations/revert/`](../../indexer/migrations/revert/)). Post-merge playbooks still treat indexer as a gated Coolify redeploy after migrate, separate from the frontend rebuild. Auto-deploy without a forward-fix/rollback rule and a skew window is not supportable.

## Non-goals

- CAC public `/health` SHA, occupancy, or SOURCE stamp ([agent-control #348](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/348) / leftover [#352](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/352)).
- CAC `drain_deploy_url` / `app_uuid_for_path` two-UUID map. If CAC must deploy two apps per path, that is a **separate** agent-control issue. Do not block indexer auto-deploy on it.
- `GET /api/v1/health/fee-discount` (LCD probe only — **H6b**).
- A second public `/status`, Prometheus `/metrics`, or DB/LCD on generic `/health` (H11 stays liveness).
- Fetching Coolify from the indexer process. Scraping Coolify logs. Publishing Coolify tokens, app UUIDs, hosts, or `/status` JSON.
- Nixpacks. Production stays `docker/indexer/Dockerfile`.
- Numeric overflow on volume_aggregator ([#1277](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1277)) — out of scope, not approved.
- Frontend stale Vite chunks ([#706](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/706)) — complementary UX.
- Flipping Coolify UI from this repo, HMAC/`autonomy.rs` self-approval, or a founder card for this ordinary design.

## Decision

### Field name

**`git_sha`.** Not `commit`. Matches `.qa-deploy-stamp` naming. Omit the key entirely when the parser rejects the env (JSON `{"status":"ok"}` with no `git_sha`).

### Env / build-arg (mandatory in the image recipe)

| Name | Role |
|------|------|
| **`GIT_SHA`** | Primary Docker `ARG` + runtime `ENV`. Coolify build-arg mapped from the platform commit SHA. |
| **`SOURCE_COMMIT`** | Alias `ARG` + runtime `ENV` so Coolify “include source commit” injection works without a second mapping. |

Parser read order: `GIT_SHA`, then `SOURCE_COMMIT`. First **set** (non-unset) value is parsed; if that value is omitted by the parser, **do not** fall through to the other var (avoids mixing a rejected `HEAD` in `GIT_SHA` with a leftover hex in `SOURCE_COMMIT`). If the first var is unset, read the second.

Local `cargo run` / tests: both unset → omit field.

Dockerfile (implement slice 1; do not run `git` in the image):

```dockerfile
ARG GIT_SHA=
ARG SOURCE_COMMIT=
ENV GIT_SHA=${GIT_SHA} \
    SOURCE_COMMIT=${SOURCE_COMMIT}
```

Coolify leftover: pass a **40-char lowercase hex**, never the string `HEAD`. Prefer include-source-commit **and** explicit `GIT_SHA=<commit sha>` build-arg. Watch paths (when the UI supports them): `/indexer/**` and `/docker/indexer/**` so frontend-only lands skip the Rust rebuild. If watch paths are unavailable, extra indexer rebuilds on frontend-only `main` lands are accepted **cost**, not a correctness hole.

### Parser (uppercase → normalize; else omit)

Pure function, no I/O. Input: the chosen env string.

1. If unset → omit.
2. Trim ASCII whitespace. Empty after trim → omit.
3. If the trimmed value still contains whitespace → omit.
4. If it contains `=` → omit.
5. If it equals `HEAD` case-insensitively → omit.
6. If it starts with a secret prefix → omit: `Bearer `, `bearer `, `sk-`, `ghp_`, `gho_`, `ghu_`, `ghs_`, `glpat-`, `N|` (Sanctum-shaped).
7. Lowercase ASCII. If it does not match `^[0-9a-f]{7,40}$` → omit.
8. Else emit that lowercase hex as `git_sha`.

**Uppercase hex is normalized** (not omitted). `HEAD` / empty / secret-shaped / non-hex is omitted, never echoed.

`/health` stays fast: no Postgres, no LCD, no Coolify client. `status` is always `"ok"` when the process is serving (existing behavior). Docker HEALTHCHECK remains `curl -fsS …/health` HTTP 200.

### Auto-deploy policy (so the leftover checkbox is supportable)

| Rule | Meaning |
|------|---------|
| **Expand-only while auto-deploy is on** | New sqlx migrations that ride auto-deploy must be additive (nullable columns, new tables, new indexes). Breaking rewrites require the operator to **turn auto-deploy off**, apply by [`rollback-decision.md`](../runbooks/rollback-decision.md), then re-enable. |
| **Forward-fix if schema moved** | After boot migrate with **no** paired `down.sql`, do **not** roll the previous binary — schema is ahead and startup will fail. Snapshot Postgres before risky migrations. Ship a hotfix image. |
| **Binary rollback if schema did not move** | Crash/logic bug with unchanged `_sqlx_migrations`: restore the previous Coolify indexer image. Confirm `git_sha` on `/health` matches the restored digest’s commit (prefix OK). |
| **Dual-app skew** | Frontend (Vite) and indexer (Rust) are two apps on one repo. After `main` land they may differ for minutes to tens of minutes. Additive JSON is the default contract. A frontend that **requires** a new indexer field must land **after** live `/health` `git_sha` includes the indexer commit (or auto-deploy is temporarily off and the pair is coupled). Post-merge “rebuild frontend + restart indexer together” becomes “each app follows `main`; couple only for breaking contracts.” |
| **CAC drain** | Still one UUID per Forgejo path. Auto-deploy on the indexer app is independent. Do not use drain as the indexer redeploy path. |

## Component / state / interface changes

| Layer | Change |
|-------|--------|
| API | `health()` in [`indexer/src/api/mod.rs`](../../indexer/src/api/mod.rs): build JSON from parser. No new route. |
| Parser | Small helper (lib-testable) — do not inline ad-hoc regex only in the handler. |
| Image | `docker/indexer/Dockerfile` ARG/ENV as above. HEALTHCHECK unchanged (HTTP 200). |
| Tests | See Tests. `generic_health_unchanged` becomes “ok-only when env unset.” |
| Invariant | Observability unhappy-path no longer “ok-only”; new **Health git SHA (#1276)** row. |
| Runbooks | Indexer auto-deploy era + SHA glance; mainnet-soft-launch Coolify indexer bake-args. |
| Coolify | Operator leftover only. Not in git. |
| CAC | Unchanged. |
| dApp | No required chrome. Clients that `JSON.parse` `/health` must tolerate an extra key (already true if they read `status` only). |

## Affected invariants

| ID | Effect |
|----|--------|
| **H1276-1** | Generic `GET /health` is liveness: HTTP 200, `"status":"ok"`, no DB/LCD. Additive `git_sha` only. |
| **H1276-2** | Field name is `git_sha`. Parser is hex 7–40, lowercase; uppercase normalized; `HEAD` / whitespace / `=` / secret prefixes omitted. |
| **H1276-3** | Dockerfile **must** declare `GIT_SHA` + `SOURCE_COMMIT` ARG/ENV. Image does not run `git`. Nixpacks is not the prod path. |
| **H1276-4** | `GET /api/v1/health/fee-discount` unchanged (**H6b**). |
| **H1276-5** | No Coolify UUID, token, host, occupancy, or inventory in `/health` or issue comments. |
| **H1276-6** | Auto-deploy is the indexer Coolify protected-branch flag (operator / #297). CAC drain is not the indexer redeploy path. CAC `/health` stays SHA-free. |
| **H1276-7** | While auto-deploy is on: expand-only migrations; schema-ahead → forward-fix; dual-app skew is expected. |
| **H1276-8** | #1277, CAC map shape, #706, Nixpacks, and a second `/status` stay out of this ticket. |

Existing Observability exception for fee-discount LCD probe is unchanged. Generic health is **no longer** “ok-only.”

## Alternatives

| Option | Why not |
|--------|---------|
| Field name `commit` | Collides with colloquial Coolify `HEAD` tokens; `git_sha` matches in-repo stamp files. |
| Omit uppercase instead of normalize | Operators paste mixed-case hex; normalize is the same commit. Invalid non-hex still omitted. |
| Optional Dockerfile stamp | Live leftover AC cannot pass; Coolify `HEAD` stays invisible. Bake path is **required**. |
| Run `git` in the image | Build context is `indexer/` only; no `.git`. Forbidden. |
| Nixpacks | Prod is the Dockerfile. Do not add a second bake path. |
| Put SHA on fee-discount health | Different probe; keep LCD-only. |
| Put SHA on CAC `/health` | Different process (#348 / #352). |
| Close Coolify checkbox in the same implement slice | #297 operator action; ticket would be unfinishable for implement. |
| Path-filter in-repo (custom CI webhook) | Coolify watch paths are leftover UI; do not invent a second deploy controller here. |
| Dual UUID CAC map in this repo | Wrong repo; separate agent-control issue. |
| Readiness `/health` with DB/LCD | Rejected under H11; DoS and fee-discount already cover LCD. |

## Complexity added / removed

**Added:** parser + two Docker ARG/ENV; one optional JSON key; auto-deploy rollback/skew rules; operator leftover checklist.

**Removed:** “scrape Coolify logs to learn the serving SHA”; implicit “indexer is always a manual Coolify click after every `main` land” as the **only** documented path (manual remains available when auto-deploy is off or disabled for a breaking migrate).

Net: small public surface in exchange for attest + a supportable auto-deploy leftover.

## Migration

No sqlx migration for this ticket. `sqlx::migrate!()` on boot is unchanged and is why auto-deploy needs the expand-only / forward-fix rule.

Additive `/health` JSON: old clients that require exact `{"status":"ok"}` **outside this repo** may break; in-repo tests are updated in slice 1. Docker HEALTHCHECK does not parse JSON.

## Observability

- Attest: `GET https://indexer.dex.cl8y.com/health` → `git_sha` prefix-matches `git rev-parse HEAD` of the indexer image’s commit.
- Tracing: optional debug log that SHA was omitted (reason enum: unset / rejected), **never** log the rejected raw value if it looked secret-shaped (log `rejected` only).
- Do **not** scrape Coolify logs. Do **not** add `/metrics`.

## Failure modes

| Failure | Behavior |
|---------|----------|
| Env unset (local cargo, tests) | `{"status":"ok"}` — 200. |
| Coolify baked `HEAD` | Omit `git_sha` — 200. Leftover glance **fails** until bake-arg is a real hex. |
| Secret-shaped env | Omit — 200. No echo. |
| Auto-deploy rebuild on frontend-only land (no watch paths) | Extra Rust image build + process restart; migrate no-ops if schema unchanged. Cost only. |
| Auto-deploy land with new expand-only migration | Boot migrate succeeds; previous image **cannot** start on the new schema → forward-fix if the new binary is bad. |
| Auto-deploy land with breaking migration while flag on | High risk. Policy: do not ship breaking migrations with auto-deploy on. |
| Vite at tip T, indexer still T-n | Skew window. Additive APIs OK. Breaking consume-after-expand. |
| Implement agent tries Coolify UI | Forbidden (#297). Document leftover only. |
| Injected `/health` wait on LCD | Forbidden. Fee-discount stays the slow probe. |

## Ordered implementation slices

| Slice | Who | Deliverable | Blocks |
|-------|-----|-------------|--------|
| **0 — this design** | design_author | ADR 0006, architecture pointer, invariant amendment, runbook policy, skill | Slice 1 |
| **1 — code + tests** | implement | Parser, `health()`, Dockerfile ARG/ENV, tests, `make verify-issue-1276` (local; live Coolify SKIP unless `VERIFY1276_REQUIRE_LIVE=1`) | Slice 3 leftover glance |
| **2 — remaining docs in the code MR** | implement | `docs/testing.md` verify row, `indexer/.env.example` optional `GIT_SHA`, post-merge stack “together” wording, AGENTS.md verify list if that file is already listing 1276 children | none |
| **3 — operator leftover** | founder/operator (#297) | Coolify indexer: protected-branch auto-deploy **on**; include-source-commit / `GIT_SHA` hex build-arg; optional watch paths; live `/health` `git_sha` matches `main` (prefix OK). No log scrape. No UUID in comments. | Closes leftover AC only |

Slice 1 **must not** wait on slice 3. Slice 3 **must not** be attempted by implement agents.

No product-issue dependencies. Do not sequence on #1277.

## Tests (slice 1)

| Case | Expected |
|------|----------|
| Both env unset | 200, exact `{"status":"ok"}` (update `generic_health_unchanged`) |
| Valid 40-char lowercase | 200, `status=ok`, `git_sha` equal |
| Valid 7-char prefix | 200, `git_sha` equal |
| Uppercase 40-char hex | 200, `git_sha` lowercased |
| `HEAD` / `head` | 200, no `git_sha` |
| Empty / whitespace-only | 200, no `git_sha` |
| Internal whitespace / contains `=` | 200, no `git_sha` |
| `N\|…` / `Bearer …` / `sk-…` / `glpat-…` | 200, no `git_sha` |
| `GIT_SHA=HEAD` and `SOURCE_COMMIT=<hex>` | omit (first-set wins, no fallthrough) |
| `GIT_SHA` unset, `SOURCE_COMMIT` valid hex | present |
| Fee-discount route | unchanged 200/fail-closed |
| Security | `/health` 200, no extra auth headers; body has no UUID/token/host inventory |

Parser unit tests live next to the helper (`cargo test --lib`). Integration: `api_health.rs`, `api_fee_discount_health.rs`, `security.rs`.

## Rollout

1. Merge slice 1–2 to `main` (frontend auto-deploy may ship the dApp first; indexer still manual until leftover).
2. Operator leftover: bake-arg + auto-deploy + watch paths.
3. Glance: `curl -sS https://indexer.dex.cl8y.com/health` — `git_sha` prefix of current `main` after an indexer-touching land (or after a manual deploy of that tip).
4. Post-merge playbooks that required a coupled restart: treat skew as normal unless the land is a breaking indexer contract.

## Rollback

- **Health field:** revert the handler (clients tolerate missing `git_sha`). HEALTHCHECK unchanged.
- **Dockerfile ARG:** removing ARG is compatible with empty env (omit field).
- **Indexer binary after auto-deploy:** see Decision table. Schema-ahead → forward-fix. Attest rollback with `/health` `git_sha`, not Coolify log scrape.
- **Auto-deploy flag:** operator may turn it **off** (#297) without a code revert.

## Integration completion criteria

**Implement merge (slices 1–2) is complete when:**

- Parser + `/health` + Dockerfile ARG/ENV + tests above pass locally (`make verify-issue-1276` without live Coolify).
- Invariant row **Health git SHA (#1276)** matches shipped JSON.
- Fee-discount tests still pass.
- No Coolify secret/UUID in the MR.

**#1276 leftover is complete when (operator, not implement):**

- Indexer Coolify protected-branch auto-deploy is on (documented; no UUID pasted).
- Live `GET /health` includes hex `git_sha` matching the baked commit after a `main` land (not `HEAD`).
- Glance used public `/health` only.

Keywords on the issue (“architecture”, “deploy”) are **not** approval to flip Coolify or to write `DESIGN: APPROVE`.
