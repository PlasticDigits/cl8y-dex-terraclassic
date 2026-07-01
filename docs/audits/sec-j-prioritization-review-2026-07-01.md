# SEC-J Prioritization Review -- Sub-$100k TVL Launch

**Date:** 2026-07-01
**Reviewer:** totdking (QA)
**Scope:** All 62 filed issues (help/issues.md, help/issues2.md) and all uncovered checklist items SEC-A through SEC-I
**GitLab tracking issue:** #464

---

## Summary

Prioritization review of all filed issues and uncovered checklist items against the sub-$100k TVL launch criteria in SEC-J01 through SEC-J05.

- All 11 P0 launch blockers are confirmed closed as of 2026-07-01
- All filed P1 finding issues are confirmed closed as of 2026-07-01
- Five parent doc log reviews (#446, #447, #452, #453, #454) remain open pending QA verification
- 22 items accepted as low-ROI at current TVL with documented rationale and revisit thresholds
- 15 checklist items not yet reviewed; deferred to post-$100k TVL or pending environment availability
- Current launch decision: GO with accepted risk

---

## J01: P0 Launch Blockers

P0 criteria: admin controls broken, value-flow invariant failing, deploy/runbook missing, or user cannot see pause/blacklist/rate-limit risk.

All 11 P0 items confirmed closed before this review was written.

| Checklist | Issue | Status | Category |
|-----------|-------|--------|---------|
| SEC-A06 | #391 | CLOSED | Deploy/runbook -- no go/no-go section or mandatory signoff gate in launch runbook |
| SEC-B05 | #395 | CLOSED | User cannot see risk -- SwapPage and PoolPage did not query pair pause state |
| SEC-B09 | #397 | CLOSED | Admin controls -- no evidence emergency controls executable from actual governance multisig |
| SEC-C01 | #401 | CLOSED | Value-flow invariant -- swap math missing edge-reserve and u128 boundary tests |
| SEC-C04 | #404 | CLOSED | Value-flow invariant -- min_return enforcement untested on router and hybrid swap paths |
| SEC-C14 | #405 | CLOSED | Deploy/runbook -- migration tests covered only version validation, not state preservation |
| SEC-E01 | #425 | CLOSED | User cannot see risk -- PoolPage and LimitOrdersPage had no blacklist gating |
| SEC-H03 | #441 | CLOSED | Deploy/runbook -- no automated post-deploy contract config assertion script |
| SEC-H04 | #442 | CLOSED | Deploy/runbook -- no env-to-chain address cross-check script |
| SEC-H08 | #444 | CLOSED | Deploy/runbook -- release checklist did not gate deploy on pasted test outputs |
| SEC-H09 | #445 | CLOSED | Deploy/runbook -- no rollback/forward-fix decision tree |

---

## J02: P1 Must-Fix-Before-Growth

All filed P1 finding issues confirmed closed as of 2026-07-01.

### Remaining open -- QA verification pending

Five parent doc log reviews are open and labeled `block:log_only`. All sub-issues (finding issues) under each parent are closed. These do not block launch.

| Issue | Checklist | Description |
|-------|-----------|-------------|
| #446 | SEC-I01 | LLM attacker prompt pack -- QA to verify and close |
| #447 | SEC-I02 | LLM hypothesis conversion -- QA to verify and close |
| #452 | SEC-I03 | Admin controls, blacklist/pause coverage, and value-flow bug review -- QA to verify and close |
| #453 | SEC-I04 | Indexer/API abuse and data integrity review -- QA to verify and close |
| #454 | SEC-I05 | Frontend signing and risk communication review -- QA to verify and close |

### Not-yet-reviewed items (deferred to post-$100k TVL)

| Checklist | Description |
|-----------|-------------|
| SEC-C02 | LP share accounting exploit-style regression test; whitelist-only admission reduces risk |
| SEC-C12 | Limit order escrow theft and stranding paths under fill/cancel/claim/pause/blacklist combinations |
| SEC-C13 | Hybrid swap bounded maker fills; max-fill and scan-budget enforcement under load |
| SEC-D06 | Malicious CW20 behavior: transfer failure, fee-on-transfer, rebase balance; whitelist mitigates |
| SEC-D08 | SQL injection modeling for public API sort, interval, ticker, and pair query routes |
| SEC-E02 | Pair pause banner visibility tests and action-specific copy assertions |
| SEC-E06 | Extreme slippage block unless Expert Mode explicitly enabled |
| SEC-F01 | Production API rate limits non-zero by default and not accidentally disabled |
| SEC-F06 | Internal DB and LCD errors sanitized; no SQL, stack trace, or secret leakage in 500/502 responses |
| SEC-F08 | Block ingest fails safe on bad timestamps, pagination gaps, DB errors, and reorg hash mismatch |
| SEC-F09 | Swap/fill/lifecycle dedup prevents replay and double-counting |
| SEC-F10 | Hybrid volume reconciliation does not double-count maker fills as headline volume |
| SEC-G08 | No reliance on chain halt as primary dApp incident control |
| SEC-I06 | LLM-generated exploit scripts cannot overwhelm LocalTerra/API without hitting rate limits (pending LocalTerra env) |
| SEC-I07 | Prompt pack and exploit matrix update cadence added to release process checklist |

---

## J03: Accepted Low-ROI Items (sub-$100k TVL)

Items accepted as out of scope for a $100k TVL launch. Each has a revisit threshold.

| Checklist | Rationale | Revisit At |
|-----------|-----------|------------|
| SEC-A04 | Tier 255 vs blacklist distinction is a docs-only gap; existing contract tests cover the behavior | $250k TVL |
| SEC-A05 | Indexer breach simulation; threat model documented; mitigated by FACTORY_ADDRESS provenance check | $250k TVL |
| SEC-A08 | No public changelog or security advisory format; low user expectation at $100k TVL | $250k TVL |
| SEC-A09 | No token vetting program; whitelist-only admission is the primary control at current TVL | $250k TVL |
| SEC-B05-partial | Indexer pause-sync lag window; pair contract is authoritative; lag is informational | $250k TVL |
| SEC-C02 | Osmosis-class LP share exploit style test; whitelist mitigates main attack vector | $250k TVL |
| SEC-C05 | Deadline arithmetic overflow protection beyond contract-native enforcement; no practical u64 overflow risk | $250k TVL |
| SEC-C06 | CW20 code ID whitelist blocks malicious tokens; whitelist enforcement tested; no issue filed; PASS | No threshold |
| SEC-C07 | Pair creation fee spam gate exists; minimal griefing incentive at $100k TVL | $250k TVL |
| SEC-C08 | Hooks are governance-only and bounded; covered by governance auth tests; no issue filed; PASS | No threshold |
| SEC-C09 | Router trusted-trader fee discount spoofing; minimal financial incentive at current discount tiers | $250k TVL |
| SEC-C10 | Fee discount EOA-only self-registration; accepted at $100k TVL | $250k TVL |
| SEC-C11 | Lazy deregistration stale discount bounded to one swap; accepted exposure | $250k TVL |
| SEC-C15 | No unbounded iteration on hot paths; list sizes bounded at current TVL; code review noted | $250k TVL |
| SEC-D03 | No IBC hooks callbacks; Terra Classic contracts do not use IBC hooks; not applicable | No threshold |
| SEC-D04 | Osmosis LP share accounting replay; basic add/remove tests exist; whitelist-only admission mitigates | $250k TVL |
| SEC-D05 | Oracle manipulation via stale price; no external oracle used in settlement; not applicable | No threshold |
| SEC-D07 | Fake pair or fake event to indexer; FACTORY_ADDRESS provenance check enforced; no issue filed; PASS | No threshold |
| SEC-D09 | LCD amplification by LLM agents; rate limits are primary mitigant; accepted pending SEC-I06 result | SEC-I06 completion |
| SEC-E03 | Wrap pause UI copy; partial e2e coverage exists; wrap mapper is external; accepted with existing test | $250k TVL |
| SEC-E09 | Risk acknowledgement modal; existing UI copy sufficient for launch at $100k TVL | $250k TVL |
| SEC-F02 | LCD-heavy routes stricter rate limits; minimal concurrent user load at $100k TVL | $250k TVL |
| SEC-F03 | Socket peer IP only; no reverse proxy in current deploy; accepted if no proxy is used | At proxy introduction |
| SEC-F04 | IPv4-only or documented IPv6 anti-abuse; accepted if deployment is IPv4-only | At IPv6 introduction |
| SEC-F07 | CORS allowlist; accepted if production deploy uses a dedicated domain with explicit config; verify at deploy | No threshold |
| SEC-F11 | Factory pair provenance; FACTORY_ADDRESS required in prod; provenance check enforced in indexer; PASS | No threshold |

**PASS items verified, no issue filed:**

| Checklist | Evidence |
|-----------|---------|
| SEC-B01 | Governance-only blacklist access enforced; auth tests pass |
| SEC-B03 | Token blacklist blocks both directions; bidirectional test coverage confirmed |
| SEC-B07 | Governance rotation for treasury, fee, hooks, and routers covered by governance auth matrix tests |
| SEC-B08 | LP admin rotation bounded and paginated; issue #277 closed invariant tests confirmed |
| SEC-H01 | Production deploy runbook exists and can be followed |
| SEC-H02 | LocalTerra deploy exercises pair creation fee and admin controls |
| SEC-H06 | Secrets not committed; CI/CD variable scoping verified |
| SEC-H07 | Dependency audit completed proportionately |

---

## J04: TVL and User Thresholds

Actions required when these thresholds are crossed.

**By TVL:**

| Threshold | Required Actions |
|-----------|-----------------|
| Sub-$100k (current) | All P0 items closed or risk-accepted; P1 items tracked; accepted items documented |
| $100k to $250k | All P1 items closed; formal security audit engagement started; production monitoring with alerting active |
| $250k to $1M | Formal audit complete and findings resolved; multisig or DAO governance upgrade; bounty program active; SOC-2 evaluation started; all accepted items re-evaluated |
| Over $1M | Annual third-party audit cycle; dedicated incident response team; documented on-call rotation; enterprise security monitoring |

**By active user count:**

| Threshold | Required Actions |
|-----------|-----------------|
| Under 100 users (current) | Current posture acceptable with P0 items resolved |
| 100 to 500 users | All P1 items closed; rate limit configuration tuned to observed traffic; monitoring active |
| 500 or more users | Full formal audit; dedicated incident response; documented on-call rotation; anomaly alerting live |

Whichever threshold is crossed first triggers the corresponding requirements.

---

## J05: Prelaunch Signoff

**Current status:** GO with accepted risk (all P0 and P1 finding issues confirmed closed as of 2026-07-01)

Before any mainnet deploy, the developer or named launch lead must post a comment on the J05 tracking issue (#472) containing:

- Decision: GO, PAUSE, or BLOCK
- Date (UTC)
- P0 completion status: each item listed as closed (with issue link) or risk-accepted (with rationale)
- Residual risks: P1 items remaining open at launch with rationale
- P1 next steps: confirm follow-up issues exist for all deferred items

**Pending before signoff can be posted:**
- QA verification and closure of #446, #447, #452, #453, #454 (block:log_only doc log reviews)
- J05 GitLab issue closed by QA after developer posts the formal signoff comment

---

## Verification Checklist

- [x] All 11 P0 items confirmed closed as of 2026-07-01
- [x] All filed P1 finding issues confirmed closed as of 2026-07-01
- [ ] Five doc log issues (#446, #447, #452, #453, #454) verified and closed by QA
- [x] Accepted low-ROI items have rationale and revisit thresholds documented
- [x] TVL and user thresholds reviewed and documented
- [ ] Prelaunch signoff posted by developer with GO/PAUSE/BLOCK decision before mainnet deploy
- [ ] J05 GitLab issue closed by QA after signoff is verified
