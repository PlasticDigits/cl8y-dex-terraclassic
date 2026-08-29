# Agent playbook: post-merge !474/!475 leftover verify (GitLab #698)

Audience: third-party agents verifying the integrated tip after [!475](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/merge_requests/475) (#694 API4 per-request caps) and [!474](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/merge_requests/474) (#695 production `VITE_DEV_MODE` reject) landed on `main` (no CI wait, no automerge). Child `make verify-issue-694` / `make verify-issue-695` already existed on the merge commits.

**Issue:** [GitLab **#698**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/698)  
**Duplicates (close as dup of #698):** [#699](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/699), [#700](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/700).  
**Parents (closed unless a merged invariant is wrong):** [#694](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/694), [#695](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/695).  
**Invariants:** [`docs/qa-invariants.md`](../docs/qa-invariants.md) **Q17** (**M698-1–M698-8**)  
**Verify:** `make verify-issue-698` (aliases: `make verify-issue-699` / `make verify-issue-700`)

Sibling leftovers: [#701](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/701) (`/pool` Vol USD / !477), [#702](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/702) (`/trade` flatten / !476). Do **not** reopen #694 / #695 for ops/QA. Do **not** wait on GitLab CI quota.

## Merged MRs

| MR | Issue | Skill |
|----|-------|-------|
| !475 | #694 API4 per-request caps | [`AGENTS_INDEXER_API4_PER_REQUEST.md`](./AGENTS_INDEXER_API4_PER_REQUEST.md) |
| !474 | #695 production `VITE_DEV_MODE` reject | [`AGENTS_FRONTEND_DEV_MODE_GUARD.md`](./AGENTS_FRONTEND_DEV_MODE_GUARD.md) |

Tip also includes !477 / !476 (`e6ddbf1d+`). Coolify frontend rebuild from that tip covers #692 / #693 leftovers on [#701](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/701) / [#702](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/702).

## Invariants (M698-1–M698-8)

| ID | Rule |
|----|------|
| **M698-1** | Local regression is `make verify-issue-698`, which runs children **694** and **695**. A child FAIL fails the stack. Live Coolify leftover probes SKIP unless `indexer.dex.cl8y.com` / `dex.cl8y.com` answers (FAIL when `VERIFY698_REQUIRE_LIVE=1` or `VERIFY698_IID` is `698`/`699`/`700`). Leftover Playwright SKIP unless LocalTerra is up (FAIL when `VERIFY698_REQUIRE_CHAIN=1`). |
| **M698-2** | Coolify indexer is redeployed from current `main` (`3c4060ab+`, no new migration for this stack). Live `GET /api/v1/compliance/blacklist-check` with **17** tokens → **400** `tokens list exceeds max 16` (fail-closed, no truncate). At-cap (16) still reaches LCD (502 on bad addrs is OK). |
| **M698-3** | Live `/gt/events` stays bounded: a 2000-block window must not return multi-MB JSON or more than **5000** events without **400** `event count exceeds 5000`. GET still does **not** `SELECT` `pair_reserves`. Progress `GET /api/v1/route/solve/progress` answers JSON (`stage`) on the same binary as LCD-heavy routes. |
| **M698-4** | Coolify **frontend rebuild** from `489268eb+` (FE-01 in the bundle). Production env has **`VITE_DEV_MODE` unset**. Live `dex.cl8y.com` wallet chunk has **no** Simulated Wallet chrome. Staging / LocalTerra may keep the flag. |
| **M698-5** | dApp progress stays advisory (#484). Omit `trader` when the parent quote already resolved `discount_bps`. Consecutive poll failures back off (1s → 8s cap). Place / Swap stay ungated. Trade page still does not wire `useRouteSolveProgress` (Swap only — non-blocking). |
| **M698-6** | Do **not** reopen #694 / #695 for ops/QA. Do **not** wait on GitLab CI. Do **not** trust `X-Forwarded-For` / change prod RPS clamp. Do **not** fail-open blacklist. Do **not** change CSP, WalletConnect, clickwrap, or the runtime `devWallet.ts` gate. Do **not** commit `scripts/tmp-558-*`. |
| **M698-7** | Close RE-01 / RE-02 / RE-03 / FE-01 in [`audits/INTERNAL_GROK46_1787908099.md`](../audits/INTERNAL_GROK46_1787908099.md) **only after** Coolify/prod evidence (this leftover). In-repo `make verify-issue-694` / `695` is not a prod close. Optional leftover Playwright is Swap e2e-smoke at **5 workers** when `VERIFY698_LEFTOVER_E2E=1`. `e2e-tx` stays **1 worker**. Do **not** leak a non-3173 `PLAYWRIGHT_WEB_PORT` into children (CORS). |
| **M698-8** | This playbook + **Q17** + child skills stay crosslinked. `#699` / `#700` are duplicate leftovers of this ticket (`make verify-issue-699` aliases #698). GitLab CI quota is not a substitute for local verify. |

## Coolify leftovers (operator)

Redeploy indexer from current `main` (includes !475). Rebuild frontend; leave production `VITE_DEV_MODE` **unset**.

```bash
# blacklist oversize → 400
curl -sS -o /dev/null -w "%{http_code}\n" \
  "https://indexer.dex.cl8y.com/api/v1/compliance/blacklist-check?tokens=$(python3 -c 'print(",".join(["terra1x"]*17))')"
# expect 400

# GT window stays bounded (over-cap 400 when combined swap+liq > 5000)
curl -sS -o /tmp/gt-events.json -w "%{http_code} bytes %{size_download}\n" \
  "https://indexer.dex.cl8y.com/gt/events?fromBlock=H1&toBlock=H2"
```

`make verify-issue-698` records leftover probes as SKIP unless the hosts answer. Fail closed with `VERIFY698_REQUIRE_LIVE=1`.

## Do / don’t

- **Do** run `make verify-issue-698` from a git worktree after pulling `main`.
- **Do** `make setup-indexer-postgres` for child #694.
- **Do** link `frontend-dapp/node_modules` from the primary checkout in a git worktree. Do **not** `npm install` over a worktree `node_modules` symlink.
- **Do** expect leftover bootstrap to copy LocalTerra `.env.local` (`VITE_DEV_MODE=true`). Child #695 Vitest must pin `process.env.VITE_DEV_MODE` to `''`/`'false'` so `loadEnv` does not re-inject the flag into `loadConfigFromFile` production-allow cases.
- **Don’t** reopen #694 / #695 unless a merged invariant is wrong.
- **Don’t** fail-open blacklist (oversize stays **400**).
- **Don’t** treat GitLab CI quota as leftover evidence.
- **Don’t** work #692 / #693 on this ticket — those leftovers are [#701](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/701) / [#702](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/702).

## Regression

```bash
make verify-issue-698
# docs + source + live (no children):
VERIFY698_SKIP_CHILDREN=1 make verify-issue-698
# docs + children only:
VERIFY698_SKIP_LIVE=1 VERIFY698_SKIP_CHAIN=1 make verify-issue-698
# fail if Coolify leftovers cannot run:
VERIFY698_REQUIRE_LIVE=1 make verify-issue-698
```
