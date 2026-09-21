# ADR 0009: `verify-issue-1290` bundle for hub wrap vs CEX labels

## Status

Proposed ([#1306](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1306))

Leftover QA / regression wiring after product labels and the weaker !1302 sketch landed. This ADR does **not** relabel `/protocol`, change indexer or Venus JSON, flip Coolify, deploy, spend, expand custody/policy, or write `DESIGN: APPROVE`. Keywords on the issue (“architecture”, “deploy”) are not approval. Deploy/spend/custody/policy expansion stays under [agent-control #297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297).

Playbooks (do not duplicate P1240 here): [`skills/AGENTS_FRONTEND_PROTOCOL_STATS.md`](../../skills/AGENTS_FRONTEND_PROTOCOL_STATS.md) (**P1240-1–P1240-8**), [`skills/AGENTS_FRONTEND_PROTOCOL_HUB.md`](../../skills/AGENTS_FRONTEND_PROTOCOL_HUB.md) (**H11–H16**). Overview: [`architecture.md`](../architecture.md#dex-hub-wrap-labels). Product table: [`frontend.md`](../frontend.md) **P1240**. Leftover row: [`qa-invariants.md`](../qa-invariants.md) **Q22**.

ADR **0007** is reserved by the #1265 census (`docs/adr/0007-route-solve-remaining-failures.md`). ADR **0008** and **Q21** `{#post-merge-ops-1300}` are reserved by leftover-ops [#1300](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1300) (`docs/adr/0008-post-merge-leftover-1287-1298.md` on `origin/cac-design-issue-1300`). This ticket keeps **0009** / **Q22** (first reserved on `cac-design-issue-1302`; retargeted here to leftover **#1306**). Do not keep dual `0008-*.md` or two **Q21** tables. [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) leftover 3 already forbids merging `cac-design-issue-1300` / `cac-design-issue-1302` as-is. Do not merge this design branch (`cac-design-issue-1306`) as a design-only PR. Implement must keep Status **Proposed ([#1306])** until a reviewer accepts the design — do not self-mark **Accepted**.

## Outcome

1. **Bundle.** `make verify-issue-1290` is the leftover regression entry for hub wrap **cLUNC** vs CEX **LUNC** after [!1290](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1290) merged the #1240 naming follow-up. Makefile aliases: **`verify-issue-1302`** (issue AC **B1290-1**) and **`verify-issue-1306`** (this leftover iid). The script always runs child `make verify-issue-1240` (**P1240-1–P1240-8**), then `hubPriceTicker` Vitest, then optional Playwright `e2e/protocol-page.spec.ts` at **5 workers**, then the full **B1290-4** greps (eight docs **plus** recipe-anchored Makefile alias recipes so dropping an alias **recipe** fails the step), then a **separate** `run_step` (not B1290-4) that pins Cloud Agent + Frontend `@echo` lines for **1290 / 1302 / 1306**. Help / `.PHONY` hits are not B1290-4. Tests-only `make help` without that `run_step` is not leftover-complete.
2. **No product delta.** Hub maps, oracle tabs, and CSS `uppercase` / `text-transform: none` stay as shipped on `main` (`HUB_PRICE_TICKER_LABEL.lunc = cLUNC`; API id stays `lunc`; no `clunc` path). Implement does not reopen [#1240](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1240) or closed !1290 for copy.
3. **This leftover is [#1306](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1306), not a reopen of closed !1302.** Closed [!1302](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1302) already merged sketch `f237a6e6` (`92c84406` on `main`). Reconstruct the remaining gaps on **current `main`**. Open [!1306](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1306) (`origin/issue/1302`) may be amended/rebased; do **not** retarget the closed PR. Do **not** merge `f237a6e6` as-is. Do **not** merge `2d4e8b64` as-is (hook-forbidden trailer on that tip). Do **not** land `frontend-dapp/node_modules` (symlink / mode `120000` — [#1298](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1298) / [!1299](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1299)) or leftover `FAIL` residue in `scripts/test-commit-msg-hook.sh` ([#1300](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1300) / [!1301](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1301)). Do not touch the hook-test script. Recreate the same tree with a body that passes `.githooks/commit-msg` (no `Co-authored-by`, no mailbox, no hook-forbidden keyword). Never `--no-verify`.
4. **Leftover-complete is this #1306 wiring MR.** Alias `verify-issue-1302` **and** `verify-issue-1306`, Cloud Agent + Frontend `help` for **1290 / 1302 / 1306** proven by a dedicated `run_step` pinning `@echo` lines (not B1290-4, not Tests-only `make help`), `free_tcp_port` 30129, full **B1290-4** (eight docs **plus** recipe-anchored Makefile alias greps), leftover **#1306** naming, slice-0 docs (this ADR **0009**, architecture `#dex-hub-wrap-labels`, **Q22**, README index), and **policy-clean** commit bodies on the merge range. Green `VERIFY_ISSUE_1290_SKIP_E2E=1` is a required implement check, **not** leftover-complete and **not** a land gate on a VM that cannot provision Node 24. Before merge, leftover land evidence is a **provisioned Cloud Agent** (Node 24 via `scripts/setup-cloud-agent-toolchain.sh` / `scripts/with-node.sh`; Playwright Chromium via `scripts/setup-cloud-agent-localterra.sh` `_ensure_playwright_browsers` or the equivalent `playwright install chromium`; do **not** treat `scripts/setup-browser-cloud-agent.sh` as the Chromium install) green on **both** skip-E2E **and** full `make verify-issue-1306` (Playwright `protocol-page`, **5 workers**). Current Woodpecker is gitleaks-only (`.woodpecker.yaml`) and does **not** run this bundle — it must not stand in. A *future* CI job that actually runs `make verify-issue-1306` may substitute. Production `/protocol` visual (hub wrap vs CEX native; selected **vFDUSD** tab not **VFDUSD**) is [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) leftover 1 (related #1290 AC), **not** this ticket and **not** #297. Slice 3 is not the #1306 close gate.

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
- Open [!1306](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1306) (`origin/issue/1302` @ `2d4e8b64`, tree after `50eb149f`) already has leftover **#1306** naming, Status **Proposed ([#1306])**, aliases `verify-issue-1302` / `verify-issue-1306`, `free_tcp_port` 30129, and eight-file B1290-4 greps. That tip is **still not landable**: (1) commit body contains a `Co-authored-by` agent-mailbox trailer (`.githooks/commit-msg` / `pre-push`); (2) B1290-4 does **not** grep recipe-anchored Makefile aliases, so unanchored `grep -qE "verify-issue-1306" Makefile` still PASSes after deleting only the `verify-issue-1306: verify-issue-1290` recipe (`.PHONY` + help still hit); (3) help tokens exist on Cloud Agent + Frontend `@echo` lines but the script has **no** help `run_step` (Tests-only `make help` / unanchored Makefile greps are the same false PASS); (4) failure to provision Node 24 via `scripts/with-node.sh` / `cloud_agent_ensure_node` on a gate VM cannot run child 1240 or `hubPriceTicker` — that is an **environment gap**, not a script defect, and skip-E2E there is **not** a land gate (`VERIFY_ISSUE_1290_SKIP_E2E=1` skips Playwright only). Recreate the tree with a policy-clean message, add the recipe-anchored Makefile greps **and** the help `@echo` `run_step`, and land leftover evidence only on a provisioned Cloud Agent after skip-E2E **and** full Playwright (Chromium from `_ensure_playwright_browsers`, not `setup-browser-cloud-agent.sh`). Do **not** merge `2d4e8b64` as-is. Do **not** retarget closed !1302.
- `origin/cac-design-issue-1300` already published ADR **0008** and **Q21** `{#post-merge-ops-1300}` on the same three overlap files (`architecture.md`, `qa-invariants.md`, `README.md`). This ticket must not take 0008/Q21.
- `git ls-files frontend-dapp/node_modules` is empty on `origin/main` (merge kept !1299). `f237a6e6` still has that gitlink (`120000`). Do not reintroduce it.
- `scripts/test-commit-msg-hook.sh` is clean on `main` via !1301. `f237a6e6` duplicated a `FAIL` / `exit 1` block. Do not edit that script.

This ADR is the versioned contract (retarget of the unpublished `cac-design-issue-1302` 0009/Q22 slot). Implement lands **#1306** on current `main`. Do not retarget closed !1302. Do not merge `f237a6e6` or `2d4e8b64` as-is. This design branch (`cac-design-issue-1306`) transports the ADR only — no design-only PR.

## Non-goals

- Changing hub/oracle copy, CSS, ticker ids, `?ticker=` allowlist, or Venus headings (**P1240-8**).
- Indexer `GET /hub-prices`, oracle catalog, or Venus `eth_call`.
- A new `AGENTS_POST_MERGE_OPS_1302.md` or `AGENTS_POST_MERGE_OPS_1306.md` playbook (amend existing PROTOCOL_STATS / PROTOCOL_HUB skills).
- A new census ADR / Stay memo. Product is shipped; this is QA wiring.
- Scraping production HTML as leftover-complete. No `VERIFY1290_REQUIRE_LIVE` curl of `dex.cl8y.com`.
- Recording the Coolify `/protocol` glance on **#1306**. That glance is [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) leftover 1 (related #1290 AC).
- Playwright `install-deps` / NSS (`libnspr4.so`) as this ticket. Gate VMs use `VERIFY_ISSUE_1290_SKIP_E2E=1`. A provisioned Cloud Agent runs the full suite (Node 24 via `setup-cloud-agent-toolchain.sh` / `with-node.sh`; Playwright Chromium via `setup-cloud-agent-localterra.sh` `_ensure_playwright_browsers`). Do **not** treat `setup-browser-cloud-agent.sh` as the Chromium install (Chrome + Keplr only). Current Woodpecker is gitleaks-only and must not. A *future* CI job that actually runs `make verify-issue-1306` may substitute.
- Relanding `frontend-dapp/node_modules` (file or symlink). Relanding hook-test FAIL duplication. Editing `scripts/test-commit-msg-hook.sh`.
- Copying `verify-issue-703.sh` `ln -sfn` `frontend-dapp/node_modules` bootstrap ([#1298](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1298) / [!1299](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1299)), including to paper over a missing nvm install. Copy **only** `free_tcp_port`.
- Coolify UI, HMAC / `autonomy.rs`, founder card, or #297 deploy.
- `Fixes #1290` or `Closes #1302` on the #1306 PR body — !1290 and !1302 are already merged. Cite leftover **#1306**; related #1302 / !1290 / #1240.
- Opening a design-only PR from `cac-design-issue-1306` or `cac-design-issue-1302`.
- Retargeting closed !1302. Merging open !1306 as-is without the #1306 retarget, Proposed status, policy-clean commit bodies, recipe-anchored Makefile alias greps, the help `@echo` `run_step`, and provisioned-Cloud-Agent full Playwright (Chromium from `_ensure_playwright_browsers`, not `setup-browser-cloud-agent.sh`). Treating current Woodpecker as leftover land evidence.
- Treating a gate VM that cannot provision Node 24 via `scripts/with-node.sh` / `cloud_agent_ensure_node` as leftover-complete or as a reason to skip Vitest / child 1240. `VERIFY_ISSUE_1290_SKIP_E2E=1` skips Playwright only. `scripts/lib/cloud-agent-toolchain.sh` defaults `NVM_DIR=$HOME/.nvm` and may `curl` install nvm before failing — missing `NVM_DIR` alone is not the FAIL signal.
- Merging `2d4e8b64` as-is (hook-forbidden trailer). Using `git commit --no-verify` / `git push --no-verify` to keep that trailer.
- Taking ADR **0008** / **Q21** (those belong to [#1300](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1300) unless that design is withdrawn). Merging this design branch, `cac-design-issue-1302`, or `cac-design-issue-1300` as-is ([#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) leftover 3).
- Self-marking this ADR **Accepted**.

## Decision

**Thin wrapper over shipped P1240.** Child `verify-issue-1240` remains the invariant harness. Closed !1302 landed a weaker sketch (wrapper exists; alias / `free_tcp_port` / eight-file greps / slice-0 docs / leftover naming do not). Leftover **#1306** closes those ADR gaps. After the `2d4e8b64` tree, remaining land work is policy-clean commit bodies, recipe-anchored Makefile alias greps in B1290-4, a separate help `run_step` pinning Cloud Agent + Frontend `@echo` lines (not Tests-only `make help`), and provisioned-Cloud-Agent full Playwright with Chromium from `_ensure_playwright_browsers` — not a product rewrite.

### #1306 MR (do not retarget closed !1302)

Continue open [!1306](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1306) (`origin/issue/1302`) **or** open a new branch from **current `main`** (tip at this design: `729b097f`; use whatever `origin/main` is at implement time). Do not retarget or reopen !1302. Do not merge `f237a6e6` as-is. Do not merge `50eb149f` as-is. Do not merge `2d4e8b64` as-is (hook-forbidden trailer; B1290-4 missing recipe-anchored Makefile alias greps). This design tip is **not** an ancestor of `origin/main` (merge-base `6d34da13`).

**Slice-0 apply — insert; do not whole-file checkout.** Do **not** `git checkout <design-sha> --` `docs/architecture.md` / `docs/qa-invariants.md` / `docs/README.md` from this tip or from `cac-design-issue-1302`. #1300 (and later `main`) touches those same three files. Cherry-pick of whole-file 0008-era paths is unsafe.

1. **Insert slice 0 onto the #1306 MR:**
   - New file `docs/adr/0009-verify-issue-1290-hub-wrap-labels.md` (copy from this branch). Status **Proposed ([#1306])**. Do **not** add a second `0008-*.md`. Do **not** mark **Accepted**.
   - `docs/architecture.md`: insert section `## DEX hub wrap vs CEX labels {#dex-hub-wrap-labels}` (do not replace the whole file).
   - `docs/qa-invariants.md`: insert **Q22** `{#ops-hub-wrap-1302}` **after Q21** if #1300 already landed, else **after Q20** — still number it **Q22**; leave **Q21** / ADR **0008** to #1300. Keep the `{#ops-hub-wrap-1302}` anchor (first leftover reservation). Table text names leftover **#1306**. Add the Related-docs bullet for ADR 0009.
   - `docs/README.md`: insert the ADR 0009 index line **after ADR 0008** if present, else **after ADR 0007**. If 0008 is not yet on `main`, a parenthetical that 0008 / Q21 are reserved for #1300 leftover-ops is enough — do not invent a stub `0008-*.md` here. Architecture Overview bullet adds `#dex-hub-wrap-labels`.
2. Reconstruct `scripts/qa/verify-issue-1290.sh` (Playwright isolation + full B1290-4 greps including recipe-anchored Makefile aliases + leftover **#1306** naming + a **separate** help `run_step` pinning Cloud Agent + Frontend `@echo` lines). Copy **only** `free_tcp_port` from [`scripts/qa/verify-issue-703.sh`](../../scripts/qa/verify-issue-703.sh) — **not** 703’s `ln -sfn` `frontend-dapp/node_modules` bootstrap. Add Makefile aliases `verify-issue-1302: verify-issue-1290` and `verify-issue-1306: verify-issue-1290`. Add **1290, 1302, and 1306** to Cloud Agent and Frontend `help` (immediately after `verify-issue-1240`). On the published `2d4e8b64` tree those aliases and help tokens already exist — keep them, **grep the recipes** (B1290-4), and add the help `@echo` `run_step` (not B1290-4, not unanchored Makefile substring greps). Recreate that tree with a policy-clean commit body before merge.
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
| 4 | Grep `verify-issue-1290` in the **eight docs**: `docs/testing.md`, `AGENTS.md`, `skills/AGENTS_FRONTEND_PROTOCOL_STATS.md`, `skills/AGENTS_FRONTEND_PROTOCOL_HUB.md`, `docs/frontend.md`, this ADR (`docs/adr/0009-verify-issue-1290-hub-wrap-labels.md`), `docs/architecture.md` (`dex-hub-wrap-labels` or `verify-issue-1290`), and `docs/qa-invariants.md` **Q22**. **Also grep recipe-anchored Makefile aliases** (B1290-1): `^verify-issue-1302:[[:space:]]*verify-issue-1290[[:space:]]*$` and `^verify-issue-1306:[[:space:]]*verify-issue-1290[[:space:]]*$`. Unanchored `grep -qE "verify-issue-1306" Makefile` is **not** B1290-4 (`.PHONY` / help still hit after dropping the recipes). Cloud Agent + Frontend `help` is **step 5**, not this step. Weaker sketch greps (testing + AGENTS + PROTOCOL_STATS only, eight docs without Makefile, or unanchored Makefile substring greps) are **not** B1290-4. |
| 5 | Separate `run_step` (not B1290-4): Cloud Agent + Frontend `@echo` lines list `verify-issue-1290` / `1302` / `1306`. Pin `^[[:space:]]*@echo` so `.PHONY` / recipe hits are not a false PASS. Frontend greps require `verify-issue-1240` before 1290/1302/1306 (keep the `build-frontend` Frontend line out of that match). Unanchored `grep -qE "verify-issue-1306" Makefile` is **not** this step. This step is leftover-complete — green `make verify-issue-1306` must run it. |

**Playwright step (copy `free_tcp_port` from [`scripts/qa/verify-issue-703.sh`](../../scripts/qa/verify-issue-703.sh) — function only):**

```bash
free_tcp_port 30129
run_step "playwright e2e-smoke protocol-page (5 workers)" \
  bash -c 'PLAYWRIGHT_SKIP_CHAIN=1 PLAYWRIGHT_WEB_PORT=30129 PLAYWRIGHT_BASE_URL=http://127.0.0.1:30129 bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright test --project=e2e-smoke --workers=5 e2e/protocol-page.spec.ts'
```

**B1290-4 grep step (eight docs + recipe-anchored Makefile aliases):**

```bash
run_step "docs: #1306 verify crosslink (B1290-4)" \
  bash -c 'grep -qE "verify-issue-1290" docs/testing.md && \
  grep -qE "verify-issue-1290" AGENTS.md && \
  grep -qE "verify-issue-1290" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md && \
  grep -qE "verify-issue-1290" skills/AGENTS_FRONTEND_PROTOCOL_HUB.md && \
  grep -qE "verify-issue-1290" docs/frontend.md && \
  grep -qE "verify-issue-1290" docs/adr/0009-verify-issue-1290-hub-wrap-labels.md && \
  grep -qE "dex-hub-wrap-labels|verify-issue-1290" docs/architecture.md && \
  grep -qE "Q22|ops-hub-wrap-1302" docs/qa-invariants.md && \
  grep -qE "^verify-issue-1302:[[:space:]]*verify-issue-1290[[:space:]]*$" Makefile && \
  grep -qE "^verify-issue-1306:[[:space:]]*verify-issue-1290[[:space:]]*$" Makefile'
```

**Help `run_step` (not B1290-4; leftover-complete):** Cloud Agent and Frontend `help` `@echo` lines must list `verify-issue-1290`, `verify-issue-1302`, and `verify-issue-1306`. Open `2d4e8b64` already has the help tokens and still has **no** help grep — Tests-only `make help` is not this step. Unanchored `grep -qE "verify-issue-1306" Makefile` is **not** this assert and **not** B1290-4 (`.PHONY` still hits). Keep the `build-frontend` Frontend line out of the match.

```bash
run_step "help: Cloud Agent + Frontend list 1290/1302/1306" \
  bash -c 'grep -qE "^[[:space:]]*@echo \"Cloud Agent:.*verify-issue-1290" Makefile && \
  grep -qE "^[[:space:]]*@echo \"Cloud Agent:.*verify-issue-1302" Makefile && \
  grep -qE "^[[:space:]]*@echo \"Cloud Agent:.*verify-issue-1306" Makefile && \
  grep -qE "^[[:space:]]*@echo \"Frontend:.*verify-issue-1240.*verify-issue-1290" Makefile && \
  grep -qE "^[[:space:]]*@echo \"Frontend:.*verify-issue-1240.*verify-issue-1302" Makefile && \
  grep -qE "^[[:space:]]*@echo \"Frontend:.*verify-issue-1240.*verify-issue-1306" Makefile'
```

**`with-node.sh` / nvm:** steps 1–2 (child 1240 Vitest paths + `hubPriceTicker`) and the default-on Playwright step all go through `scripts/with-node.sh` → `cloud_agent_ensure_node`. `scripts/lib/cloud-agent-toolchain.sh` defaults `NVM_DIR=$HOME/.nvm` and may `curl` install nvm before failing. Failure to provision Node 24 via `scripts/with-node.sh` / `cloud_agent_ensure_node` is **FAIL**, not skip — not “missing `NVM_DIR`”. `VERIFY_ISSUE_1290_SKIP_E2E=1` skips Playwright only. Land leftover evidence on a **provisioned Cloud Agent** (Node 24 via `scripts/setup-cloud-agent-toolchain.sh` / `scripts/with-node.sh`; Playwright Chromium via `scripts/setup-cloud-agent-localterra.sh` `_ensure_playwright_browsers` or the equivalent `playwright install chromium`; do **not** treat `scripts/setup-browser-cloud-agent.sh` as the Chromium install). Current Woodpecker is gitleaks-only and must not stand in. A *future* CI job that actually runs `make verify-issue-1306` may substitute.

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
| QA script | Reconstruct `scripts/qa/verify-issue-1290.sh` (already on `main` from closed !1302): `free_tcp_port 30129` (copied from 703, not 703’s `ln -sfn`), full B1290-4 greps **including recipe-anchored Makefile aliases**, leftover **#1306** header, **plus** a separate help `run_step` pinning Cloud Agent + Frontend `@echo` lines (not B1290-4). Recreate `2d4e8b64` with a policy-clean commit body. |
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
| **B1290-1** | Bundle runs child **1240**. Child FAIL fails the stack. Aliases `verify-issue-1302` and `verify-issue-1306` are the same script. |
| **B1290-2** | Vitest `hubPriceTicker.test.ts` is a required step (not folded into 1240). |
| **B1290-3** | Playwright protocol-page is default-on at **5 workers**, port **30129**, `PLAYWRIGHT_SKIP_CHAIN=1`. Copy `free_tcp_port` from `verify-issue-703.sh` (function only — not 703’s `ln -sfn` bootstrap) and free **30129** first. Port + `PLAYWRIGHT_BASE_URL` only in that step’s subshell. Skip only via `VERIFY_ISSUE_1290_SKIP_E2E=1` or missing Playwright package. Installed package + crash (missing Chromium / NSS) = FAIL. Failure to provision Node 24 via `scripts/with-node.sh` / `cloud_agent_ensure_node` = FAIL (not skip). `VERIFY_ISSUE_1290_SKIP_E2E=1` skips Playwright only. |
| **B1290-4** | Crosslinks: testing, AGENTS, PROTOCOL_STATS, PROTOCOL_HUB, frontend.md, this ADR (`docs/adr/0009-verify-issue-1290-hub-wrap-labels.md`), architecture `#dex-hub-wrap-labels`, **Q22**, **and recipe-anchored Makefile aliases** `^verify-issue-1302:[[:space:]]*verify-issue-1290[[:space:]]*$` / `^verify-issue-1306:[[:space:]]*verify-issue-1290[[:space:]]*$`. Unanchored `grep -qE "verify-issue-1306" Makefile` is **not** B1290-4 (`.PHONY` / help still hit after dropping the recipes). Cloud Agent + Frontend `help` is a **separate** `run_step` pinning `@echo` lines (not this ID). Slice-0 files ride #1306 as **inserts** so those greps can pass. Docs-only greps without recipe-anchored Makefile patterns are **not** B1290-4. |
| **B1290-5** | No indexer/hub-price/Venus/API change. No `clunc` path. No product TSX in this MR unless a shipped P1240 assertion is actually wrong (then reopen #1240, do not hide it in the bundle). |
| **B1290-6** | Do not reopen closed !1290 / #1240 for ops/QA. Do not retarget closed !1302. Do not `Fixes #1290`. Do not `Closes #1302`. Do not wait on Woodpecker quota as leftover evidence. |
| **B1290-7** | Do not add `frontend-dapp/node_modules`, hook-test FAIL residue, or `.gitignore` churn. Do not edit `scripts/test-commit-msg-hook.sh`. Do not create `AGENTS_POST_MERGE_OPS_1302.md` / `AGENTS_POST_MERGE_OPS_1306.md`. Do not take ADR 0008 / Q21. Merge-range commit **bodies** must pass `.githooks/commit-msg` / `pre-push` (no `Co-authored-by`, no mailbox, no hook-forbidden keyword). Do not land `2d4e8b64` as-is. Never `--no-verify`. |
| **B1290-8** | Green `SKIP_E2E=1` ≠ leftover-complete. Leftover-complete is the **#1306 wiring MR** on `main` (aliases, `free_tcp_port`, full B1290-4 including recipe-anchored Makefile, leftover **#1306** naming, slice-0 ADR 0009 / **Q22** / architecture / README, policy-clean messages, help `run_step` pinning `@echo` lines). Land leftover evidence requires a **provisioned Cloud Agent** (Node 24 via `scripts/setup-cloud-agent-toolchain.sh` / `scripts/with-node.sh`; Playwright Chromium via `scripts/setup-cloud-agent-localterra.sh` `_ensure_playwright_browsers` or the equivalent `playwright install chromium`; do **not** treat `scripts/setup-browser-cloud-agent.sh` as the Chromium install): green skip-E2E **and** full `make verify-issue-1306` (Playwright `protocol-page`, 5 workers). Current Woodpecker is gitleaks-only and must not stand in. A *future* CI job that actually runs `make verify-issue-1306` may substitute. A gate VM that cannot provision Node 24 via `scripts/with-node.sh` / `cloud_agent_ensure_node` is **not** that runner. Production `/protocol` visual is [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) leftover 1 (related #1290 AC), not this ticket. No HTML scrape. Not #297. |

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
| Merge open !1306 (`50eb149f`) as-is | ADR self-marked **Accepted (#1302)**; leftover naming still #1302; `Closes #1302`; no `verify-issue-1306` alias. Superseded by `2d4e8b64`. |
| Merge open !1306 (`2d4e8b64`) as-is | Wiring tree is close, but commit body has a `Co-authored-by` agent-mailbox trailer (`.githooks/commit-msg`), B1290-4 still omits recipe-anchored Makefile alias greps, and help tokens have **no** help `run_step`. Recreate the tree; do not keep the trailer. |
| Unanchored Makefile greps for `verify-issue-1302` / `verify-issue-1306` | `.PHONY` and both `help` echoes still match after deleting only `verify-issue-1306: verify-issue-1290`. Use recipe-anchored `^verify-issue-1302:[[:space:]]*verify-issue-1290[[:space:]]*$` / `^verify-issue-1306:[[:space:]]*verify-issue-1290[[:space:]]*$`. Keep `help` as a separate `@echo` `run_step` (not B1290-4). |
| Tests-only `make help` / no help `run_step` | Open `2d4e8b64` already lists 1290/1302/1306 in help and still has no grep. Leftover-complete must run `run_step "help: Cloud Agent + Frontend list 1290/1302/1306"` pinning `^[[:space:]]*@echo` (Frontend greps require `verify-issue-1240` before those ids; keep `build-frontend` out). |
| Treat `setup-browser-cloud-agent.sh` as Playwright Chromium | That script is Chrome + Keplr only. `setup-cloud-agent-toolchain.sh` does **not** install Playwright. Chromium is `setup-cloud-agent-localterra.sh` `_ensure_playwright_browsers` (`playwright install chromium`). Package-present / browsers-missing is FAIL or skip-E2E treated as complete. |
| Eight-file B1290-4 without Makefile aliases | Dropping `verify-issue-1306:` still PASSes. Grep the recipes (**B1290-4**). |
| Treat gate-VM Node provision gap as leftover-complete | Child 1240 / Vitest FAIL when `scripts/with-node.sh` / `cloud_agent_ensure_node` cannot provision Node 24. Land leftover evidence on a provisioned Cloud Agent. Skip-E2E does not skip Vitest. |
| Treat current Woodpecker as leftover land evidence | `.woodpecker.yaml` is gitleaks-only; no job runs `make verify-issue-1306`. A *future* CI job that actually runs the bundle may substitute. |
| Keep ADR 0008 / Q21 on this ticket | Collides with `origin/cac-design-issue-1300`. [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) leftover 3. Keep **0009** / **Q22**. |
| Whole-file `git checkout <design-sha> --` architecture / qa-invariants / README | This tip is not an ancestor of `origin/main`; #1300 edits the same files. Insert sections. |
| Close #1306 on a Coolify glance recorded here | Splits close-out with #1305 leftover 1. Leftover-complete is the wiring MR. |
| Occupying a new ADR / Q23 slot | 0009 / Q22 already reserved for this bundle. Retarget the ticket iid only. |

## Complexity added / removed

**Added:** one bash wrapper (reconstruct), Makefile aliases `1302` + `1306`, Cloud Agent + Frontend help for **1290 / 1302 / 1306**, full B1290-4 greps (docs + recipe-anchored Makefile alias recipes), help `run_step` pinning `@echo` lines, optional E2E skip flag, `free_tcp_port` on 30129, **Q22** / **B1290**, slice-0 docs on #1306 (ADR **0009**), policy-clean merge-range messages, provisioned-Cloud-Agent Playwright land gate.

**Removed:** the gap where post-merge hub **cLUNC** vs CEX **LUNC** had no agent `make` entry besides 1240’s CSS/docs subset; the closed-PR sketch’s weaker grep / missing alias / stale-Vite flake; the 0008/Q21 collision with #1300; leftover-complete split onto a Coolify glance that #1305 already owns; dual “leftover #1302” close-out after that sketch already merged; alias-drop drift that unanchored Makefile greps miss; help-token false PASS without an `@echo` `run_step`; treating `setup-browser-cloud-agent.sh` or `setup-cloud-agent-toolchain.sh` as Playwright Chromium; treating current Woodpecker as leftover land evidence; landing a hook-forbidden trailer.

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
| B1290-4 docs greps PASS after dropping Makefile alias **recipes** | Forbidden alias drift (**B1290-1**). Unanchored `grep -qE "verify-issue-1306" Makefile` still PASSes because `.PHONY` / help remain. Use recipe-anchored `^verify-issue-1302:[[:space:]]*verify-issue-1290[[:space:]]*$` / `^verify-issue-1306:[[:space:]]*verify-issue-1290[[:space:]]*$`. Keep `help` as a separate `@echo` `run_step` (not B1290-4). |
| Help tokens present but no help `run_step` | Forbidden leftover-complete false PASS (`2d4e8b64`). Tests-only `make help` and unanchored Makefile greps still hit `.PHONY`. Pin `^[[:space:]]*@echo` Cloud Agent + Frontend lines; keep `build-frontend` out. |
| Treat `setup-browser-cloud-agent.sh` as Chromium | Package present, browsers missing → FAIL, or skip-E2E treated as complete. Use `setup-cloud-agent-localterra.sh` `_ensure_playwright_browsers`. `setup-cloud-agent-toolchain.sh` does not install Playwright. |
| `scripts/with-node.sh` / `cloud_agent_ensure_node` cannot provision Node 24 | Child 1240 and `hubPriceTicker` FAIL. Not skip. Not a product defect. Not “missing `NVM_DIR`” (`cloud-agent-toolchain.sh` defaults `NVM_DIR=$HOME/.nvm` and may `curl` install nvm first). Run leftover land evidence on a provisioned Cloud Agent. Do not copy 703 `ln -sfn` bootstrap. |
| Skip-E2E green on a gate VM that cannot provision Node 24 treated as land evidence | Forbidden (**B1290-8**). Land leftover evidence on a provisioned Cloud Agent: skip-E2E **and** full Playwright. Current Woodpecker must not. |
| Merge of `2d4e8b64` as-is | Lands a hook-forbidden `Co-authored-by` trailer. Recreate the tree with a policy-clean body. Never `--no-verify`. |
| Worktree without `.env.local` and no primary copy | Playwright may skip factory-address asserts (existing spec `try/catch`); hub/oracle casing still runs. |
| Green make with `SKIP_E2E=1` treated as leftover-complete | Forbidden (**B1290-8**). Leftover-complete is the #1306 wiring MR. |
| Coolify `/protocol` glance recorded on #1306 as the close gate | Forbidden. That glance is #1305 leftover 1. Slice 3 is not this ticket’s close gate. |
| Merge of `f237a6e6` / retarget closed !1302 | Reintroduces node_modules symlink and/or hook FAIL residue; skips alias + `free_tcp_port`. Reconstruct. |
| Merge of open !1306 as-is (`50eb149f` or `2d4e8b64`) | `50eb149f` self-marks **Accepted (#1302)**; `2d4e8b64` has a hook-forbidden trailer, unanchored (or missing) Makefile alias greps, and **no** help `run_step`. Amend / recreate first. |
| Copy 703 `ln -sfn` bootstrap | Reintroduces #1298 gitlink. Copy `free_tcp_port` only. |
| Land ADR 0008 / Q21 on this leftover | Collides with #1300. Use **0009** / **Q22**. |
| Implement flips Coolify | Forbidden (#297). |
| Implement marks this ADR **Accepted** | Forbidden self-approval. Keep **Proposed ([#1306])**. |
| `VERIFY1290_IID=1290` / `LEFTOVER_COMPLETE=1` invented as HTTP fail-closed | Out of scope. No live HTML probe. |

## Ordered implementation slices

| Slice | Who | Deliverable | Blocks |
|-------|-----|-------------|--------|
| **0 — this design** | design_author | This ADR **0009**, architecture `#dex-hub-wrap-labels`, **Q22**, `docs/README.md` index. Transported on `cac-design-issue-1306` only (no design-only PR). Builds on published `8945c85c` / `485b8145`; after !1306 review: policy-clean merge-range bodies, recipe-anchored Makefile alias greps in B1290-4, help `run_step` pinning `@echo` lines, leftover land evidence on a provisioned Cloud Agent (Node 24 via `setup-cloud-agent-toolchain.sh` / `with-node.sh`; Playwright Chromium via `setup-cloud-agent-localterra.sh` `_ensure_playwright_browsers`; skip-E2E **and** full Playwright; current Woodpecker must not; not `setup-browser-cloud-agent.sh`). | Slice 1 |
| **1 — script + Makefile** | implement | On **current `main`**: keep/reconstruct `scripts/qa/verify-issue-1290.sh` (`free_tcp_port` 30129 from 703 function only, subshell port env, full B1290-4 greps **including recipe-anchored Makefile `verify-issue-1302:` / `verify-issue-1306:` recipes**, leftover **#1306** header, **plus** help `run_step` pinning Cloud Agent + Frontend `@echo` lines); aliases `verify-issue-1302` and `verify-issue-1306`; Cloud Agent + Frontend help for **1290 / 1302 / 1306**. **Insert** slice-0 sections/file (do not whole-file checkout architecture / qa-invariants / README from this tip). Continue open !1306; do not retarget closed !1302. Do not merge `f237a6e6`, `50eb149f`, or `2d4e8b64` as-is — recreate `2d4e8b64` with a policy-clean commit body **and** the help `run_step`. Do not touch `scripts/test-commit-msg-hook.sh`. Keep `git ls-files frontend-dapp/node_modules` empty. Land leftover evidence only on a provisioned Cloud Agent (Node 24 via `setup-cloud-agent-toolchain.sh` / `with-node.sh`; Playwright Chromium via `_ensure_playwright_browsers`; not `setup-browser-cloud-agent.sh`) after skip-E2E **and** full Playwright. | Slice 2 |
| **2 — crosslinks** | implement | **Insert** slice-0 docs (ADR 0009 **Proposed [#1306]**, architecture `#dex-hub-wrap-labels`, **Q22**, `docs/README.md` index) **plus** leftover **#1306** wording in `AGENTS.md`, `docs/testing.md`, `docs/frontend.md`, PROTOCOL_STATS, PROTOCOL_HUB (**Q22** / **B1290-1–B1290-8**; related #1302 / !1290 / #1240). Optional comment on 1240 script / protocol-page spec. **No** product TSX, **no** dirty files from `f237a6e6`, **no** `Fixes #1290`, **no** `Closes #1302`. Merge-range bodies pass `.githooks/commit-msg`. | none — slices 1–2 together are leftover-complete after provisioned-Cloud-Agent full verify |
| **3 — related glance (not this ticket)** | reviewer / operator on [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) leftover 1 | After frontend Coolify rebuild: `/protocol` hub **cUSTC / USD** + **cLUNC / USD**; oracle tabs **USTC** / **LUNC** / **vFDUSD**; selected vFDUSD not **VFDUSD**. Record on **#1305**, not #1306. | **Not** the #1306 close gate |

Slice 1 **must not** wait on slice 3. Slice 3 **must not** be attempted as a Coolify checkbox, deploy, or #1306 close comment.

No product-issue dependencies. #1240, !1290, and closed !1302 are already on `main`. Open !1306 is a starting patch to amend (do not land `2d4e8b64` as-is), not a blocker. #1300 is numbering reservation (0008/Q21), not a wait. #1305 leftover 1 is the Coolify glance, not a wait for this wiring MR.

## Tests

**Implement merge (slices 1–2) — leftover-complete.** On a **provisioned Cloud Agent** (Node 24 via `scripts/setup-cloud-agent-toolchain.sh` / `scripts/with-node.sh`; Playwright Chromium via `scripts/setup-cloud-agent-localterra.sh` `_ensure_playwright_browsers` or the equivalent `playwright install chromium`; do **not** treat `scripts/setup-browser-cloud-agent.sh` as the Chromium install; `frontend-dapp/node_modules` present). Current Woodpecker is gitleaks-only and is **not** this runner. A *future* CI job that actually runs `make verify-issue-1306` may substitute. A gate VM that cannot provision Node 24 via `scripts/with-node.sh` / `cloud_agent_ensure_node` is **not** this runner.

```bash
VERIFY_ISSUE_1290_SKIP_E2E=1 make verify-issue-1290
VERIFY_ISSUE_1290_SKIP_E2E=1 make verify-issue-1302
VERIFY_ISSUE_1290_SKIP_E2E=1 make verify-issue-1306
make verify-issue-1240   # child still standalone
```

Then **without** skip (Playwright `protocol-page`, 5 workers):

```bash
make verify-issue-1306
```

Assert:

- Child 1240 still 8/8 (or current step count) without requiring Playwright.
- Bundle with skip: 1240 + hubPriceTicker + skipped E2E + **full B1290-4 greps** (eight docs **plus** recipe-anchored Makefile `^verify-issue-1302:[[:space:]]*verify-issue-1290[[:space:]]*$` / `^verify-issue-1306:[[:space:]]*verify-issue-1290[[:space:]]*$`) + help `run_step` pinning `@echo` lines, exit 0.
- Bundle without skip on a provisioned Cloud Agent: Playwright P1/P2/P4 casing (5 workers); `free_tcp_port` / 30129 subshell present in the script; **no** `ln -sfn` `frontend-dapp/node_modules` in `verify-issue-1290.sh`.
- **Help `run_step` (not B1290-4; leftover-complete):** green `make verify-issue-1306` (skip-E2E included) runs `run_step "help: Cloud Agent + Frontend list 1290/1302/1306"` pinning `^[[:space:]]*@echo` Cloud Agent and Frontend lines for **verify-issue-1290**, **verify-issue-1302**, and **verify-issue-1306**. Frontend greps require `verify-issue-1240` before those ids so the `build-frontend` Frontend line is out of that match. Unanchored `grep -qE "verify-issue-1306" Makefile` and Tests-only `make help` are **not** this step. Open `2d4e8b64` has the tokens and no help grep — do not merge that gap.
- `git ls-files frontend-dapp/node_modules` empty. `scripts/test-commit-msg-hook.sh` unchanged vs `main`.
- `make test-commit-msg-hook` still passes (do not edit that script in this MR).
- Every commit unique to the MR has a body that passes `.githooks/commit-msg` (no `Co-authored-by`, no mailbox, no hook-forbidden keyword). `2d4e8b64` must not be in the merge range as-is.
- `docs/testing.md` leftover row cites **#1306** / **Q22** / **B1290**, not the #1290 product PR and not as if #1302 were still the open leftover.
- No `docs/adr/0008-verify-issue-1290-hub-wrap-labels.md`. No second **Q21** table.
- ADR Status line is **Proposed ([#1306])**, not Accepted.

## Rollout

1. Merge slices 1–2 to `main` via **#1306** (amended open PR or a new PR) **after** policy-clean messages and provisioned-Cloud-Agent skip-E2E **plus** full Playwright. That merge **is** leftover-complete for #1306. Frontend Coolify auto-deploy may rebuild; labels are already on `main` from !1290 — this merge is verify-only + slice-0 docs.
2. Implement **must not** flip Coolify.
3. Production `/protocol` glance (hub wrap vs CEX; vFDUSD casing) is recorded on [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) leftover 1, **not** on #1306. No UUID/token.

## Rollback

Revert the #1306 follow-up (verify script deltas, Makefile aliases + help, crosslinks, slice-0 docs). Product labels remain from !1290. The weaker !1302 sketch (`verify-issue-1290` target) may remain until reverted separately. No schema, wasm, or env rollback. Frontend image rollback is unrelated (verify-only MR). Do not revert #1300’s ADR 0008 / Q21 as part of this rollback.

## Integration completion criteria

**#1306 leftover-complete is this wiring MR.** Slices 1–2 on `main` are that MR. Green `SKIP_E2E=1` alone does **not** close leftover (the MR also needs aliases, the help `@echo` `run_step`, `free_tcp_port`, full B1290-4 including recipe-anchored Makefile, leftover **#1306** naming, slice-0 docs, policy-clean messages). Slice 3 is not this ticket’s close gate.

The MR is leftover-complete when:

- Slice-0 is on `main` via **inserts** (this ADR **0009** **Proposed [#1306]**, architecture `#dex-hub-wrap-labels`, **Q22**, `docs/README.md` index) — not left only on `cac-design-issue-1306` / `cac-design-issue-1302`, and not a whole-file checkout of overlap files from this tip.
- No `0008-verify-issue-1290-*.md`. **Q21** / ADR **0008** remain #1300’s if/when that design lands.
- `VERIFY_ISSUE_1290_SKIP_E2E=1 make verify-issue-1290` **and** `VERIFY_ISSUE_1290_SKIP_E2E=1 make verify-issue-1306` exit 0 on a **Node-capable** worktree from `main` + that MR, including full B1290-4 greps (eight docs **plus** recipe-anchored Makefile aliases) **and** the help `@echo` `run_step`.
- Full `make verify-issue-1306` (Playwright `protocol-page`, 5 workers) is green on a **provisioned Cloud Agent** (Node 24 via `scripts/setup-cloud-agent-toolchain.sh` / `scripts/with-node.sh`; Playwright Chromium via `scripts/setup-cloud-agent-localterra.sh` `_ensure_playwright_browsers` or the equivalent `playwright install chromium`; do **not** treat `scripts/setup-browser-cloud-agent.sh` as the Chromium install). Current Woodpecker is gitleaks-only and is not that evidence. A *future* CI job that actually runs `make verify-issue-1306` may substitute. A gate VM that cannot provision Node 24 via `scripts/with-node.sh` / `cloud_agent_ensure_node` is not that evidence.
- Aliases `make verify-issue-1302` and `make verify-issue-1306` are the same target; B1290-4 greps both **recipes** in `Makefile`.
- Cloud Agent and Frontend `help` list **verify-issue-1290**, **verify-issue-1302**, and **verify-issue-1306**, proven by `run_step "help: Cloud Agent + Frontend list 1290/1302/1306"` pinning `^[[:space:]]*@echo` (Frontend greps require `verify-issue-1240` before those ids; **separate** from B1290-4; not Tests-only `make help`).
- Script copies `free_tcp_port` from 703 (function only) and keeps 30129 env in the Playwright subshell. No `ln -sfn` `frontend-dapp/node_modules` bootstrap.
- **B1290-1–B1290-8** greppable from this ADR + **Q22**.
- Forbidden paths from `f237a6e6` are absent. Hook-test script untouched.
- Merge-range commit bodies pass `.githooks/commit-msg`. `2d4e8b64` is not merged as-is.
- No Coolify secret/UUID in the MR.
- PR body does not `Fixes #1290` and does not `Closes #1302`. Crosslinks name leftover **#1306**.
- Production `/protocol` glance is **not** required on #1306; it lives on [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) leftover 1.

Keywords on the issue are **not** approval to deploy or to write `DESIGN: APPROVE`.
