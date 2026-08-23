# Agent playbook: post-merge !400 LocalTerra E9 + columbus-5 unwrap gas (GitLab #600)

Audience: third-party agents verifying the **unwrap+≥2hop USTR→USTC** envelope after [!400](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/merge_requests/400) landed on `main` without chain QA (implementation workspace had no LocalTerra; GitLab CI died on `ci_quota_exceeded`).

**Issue:** [GitLab **#600**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/600)  
**Parent:** [#599](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/599) (closed — do **not** reopen unless 3.11M still OOGs)  
**Invariants:** [`docs/qa-invariants.md`](../docs/qa-invariants.md) **Q8** (**M600-1–M600-8**)  
**Verify:** `make verify-issue-600`  
**Child gas playbook:** [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md) (§14c / `#599`)  
**Wrap combo (still one-tx):** [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md) (§14b / `#587`) · post-merge [`AGENTS_POST_MERGE_OPS_590.md`](./AGENTS_POST_MERGE_OPS_590.md)

Unit + inventory for `UNWRAP_ROUTER_COMBO_OVERHEAD_GAS` (400k → combined **3,110,000**) are already on `main`. This ticket is the **live** LocalTerra E9 + columbus-5 USTR→USTC rung.

## Invariants (M600-1–M600-8)

| ID | Rule |
|----|------|
| **M600-1** | Local regression is `make verify-issue-600`, which runs children **599** and **587**. A child FAIL fails the stack. E9 SKIP (no LocalTerra) is allowed unless `VERIFY600_REQUIRE_CHAIN=1`. |
| **M600-2** | **P599-1.** `CI=1` Playwright `e2e/wrap-swap.spec.ts --project=e2e-tx` **E9** (USTR or JADE/RUBY → USTC). One submit; no OOG toast; LCD `gas_used < gas_wanted`. **1 worker** (shared LocalTerra account). |
| **M600-3** | **P599-3.** LUNC→USTR and USTC→USTR stay wrap+combo one-tx (#587). E7 when chain is up. Do not tell users to hop manually. |
| **M600-4** | **P599-4.** Direct mapper unwrap (cUSTC→USTC / cLUNC→LUNC) stays **`UNWRAP_GAS_LIMIT` (800k)**. Do not raise that floor for the hub InstantWithdraw path. |
| **M600-5** | Envelope is **3,110,000** (~88.09 LUNC at 28.325) — tens-of-LUNC class, not hybrid 15M. **P599-5:** if a captured columbus-5 `gasUsed` ≥ 3.11M, open a **new** envelope ticket. Do **not** silently bump `UNWRAP_GAS_LIMIT`. |
| **M600-6** | **P599-2.** Columbus-5 `/` USTR→USTC is operator-run (funded Station/Keplr). Route must show the hub unwrap path; Network fee ~88 LUNC class. Record hash + `gasWanted` / `gasUsed` via `VERIFY600_COLUMBUS_TX=<hash>`. Do **not** attach hybrid / `book_input` (**H596-7**). |
| **M600-7** | Do **not** reopen [#599](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/599) unless the 3.11M envelope still OOGs. File a new ticket if the named combo must rise. |
| **M600-8** | This playbook + **Q8** + child skills stay crosslinked. Do not wait for GitHub Actions; GitLab CI may be quota-blocked — local `make verify-issue-*` is the gate. |

## Do / don’t

- **Do** run `make verify-issue-600` from a git worktree after pulling `main`.
- **Do** `make setup-cloud-localterra` when E9 SKIP and the issue still has **P599-1** open. Then `VERIFY600_REQUIRE_CHAIN=1 make verify-issue-600`.
- **Do** keep `executeNativeSwap` / router `unwrap_output` as one tx. Raise **`UNWRAP_ROUTER_COMBO_OVERHEAD_GAS`** (not `UNWRAP_GAS_LIMIT`) if measured used exceeds 3.11M.
- **Don’t** treat green `make verify-issue-599` (Vitest + docs) as E9 or columbus-5 clearance.
- **Don’t** attach hybrid / `book_input` to wrap or unwrap native paths (**H596-7**).
- **Don’t** close #600 while **M600-2** (E9) remains unexecuted if the issue still has that checkbox open — run with `VERIFY600_REQUIRE_CHAIN=1`.
- **Don’t** close #600 while **M600-6** (columbus-5 hash) is missing unless an operator comment records `gas_used < gas_wanted` on a live USTR→USTC swap.

## Regression

```bash
make verify-issue-600
# docs + children only (no Playwright):
VERIFY600_SKIP_E2E=1 make verify-issue-600
# after LocalTerra:
make setup-cloud-localterra
VERIFY600_REQUIRE_CHAIN=1 make verify-issue-600
# optional columbus-5 LCD record (P599-2 / P599-5):
VERIFY600_COLUMBUS_TX=<hash> make verify-issue-600
```

Child playbooks: [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md) (#599 / #587), [`AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md`](./AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md) (**H596-7**), [`AGENTS_POST_MERGE_OPS_590.md`](./AGENTS_POST_MERGE_OPS_590.md) (E7/E8 wrap combo).

E2E stand-in: LocalTerra may lack USTR — JADE/RUBY → USTC is the documented ≥2-hop path (`wrap-swap.spec.ts` E9).
