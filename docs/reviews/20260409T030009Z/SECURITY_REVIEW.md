# Security review (governance trusted)

**Scope:** `cl8y-dex-terraclassic` — Terra Classic DEX contracts, indexer HTTP API, frontend. **Out of program scope:** bridge / cross-chain (not integrated here; no bridge-specific review items).

**Assumption:** governance multisig is honest but may make operational mistakes. Findings distinguish “unsafe even if governance is honest” (implementation bugs) from “footgun under trusted governance” (misconfiguration).

---

## Smart contract / protocol

### SC-01 — Hybrid hook fee under-reporting (integrator risk)

| Field | Detail |
|-------|--------|
| **Severity** | medium (integrity / accounting visibility, not fund loss on pair) |
| **Components** | `pair` `execute_swap`, post-swap hooks |
| **Evidence** | Invariant **L7** [`docs/contracts-security-audit.md`](../../contracts-security-audit.md); `commission_amount` in hook only reflects pool leg ([`contract.rs`](../../../smartcontracts/contracts/pair/src/contract.rs) ~882–899). |
| **Failure scenario** | External protocol assumes `commission_amount` is total protocol fee; under-accrues revenue share or mis-reports to users. |
| **Gov trust** | Does not reduce severity for **integrators**; on-chain users still receive correct net tokens from pair. |
| **Remediation** | Document; optionally extend hook schema with `book_commission` / attributes aggregation. **DEX-P2-005**. |

### SC-02 — Quote / execution mismatch for hybrid and router simulate

| Field | Detail |
|-------|--------|
| **Severity** | high (economic / UX) |
| **Components** | pair queries, router `query_simulate_swap_operations` |
| **Evidence** | Router comments [`router/src/contract.rs`](../../../smartcontracts/contracts/router/src/contract.rs) ~428–497; audit **L8**. |
| **Failure scenario** | User or bot simulates expected out, executes hybrid, receives different amount → MEV/slippage confusion, failed arb strategies. |
| **Gov trust** | Not a governance malice issue; protocol design + docs. |
| **Remediation** | New query path, off-chain book simulator, or hard UI warnings. **DEX-P1-003**, **DEX-P1-005**. |

### SC-03 — Pause freezes limit cancels

| Field | Detail |
|-------|--------|
| **Severity** | medium (liveness / user expectation) |
| **Components** | pair pause, `CancelLimitOrder` |
| **Evidence** | **L6** [`docs/contracts-security-audit.md`](../../contracts-security-audit.md), [`docs/limit-orders.md`](../../limit-orders.md). |
| **Failure scenario** | Market stress → governance pauses → makers cannot exit resting orders until unpause. |
| **Gov trust** | Intentional policy; **footgun** if ops unaware. |
| **Remediation** | Runbook + UI copy. **DEX-P2-012**. |

### SC-04 — Trusted `trader` and discount theft prevention

| Field | Detail |
|-------|--------|
| **Severity** | note (well covered) |
| **Components** | `fee-discount` `GetDiscount { trader, sender }`, pair |
| **Evidence** | **P6** audit; [`docs/security-model.md`](../../security-model.md). |
| **Failure scenario** | Untrusted router could spoof trader if check wrong — **tests exist** (`fee_discount_coverage_tests`). |

### SC-05 — Orderbook DoS / gas bounds

| Field | Detail |
|-------|--------|
| **Severity** | low |
| **Components** | `orderbook.rs`, `MAX_ADJUST_STEPS_HARD_CAP`, `MAX_MAKER_FILLS_HARD_CAP` in [`dex-common/pair.rs`](../../../smartcontracts/packages/dex-common/src/pair.rs) |
| **Evidence** | Insert walk capped; match capped (audit **L5**). |
| **Failure scenario** | User sets extreme `max_adjust_steps` → placement error (bounded). |

### SC-06 — Rounding / fee-on-transfer / reserve desync

| Field | Detail |
|-------|--------|
| **Severity** | medium (known class) |
| **Components** | pair reserves vs CW20 balance |
| **Evidence** | Audit **P2**; sweep path; adversarial token tests referenced in audit matrix. |

### SC-07 — Hook reentrancy / griefing

| Field | Detail |
|-------|--------|
| **Severity** | medium if malicious hook registered |
| **Components** | hooks |
| **Evidence** | Actor model + **H1**; governance registers hooks. |
| **Gov trust** | Primary control knob. |

### SC-08 — Slippage assertion scope on hybrid

| Field | Detail |
|-------|--------|
| **Severity** | medium |
| **Components** | `assert_max_spread` in `execute_swap` |
| **Evidence** | Pool leg uses spread check ([`contract.rs`](../../../smartcontracts/contracts/pair/src/contract.rs) ~835–842); book leg adds to `total_return` — users rely on `max_spread` relative to combined path per implementation (verify when changing). |

---

## Backend / indexer

### IX-01 — Read-only API surface

| Field | Detail |
|-------|--------|
| **Severity** | note |
| **Evidence** | [`docs/indexer-invariants.md`](../../indexer-invariants.md) — GET only, parameterized SQL, sort allowlists. |

### IX-02 — Internal error sanitization

| Field | Detail |
|-------|--------|
| **Severity** | note |
| **Evidence** | `internal_err` in [`indexer/src/api/mod.rs`](../../../indexer/src/api/mod.rs). |

### IX-03 — Chain reorg / tx reorder

| Field | Detail |
|-------|--------|
| **Severity** | medium (operational) |
| **Evidence** | Indexer polls LCD; swap dedup by `(tx_hash, pair_id)` per indexer invariants — **reorg** handling not documented in repo. |
| **Remediation** | Runbook: replay/backfill strategy. **DEX-P2-016**. |

### IX-04 — Block time parse failure

| Field | Detail |
|-------|--------|
| **Severity** | low |
| **Evidence** | Indexer invariants: warn + `Utc::now()` fallback skews candles. |

### IX-05 — Route solver vs execution drift

| Field | Detail |
|-------|--------|
| **Severity** | high |
| **Evidence** | `route_solver.rs` pool-only + optional router simulate; hybrid patched client-side. |

---

## Frontend / client

### FE-01 — Hybrid without quote parity

| Field | Detail |
|-------|--------|
| **Severity** | high |
| **Evidence** | [`SwapPage.tsx`](../../../frontend-dapp/src/pages/SwapPage.tsx) constructs `hybrid`. |
| **Remediation** | Warnings + link to docs. **DEX-P1-005**. |

### FE-02 — Wrong chain / wallet

| Field | Detail |
|-------|--------|
| **Severity** | medium |
| **Evidence** | `VITE_NETWORK` [`docs/frontend.md`](../../frontend.md); standard wallet responsibility. |

### FE-03 — Approvals

| Field | Detail |
|-------|--------|
| **Severity** | medium |
| **Evidence** | CW20 allowances for pair/router; pool page patterns. |

---

## Summary table

| ID | Severity | Governance mitigates? |
|----|----------|------------------------|
| SC-01 | medium | No (integrator doc) |
| SC-02 | high | No |
| SC-03 | medium | Partially (policy) |
| SC-04 | note | N/A |
| SC-05 | low | N/A |
| SC-06 | medium | Partially (whitelist) |
| SC-07 | medium | Yes (don’t register bad hooks) |
| SC-08 | medium | No |
| IX-03 | medium | No |
| IX-05 | high | No |
| FE-01 | high | No |

---

*Bridge / cross-chain security topics: **out of program scope** for this repository.*
