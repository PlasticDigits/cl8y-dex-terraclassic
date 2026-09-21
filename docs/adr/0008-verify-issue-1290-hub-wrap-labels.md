# ADR 0008: `verify-issue-1290` bundle for hub wrap vs CEX labels

## Status

Proposed ([#1302](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1302))

Leftover QA / regression entry point after product labels landed. This ADR does **not** relabel `/protocol`, change indexer or Venus JSON, flip Coolify, deploy, spend, expand custody/policy, or write `DESIGN: APPROVE`. Keywords on the issue (“architecture”, “deploy”) are not approval. Deploy/spend/custody/policy expansion stays under [agent-control #297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297).

Playbooks (do not duplicate P1240 here): [`skills/AGENTS_FRONTEND_PROTOCOL_STATS.md`](../../skills/AGENTS_FRONTEND_PROTOCOL_STATS.md) (**P1240-1–P1240-8**), [`skills/AGENTS_FRONTEND_PROTOCOL_HUB.md`](../../skills/AGENTS_FRONTEND_PROTOCOL_HUB.md) (**H11–H16**). Overview: [`architecture.md`](../architecture.md#dex-hub-wrap-labels). Product table: [`frontend.md`](../frontend.md) **P1240**. Leftover row: [`qa-invariants.md`](../qa-invariants.md) **Q21**.

ADR **0007** is reserved by the #1265 census (`docs/adr/0007-route-solve-remaining-failures.md`). This ticket is **0008**.

## Outcome

1. **Bundle.** `make verify-issue-1290` (Makefile alias `verify-issue-1302`) is the post-merge regression entry for hub wrap **cLUNC** vs CEX **LUNC** after [!1290](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1290) merged the #1240 naming follow-up. It always runs child `make verify-issue-1240` (**P1240-1–P1240-8**), then `hubPriceTicker` Vitest, then optional Playwright `e2e/protocol-page.spec.ts` at **5 workers**, then doc/skill/ADR crosslink greps.
2. **No product delta.** Hub maps, oracle tabs, and CSS `uppercase` / `text-transform: none` stay as shipped on `main` (`HUB_PRICE_TICKER_LABEL.lunc = cLUNC`; API id stays `lunc`; no `clunc` path). Implement does not reopen [#1240](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1240) or closed !1290 for copy.
3. **Clean tree.** Reconstruct the bundle on current `main`. Do **not** land the accidental paths on `origin/issue/1290` (`f237a6e6`): `frontend-dapp/node_modules` symlink ([#1298](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1298) / [!1299](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1299)), leftover `FAIL` residue in `scripts/test-commit-msg-hook.sh` ([#1300](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1300) / [!1301](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1301)), or `.gitignore` `node_modules` churn.
4. **Leftover visual.** Production `/protocol` glance (hub wrap vs CEX native; selected **vFDUSD** tab not **VFDUSD**) is reviewer leftover after frontend Coolify auto-deploy. It is **not** #297 and **not** inferred from green make.

## Context

`/protocol` has two USD surfaces that share the English word “LUNC”:

| Surface | Visible tickers | API / path ids |
|---------|-----------------|----------------|
| DEX hub wrap ([#556](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/556) / [#570](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/570)) | **cUSTC**, **cLUNC**, UST1, USTR | `custc`, `lunc`, `ust1`, `ustr` |
| CEX oracle ([#515](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/515) / [#571](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/571)) | **USTC**, **LUNC**, **vFDUSD** | `ustc`, `lunc`, `vfdusd` |

[#1240](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1240) stopped Tailwind `uppercase` from flattening mixed-case product tickers. !1290 (merged) made the hub wrap column read **cLUNC / USD** instead of native **LUNC / USD**, while CEX tabs stayed **USTC** / **LUNC**. `parseHubPriceTicker('cLUNC')` stays `null` (no `clunc` alias). Selected oracle `.btn-primary` keeps `text-transform: none` so **vFDUSD** is not **VFDUSD**.

On `main` today:

- `scripts/qa/verify-issue-1240.sh` covers ProtocolPage + StatBox RTL, static `uppercase` greps, ticker maps, and P1240 doc/skill greps. It does **not** run `hubPriceTicker.test.ts` or Playwright.
- `frontend-dapp/src/utils/__tests__/hubPriceTicker.test.ts` and `frontend-dapp/e2e/protocol-page.spec.ts` **P1 / P2 / P4** already assert the wrap vs CEX split (exact-case hub `<dt>`, oracle H2 **USTC / USD** with zero **cUSTC / USD** headings, selected **vFDUSD** `text-transform: none`).
- There is no `verify-issue-1290` Makefile target.

Open [PR #1302](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1302) already points `issue/1290` at a one-commit bundle (`f237a6e6`). That commit is a useful sketch (child 1240 + Vitest + 5-worker Playwright + crosslinks) and a **dirty** sketch (symlink + hook FAIL residue). This ADR is the versioned contract; implement rebuilds a clean commit on `main` rather than merging that tip as-is.

## Non-goals

- Changing hub/oracle copy, CSS, ticker ids, `?ticker=` allowlist, or Venus headings (**P1240-8**).
- Indexer `GET /hub-prices`, oracle catalog, or Venus `eth_call`.
- A new `AGENTS_POST_MERGE_OPS_1302.md` playbook (amend existing PROTOCOL_STATS / PROTOCOL_HUB skills).
- A new census ADR / Stay memo. Product is shipped; this is QA wiring.
- Scraping production HTML as leftover-complete. No `VERIFY1290_REQUIRE_LIVE` curl of `dex.cl8y.com`.
- Playwright `install-deps` / NSS (`libnspr4.so`) as this ticket. Gate VMs use `VERIFY_ISSUE_1290_SKIP_E2E=1`. Provisioned Cloud Agent / CI run the full suite.
- Relanding `frontend-dapp/node_modules` (file or symlink). Relanding hook-test FAIL duplication.
- Coolify UI, HMAC / `autonomy.rs`, founder card, or #297 deploy.
- `Fixes #1290` on the implement PR body — !1290 is already merged. Cite #1302; related #1240 / !1290.
- Opening a design-only PR from `cac-design-issue-1302`.

## Decision

**Thin wrapper over shipped P1240.** Child `verify-issue-1240` remains the invariant harness. #1290 adds the two gaps that harness omitted (label map unit test + protocol-page smoke) plus an agent-discoverable `make` target.

### Makefile

Place immediately after `verify-issue-1240`:

```makefile
.PHONY: verify-issue-1290 verify-issue-1302
verify-issue-1290:
	@chmod +x scripts/qa/verify-issue-1290.sh scripts/qa/verify-issue-1240.sh scripts/with-node.sh
	./scripts/qa/verify-issue-1290.sh
verify-issue-1302: verify-issue-1290
```

Alias matches `verify-issue-1289: verify-issue-1265`. Add `verify-issue-1290` to the Makefile Frontend help echo (optional `verify-issue-1302`).

### `scripts/qa/verify-issue-1290.sh`

Order (FAIL fails the stack; skip is not FAIL):

| Step | Rule |
|------|------|
| 1 | `./scripts/qa/verify-issue-1240.sh` (**P1240-1–P1240-8**). Child FAIL → bundle FAIL. |
| 2 | `bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run src/utils/__tests__/hubPriceTicker.test.ts` — wrap labels `cUSTC` / `cLUNC`; `parseHubPriceTicker('cLUNC')` is `null`; reject `clunc` / CEX `ustc` / injection. |
| 3 | Playwright `e2e-smoke` **5 workers** on `e2e/protocol-page.spec.ts` with `PLAYWRIGHT_SKIP_CHAIN=1`, dedicated `PLAYWRIGHT_WEB_PORT=30129` / `PLAYWRIGHT_BASE_URL=http://127.0.0.1:30129`. Do not leak that port into other tests (CORS). |
| 4 | Grep `verify-issue-1290` in `docs/testing.md`, `AGENTS.md`, `skills/AGENTS_FRONTEND_PROTOCOL_STATS.md`, `skills/AGENTS_FRONTEND_PROTOCOL_HUB.md`, `docs/frontend.md`, this ADR, `docs/architecture.md` (`dex-hub-wrap-labels` or `verify-issue-1290`), and `docs/qa-invariants.md` **Q21**. |

**E2E skip (exit 0 for the step, labeled skipped):**

- `VERIFY_ISSUE_1290_SKIP_E2E=1`
- Playwright package absent (`frontend-dapp/node_modules/@playwright/test` and `…/playwright` both missing)

**E2E FAIL:** package present but `playwright test` non-zero (including missing Chromium / `libnspr4.so`). Do not map that to PASS. Operators set `SKIP_E2E=1` on gate VMs without browser deps.

**Worktree `.env.local`:** if this worktree lacks `frontend-dapp/.env.local` and the primary checkout (`git rev-parse --git-common-dir` / `..`) has a gitignored copy, copy it for Playwright. Do not commit `.env.local`.

Optional comment-only lines in `verify-issue-1240.sh` header or `protocol-page.spec.ts` may cite #1290. No assertion edits required — P1/P2/P4 already match **P1240**.

### Forbidden paths (do not copy from `f237a6e6`)

| Path | Why |
|------|-----|
| `frontend-dapp/node_modules` | Symlink / gitlink. Dropped on `main` by !1299. |
| `scripts/test-commit-msg-hook.sh` leftover duplicate `FAIL` / `exit 1` | Hook-test merge residue. Dropped on `main` by !1301. |
| `.gitignore` `node_modules` line churn | Unrelated to the bundle. |

### Existing PR #1302

Implement may retarget/replace the open PR head with a **new commit on current `main`**. Do not merge `f237a6e6` as-is. Do not force-push `main`. This design branch (`cac-design-issue-1302`) transports the ADR only — no design-only PR.

## Component / state / interface changes

| Layer | Change |
|-------|--------|
| QA script | **Add** `scripts/qa/verify-issue-1290.sh` (executable). |
| Makefile | **Add** `verify-issue-1290` + alias `verify-issue-1302`; help echo. |
| Docs | `docs/testing.md` row; `docs/frontend.md` regression line includes `verify-issue-1290`; `AGENTS.md` Cloud Agent + lint table. |
| Skills | PROTOCOL_STATS / PROTOCOL_HUB: issue line + `make verify-issue-1290` (no new skill file). |
| dApp TS/CSS | **None** required. |
| Indexer / wasm / env | **None**. |
| HTTP | **None**. |

## Affected invariants

| ID | Effect |
|----|--------|
| **P1240-1–P1240-8** | Unchanged rules. Bundle **runs** the existing 1240 harness plus the omitted Vitest + smoke. |
| **H11–H16** | Unchanged hub identity. Skill gains the bundle verify pointer. |
| **B1290-1–B1290-8** | New bundle/leftover IDs (this ADR + **Q21**). |
| **#1298 / #1300** | Bundle MR must not reintroduce those defects. |
| **Playwright 5 workers** | `e2e-smoke` stays 5 workers ([`.cursor/rules/playwright-workers.mdc`](../../.cursor/rules/playwright-workers.mdc)). Not `e2e-tx`. |

**B1290-1–B1290-8**

| ID | Rule |
|----|------|
| **B1290-1** | `make verify-issue-1290` runs child **1240**. Child FAIL fails the stack. Alias `verify-issue-1302` is the same script. |
| **B1290-2** | Vitest `hubPriceTicker.test.ts` is a required step (not folded into 1240). |
| **B1290-3** | Playwright protocol-page is default-on at **5 workers**, port **30129**, `PLAYWRIGHT_SKIP_CHAIN=1`. Skip only via `VERIFY_ISSUE_1290_SKIP_E2E=1` or missing Playwright package. Installed package + crash = FAIL. |
| **B1290-4** | Crosslinks: testing, AGENTS, PROTOCOL_STATS, PROTOCOL_HUB, frontend.md, this ADR, architecture anchor, **Q21**. |
| **B1290-5** | No indexer/hub-price/Venus/API change. No `clunc` path. No product TSX in this MR unless a shipped P1240 assertion is actually wrong (then reopen #1240, do not hide it in the bundle). |
| **B1290-6** | Do not reopen closed !1290 / #1240 for ops/QA. Do not `Fixes #1290`. Do not wait on Woodpecker quota as leftover evidence. |
| **B1290-7** | Do not add `frontend-dapp/node_modules`, hook-test FAIL residue, or `.gitignore` churn. Do not create `AGENTS_POST_MERGE_OPS_1302.md`. |
| **B1290-8** | Green `SKIP_E2E=1` is **implement-complete**, not leftover-complete. Leftover is reviewer visual on production `/protocol` after frontend rebuild. No HTML scrape. Not #297. |

## Alternatives

| Option | Why not |
|--------|---------|
| Fold Vitest + Playwright into `verify-issue-1240.sh` | 1240 stays the no-chain docs/RTL harness. Agents already call it. Extending it makes SKIP_E2E a 1240 concern and slows every P1240 grep run. |
| New leftover skill `AGENTS_POST_MERGE_OPS_1302.md` | P1240 already lives in PROTOCOL_STATS / HUB. A third playbook duplicates narrative. |
| Census Stay ADR | Product already merged. No remaining-gap decide. |
| Live `curl` of `dex.cl8y.com/protocol` | HTML/CSS-in-JS is not a stable assert; Coolify skew is visual. Reviewer leftover. |
| Alias-only `verify-issue-1290: verify-issue-1240` | Misses hubPriceTicker + Playwright — the actual gap. |
| Merge `origin/issue/1290` as-is | Reintroduces !1299 / !1301 defects. |

## Complexity added / removed

**Added:** one bash wrapper, one Makefile target + alias, crosslink greps, optional E2E skip flag, **Q21** / **B1290**.

**Removed:** the gap where post-merge hub **cLUNC** vs CEX **LUNC** had no agent `make` entry besides 1240’s CSS/docs subset.

Net: QA discoverability without a second product surface.

## Migration

None. No wasm, no sqlx, no Coolify env, no dApp `VITE_*`. Frontend auto-deploy already ships the merged !1290 labels; this MR only adds verify wiring.

## Observability

Script prints `[PASS]` / `[FAIL]` / skipped E2E lines and a counts footer; exit `1` iff `FAIL > 0`. Playwright traces stay the existing `e2e-smoke` config. No new indexer `tracing` events. Do not paste Coolify UUIDs, tokens, or `/status` JSON on the issue.

## Failure modes

| Failure | Behavior |
|---------|----------|
| Child 1240 FAIL | Bundle FAIL. Fix P1240 / docs; do not skip the child. |
| `hubPriceTicker` Vitest FAIL | Bundle FAIL. Label map or parse guard drifted. |
| Playwright missing package | Skip E2E (not FAIL). |
| Playwright installed, Chromium / NSS missing | FAIL unless `VERIFY_ISSUE_1290_SKIP_E2E=1`. |
| Port 30129 in use / CORS from leaking `PLAYWRIGHT_WEB_PORT` | FAIL. Keep 30129 local to this step. |
| Worktree without `.env.local` and no primary copy | Playwright may skip factory-address asserts (existing spec `try/catch`); hub/oracle casing still runs. |
| Green make with `SKIP_E2E=1` treated as leftover-complete | Forbidden (**B1290-8**). |
| Merge of `f237a6e6` | Reintroduces node_modules symlink and hook FAIL residue. Reconstruct. |
| Implement flips Coolify | Forbidden (#297). |
| `VERIFY1290_IID=1290` / `LEFTOVER_COMPLETE=1` invented as HTTP fail-closed | Out of scope. No live HTML probe. |

## Ordered implementation slices

| Slice | Who | Deliverable | Blocks |
|-------|-----|-------------|--------|
| **0 — this design** | design_author | ADR 0008, architecture pointer, **Q21**, `docs/README.md` index | Slice 1 |
| **1 — script + Makefile** | implement | `scripts/qa/verify-issue-1290.sh`, `verify-issue-1290` / `verify-issue-1302`, help echo | Slice 2 |
| **2 — crosslinks** | implement | AGENTS.md, `docs/testing.md`, `docs/frontend.md` regression line, PROTOCOL_STATS + PROTOCOL_HUB pointers. Optional comment on 1240 script / protocol-page spec. **No** product TSX, **no** dirty files from `f237a6e6`. | none |
| **3 — reviewer leftover** | reviewer / operator | After frontend Coolify rebuild of the bundle merge (or already-shipped !1290 labels): `/protocol` hub **cUSTC / USD** + **cLUNC / USD**; oracle tabs **USTC** / **LUNC** / **vFDUSD**; selected vFDUSD not **VFDUSD**. | Closes leftover AC only |

Slice 1 **must not** wait on slice 3. Slice 3 **must not** be attempted as a Coolify checkbox or deploy.

No product-issue dependencies. #1240 and !1290 are already on `main`.

## Tests

**Implement merge (slices 1–2):**

```bash
VERIFY_ISSUE_1290_SKIP_E2E=1 make verify-issue-1290
make verify-issue-1302   # same, with skip still exported if needed
make verify-issue-1240   # child still standalone
```

Provisioned agent / CI (Playwright + Chromium present):

```bash
make verify-issue-1290
```

Assert:

- Child 1240 still 8/8 (or current step count) without requiring Playwright.
- Bundle with skip: 1240 + hubPriceTicker + skipped E2E + greps, exit 0.
- Bundle without skip on a provisioned VM: Playwright P1/P2/P4 casing (5 workers).
- `git ls-files frontend-dapp/node_modules` empty. `scripts/test-commit-msg-hook.sh` has a single pre-push `--not --remotes` assert (no duplicated `FAIL` block).
- `make test-commit-msg-hook` still passes (do not edit that script in this MR).

## Rollout

1. Merge slices 1–2 to `main`. Frontend Coolify auto-deploy may rebuild; labels are already on `main` from !1290 — this merge is verify-only.
2. Implement **must not** flip Coolify.
3. Reviewer leftover glance on `https://dex.cl8y.com/protocol` (hub wrap vs CEX; vFDUSD casing). Record pass/fail on #1302; no UUID/token.

## Rollback

Revert the verify script, Makefile target, and crosslinks. Product labels remain from !1290. No schema, wasm, or env rollback. Frontend image rollback is unrelated (verify-only MR).

## Integration completion criteria

**Implement merge is complete when:**

- `VERIFY_ISSUE_1290_SKIP_E2E=1 make verify-issue-1290` exits 0 on a worktree from `main` + this MR.
- Alias `make verify-issue-1302` is the same target.
- **B1290-1–B1290-8** greppable from this ADR + **Q21**.
- Forbidden paths from `f237a6e6` are absent.
- No Coolify secret/UUID in the MR.
- PR body does not `Fixes #1290`.

**#1302 leftover is complete when (reviewer, not implement):**

1. Production `/protocol` shows hub **cUSTC / USD** and **cLUNC / USD** (not hub **LUNC / USD**, not **CUSTC**).
2. Oracle tabs **USTC** / **LUNC** / **vFDUSD**; selected vFDUSD is not **VFDUSD**.
3. Glance is visual; not an HTML scrape and not a Coolify checkbox.

Green make with `SKIP_E2E=1` does **not** close leftover.

Keywords on the issue are **not** approval to deploy or to write `DESIGN: APPROVE`.
