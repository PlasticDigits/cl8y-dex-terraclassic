# QA — Forgejo #1265 remaining GET `/route/solve` failures census

Verify (no chain): `make verify-issue-1265`

Playbook: [`skills/AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md`](../../../skills/AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md) · ADR [`docs/adr/0007-route-solve-remaining-failures.md`](../../adr/0007-route-solve-remaining-failures.md) · invariants **R-CENSUS-1–R-CENSUS-8**.

This research MR does **not** change indexer ranking, hop caps, cache buckets, wrap-mapping, or Swap/Trade quote branches.

## Automated

- ADR 0007 has required headings, F0–F10 scored **Stay**, dated 2026-09-21 public solve summaries
- **R-CENSUS-1–R-CENSUS-8** present in ADR + skill
- Production `best_execution.rs` / `route_paths.rs` / `route_solver.rs` / `cw20RouteSolveQuote.ts` / `SwapPage.tsx` unchanged vs `main`
- `python3 scripts/check_route_solver_docs.py` still green
- #1203 / #690 / #1222 / #1264 named as out of census; #1218 / #1257 stay owners of F0 / F1

## Manual / operator (not this verify)

1. Reviewers can re-fetch the cited public GET summaries (token ids + hops + `estimated_amount_out`) or treat a family as **unmeasured**.
2. Native wrap-enter still skips solve — that is **#1218**, not a reason to bump `MAX_PATH_CANDIDATES` here.
3. USTR→USDT mixed-decimal honesty stays **#1257** (`make verify-issue-1257`).
4. Do **not** open an implement issue unless the ADR decision stops being Stay.

## Out of scope here

- Wrap-enter mapping or hop-count ranking ([#1218](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1218)).
- USTR→USDT decimal/scale honesty ([#1257](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1257)).
- Wrap+2hop execute gas ([#1264](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1264)) / retail gas census ([#1222](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1222)).
- Parallel split routing ([#1203](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1203)).
- Other-DEX v2 hops ([#690](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/690)).
