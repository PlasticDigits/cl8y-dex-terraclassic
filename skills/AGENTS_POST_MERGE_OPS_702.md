# Agent playbook: post-merge !476 leftover verify (GitLab #702)

Audience: third-party agents verifying Coolify frontend rebuild + `/trade` visual QA after [!476](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/merge_requests/476) (#693 `/trade` ticket Market default + flatten) landed on `main` (`e6ddbf1d`). Child `make verify-issue-693` already existed on the merge commit.

**Issue:** [GitLab **#702**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/702)  
**Parent (closed unless a merged invariant is wrong):** [#693](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/693).  
**Invariants:** [`docs/qa-invariants.md`](../docs/qa-invariants.md) **Q19** (**M702-1–M702-8**)  
**Verify:** `make verify-issue-702`

Indexer leftover for !477 (#692 Vol USD) stays on [#701](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/701) — this ticket is frontend `/trade` only. Sibling API4/FE-01 leftover is [#698](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/698). Do **not** reopen #693 for ops/QA. Do **not** wait on GitLab CI quota.

## Merged MR

| MR | Issue | Skill |
|----|-------|-------|
| !476 | #693 `/trade` Market default + flatten | [`AGENTS_FRONTEND_TRADE_TICKET_FLATTEN.md`](./AGENTS_FRONTEND_TRADE_TICKET_FLATTEN.md) |

Makefile conflict with !477 was unioned (`verify-issue-692` + `verify-issue-693`).

## Invariants (M702-1–M702-8)

| ID | Rule |
|----|------|
| **M702-1** | Local regression is `make verify-issue-702`, which runs children **693, 563, 653**. A child FAIL fails the stack. Live Coolify leftover probes SKIP unless `dex.cl8y.com` answers (FAIL when `VERIFY702_REQUIRE_LIVE=1` or `VERIFY702_IID=702`). Leftover Playwright SKIP unless LocalTerra is up (FAIL when `VERIFY702_REQUIRE_CHAIN=1`). |
| **M702-2** | Coolify **frontend rebuild** from `e6ddbf1d+`. No new indexer migration for this MR. Live Trade chunk includes `trade-order-text-tab` / `trade-order-tab-market` / `trade-order-mode-docs`. |
| **M702-3** | Fresh `/trade/:pair` is **Market** (`trade-order-tab-market` `aria-selected="true"`). Compact underlined tabs (not `tab-glass*` pills). Heading TokenLogo + clamped non-orange wash. No Side / Top buy `TicketSection`. |
| **M702-4** | Limit still reachable; book **Edit** / **Place another** stay Limit. Slippage chips only after opening Advanced; closed Advanced still applies store **5%** `max_spread`. `/limits` place card stays Limit-only with chips beside the price input. |
| **M702-5** | Light + dark at ~375px and ~1280px (issue **10.2.19**). Escrow mapping, GET `/route/solve`, hybrid always-on, footer CTA dock, and on-chain `side` stay unchanged. |
| **M702-6** | Do **not** reopen #693 for ops/QA. Do **not** wait on GitLab CI. Do **not** restyle `/limits` place card. |
| **M702-7** | Optional LocalTerra leftover Playwright is `e2e/trade-page-responsive.spec.ts` at **5 workers** when `VERIFY702_LEFTOVER_E2E=1` or chain is required. Expiry measurement opens Advanced first. P1 T527-1 overlap is clipped to **visible** `trade-order-ticket-scroll` (do **not** restyle the footer). Do **not** leak a non-3173 `PLAYWRIGHT_WEB_PORT` into children (CORS). `e2e-tx` stays **1 worker**. |
| **M702-8** | This playbook + **Q19** + child skills stay crosslinked. GitLab CI quota is not a substitute for local verify. |

## Coolify leftovers (operator)

Rebuild frontend from current `main`. Then walk `/trade/:pair` (issue **10.2.19**): Market default, compact tabs, heading logo + non-orange wash, Limit still reachable, slippage under Advanced.

`make verify-issue-702` records leftover probes as SKIP unless `dex.cl8y.com` answers. Fail closed with `VERIFY702_REQUIRE_LIVE=1`.

## Do / don’t

- **Do** run `make verify-issue-702` from a git worktree after pulling `main`.
- **Do** link `frontend-dapp/node_modules` from the primary checkout in a git worktree. Do **not** `npm install` over a worktree symlink.
- **Don’t** reopen #693 unless a merged invariant is wrong.
- **Don’t** change escrow, hybrid, or footer dock to “fix” flatten leftovers.
- **Don’t** treat GitLab CI quota as leftover evidence.

## Regression

```bash
make verify-issue-702
VERIFY702_SKIP_CHILDREN=1 make verify-issue-702
VERIFY702_SKIP_LIVE=1 VERIFY702_SKIP_CHAIN=1 make verify-issue-702
VERIFY702_REQUIRE_LIVE=1 make verify-issue-702
VERIFY702_LEFTOVER_E2E=1 make verify-issue-702
```
