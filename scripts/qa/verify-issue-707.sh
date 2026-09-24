#!/usr/bin/env bash
# Static documentation verification for Forgejo #707.
# No contract build, frontend test, LocalTerra, or chain migration is required.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0

run_step() {
  local label="$1"
  shift
  if "$@"; then
    printf '[PASS] %s\n' "$label"
    PASS=$((PASS + 1))
  else
    printf '[FAIL] %s\n' "$label" >&2
    FAIL=$((FAIL + 1))
  fi
}

run_step "integrator guide is first-screen" python3 -c 'from pathlib import Path; s=Path("docs/integrators.md").read_text(); assert s.index("## Pair swaps: pool-only or best execution (I707)") < s.index("## Hybrid swaps and post-swap hooks")'
run_step "omitted hybrid and greedy means pool-only" grep -Fq 'omitting both `hybrid` and `greedy` is pool-only' docs/integrators.md
run_step "best-execution GET requires amount_in and returns executable operations" grep -Fq 'amount_in=…' docs/integrators.md
run_step "single-hop pair-direct keeps solver hybrid" grep -Fq "one-hop result can execute pair-direct only when it carries that hop's returned" docs/integrators.md
run_step "Pattern C quote and execute share params; no guessed split" grep -Fq 'Never invent a split' docs/integrators.md
run_step "pool-only query and removed Simulation are explicit" grep -Fq 'legacy pair `Simulation` is removed' docs/integrators.md
run_step "arb does not retroactively improve a pool-only taker" grep -Fq 'already accepted the pool price' docs/integrators.md
run_step "current greedy and #718 future handoff are distinct" grep -Fq 'future default change' docs/integrators.md
run_step "all I707 invariants are present" grep -Fq 'I707-7' docs/integrators.md
run_step "architecture marks the displayed flow pool-only" grep -Fq 'This sequence is the pool-only path' docs/architecture.md
run_step "limit-orders Pattern C warns pair-direct callers" grep -Fq 'Integrator warning (I707)' docs/limit-orders.md
run_step "ADR records pair-direct integrator semantics" grep -Fq 'Pair-direct integrator semantics' docs/adr/0001-hybrid-quoting-and-routing.md
run_step "docs index links the I707 section and agent skill" grep -Fq 'integrators.md#pair-swap-pool-only-vs-best-execution-forgejo-707' docs/README.md
run_step "route-solver identifies pair-direct consumers" grep -Fq 'Pair-direct integrators use this same best-execution API' docs/route-solver.md
run_step "hybrid quoting skill records the three execution choices" grep -Fq 'Integrator semantics (Forgejo #707)' skills/AGENTS_HYBRID_QUOTING.md
run_step "indexer best-execution skill points to I707" grep -Fq 'Pair-direct bot warning' skills/AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md
run_step "indexer HTTP skill points to execute guidance" grep -Fq 'pair-direct vs solver guidance' skills/AGENTS_INDEXER_HTTP_PACK.md
run_step "hybrid volume guide links execution choice" grep -Fq 'I707 pair-direct vs best-execution guidance' docs/integrators-hybrid-volume.md
run_step "testing matrix documents the docs-only verification" grep -Fq 'Pair-direct pool-only vs solver best execution' docs/testing.md
run_step "Makefile target and AGENTS instructions are present" bash -c 'grep -Fq "verify-issue-707:" Makefile && grep -Fq "make verify-issue-707" AGENTS.md'

if git status --short -- smartcontracts frontend-dapp/src | grep -q .; then
  printf '[FAIL] docs-only scope (contract or dApp source changed)\n' >&2
  FAIL=$((FAIL + 1))
else
  printf '[PASS] docs-only scope (no contract or dApp source changed)\n'
  PASS=$((PASS + 1))
fi

printf '\nResults: %d passed, %d failed\n' "$PASS" "$FAIL"
test "$FAIL" -eq 0
