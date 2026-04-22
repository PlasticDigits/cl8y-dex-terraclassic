# Issue backlog (GitLab-ready)

**Last reviewed:** 2026-04-22 (synced with `main`; live tracking in [GitLab issues](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues)).

Use labels: `contracts`, `backend`, `frontend`, `indexer`, `infra`, `docs`, `security`, `testing`, `architecture`, `v2`, `limit-orders`, `hybrid`, `launch-blocker`, `nice-to-have`.

Priority: **P0** correctness/architecture, **P1** production completeness, **P2** important, **P3** polish.

---

## Epic 1: Architecture and design clarification

### DEX-P0-EPIC — Product spec: hybrid “best execution”

- **Problem:** Repo implements manual Pattern C hybrid; “production hybrid” is undefined (auto-split? solver? indexer?).
- **Evidence:** [`SwapPage.tsx`](../../../frontend-dapp/src/pages/SwapPage.tsx); [`route_solver.rs`](../../../indexer/src/api/route_solver.rs).
- **Why it matters:** Engineering work diverges without a target UX and safety model.
- **Scope:** 1–2 page decision doc; stakeholder sign-off.
- **Dependencies:** None.
- **Acceptance criteria:** Document states quote strategy, routing ownership (client vs server), and V1 vs V2 feature boundaries.
- **GitLab:** Track execution/indexer scope under [**#108** — indexer best execution for default swap](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/108) (epic); architecture epic [**#56**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/56).
- **Labels:** `architecture`, `product`, `hybrid`, `docs`
- **Owner:** product + architecture
- **Priority:** P0
- **Blockers:** hybrid launch (definition)

---

## Epic 2: v2 swaps completion

### DEX-P1-009 — CI/CD: optimizer wasm parity

- **Problem:** CI builds `cargo` wasm; production uses Docker optimizer ([`Makefile`](../../../Makefile) L78–87 vs [`.github/workflows/test.yml`](../../../.github/workflows/test.yml) L59–78).
- **Why it matters:** Subtle codegen/size differences; release discipline.
- **Scope:** Add CI job to run optimizer (cached) **or** verify checksums against release artifacts; document policy.
- **Acceptance criteria:** Release checklist states which artifact is canonical; CI enforces one policy.
- **Labels:** `infra`, `testing`, `v2`, `launch-blocker`
- **Priority:** P1
- **Blockers:** v2 launch

### DEX-P1-006 — Launch checklist: governance, pause, hooks, treasury

- **Problem:** Operational steps spread across [`deployment-guide.md`](../../deployment-guide.md) and [`security-model.md`](../../security-model.md).
- **Scope:** Single `docs/runbooks/launch-checklist.md` with ordered steps and verification queries.
- **Acceptance criteria:** Checklist includes factory admin, router trust, hook list, treasury address verification.
- **Labels:** `docs`, `infra`, `v2`, `security`
- **Priority:** P1
- **Blockers:** v2 launch

### DEX-P2-013 — Align fee tier examples across docs

- **Problem:** [`deployment-guide.md`](../../deployment-guide.md) tier examples differ numerically from [`architecture.md`](../../architecture.md) table (e.g. discount bps / min tokens).
- **Scope:** Single source of truth; fix contradictory examples.
- **Labels:** `docs`
- **Priority:** P2
- **Blockers:** none

### DEX-P2-029 — Pin LocalTerra / third-party image versions in compose

- **Problem:** Reproducible local QA requires pinned tags (verify [`docker-compose.yml`](../../../docker-compose.yml)).
- **Scope:** Document or pin versions; changelog when bumped.
- **Labels:** `infra`, `testing`
- **Priority:** P3

---

## Epic 3: Limit order system completion

### DEX-P1-001 — Limit book visibility (shallow + head) — **done**; deep book remains

- **Status:** **Shipped (2026):** indexer proxies `GET /api/v1/pairs/{addr}/order-book-head` and `.../limit-book-shallow` (default depth 10, max 20); [`LimitOrdersPage.tsx`](../../../frontend-dapp/src/pages/LimitOrdersPage.tsx) consumes the shallow book. See [ADR 0002](../../adr/0002-limit-book-surfacing.md) and GitLab [**#71**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/71).
- **Remaining scope:** CEX-style **deep** book, pagination, and load — GitLab [**#102**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/102).
- **Labels:** `backend`, `indexer`, `limit-orders`, `frontend` (historical)
- **Priority:** P1 → follow **#102** for product depth
- **Blockers:** none for shallow UX; deep work tracked on **#102**

### DEX-P1-010 — E2E: limit place + cancel (tx path)

- **Problem:** [`e2e/limit-orders.spec.ts`](../../../frontend-dapp/e2e/limit-orders.spec.ts) is UI smoke; on-chain flows live in [`e2e/limit-orders-tx.spec.ts`](../../../frontend-dapp/e2e/limit-orders-tx.spec.ts) with **conditional** `test.skip` (e.g. paused pair).
- **Scope:** Playwright flow with funded wallet: place bid/ask, cancel, assert balances or tx success.
- **Policy:** On the **default** CI/local path, tx E2E must **not** skip solely for missing funds — GitLab [**#103**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/103). Feature tracking: [**#72**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/72).
- **Labels:** `testing`, `frontend`, `limit-orders`
- **Priority:** P1
- **Blockers:** limit launch (confidence)

### DEX-P2-012 — UX/docs: pause blocks limit cancel (L6)

- **Problem:** Users may expect cancel always available.
- **Scope:** Banner on `/limits` when pair paused; link to governance status.
- **Labels:** `frontend`, `docs`, `limit-orders`
- **Priority:** P2

### DEX-P2-015 — Indexer: metadata nulls for older pair wasm

- **Problem:** [`docs/limit-orders.md`](../../limit-orders.md) notes older pairs omit wasm attributes → null DB columns.
- **Scope:** Document version matrix; optional backfill from tx raw logs if feasible.
- **Labels:** `indexer`, `docs`
- **Priority:** P2

---

## Epic 4: Hybrid routing / execution completion

### DEX-P1-003 — Hybrid quote divergence: technical design

- **Problem:** L8 — simulation ignores book ([`router/src/contract.rs`](../../../smartcontracts/contracts/router/src/contract.rs)).
- **Scope:** Pick approach: (1) new pair query walking book read-only, (2) off-chain service with LCD order queries + pool math, (3) explicit “no quote” mode.
- **Acceptance criteria:** ADR committed; linked from [`limit-orders.md`](../../limit-orders.md).
- **Labels:** `architecture`, `contracts`, `hybrid`, `launch-blocker`
- **Priority:** P1
- **Blockers:** hybrid launch

### DEX-P1-004 — Route solver: hybrid strategy (future)

- **Problem (updated):** `GET /api/v1/route/solve` still emits pool-only ops (`hybrid: null`). `POST` accepts `hybrid_by_hop` and merges hybrid into `router_operations` before optional LCD `simulate_swap_operations` ([`route_solver.rs`](../../../indexer/src/api/route_solver.rs)).
- **Scope:** After DEX-P0-EPIC, server-suggested splits / default-path hybrid routing — see GitLab [**#101**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/101) (GET hybrid routes, multihop UX).
- **Labels:** `backend`, `indexer`, `hybrid`
- **Priority:** P1 (after epic decision)
- **Blockers:** hybrid (if server-side routing desired)

### DEX-P1-005 — Frontend: disclosure when hybrid enabled

- **Problem:** Users may trust `Simulation` / displayed output.
- **Scope:** Warning callout + link to docs when `book_input > 0` or hybrid object set.
- **Acceptance criteria:** Copy reviewed; visible before submit.
- **Labels:** `frontend`, `hybrid`, `security`
- **Priority:** P1
- **Blockers:** hybrid launch (honest UX)

### DEX-P1-007 — Integration test: single-hop hybrid with book consumption

- **Problem:** Need always-on regression for match + pool remainder.
- **Evidence:** `limit_order_tests` — extend or tag as release gate.
- **Labels:** `testing`, `contracts`, `hybrid`
- **Priority:** P1

### DEX-P1-008 — Integration test: multi-hop with hybrid on one leg

- **Problem:** Router hybrid forward untested in multi-hop composition.
- **Labels:** `testing`, `contracts`, `hybrid`, `v2`
- **Priority:** P1

### DEX-P1-011 — E2E: hybrid swap path

- **Problem:** No Playwright coverage for book leg.
- **Scope:** Seed limit + swap with book split on LocalTerra.
- **Labels:** `testing`, `frontend`, `hybrid`
- **Priority:** P1

---

## Epic 5: Security hardening

### DEX-P2-005 — Document hook `commission_amount` for hybrid (L7)

- **Problem:** Integrators mis-account fees.
- **Scope:** [`contracts-security-audit.md`](../../contracts-security-audit.md) + hook README + OpenAPI if any.
- **Labels:** `docs`, `security`, `hybrid`
- **Priority:** P2

### DEX-P2-011 — Review `assert_max_spread` behavior for hybrid totals

- **Problem:** Spread check uses pool path inputs; confirm product intent for combined return.
- **Scope:** Contract review + test vectors.
- **Labels:** `contracts`, `security`, `hybrid`
- **Priority:** P2

---

## Epic 6: Testing and verification

### DEX-P2-017 — Indexer integration: hybrid attributes on swap_events

- **Problem:** Ensure parser maps `book_return_amount`, `limit_book_offer_consumed`, `effective_fee_bps`.
- **Scope:** Test with fixture tx JSON.
- **Labels:** `testing`, `indexer`, `hybrid`
- **Priority:** P2

### DEX-P2-018 — Integration: fee discount applies to book leg

- **Problem:** Confirm `effective_fee_bps` in match path matches pool path for same trader.
- **Labels:** `testing`, `contracts`
- **Priority:** P2

### DEX-P2-019 — Vitest: hybrid message shape from `SwapPage`

- **Problem:** Guard router msg construction.
- **Labels:** `testing`, `frontend`
- **Priority:** P2

### DEX-P2-020 — Boundary: `max_maker_fills` at hard cap

- **Labels:** `testing`, `contracts`
- **Priority:** P3

### DEX-P2-021 — Post-migration smoke script (manual or CI optional)

- **Labels:** `infra`, `testing`
- **Priority:** P2

### DEX-P2-022 — E2E or integration: pause affects swap + limits

- **Labels:** `testing`, `contracts`, `frontend`
- **Priority:** P2

---

## Epic 7: Frontend / UX safety and state clarity

### DEX-P2-025 — `.env.example` for frontend + indexer

- **Problem:** Discoverability of required vars ([`config.rs`](../../../indexer/src/config.rs), `VITE_*`).
- **Scope:** Add `frontend-dapp/.env.example`, `indexer/.env.example` (if not present).
- **Labels:** `docs`, `infra`, `frontend`, `indexer`
- **Priority:** P2

### DEX-P2-031 — Charts / trades: explain “hybrid” badge

- **Evidence:** [`TradesTable.tsx`](../../../frontend-dapp/src/components/ui/TradesTable.tsx)
- **Scope:** Tooltip copy referencing indexer fields.
- **Labels:** `frontend`, `docs`
- **Priority:** P3

---

## Epic 8: Observability and operations

### DEX-P2-023 — Indexer: fail fast or warn on missing critical env in prod profile

- **Scope:** Optional `RUN_MODE=prod` stricter validation.
- **Labels:** `backend`, `indexer`, `infra`
- **Priority:** P2

### DEX-P2-024 — Metrics export for indexer (optional Prometheus)

- **Labels:** `backend`, `indexer`, `infra`
- **Priority:** P2

### DEX-P2-016 — Runbook: indexer reorg / replay / backfill

- **Labels:** `docs`, `infra`, `indexer`
- **Priority:** P2

### DEX-P2-026 — Runbook: contract migration / admin key rotation

- **Labels:** `docs`, `infra`, `security`
- **Priority:** P2

### DEX-P2-028 — Document secrets handling (LCD API keys, DB, etc.)

- **Labels:** `docs`, `security`, `infra`
- **Priority:** P2

### DEX-P2-030 — Incident response template for DEX + indexer

- **Labels:** `docs`, `infra`
- **Priority:** P2

### DEX-P2-027 — Environment matrix (local / testnet / mainnet) single table

- **Labels:** `docs`, `infra`
- **Priority:** P3

---

## Epic 9: Docs and launch readiness

### DEX-P2-009 — Integrator guide: quoting and routing limitations

- **Scope:** Single `docs/integrators.md`: L7, L8, simulation, indexer route solve.
- **Labels:** `docs`, `hybrid`, `architecture`
- **Priority:** P2
- **Blockers:** hybrid (external integrators)

### DEX-P3-032 — README: link to this review bundle

- **Scope:** Add “Reviews” subsection under docs.
- **Labels:** `docs`, `nice-to-have`
- **Priority:** P3

---

## Dependency graph (high level)

```text
DEX-P0-EPIC ──► DEX-P1-004 (if server routing); GitLab #108, #101
DEX-P1-003 ──► DEX-P1-005 (UI follows ADR)
DEX-P1-009 ──► v2 launch gate
DEX-P1-001 (shallow done) ──► #102 deep book; DEX-P1-010 / #72, #103 (E2E strict path)
DEX-P1-007, DEX-P1-008 ──► hybrid launch confidence
```

---

## Suggested first sprint (issues to create in GitLab)

Many items above already have GitLab IIDs — see [GLAB_ISSUES.md](./GLAB_ISSUES.md). For **new** work, prioritize:

1. DEX-P1-009  
2. DEX-P1-003  
3. DEX-P1-005  
4. DEX-P1-006  
5. ~~DEX-P1-001~~ → shallow done; use **#102** for depth  
6. DEX-P0-EPIC (**#56**, **#108**)  
7. DEX-P1-007  
8. DEX-P1-008  
9. DEX-P2-005  
10. DEX-P2-009  
