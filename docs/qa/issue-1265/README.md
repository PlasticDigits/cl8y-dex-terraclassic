# QA — Forgejo #1265 remaining GET `/route/solve` failures census

Implement PR: [#1289](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1289) (merged). Verify (no chain): `make verify-issue-1265` or `make verify-issue-1289` (alias).

Playbook: [`skills/AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md`](../../../skills/AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md) · ADR [`docs/adr/0007-route-solve-remaining-failures.md`](../../adr/0007-route-solve-remaining-failures.md) · invariants **R-CENSUS-1–R-CENSUS-8**.

This research MR does **not** change indexer ranking, hop caps, cache buckets, wrap-mapping, or Swap/Trade quote branches.

## Closeout review — 2026-09-24

- Current `origin/main` verification: `make verify-issue-1265` passed **9/9**; route-solver docs/constants agree and no production quote-path files changed.
- Accept the 2026-09-21 public GET summaries and the explicit wallet-simulation **unmeasured** rows. That is the documented limit of this docs-only **Stay**, not a pending census action.
- PR [#1289](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1289) implements the census; post-merge tracker [#1300](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1300) is closed.
- Native wrap mapping / solver execution landed under [#1218](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1218) / PR #1297; any remaining native-swap verification stays separate there. Mixed-decimal [#1257](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1257) and execute-gas [#1264](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1264) are closed. Adjacent Swap route-display work is [#1330](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1330), separate from this census.

## Automated

- ADR 0007 has required headings, F0–F10 scored **Stay**, dated 2026-09-21 public solve summaries
- **R-CENSUS-1–R-CENSUS-8** present in ADR + skill
- Production `best_execution.rs` / `route_paths.rs` / `route_solver.rs` / `cw20RouteSolveQuote.ts` / `SwapPage.tsx` unchanged vs `main`
- `python3 scripts/check_route_solver_docs.py` still green
- #1203 / #690 / #1222 / #1264 named as out of census; #1218 remains the separate F0 follow-up; F1 resolved under closed #1257

## Manual / operator (not this verify)

1. Reviewers may re-fetch the cited public GET summaries (token ids + hops + `estimated_amount_out`); this closeout accepts the documented unmeasured wallet-simulation comparison.
2. Native denoms remain invalid direct indexer graph ids; the dApp wrap-map/solver implementation landed under **#1218** / PR #1297. Any remaining native-swap check stays there, not in this census or as a reason to bump `MAX_PATH_CANDIDATES`.
3. USTR→USDT mixed-decimal honesty was resolved under closed **#1257** (`make verify-issue-1257`).
4. Do **not** open an implement issue unless the ADR decision stops being Stay.

## Out of scope here

- Wrap-enter mapping or hop-count ranking ([#1218](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1218)).
- USTR→USDT decimal/scale honesty ([#1257](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1257)).
- Wrap+2hop execute gas (closed [#1264](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1264)) / retail gas census (closed [#1222](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1222)).
- Parallel split routing ([#1203](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1203)).
- Other-DEX v2 hops ([#690](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/690)).
