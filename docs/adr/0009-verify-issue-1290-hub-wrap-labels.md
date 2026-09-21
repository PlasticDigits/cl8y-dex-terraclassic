# ADR 0009: `verify-issue-1290` bundle for hub wrap vs CEX labels

## Status

Proposed ([#1306](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1306))

Leftover QA / regression wiring after product labels and the weaker !1302 sketch landed. This ADR does **not** relabel `/protocol`, change indexer or Venus JSON, flip Coolify, deploy, spend, expand custody/policy, or write `DESIGN: APPROVE`. Keywords on the issue (“architecture”, “deploy”) are not approval. Deploy/spend/custody/policy expansion stays under [agent-control #297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297).

Playbooks (do not duplicate P1240 here): [`skills/AGENTS_FRONTEND_PROTOCOL_STATS.md`](../../skills/AGENTS_FRONTEND_PROTOCOL_STATS.md) (**P1240-1–P1240-8**), [`skills/AGENTS_FRONTEND_PROTOCOL_HUB.md`](../../skills/AGENTS_FRONTEND_PROTOCOL_HUB.md) (**H11–H16**). Overview: [`architecture.md`](../architecture.md#dex-hub-wrap-labels). Product table: [`frontend.md`](../frontend.md) **P1240**. Leftover row: [`qa-invariants.md`](../qa-invariants.md) **Q22**.

ADR **0007** is reserved by the #1265 census (`docs/adr/0007-route-solve-remaining-failures.md`). Live **Q21** `{#luncdash-wc-1308}` is Lunc Dash payload ([#1308](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1308) / !1310). ADR **0008** is unused (closed leftover-ops [#1300](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1300); `docs/adr/0008-post-merge-leftover-1287-1298.md` remains only on unpublished `origin/cac-design-issue-1300` — do not merge as-is). This ticket keeps **0009** / **Q22** (first reserved on `cac-design-issue-1302`; retargeted here to leftover **#1306**). Do not keep dual `0008-*.md` or two **Q21** tables. [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) leftover 3 already forbids merging `cac-design-issue-1300` / `cac-design-issue-1302` as-is. Production payload Coolify leftover is [#1311](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1311) (**Q24** / ADR **0011**), not this wiring MR. Do not merge this design branch (`cac-design-issue-1306`) as a design-only PR. Implement must keep Status **Proposed ([#1306])** until a reviewer accepts the design — do not self-mark **Accepted**.

## Outcome

1. **Bundle.** `make verify-issue-1290` is the leftover regression entry for hub wrap **cLUNC** vs CEX **LUNC** after [!1290](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1290) merged the #1240 naming follow-up. Makefile aliases: **`verify-issue-1302`** (issue AC **B1290-1**) and **`verify-issue-1306`** (this leftover iid). The script always runs child `make verify-issue-1240` (**P1240-1–P1240-8**), then `hubPriceTicker` Vitest, then optional Playwright `e2e/protocol-page.spec.ts` at **5 workers**, then the full **B1290-4** eight-file greps.
2. **No product delta.** Hub maps, oracle tabs, and CSS `uppercase` / `text-transform: none` stay as shipped on `main` (`HUB_PRICE_TICKER_LABEL.lunc = cLUNC`; API id stays `lunc`; no `clunc` path). Implement does not reopen [#1240](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1240) or closed !1290 for copy.
3. **This leftover is [#1306](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1306), not a reopen of closed !1302.** Closed [!1302](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1302) already merged sketch `f237a6e6` (`92c84406` on `main`). Reconstruct the remaining gaps on **current `main`**. Open [!1306](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1306) (`origin/issue/1302`) may be amended/rebased; do **not** retarget the closed PR. Do **not** merge `f237a6e6` as-is. Do **not** land `frontend-dapp/node_modules` (symlink / mode `120000` — [#1298](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1298) / [!1299](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1299)) or leftover `FAIL` residue in `scripts/test-commit-msg-hook.sh` ([#1300](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1300) / [!1301](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1301)). Do not touch the hook-test script.
4. **Leftover-complete is this #1306 wiring MR.** Alias `verify-issue-1302` **and** `verify-issue-1306`, Cloud Agent + Frontend `help` for **1290 / 1302 / 1306**, `free_tcp_port` 30129, full **B1290-4**, leftover **#1306** naming, and slice-0 docs (this ADR **0009**, architecture `#dex-hub-wrap-labels`, **Q22**, README index). Green `VERIFY_ISSUE_1290_SKIP_E2E=1` is a required implement check, **not** leftover-complete. Production `/protocol` visual (hub wrap vs CEX native; selected **vFDUSD** tab not **VFDUSD**) is [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) leftover 1 (related #1290 AC), **not** this ticket and **not** #297. Slice 3 is not the #1306 close gate.

## Context

`/protocol` has two USD surfaces that share the English word “LUNC”:

| Surface | Visible tickers | API / path ids |
|---------|-----------------|----------------|
| DEX hub wrap ([#556](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/556) / [#570](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/570)) | **cUSTC**, **cLUNC**, UST1, USTR | `custc`, `lunc`, `ust1`, `ustr` |
| CEX oracle ([#515](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/515) / [#571](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/571)) | **USTC**, **LUNC**, **vFDUSD** | `ustc`, `lunc`, `vfdusd` |

[#1240](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1240) stopped Tailwind `uppercase` from flattening mixed-case product tickers. !1290 (merged) made the hub wrap column read **cLUNC / USD** instead of native **LUNC / USD**, while CEX tabs stayed **USTC** / **LUNC**. `parseHubPriceTicker('cLUNC')` stays `null` (no `clunc` alias). Selected oracle `.btn-primary` keeps `text-transform: none` so **vFDUSD** is not **VFDUSD**.

On `origin/main` after closed !1302 (`92c84406` merged `f237a6e6`; tip at this design: `729b097f` including !1303 / !1304):

- `scripts/qa/verify-issue-1240.sh` covers ProtocolPage + StatBox RTL, static `uppercase` greps, ticker maps, and P1240 doc/skill greps. It does **not** run `hubPriceTicker.test.ts` or Playwright (those live in the 1290 wrapper).
- `frontend-dapp/src/utils/__tests__/hubPriceTicker.test.ts` and `frontend-dapp/e2e/protocol-page.spec.ts` **P1 / P2 / P4** already assert the wrap vs CEX split.
- `scripts/qa/verify-issue-1290.sh` **exists** (child 1240 + Vitest + 5-worker Playwright on port 30129 in a subshell). Gaps vs this ADR: no `free_tcp_port 30129`; greps are only `docs/testing.md` + `AGENTS.md` + `PROTOCOL_STATS` (missing PROTOCOL_HUB, `frontend.md`, this ADR, architecture `#dex-hub-wrap-labels`, **Q22**); header/echo title leftover as **#1290 implement PR**.
- Makefile has `.PHONY` / recipe `verify-issue-1290`. **Missing on `main`:** aliases `verify-issue-1302` and `verify-issue-1306`. On `729b097f` **neither** Cloud Agent nor Frontend `help` lists `verify-issue-1290` (only the target exists). Add **1290, 1302, and 1306** to those help lines.
- Crosslinks title the bundle as the **#1290 product / implement PR** and closed !1302 said `Fixes #1290`. Product labels already merged in !1290; first leftover sketch is closed !1302; **this leftover ticket is #1306**.
- Slice-0 files are **not** on `main`: this ADR, `docs/architecture.md` `#dex-hub-wrap-labels`, `docs/qa-invariants.md` **Q22**, `docs/README.md` ADR 0009 index. B1290-4 greps those paths — they must ride the #1306 MR as **inserts**, not a whole-file checkout of this tip or of `cac-design-issue-1302`.
- Open [!1306](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1306) (`origin/issue/1302` @ `50eb149f`) already drafts most of that wiring but is **not** leftover-complete: ADR Status **Accepted (#1302)** (forbidden self-mark), leftover naming still **#1302**, PR body `Closes #1302`, no `verify-issue-1306` alias. Amend that PR (or a new branch from current `main`); do not merge it as-is.
- `origin/cac-design-issue-1300` already published ADR **0008** and **Q21** `{#post-merge-ops-1300}` on the same three overlap files (`architecture.md`, `qa-invariants.md`, `README.md`). This ticket must not take 0008/Q21.
- `git ls-files frontend-dapp/node_modules` is empty on `origin/main` (merge kept !1299). `f237a6e6` still has that gitlink (`120000`). Do not reintroduce it.
- `scripts/test-commit-msg-hook.sh` is clean on `main` via !1301. `f237a6e6` duplicated a `FAIL` / `exit 1` block. Do not edit that script.

This ADR is the versioned contract (retarget of the unpublished `cac-design-issue-1302` 0009/Q22 slot). Implement lands **#1306** on current `main`. Do not retarget closed !1302. Do not merge `f237a6e6` as-is. This design branch (`cac-design-issue-1306`) transports the ADR only — no design-only PR.

## Non-goals

- Changing hub/oracle copy, CSS, ticker ids, `?ticker=` allowlist, or Venus headings (**P1240-8**).
- Indexer `GET /hub-prices`, oracle catalog, or Venus `eth_call`.
- A new `AGENTS_POST_MERGE_OPS_1302.md` or `AGENTS_POST_MERGE_OPS_1306.md` playbook (amend existing PROTOCOL_STATS / PROTOCOL_HUB skills).
- A new census ADR / Stay memo. Product is shipped; this is QA wiring.
- Scraping production HTML as leftover-complete. No `VERIFY1290_REQUIRE_LIVE` curl of `dex.cl8y.com`.
- Recording the Coolify `/protocol` glance on **#1306**. That glance is [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) leftover 1 (related #1290 AC).
- Playwright `install-deps` / NSS (`libnspr4.so`) as this ticket. Gate VMs use `VERIFY_ISSUE_1290_SKIP_E2E=1`. Provisioned Cloud Agent / CI run the full suite.
- Relanding `frontend-dapp/node_modules` (file or symlink). Relanding hook-test FAIL duplication. Editing `scripts/test-commit-msg-hook.sh`.
- Copying `verify-issue-703.sh` `ln -sfn` `frontend-dapp/node_modules` bootstrap ([#1298](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1298) / [!1299](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1299)). Copy **only** `free_tcp_port`.
- Coolify UI, HMAC / `autonomy.rs`, founder card, or #297 deploy.
- `Fixes #1290` or `Closes #1302` on the #1306 PR body — !1290 and !1302 are already merged. Cite leftover **#1306**; related #1302 / !1290 / #1240.
- Opening a design-only PR from `cac-design-issue-1306` or `cac-design-issue-1302`.
- Retargeting closed !1302. Merging open !1306 as-is without the #1306 retarget and Proposed status.
- Taking ADR **0008** / **Q21** (those belong to [#1300](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1300) unless that design is withdrawn). Merging this design branch, `cac-design-issue-1302`, or `cac-design-issue-1300` as-is ([#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) leftover 3).
- Self-marking this ADR **Accepted**.

## Decision

**Thin wrapper over shipped P1240.** Child `verify-issue-1240` remains the invariant harness. Closed !1302 landed a weaker sketch (wrapper exists; alias / `free_tcp_port` / eight-file greps / slice-0 docs / leftover naming do not). Leftover **#1306** closes those ADR gaps.

### #1306 MR (do not retarget closed !1302)

Continue open [!1306](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1306) (`origin/issue/1302`) **or** open a new branch from **current `main`** (tip at this design: `729b097f`; use whatever `origin/main` is at implement time). Do not retarget or reopen !1302. Do not merge `f237a6e6` as-is. Do not merge `50eb149f` as-is. This design tip is **not** an ancestor of `origin/main` (merge-base `6d34da13`).

**Slice-0 apply — insert; do not whole-file checkout.** Do **not** `git checkout <design-sha> --` `docs/architecture.md` / `docs/qa-invariants.md` / `docs/README.md` from this tip or from `cac-design-issue-1302`. #1300 (and later `main`) touches those same three files. Cherry-pick of whole-file 0008-era paths is unsafe.

1. **Insert slice 0 onto the #1306 MR:**
   - New file `docs/adr/0009-verify-issue-1290-hub-wrap-labels.md` (copy from this branch). Status **Proposed ([#1306])**. Do **not** add a second `0008-*.md`. Do **not** mark **Accepted**.
   - `docs/architecture.md`: insert section `## DEX hub wrap vs CEX labels {#dex-hub-wrap-labels}` (do not replace the whole file).
   - `docs/qa-invariants.md`: insert **Q22** `{#ops-hub-wrap-1302}` **after Q21** if #1300 already landed, else **after Q20** — still number it **Q22**; leave **Q21** / ADR **0008** to #1300. Keep the `{#ops-hub-wrap-1302}` anchor (first leftover reservation). Table text names leftover **#1306**. Add the Related-docs bullet for ADR 0009.
   - `docs/README.md`: insert the ADR 0009 index line **after ADR 0008** if present, else **after ADR 0007**. If 0008 is not yet on `main`, a parenthetical that 0008 / Q21 are reserved for #1300 leftover-ops is enough — do not invent a stub `0008-*.md` here. Architecture Overview bullet adds `#dex-hub-wrap-labels`.
2. Reconstruct `scripts/qa/verify-issue-1290.sh` (Playwright isolation + full B1290-4 greps + leftover **#1306** naming). Copy **only** `free_tcp_port` from [`scripts/qa/verify-issue-703.sh`](../../scripts/qa/verify-issue-703.sh) — **not** 703’s `ln -sfn` `frontend-dapp/node_modules` bootstrap. Add Makefile aliases `verify-issue-1302: verify-issue-1290` and `verify-issue-1306: verify-issue-1290`. Add **1290, 1302, and 1306** to Cloud Agent and Frontend `help` (immediately after `verify-issue-1240`).
3. Retitle crosslinks to leftover **#1306** (wording below). Keep `git ls-files frontend-dapp/node_modules` empty. Do not touch `scripts/test-commit-msg-hook.sh`.
4. PR body cites **#1306**. Related #1302 / !1290 / #1240. Do **not** `Fixes #1290`. Do **not** `Closes #1302`. Production `/protocol` glance stays on [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) leftover 1.

### Makefile

Place immediately after `verify-issue-1240` (current `main` already has `verify-issue-1290` without aliases):

```makefile
.PHONY: verify-issue-1290 verify-issue-1302 verify-issue-1306
verify-issue-1290:
	@chmod +x scripts/qa/verify-issue-1290.sh scripts/qa/verify-issue-1240.sh scripts/with-node.sh
	./scripts/qa/verify-issue-1290.sh
verify-issue-1302: verify-issue-1290
verify-issue-1306: verify-issue-1290
```

Add `verify-issue-1290`, `verify-issue-1302`, and `verify-issue-1306` immediately after `verify-issue-1240` on:

- the `Cloud Agent:` `help` echo
- the `Frontend:` `help` echo that already lists `verify-issue-1240` (the `make dev | verify-issue-678 | …` line)

Do not treat the other `Frontend: make dev | build-frontend | …` line as that list.

### `scripts/qa/verify-issue-1290.sh`

Header/echo name leftover **#1306** (Q22 / B1290), related #1302 / !1290 / #1240 — not “#1290 implement PR” and not “leftover #1302” as the close ticket.

Order (FAIL fails the stack; skip is not FAIL):

| Step | Rule |
|------|------|
| 1 | `./scripts/qa/verify-issue-1240.sh` (**P1240-1–P1240-8**). Child FAIL → bundle FAIL. |
| 2 | `bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run src/utils/__tests__/hubPriceTicker.test.ts` — wrap labels `cUSTC` / `cLUNC`; `parseHubPriceTicker('cLUNC')` is `null`; reject `clunc` / CEX `ustc` / injection. |
| 3 | Playwright `e2e-smoke` **5 workers** on `e2e/protocol-page.spec.ts`. Isolation matches sibling `verify-issue-703.sh`: copy `free_tcp_port` **only**, call `free_tcp_port 30129`, then run the test in a **subshell** with `PLAYWRIGHT_SKIP_CHAIN=1 PLAYWRIGHT_WEB_PORT=30129 PLAYWRIGHT_BASE_URL=http://127.0.0.1:30129` **only for that step**. Do not `export` those into the parent shell (CORS). Stale Vite on 30129 is a flake FAIL without `free_tcp_port`. Do **not** copy 703’s `ln -sfn` `frontend-dapp/node_modules` bootstrap. |
| 4 | Grep `verify-issue-1290` in the **full B1290-4 set** (eight files): `docs/testing.md`, `AGENTS.md`, `skills/AGENTS_FRONTEND_PROTOCOL_STATS.md`, `skills/AGENTS_FRONTEND_PROTOCOL_HUB.md`, `docs/frontend.md`, this ADR (`docs/adr/0009-verify-issue-1290-hub-wrap-labels.md`), `docs/architecture.md` (`dex-hub-wrap-labels` or `verify-issue-1290`), and `docs/qa-invariants.md` **Q22**. Weaker sketch greps (testing + AGENTS + PROTOCOL_STATS only) are **not** B1290-4. |

**Playwright step (copy `free_tcp_port` from [`scripts/qa/verify-issue-703.sh`](../../scripts/qa/verify-issue-703.sh) — function only):**

```bash
free_tcp_port 30129
run_step "playwright e2e-smoke protocol-page (5 workers)" \
  bash -c 'PLAYWRIGHT_SKIP_CHAIN=1 PLAYWRIGHT_WEB_PORT=30129 PLAYWRIGHT_BASE_URL=http://127.0.0.1:30129 bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright test --project=e2e-smoke --workers=5 e2e/protocol-page.spec.ts'
```

**E2E skip (exit 0 for the step, labeled skipped):**

- `VERIFY_ISSUE_1290_SKIP_E2E=1`
- Playwright package absent (`frontend-dapp/node_modules/@playwright/test` and `…/playwright` both missing)

**E2E FAIL:** package present but `playwright test` non-zero (including missing Chromium / `libnspr4.so`). Do not map that to PASS. Operators set `SKIP_E2E=1` on gate VMs without browser deps.

**Worktree `.env.local`:** if this worktree lacks `frontend-dapp/.env.local` and the primary checkout (`git rev-parse --git-common-dir` / `..`) has a gitignored copy, copy it for Playwright. Do not commit `.env.local`.

Optional comment-only lines in `verify-issue-1240.sh` header or `protocol-page.spec.ts` may cite leftover **#1306** (related #1302 / !1290 / #1240). No assertion edits required — P1/P2/P4 already match **P1240**.

### Crosslink wording (leftover **#1306**, not product #1290)

Do not title rows as the #1290 product PR. Green skip-E2E is a required implement check, not leftover-complete. Leftover-complete is the **#1306** wiring MR.

**`docs/testing.md`** (new or replacement row; keep the #1240 product row):

`| Hub wrap vs CEX leftover verify | [#1306](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1306) | [\`make verify-issue-1290\`](../Makefile) (aliases \`verify-issue-1302\` / \`verify-issue-1306\`) | **Q22** / **B1290-1–B1290-8** — child **1240** + \`hubPriceTicker\` Vitest + optional Playwright \`protocol-page\` (5 workers). Related [#1302](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1302) / [!1290](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1290) / [#1240](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1240). Green \`VERIFY_ISSUE_1290_SKIP_E2E=1\` is a required implement check, not leftover-complete. Leftover-complete is the #1306 wiring MR. Production \`/protocol\` glance is [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) leftover 1 |`

**`AGENTS.md`** Cloud Agent line (after `verify-issue-1240`): leftover **#1306** hub wrap vs CEX (**Q22** / **B1290**; aliases `verify-issue-1302` / `verify-issue-1306`; related #1302 / !1290 / #1240). Lint table: keep **1240** as the product harness; leftover row points at `make verify-issue-1290` · `make verify-issue-1302` · `make verify-issue-1306`. PROTOCOL_STATS playbook line already cites P1240; add leftover `make verify-issue-1290`.

**`skills/AGENTS_FRONTEND_PROTOCOL_STATS.md`:** issue line cites leftover [**#1306**](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1306) (**Q22** / **B1290-1–B1290-8**), related #1302 / !1290 / #1240 — not “#1290 implement bundle”. Verify block: `make verify-issue-1290  # leftover #1306 (Q22 / B1290; aliases verify-issue-1302 / verify-issue-1306)`.

**`skills/AGENTS_FRONTEND_PROTOCOL_HUB.md`:** leftover verify `make verify-issue-1290` (aliases `verify-issue-1302` / `verify-issue-1306`) — **#1306**, **Q22** / **B1290**; related #1302 / !1290 / #1240.

**`docs/frontend.md`:** regression line includes `make verify-issue-1290` (optional `verify-issue-1302` / `verify-issue-1306`). Product **P1240** table stays #1240.

### Forbidden paths (do not copy from `f237a6e6`)

| Path | Why |
|------|-----|
| `frontend-dapp/node_modules` | Symlink / gitlink (`120000` on `f237a6e6`). Dropped on `main` by !1299; keep `git ls-files` empty. |
| `scripts/test-commit-msg-hook.sh` | Hook-test merge residue on that tip. Dropped on `main` by !1301. **Do not edit this file.** |
| `.gitignore` `node_modules` line churn | Unrelated to the bundle. |
| 703 `ln -sfn` bootstrap | Would reintroduce the #1298 gitlink. Copy `free_tcp_port` only. |

## Component / state / interface changes

| Layer | Change |
|-------|--------|
| Design docs (slice 0 — **must land on the #1306 MR as inserts**) | New `docs/adr/0009-verify-issue-1290-hub-wrap-labels.md` (**Proposed [#1306]**); insert `docs/architecture.md` `#dex-hub-wrap-labels`; insert `docs/qa-invariants.md` **Q22**; insert `docs/README.md` ADR 0009 index. B1290-4 greps these files. Do not whole-file checkout those three overlap files from this tip. |
| QA script | Reconstruct `scripts/qa/verify-issue-1290.sh` (already on `main` from closed !1302): `free_tcp_port 30129` (copied from 703, not 703’s `ln -sfn`), full B1290-4 greps, leftover **#1306** header. |
| Makefile | Keep `verify-issue-1290`; **add** aliases `verify-issue-1302` and `verify-issue-1306`; add **1290 / 1302 / 1306** to Cloud Agent and Frontend `help`. |
| Docs / skills (slice 2) | `docs/testing.md` leftover **#1306** row; `docs/frontend.md` regression line; `AGENTS.md` Cloud Agent + lint table; PROTOCOL_STATS / PROTOCOL_HUB leftover **#1306** pointers (**Q22** / **B1290**; related #1302 / !1290 / #1240). |
| dApp TS/CSS | **None** required. |
| Indexer / wasm / env | **None**. |
| HTTP | **None**. |
| `scripts/test-commit-msg-hook.sh` | **Do not touch.** |

## Affected invariants

| ID | Effect |
|----|--------|
| **P1240-1–P1240-8** | Unchanged rules. Bundle **runs** the existing 1240 harness plus the omitted Vitest + smoke. |
| **H11–H16** | Unchanged hub identity. Skill gains the leftover **#1306** verify pointer. |
| **B1290-1–B1290-8** | Bundle/leftover IDs (this ADR + **Q22**). Keep Q22 in sync with the table below. |
| **Q21 / ADR 0008** | Unchanged ownership: leftover-ops [#1300](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1300). This ticket does not occupy those slots. |
| **#1298 / #1300 / #1301** | Follow-up must not reintroduce those defects. |
| **Playwright 5 workers** | `e2e-smoke` stays 5 workers ([`.cursor/rules/playwright-workers.mdc`](../../.cursor/rules/playwright-workers.mdc)). Not `e2e-tx`. |

**B1290-1–B1290-8**

| ID | Rule |
|----|------|
| **B1290-1** | `make verify-issue-1290` runs child **1240**. Child FAIL fails the stack. Aliases `verify-issue-1302` and `verify-issue-1306` are the same script. |
| **B1290-2** | Vitest `hubPriceTicker.test.ts` is a required step (not folded into 1240). |
| **B1290-3** | Playwright protocol-page is default-on at **5 workers**, port **30129**, `PLAYWRIGHT_SKIP_CHAIN=1`. Copy `free_tcp_port` from `verify-issue-703.sh` (function only — not 703’s `ln -sfn` bootstrap) and free **30129** first. Port + `PLAYWRIGHT_BASE_URL` only in that step’s subshell. Skip only via `VERIFY_ISSUE_1290_SKIP_E2E=1` or missing Playwright package. Installed package + crash (missing Chromium / NSS) = FAIL. |
| **B1290-4** | Crosslinks: testing, AGENTS, PROTOCOL_STATS, PROTOCOL_HUB, frontend.md, this ADR (`docs/adr/0009-verify-issue-1290-hub-wrap-labels.md`), architecture `#dex-hub-wrap-labels`, **Q22**. Implement greps **all eight**. Slice-0 files ride #1306 as **inserts** so those greps can pass. |
| **B1290-5** | No indexer/hub-price/Venus/API change. No `clunc` path. No product TSX in this MR unless a shipped P1240 assertion is actually wrong (then reopen #1240, do not hide it in the bundle). |
| **B1290-6** | Do not reopen closed !1290 / #1240 for ops/QA. Do not retarget closed !1302. Do not `Fixes #1290`. Do not `Closes #1302`. Do not wait on Woodpecker quota as leftover evidence. |
| **B1290-7** | Do not add `frontend-dapp/node_modules`, hook-test FAIL residue, or `.gitignore` churn. Do not edit `scripts/test-commit-msg-hook.sh`. Do not create `AGENTS_POST_MERGE_OPS_1302.md` / `AGENTS_POST_MERGE_OPS_1306.md`. Do not take ADR 0008 / Q21. |
| **B1290-8** | Green `SKIP_E2E=1` ≠ leftover-complete. Leftover-complete is the **#1306 wiring MR** on `main` (aliases, `free_tcp_port`, full B1290-4, leftover **#1306** naming, slice-0 ADR 0009 / **Q22** / architecture / README). Production `/protocol` visual is [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) leftover 1 (related #1290 AC), not this ticket. No HTML scrape. Not #297. |

## Alternatives

| Option | Why not |
|--------|---------|
| Fold Vitest + Playwright into `verify-issue-1240.sh` | 1240 stays the no-chain docs/RTL harness. Agents already call it. Extending it makes SKIP_E2E a 1240 concern and slows every P1240 grep run. |
| New leftover skill `AGENTS_POST_MERGE_OPS_1306.md` | P1240 already lives in PROTOCOL_STATS / HUB. A third playbook duplicates narrative. |
| Census Stay ADR | Product already merged. No remaining-gap decide. |
| Live `curl` of `dex.cl8y.com/protocol` | HTML/CSS-in-JS is not a stable assert; Coolify skew is visual. That glance is #1305 leftover 1, not this ticket. |
| Alias-only `verify-issue-1290: verify-issue-1240` | Misses hubPriceTicker + Playwright — the actual gap. |
| Merge `origin/issue/1290` / `f237a6e6` as-is | Reintroduces !1299 / !1301 defects; weaker greps; no alias; no `free_tcp_port`. |
| Retarget closed !1302 | PR is merged. Continue open !1306 or a new branch on current `main`. |
| Merge open !1306 (`50eb149f`) as-is | ADR self-marked **Accepted (#1302)**; leftover naming still #1302; `Closes #1302`; no `verify-issue-1306` alias. Amend. |
| Keep ADR 0008 / Q21 on this ticket | Collides with `origin/cac-design-issue-1300`. [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) leftover 3. Keep **0009** / **Q22**. |
| Whole-file `git checkout <design-sha> --` architecture / qa-invariants / README | This tip is not an ancestor of `origin/main`; #1300 edits the same files. Insert sections. |
| Close #1306 on a Coolify glance recorded here | Splits close-out with #1305 leftover 1. Leftover-complete is the wiring MR. |
| Occupying a new ADR / Q23 slot | 0009 / Q22 already reserved for this bundle. Retarget the ticket iid only. |

## Complexity added / removed

**Added:** one bash wrapper (reconstruct), Makefile aliases `1302` + `1306`, Cloud Agent + Frontend help for **1290 / 1302 / 1306**, full B1290-4 greps, optional E2E skip flag, `free_tcp_port` on 30129, **Q22** / **B1290**, slice-0 docs on #1306 (ADR **0009**).

**Removed:** the gap where post-merge hub **cLUNC** vs CEX **LUNC** had no agent `make` entry besides 1240’s CSS/docs subset; the closed-PR sketch’s weaker grep / missing alias / stale-Vite flake; the 0008/Q21 collision with #1300; leftover-complete split onto a Coolify glance that #1305 already owns; dual “leftover #1302” close-out after that sketch already merged.

Net: QA discoverability without a second product surface and without dual ADR/Q numbering.

## Migration

None. No wasm, no sqlx, no Coolify env, no dApp `VITE_*`. Frontend auto-deploy already ships the merged !1290 labels; this leftover only completes verify wiring + slice-0 docs.

## Observability

Script prints `[PASS]` / `[FAIL]` / skipped E2E lines and a counts footer; exit `1` iff `FAIL > 0`. Playwright traces stay the existing `e2e-smoke` config. No new indexer `tracing` events. Do not paste Coolify UUIDs, tokens, or `/status` JSON on the issue.

## Failure modes

| Failure | Behavior |
|---------|----------|
| Child 1240 FAIL | Bundle FAIL. Fix P1240 / docs; do not skip the child. |
| `hubPriceTicker` Vitest FAIL | Bundle FAIL. Label map or parse guard drifted. |
| Playwright missing package | Skip E2E (not FAIL). |
| Playwright installed, Chromium / NSS missing | FAIL unless `VERIFY_ISSUE_1290_SKIP_E2E=1`. |
| Stale Vite on 30129 | FAIL unless `free_tcp_port` ran first (**B1290-3**). |
| Port 30129 leaked via parent `export PLAYWRIGHT_WEB_PORT` | CORS flake. Keep env in the Playwright subshell only. |
| B1290-4 greps fail because slice-0 docs stayed on the design branch | #1306 must **insert** the slice-0 sections/file. Do not shrink the grep set to the sketch. Do not whole-file checkout overlap files from this tip. |
| Worktree without `.env.local` and no primary copy | Playwright may skip factory-address asserts (existing spec `try/catch`); hub/oracle casing still runs. |
| Green make with `SKIP_E2E=1` treated as leftover-complete | Forbidden (**B1290-8**). Leftover-complete is the #1306 wiring MR. |
| Coolify `/protocol` glance recorded on #1306 as the close gate | Forbidden. That glance is #1305 leftover 1. Slice 3 is not this ticket’s close gate. |
| Merge of `f237a6e6` / retarget closed !1302 | Reintroduces node_modules symlink and/or hook FAIL residue; skips alias + `free_tcp_port`. Reconstruct. |
| Merge of open !1306 as-is | Lands **Accepted (#1302)** and `Closes #1302`. Amend first. |
| Copy 703 `ln -sfn` bootstrap | Reintroduces #1298 gitlink. Copy `free_tcp_port` only. |
| Land ADR 0008 / Q21 on this leftover | Collides with #1300. Use **0009** / **Q22**. |
| Implement flips Coolify | Forbidden (#297). |
| Implement marks this ADR **Accepted** | Forbidden self-approval. Keep **Proposed ([#1306])**. |
| `VERIFY1290_IID=1290` / `LEFTOVER_COMPLETE=1` invented as HTTP fail-closed | Out of scope. No live HTML probe. |

## Ordered implementation slices

| Slice | Who | Deliverable | Blocks |
|-------|-----|-------------|--------|
| **0 — this design** | design_author | This ADR **0009**, architecture `#dex-hub-wrap-labels`, **Q22**, `docs/README.md` index. Transported on `cac-design-issue-1306` only (no design-only PR). Builds on unpublished `cac-design-issue-1302` 0009/Q22; retargets leftover iid to **#1306**. | Slice 1 |
| **1 — script + Makefile** | implement | On **current `main`**: reconstruct `scripts/qa/verify-issue-1290.sh` (`free_tcp_port 30129` from 703 function only, subshell port env, full B1290-4 greps, leftover **#1306** header); aliases `verify-issue-1302` and `verify-issue-1306`; Cloud Agent + Frontend help for **1290 / 1302 / 1306**. **Insert** slice-0 sections/file (do not whole-file checkout architecture / qa-invariants / README from this tip). Continue open !1306 or a new branch; do not retarget closed !1302. Do not merge `f237a6e6` or `50eb149f` as-is. Do not touch `scripts/test-commit-msg-hook.sh`. Keep `git ls-files frontend-dapp/node_modules` empty. | Slice 2 |
| **2 — crosslinks** | implement | **Insert** slice-0 docs (ADR 0009 **Proposed [#1306]**, architecture `#dex-hub-wrap-labels`, **Q22**, `docs/README.md` index) **plus** leftover **#1306** wording in `AGENTS.md`, `docs/testing.md`, `docs/frontend.md`, PROTOCOL_STATS, PROTOCOL_HUB (**Q22** / **B1290-1–B1290-8**; related #1302 / !1290 / #1240). Optional comment on 1240 script / protocol-page spec. **No** product TSX, **no** dirty files from `f237a6e6`, **no** `Fixes #1290`, **no** `Closes #1302`. | none — slices 1–2 together are leftover-complete |
| **3 — related glance (not this ticket)** | reviewer / operator on [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) leftover 1 | After frontend Coolify rebuild: `/protocol` hub **cUSTC / USD** + **cLUNC / USD**; oracle tabs **USTC** / **LUNC** / **vFDUSD**; selected vFDUSD not **VFDUSD**. Record on **#1305**, not #1306. | **Not** the #1306 close gate |

Slice 1 **must not** wait on slice 3. Slice 3 **must not** be attempted as a Coolify checkbox, deploy, or #1306 close comment.

No product-issue dependencies. #1240, !1290, and closed !1302 are already on `main`. Open !1306 is a starting patch to amend, not a blocker. #1300 is numbering reservation (0008/Q21), not a wait. #1305 leftover 1 is the Coolify glance, not a wait for this wiring MR.

## Tests

**Implement merge (slices 1–2) — leftover-complete:**

```bash
VERIFY_ISSUE_1290_SKIP_E2E=1 make verify-issue-1290
make verify-issue-1302   # same, with skip still exported if needed
make verify-issue-1306   # same
make verify-issue-1240   # child still standalone
```

Provisioned agent / CI (Playwright + Chromium present):

```bash
make verify-issue-1290
```

Assert:

- Child 1240 still 8/8 (or current step count) without requiring Playwright.
- Bundle with skip: 1240 + hubPriceTicker + skipped E2E + **full B1290-4 greps** (including this ADR **0009**, architecture, **Q22**), exit 0.
- Bundle without skip on a provisioned VM: Playwright P1/P2/P4 casing (5 workers); `free_tcp_port` / 30129 subshell present in the script; **no** `ln -sfn` `frontend-dapp/node_modules` in `verify-issue-1290.sh`.
- `make help` Cloud Agent and Frontend lines list **verify-issue-1290**, **verify-issue-1302**, and **verify-issue-1306**.
- `git ls-files frontend-dapp/node_modules` empty. `scripts/test-commit-msg-hook.sh` unchanged vs `main`.
- `make test-commit-msg-hook` still passes (do not edit that script in this MR).
- `docs/testing.md` leftover row cites **#1306** / **Q22** / **B1290**, not the #1290 product PR and not as if #1302 were still the open leftover.
- No `docs/adr/0008-verify-issue-1290-hub-wrap-labels.md`. No second **Q21** table.
- ADR Status line is **Proposed ([#1306])**, not Accepted.

## Rollout

1. Merge slices 1–2 to `main` via **#1306** (amended open PR or a new PR). That merge **is** leftover-complete for #1306. Frontend Coolify auto-deploy may rebuild; labels are already on `main` from !1290 — this merge is verify-only + slice-0 docs.
2. Implement **must not** flip Coolify.
3. Production `/protocol` glance (hub wrap vs CEX; vFDUSD casing) is recorded on [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) leftover 1, **not** on #1306. No UUID/token.

## Rollback

Revert the #1306 follow-up (verify script deltas, Makefile aliases + help, crosslinks, slice-0 docs). Product labels remain from !1290. The weaker !1302 sketch (`verify-issue-1290` target) may remain until reverted separately. No schema, wasm, or env rollback. Frontend image rollback is unrelated (verify-only MR). Do not revert #1300’s ADR 0008 / Q21 as part of this rollback.

## Integration completion criteria

**#1306 leftover-complete is this wiring MR.** Slices 1–2 on `main` are that MR. Green `SKIP_E2E=1` alone does **not** close leftover (the MR also needs aliases, help, `free_tcp_port`, full B1290-4, leftover **#1306** naming, slice-0 docs). Slice 3 is not this ticket’s close gate.

The MR is leftover-complete when:

- Slice-0 is on `main` via **inserts** (this ADR **0009** **Proposed [#1306]**, architecture `#dex-hub-wrap-labels`, **Q22**, `docs/README.md` index) — not left only on `cac-design-issue-1306` / `cac-design-issue-1302`, and not a whole-file checkout of overlap files from this tip.
- No `0008-verify-issue-1290-*.md`. **Q21** / ADR **0008** remain #1300’s if/when that design lands.
- `VERIFY_ISSUE_1290_SKIP_E2E=1 make verify-issue-1290` exits 0 on a worktree from `main` + that MR, including full B1290-4 greps.
- Aliases `make verify-issue-1302` and `make verify-issue-1306` are the same target.
- Cloud Agent and Frontend `help` list **verify-issue-1290**, **verify-issue-1302**, and **verify-issue-1306**.
- Script copies `free_tcp_port` from 703 (function only) and keeps 30129 env in the Playwright subshell. No `ln -sfn` `frontend-dapp/node_modules` bootstrap.
- **B1290-1–B1290-8** greppable from this ADR + **Q22**.
- Forbidden paths from `f237a6e6` are absent. Hook-test script untouched.
- No Coolify secret/UUID in the MR.
- PR body does not `Fixes #1290` and does not `Closes #1302`. Crosslinks name leftover **#1306**.
- Production `/protocol` glance is **not** required on #1306; it lives on [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) leftover 1.

Keywords on the issue are **not** approval to deploy or to write `DESIGN: APPROVE`.
