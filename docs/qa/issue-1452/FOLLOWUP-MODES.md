# Issue #1452 follow-up: exact local execution modes

This supplements [agent verification](README.md), the [solver contract](../../route-solver.md#local-quote-contract-1452)
and [LQ1452](../../../skills/AGENTS_LOCAL_SWAP_QUOTES.md). It records source
scope; it does not establish a deployed revision or complete issue acceptance.

## Mode invariants

- Deducting-hook receive sizing inverts each hook's independent integer floor
  against the original post-commission ask amount. Several floors can make net
  output decrease at a rounding collision; binary search on net output is invalid.
  Observation-time range-max tables cover the 10,000 integer residues. The first
  eligible quotient and first-reaching residue give the minimal qualifying gross
  output. AMM output can jump over that residue, so exact local pool output is
  checked and the next qualifying original output above a failed reachable value
  is considered. At most 64 refinements per hop are allowed; exhaustion fails
  closed. Immutable shared tables make each lookup logarithmic, without chain
  queries or a request-time linear residue scan.
- POST `hybrid_by_hop` is an exact declared execution budget, not an optimization
  hint. A candidate must have the declared hop count and satisfy each hop's
  actual incoming budget; stale/mismatched budgets, unsupported hints, overflow
  and out-of-range maker limits fail closed. Null entries are pool-only.
  Hook deductions, taxes, L9 spread and positive per-hop/final floors still apply.
  A taxed gross-ratio declaration is rescaled only by the exact v1 router rule.
- Manual result identity binds the entire declared plan, including every maker
  cap, plus namespace, amount, protection, actor and observation generation.
  A background improvement cannot replace a manually reviewed plan. Book
  expiration from every declared hop bounds the result's lifetime.
- Native↔its own wrapped token is a zero-pair mapping. Local forward and receive
  sizing retain the mapper fee, actual capped bank tax, independently fresh fair
  reference, config/generation, rate budget and positive wallet/base floors.
  It does not fabricate an intermediate pool route to quote a direct mapping.

## Direct zero-hop signing remains unsupported

Current external treasury `WrapDeposit {}` and mapper `Unwrap { recipient }`
messages expose no minimum-output field. The DEX router rejects empty operations
in `execute_swap_operations` ([contract](../../../smartcontracts/contracts/router/src/contract.rs)).
Sending a reviewed floor as an ignored extra JSON field would not protect output.

Therefore these responses explicitly set `execution_metadata.execution_supported`
to **false**, carry the `wrapper_only` policy and report a single signing blocker.
The frontend may display the local result and use it to size a receive target;
Swap submission and native execution refuse it before broadcasting. Existing
pool-bearing native routes retain their signed floors. A compatible external
mapper/treasury minimum-output protocol or reviewed zero-hop router settlement
is needed before direct wrapping can become an executable local plan. This is an
implementation limitation, not a deployment or approval gate to waive.

## Focused evidence

Worktree baseline: `c131e63f6fcac6d5bcc85a524105c14ad6f46a9d` plus the
reviewable follow-up changes. Recorded UTC: `2026-10-09T12:57:35Z`.

| Check | Evidence and outcome |
| --- | --- |
| Hook inverse | `cargo test --locked --manifest-path indexer/Cargo.toml --lib local_hook_pricing -- --test-threads=1`: 8 PASS, including brute first-reaching comparison through 100,000 original outputs, lower-bound residue search, nonmonotone collisions, full deduction, overdraw and Uint128 bounds. |
| Client plans and zero-hop | Repository Node wrapper, Vitest `src/utils/cw20RouteSolveQuote.test.ts src/utils/localWrapperOnlyQuote.test.ts`: 22 PASS; exact POST controls, changed-plan rejection, both wrapper directions and receive sizing, zero provider/broadcast calls and explicit mutation rejection. |
| Local mode engine | `cargo test --locked --manifest-path indexer/Cargo.toml --lib local_quote:: -- --test-threads=1`: 45 PASS; exact pool-reachable hook refinement, insufficient composed inverse rejection/healthy alternate, manual multi-hop budgets, plan/maker cache identity, no manual deferred replacement, both wrapper directions and positive receive sizing blocked from signing. |
| API compatibility | `cargo test --locked --manifest-path indexer/Cargo.toml --test api_route_solve -- --test-threads=1`: 30 PASS (29 route criteria plus QA isolation guard). Explicit synthetic observations replace request-time mocks; independent integer pool/book fee, output and floor formulas cover short/long routes, cache/tier identity, input validation and zero-provider progress. Fixture namespace and missing progress-amount failures were corrected and retested. |
| Existing routed floor behavior | Vitest `src/utils/cw20RouteSolveQuote.test.ts src/utils/localRouteQuote.test.ts`: 28 PASS before adding the new manual case; focused final retest is recorded in [RESULTS](RESULTS.md). |
| Type validation | Repository Node wrapper `tsc -b --pretty false`: PASS after mode/types changes. |

The final integrated verifier and corrected/retested backend mode counts belong
in [RESULTS](RESULTS.md); earlier successful runs do not cover later edits.

## Agent review checklist

- [ ] Run `make verify-issue-1452`, including the wrapper-only client corpus.
- [x] Inspect `inverse_is_exact_across_nonmonotone_individual_floor_collisions`
  and `cached_hooks_deduct_before_positive_floors_and_inverse_targets_delivered_net`:
  verify delivered net satisfies target and smaller supported offers do not.
  Include `inverse_refines_when_mixed_decimal_pool_jumps_over_hook_qualified_residue`
  and `inverse_from_skips_unreachable_residues_without_assuming_monotone_net`;
  a pool jump across coincident hook floors must refine rather than lose a feasible
  candidate. Whole-route forward scoring remains mandatory after composition.
  `composed_hook_inverse_rejects_insufficient_candidate_and_preserves_healthy_alternate`
  covers upstream overshoot into a downstream rounding dip: reject the insufficient
  composed candidate and preserve a sufficient healthy alternate. This bounded
  inverse does not claim complete composed-model or global route optimality.
- [ ] Inspect manual split tests: exact multi-hop budgets, mismatched plan errors,
  maker-cap cache identity, no deferred replacement and expired books on any hop.
- [x] Trace typed direct Swap/Trade book input through POST and compare quoted
  operations with the exact plan retained for submission. A changed plan must fail.
- [x] Test direct native wrap/unwrap forward and receive quotes with every provider
  query throwing. Confirm positive useful local result, compatible generation and
  `execution_supported:false`; assert no broadcast on submission.
- [ ] Keep direct zero-hop executable acceptance unchecked until its contract
  protocol enforces the reviewed floor; do not replace it with a floorless send.
- [ ] Retest corrections and append revision, UTC time, command/counts and exact
  skipped scope to [RESULTS](RESULTS.md).

An agent unable to complete a check may mark it **SKIP** with a concrete reason
and use overall **PASS WITH SKIPS** if all executed assertions pass. Leave the
skipped criterion unchecked. An executed failure stays **FAIL**; skipping a
workflow gate never disables runtime protection or proves deployed acceptance.
