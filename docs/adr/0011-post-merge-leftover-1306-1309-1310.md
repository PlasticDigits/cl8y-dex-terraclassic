# ADR 0011: Post-merge leftover after PRs 1306 / 1309 / 1310

## Status

Proposed ([#1311](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1311))

Ordinary leftover ops after PRs **1306 / 1309 / 1310** landed on `origin/main` (`0d6eaeee`). This ADR does **not** change pairing TS, flip Coolify auto-deploy, store wasm, spend, expand custody/policy, or write `DESIGN: APPROVE`. Keywords on [#1311](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1311) (`architecture`, `deploy`, Queue implement gate) are **not** approval. Deploy/spend/custody/policy expansion stays under [agent-control #297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297).

Playbook: [`skills/AGENTS_POST_MERGE_OPS_1311.md`](../../skills/AGENTS_POST_MERGE_OPS_1311.md) (**M1311-1–M1311-8**). Overview: [`architecture.md`](../architecture.md#post-merge-leftover-ops-1311). Invariants: [`qa-invariants.md`](../qa-invariants.md) **Q24**. Product pairing: [`AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md`](../../skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md) (**WC-M1–WC-M12**, **L1308**). Do **not** duplicate **WC-M** or **P1240** here.

**Numbering.** On current `main`: live **Q21** is Lunc Dash `payload` ([#1308](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1308) / !1310); live **Q22** / ADR **0009** is hub-wrap leftover [#1306](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1306). ADR **0008** is unused (closed [#1300](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1300)). Unpublished sister leftover [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) already reserved ADR **0010** / **Q23** / architecture `{#post-merge-leftover-ops}` on `origin/cac-design-issue-1305`. This leftover is **0011** / **Q24** / `{#post-merge-leftover-ops-1311}`. Keep this file as `docs/adr/0011-post-merge-leftover-1306-1309-1310.md`. Do **not** merge `origin/cac-design-issue-1300` / `1302` / `1305` / `1306` as-is. Do **not** open a design-only PR from `cac-design-issue-1311`. Implement must keep Status **Proposed ([#1311])** until a reviewer accepts the design — do not self-mark **Accepted**.

## Outcome

Close leftover **#1311** only after the leftover-complete close-comment template (Rollout / Integration) is pasted on the issue — not after green `make verify-issue-1308` / `make verify-issue-1311` on a laptop. HTTP `[PASS]` is **not** leftover-complete. Child Vitest is **not** leftover-complete. `VERIFY1308_DEVICE_COMPLETE=1` / `VERIFY1308_IID=1308` **must FAIL**. `VERIFY1311_DEVICE_COMPLETE=1` / `VERIFY1311_IID=1311` **must FAIL**. Do **not** invent `VERIFY1311_LEFTOVER_COMPLETE=1` as a pass.

1. **Coolify frontend cut** (issue leftover 1). Production `https://dex.cl8y.com` must serve the !1310 helper `luncdash://wallet_connect?payload=<encoded wc:>` (real `payload` query key), **not** the pre-1310 double-encoded blob (`luncdash://wallet_connect?` + percent-encoding of the entire `payload=…` string). Frontend auto-deploy on the Vite app is an existing operator flag ([ADR 0006](./0006-indexer-health-git-sha.md)); this ticket does **not** flip it. If the serving image is still pre-`ead2ffe4`, the operator **manually** deploys the frontend app at `ead2ffe4+` / `0d6eaeee+`. Hashed-chunk HTTP pin unquoted **`wallet_connect?payload=`** (any quote style; never require source `'`) is **supporting rebuild-presence**. It is **not** leftover-1 complete and **not** leftover-complete. Legacy blob used `payload%3D` (no literal `?payload=`). Do **not** grep concatenated runtime hrefs.

2. **Device QA after that cut** (issue leftover 2 / closed [#1308](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1308) **AC2 / AC3 / AC7**). Phone Connect → LuncDash **Open Lunc Dash** → pairing prompt → header `terra1…`; Lunc Dash WalletConnect settings list **`dex.cl8y.com`**; small Swap approve **or** reject. Inbox: [`QA_TEMPLATE.md`](../../QA_TEMPLATE.md) **1.5.1 / 1.5.1a / 1.5.1b** + **1.5.6 / 1.5.7**, as **cl8y-pm inbox cards** (one action, ≤5 numbered taps). `make verify-issue-1308` is pre-check only. Do **not** reopen [#1279](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1279). Do **not** file those cards from the design slice. The design slice does **not** file a founder card for ordinary design.

3. **Docs nits** (issue leftover 3 — **optional**, not a product gate). [`docs/runbooks/forgejo-pr-merge.md`](../runbooks/forgejo-pr-merge.md) still describes catch-all `CODEOWNERS` (`.* @code/maintainers`); !1309 removed that file. ADR **0009** / `docs/README.md` still reserve **Q21** for closed #1300; live **Q21** is #1308. Slice 1 **may** land those inserts; leftover-complete does **not** wait on them.

**Not this issue**

- Production `/protocol` hub glance stays [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) leftover 1.
- Coolify indexer migrate (`20260921130000`) stays #1305 leftover 2.
- Other-worker `cac-design-issue-*` branches stay on origin; do not delete or merge from this leftover.

## Context

Tracking leftover after merging:

| PR | Linked | On `main` | Leftover class |
|----|--------|-----------|----------------|
| 1309 | cl8y-forgejo#48 | Removed catch-all `CODEOWNERS`. Branch had been reset to `main`; restored `dac394c7` then merged (`cae5d0e1`). | Optional runbook nit (leftover 3) |
| 1310 | #1308 | `buildLuncDashDeepLink` emits a real `payload` query key (`ead2ffe4`). `Fixes #1308` closed the implement ticket. | **Coolify frontend cut** + **device AC2/AC3/AC7** |
| 1306 | Q22 / B1290 | Leftover hub wrap `verify-issue-1290` wiring (aliases 1302/1306, ADR 0009) at `0d6eaeee`. Conflicts with !1310 resolved: **Q21 = payload**, **Q22 = hub wrap**. | Wiring already leftover-complete on `main`; production `/protocol` glance stays **#1305** leftover 1 |

Woodpecker `ci/woodpecker/pr/woodpecker` succeeded on each head. Local gitleaks clean. Workstation `make verify-issue-1290` 4/4 including Playwright `protocol-page` (`--workers=5`). `make verify-issue-1308` including children 519/554/1279. That workstation verify is **not** leftover-complete.

In-repo AC covered by merged !1310: **AC1** sheet (WC-M1), **AC4** copy `wc:`, **AC5** desktop QR, **AC6** cancel/stacking, **AC8** terms hint. Device leftover after Coolify: **AC2 / AC3 / AC7**.

## Non-goals

- Changing `buildLuncDashDeepLink`, pairing sheet, mobile detect, or cosmes patch (product already on `main` via !1310).
- Reopening [#1308](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1308), [#1279](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1279), [#519](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/519), [#554](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/554), [#566](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/566), [#658](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/658).
- Adding Leap, a second Lunc Dash scheme, AppKit/Web3Modal, Legal ADR-036, or a DEX LCD-sim of Station/LuncDash WC post (ADR 0004 Stay).
- Production `/protocol` glance, indexer sqlx migrate, or heal logs (#1305).
- Merging unpublished `cac-design-issue-1300` / `1302` / `1305` / `1306`.
- Occupying ADR **0010** / **Q23** / `{#post-merge-leftover-ops}` (#1305).
- Taking live **Q21** away from #1308 or **Q22** away from #1306.
- Flip Coolify auto-deploy, HMAC / `autonomy.rs`, founder card, or #297 deploy from this ticket.
- Closing leftover on Vitest, hashed-chunk HTTP, or green make.
- `Fixes #1308` / `Closes #1279` on a leftover MR.
- Opening a design-only PR from this branch.
- Self-marking this ADR **Accepted**.

## Decision

**Ops leftover of a shipped pairing fix.** !1310 already landed the `payload` query helper. #1311 attests production serves that helper, then device AC against live `dex.cl8y.com`.

### Slice-0 apply — insert; do not whole-file checkout

Do **not** `git checkout <design-sha> --` `docs/architecture.md` / `docs/qa-invariants.md` / `docs/README.md` from this tip. #1305 (and later `main`) touches overlapping docs. Insert:

1. New file `docs/adr/0011-post-merge-leftover-1306-1309-1310.md` (this ADR). Status **Proposed ([#1311])**. Do **not** add `0008-*.md` / `0010-*.md` here.
2. `docs/architecture.md`: insert `## Post-merge leftover after PRs 1306/1309/1310 {#post-merge-leftover-ops-1311}` (do not replace the whole file; do not use `#post-merge-leftover-ops`).
3. `docs/qa-invariants.md`: insert **Q24** `{#post-merge-ops-1311}` **after Q22**. Leave **Q23** to #1305.
4. `docs/README.md`: insert the ADR 0011 index line after ADR 0009. Architecture Overview bullet adds `#post-merge-leftover-ops-1311`. Patch the ADR 0009 parenthetical: live **Q21** is #1308, not reserved #1300.
5. Playbook `skills/AGENTS_POST_MERGE_OPS_1311.md` (**M1311-1–M1311-8**). Crosslinks in `AGENTS.md`, `docs/testing.md`, WC-M / L1279 skills.
6. Optional leftover 3 on the same MR: runbook CODEOWNERS paragraph (file removed by !1309); ADR 0009 numbering sentences that still assign **Q21** to #1300.

### `make verify-issue-1311` (implement — not this design branch)

Do **not** add `scripts/qa/verify-issue-1311.sh` or the Makefile target on `cac-design-issue-1311`. Implement on a **code** PR from current `main`:

| Step | Rule |
|------|------|
| 1 | Child `./scripts/qa/verify-issue-1308.sh`. Child FAIL → stack FAIL. Honor `VERIFY1308_SKIP_CHILDREN` if already exported. Nested `VERIFY1308_DEVICE_COMPLETE=1` still FAIL. |
| 2 | Docs greps: this ADR, architecture `#post-merge-leftover-ops-1311`, **Q24**, playbook **M1311**, `docs/testing.md` leftover **#1311** row. |
| 3 | `git ls-files CODEOWNERS` empty ( !1309 ). |
| 4 | Optional live HTTP: hashed-chunk unquoted **`wallet_connect?payload=`** on `dex.cl8y.com` (any quote style). SKIP unless hosts answer. FAIL when `VERIFY1311_REQUIRE_LIVE=1` or `VERIFY1311_IID=1311` **and** the host is unreachable. HTTP PASS is **not** leftover-1 complete. |
| 5 | `VERIFY1311_IID=1311` or `VERIFY1311_DEVICE_COMPLETE=1` **must FAIL** (device leftover never make). Do **not** invent a pass `LEFTOVER_COMPLETE` flag. |

Makefile: `.PHONY: verify-issue-1311` immediately after `verify-issue-1308`. Add `verify-issue-1311` to Cloud Agent and Frontend `help` after `verify-issue-1308`.

Do **not** default-run child `verify-issue-1290` ( #1306 wiring is already on `main`; glance is #1305).

## Component / state / interface changes

| Layer | Change |
|-------|--------|
| Design docs (slice 0) | New ADR **0011** **Proposed [#1311]**; insert architecture `#post-merge-leftover-ops-1311`; insert **Q24**; insert README index; playbook **M1311**. |
| QA script / Makefile | Implement only: `verify-issue-1311` wrapper + help. Not on this design branch. |
| Docs / skills | `docs/testing.md` leftover **#1311** row; `AGENTS.md` verify + playbook; WC-M / L1279 leftover **#1311** pointer. Optional runbook CODEOWNERS + ADR 0009 numbering. |
| dApp TS / indexer / wasm / env | **None**. |
| HTTP | Optional supporting hashed-chunk grep only. No new API. |
| Inbox | Operator leftover 2 files cl8y-pm cards. Design does not. |

## Affected invariants

| ID | Effect |
|----|--------|
| **L1308-1–L1308-4** / **Q21** | Unchanged product rules. Leftover 1–2 attest production + device AC after Coolify. |
| **WC-M1–WC-M12** | Unchanged. Device leftover is AC2/AC3/AC7 of closed #1308. |
| **L1279-1–L1279-8** / **Q20** | Unchanged. Do **not** reopen #1279. |
| **Q22** / **B1290** / ADR **0009** | Unchanged hub-wrap bundle. Numbering nit: live Q21 is #1308. |
| **Q23** / ADR **0010** | Reserved by unpublished #1305. This ticket does not occupy those slots. |
| **#297** | Coolify cut is operator leftover of an existing frontend app. Do not flip indexer auto-deploy. |

**M1311-1–M1311-8**

| ID | Rule |
|----|------|
| **M1311-1** | Local regression is `make verify-issue-1311` **(implement)** → child **1308** + docs + empty `CODEOWNERS`. Make is **not** leftover-complete. |
| **M1311-2** | Coolify **frontend** cut of `ead2ffe4+` / `0d6eaeee+`. Supporting HTTP: unquoted **`wallet_connect?payload=`**. HTTP is not leftover-1 complete. |
| **M1311-3** | Leftover-complete is device **AC2 / AC3 / AC7** on production after leftover 1, via cl8y-pm inbox **1.5.1 / 1.5.1a / 1.5.1b** + **1.5.6 / 1.5.7**. |
| **M1311-4** | `VERIFY1308_DEVICE_COMPLETE=1` / `VERIFY1311_DEVICE_COMPLETE=1` / matching IID flags **must FAIL**. |
| **M1311-5** | Do **not** reopen #1279 / #1308 / #519 / #554. Do **not** close #1305 from this ticket. |
| **M1311-6** | Do **not** merge unpublished sister design branches. Do **not** take ADR **0010** / **Q23**. Do **not** flip Coolify auto-deploy. Do **not** file a founder card from design. |
| **M1311-7** | Optional leftover 3 (runbook CODEOWNERS; ADR 0009 Q21 numbering) is not a close gate. |
| **M1311-8** | Playbook + this Q24 + ADR 0011 + L1308 skills stay crosslinked. |

## Alternatives

| Option | Why not |
|--------|---------|
| Close #1311 on green `make verify-issue-1308` | Issue: make is pre-check; device AC is cl8y-pm after Coolify. |
| Treat hashed-chunk `wallet_connect?payload=` as leftover-complete | Supporting rebuild-presence only (same class as #1305 HTTP vs glance). |
| Reopen #1279 for device QA | Issue: do not reopen #1279. #1308 was the product defect; this leftover owns post-cut AC. |
| Occupy ADR 0010 / Q23 | Collides with unpublished #1305. Keep **0011** / **Q24**. |
| Fold leftover 1 into #1305 leftover 1 | #1305 is `/protocol` hub glance + indexer migrate. Payload Coolify is this ticket. |
| Wait leftover-complete on leftover 3 docs nits | Issue: optional, not a product gate. |
| Design slice files inbox cards | Ordinary leftover device work is operator after Coolify; design does not file founder cards. |
| Agent flips Coolify | #297. Record evidence; do not expand deploy policy. |

## Complexity added / removed

**Added:** leftover **Q24** / **M1311**, ADR **0011**, implement verify wrapper, optional runbook CODEOWNERS truth, inbox card split for post-cut AC2/AC3/AC7.

**Removed:** the gap where !1310 closed #1308 on Vitest while production could still serve the double-encoded blob; stale runbook catch-all CODEOWNERS as if !1309 never landed; ADR 0009 claiming live **Q21** is #1300.

Net: ops discoverability without a second pairing surface and without colliding #1305 numbering.

## Migration

None. No wasm, no sqlx, no dApp `VITE_*`. Frontend Coolify rebuild (auto or manual) is leftover 1.

## Observability

Script prints `[PASS]` / `[FAIL]` / skipped live lines; exit `1` iff `FAIL > 0`. Device notes: OS/browser, pass/fail. Do **not** paste Coolify UUIDs, WalletConnect secrets, mnemonics, session keys, inbox tokens, or `/status` JSON on the issue.

Supporting HTTP: hashed Vite chunks containing unquoted **`wallet_connect?payload=`**. Never require source `'`. Do not grep `payload%3D` as success.

## Failure modes

| Failure | Behavior |
|---------|----------|
| Child 1308 FAIL | Bundle FAIL. Fix pairing helper / docs; do not skip the child to close leftover. |
| Production still serves encoded blob | Leftover 1 incomplete. Manual frontend deploy of `ead2ffe4+`. Do not treat HTTP miss as #297 flip. |
| HTTP `wallet_connect?payload=` PASS, device AC fail | Leftover-complete still FAIL. HTTP is supporting only. |
| Green make treated as leftover-complete | Forbidden (**M1311-1** / **M1311-4**). |
| `VERIFY1308_DEVICE_COMPLETE=1` exits 0 | Forbidden. Pre-check must FAIL that flag. |
| Reopen #1279 | Forbidden. |
| Device QA before Coolify cut | AC2/AC3 will re-fail the blob. Leftover 2 after leftover 1. |
| Merge `cac-design-issue-1305` as-is | Steals 0010/Q23 onto a stale Q21=#1300 story. Insert only. |
| Implement marks this ADR **Accepted** | Forbidden self-approval. |
| Design slice files inbox / founder card | Forbidden for ordinary design. |

## Ordered implementation slices

| Slice | Who | Deliverable | Blocks |
|-------|-----|-------------|--------|
| **0 — this design** | design slice | This ADR **0011**, architecture `#post-merge-leftover-ops-1311`, **Q24**, playbook **M1311**, README index. Transported on `cac-design-issue-1311` only (no design-only PR). | Slice 1 |
| **1 — verify wiring + optional nits** | implement | On **current `main`**: **insert** slice-0 docs; `scripts/qa/verify-issue-1311.sh` + Makefile + help; WC-M / L1279 / testing / AGENTS crosslinks. Optional leftover 3 runbook + ADR 0009 numbering. No product TSX. No `Fixes #1308`. | Slice 2 |
| **2 — Coolify frontend cut** | operator | Frontend app serves `ead2ffe4+` / `0d6eaeee+`. Supporting HTTP `wallet_connect?payload=` optional. Do **not** flip auto-deploy from the agent. | Slice 3 |
| **3 — device QA** | operator / cl8y-pm inbox | **AC2 / AC3 / AC7** via QA_TEMPLATE **1.5.1 / 1.5.1a / 1.5.1b** + **1.5.6 / 1.5.7**. Paste leftover-complete comment on **#1311**. | leftover-complete |
| **4 — #1305 (not this ticket)** | reviewer / operator on [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) | `/protocol` glance + indexer migrate. | **Not** the #1311 close gate |

No product-issue wait: !1310 / !1309 / !1306 are on `main`. #1305 is a sister leftover, not a blocker. #1308 is closed implement. #1279 stays closed.

## Tests

**Implement merge (slice 1) — not leftover-complete:**

```bash
make verify-issue-1308
make verify-issue-1311          # after implement adds the target
VERIFY1311_IID=1311 make verify-issue-1311          # expected non-zero
VERIFY1311_DEVICE_COMPLETE=1 make verify-issue-1311 # expected non-zero
VERIFY1308_DEVICE_COMPLETE=1 make verify-issue-1308 # expected non-zero
git ls-files CODEOWNERS         # empty
```

Assert:

- Child 1308 still green as pre-check without device flags.
- Device-complete flags FAIL.
- Slice-0 files greppable: ADR **0011**, `#post-merge-leftover-ops-1311`, **Q24**, **M1311**.
- No `docs/adr/0010-*.md` from this leftover MR.
- ADR Status **Proposed ([#1311])**.
- Playwright 5 workers unchanged on unrelated children; this leftover has **no** new e2e spec.

## Rollout

1. Merge leftover *implement* (script + wiring) to `main` via a **code** PR (not this design branch). Normal merge; no `force_merge`. CODEOWNERS file is gone (!1309) — no catch-all self-request to dismiss.
2. Operator: confirm frontend Coolify serves `ead2ffe4+`. Manual frontend deploy if auto-deploy did not. Do **not** flip the indexer #1276 checkbox.
3. Operator: file cl8y-pm inbox cards (≤5 taps) for **1.5.1 / 1.5.1a / 1.5.1b** + **1.5.6 / 1.5.7**. Put https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1311 in the card body.
4. Close **#1311** by pasting the leftover-complete comment below. Leave **#1305** / **#1279** alone.

### Leftover-complete close-comment template (required; no UUID/token/host)

Paste on [#1311](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1311) when leftover-complete is claimed.

```
frontend cut leftover-1: dex.cl8y.com serves ead2ffe4+
      Open Lunc Dash href uses real payload query
      (not double-encoded blob)

device leftover-2:
      AC2 Open Lunc Dash → pairing prompt → header terra1…
      AC3 Lunc Dash WalletConnect settings list dex.cl8y.com
      AC7 small Swap approve or reject (not a hang)

inbox: QA_TEMPLATE 1.5.1 / 1.5.1a / 1.5.1b + 1.5.6 / 1.5.7
      (did not reopen #1279)

FORBIDDEN in this comment: green make verify-issue-1308,
VERIFY1308_DEVICE_COMPLETE PASS, hashed-chunk-only PASS,
GET /health, protocol-top-pairs, tokens / inbox ids
```

No wasm store from the leftover MR.

## Rollback

Frontend: redeploy previous Vite app (pairing blob returns; device AC2/AC3 fail again). Do not revert !1310 product to “close” leftover. No schema, wasm, or env rollback. This leftover is not a chain halt.

## Integration completion criteria

- Slice-0 on `main` via **inserts** (this ADR **0011** **Proposed [#1311]**, architecture `#post-merge-leftover-ops-1311`, **Q24**, playbook **M1311**, README index) — not left only on `cac-design-issue-1311`, and not a whole-file checkout of overlap files from this tip.
- No `0010-post-merge-leftover-1306-*.md`. **Q23** / ADR **0010** remain #1305’s if/when that design lands.
- `make verify-issue-1311` exists on `main` and is green locally **without** device-complete flags. Green make is **not** leftover-complete.
- Device-complete / IID flags FAIL.
- Coolify leftover-1: production serves `ead2ffe4+` payload helper. HTTP `wallet_connect?payload=` is supporting, not leftover-complete.
- Device leftover-2: **AC2 / AC3 / AC7** recorded via inbox cards. #1279 not reopened.
- Optional leftover 3 may be absent at close.
- `cac-design-issue-1300` / `1302` / `1305` / `1306` not merged as-is.
- #1305 still the owner of `/protocol` glance + indexer migrate.
- No founder card from design. No Coolify auto-deploy flip. No `DESIGN: APPROVE`.
- Close **#1311** only by pasting the leftover-complete comment (no UUID/token/host).

Keywords on the issue are **not** approval to deploy or to write `DESIGN: APPROVE`.

## Links

- Issue: [#1311](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1311)
- Closed implement: [#1308](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1308) / [!1310](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1310)
- Sister leftover: [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) (PRs 1302–1304; ADR **0010** / **Q23** reserved)
- Hub-wrap wiring: [ADR 0009](./0009-verify-issue-1290-hub-wrap-labels.md) / **Q22** / [#1306](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1306)
- CODEOWNERS removal: [!1309](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1309)
- Playbook: [`AGENTS_POST_MERGE_OPS_1311.md`](../../skills/AGENTS_POST_MERGE_OPS_1311.md)
- Pairing: [`AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md`](../../skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md)
- L1279 pre-check: [`AGENTS_OPS_LUNCDASH_VERIFY.md`](../../skills/AGENTS_OPS_LUNCDASH_VERIFY.md)
- Merge: [`runbooks/forgejo-pr-merge.md`](../runbooks/forgejo-pr-merge.md)
- Verify (implement): `make verify-issue-1311` · child `make verify-issue-1308`
- Authority: [agent-control #297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297)
