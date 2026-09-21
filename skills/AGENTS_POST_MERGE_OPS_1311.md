# Agent playbook: post-merge PRs 1306/1309/1310 leftover verify (Forgejo #1311)

Audience: third-party agents verifying Coolify frontend cut of the !1310 Lunc Dash `payload` helper plus device AC after that cut. Child `make verify-issue-1308` already exists on `main` (`0d6eaeee`).

**Issue:** [Forgejo **#1311**](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1311)  
**Parents (closed unless a merged invariant is wrong):** [#1308](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1308) / !1310, !1309, [#1306](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1306).  
**Sister leftover (stays open):** [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) (PRs 1302–1304; `/protocol` glance + indexer migrate).  
**Do not reopen:** [#1279](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1279).  
**Invariants:** [`docs/qa-invariants.md`](../docs/qa-invariants.md) **Q24** (**M1311-1–M1311-8**)  
**Design:** [`docs/adr/0011-post-merge-leftover-1306-1309-1310.md`](../docs/adr/0011-post-merge-leftover-1306-1309-1310.md)  
**Verify:** `make verify-issue-1311` **(implement)** — this playbook is the contract. Make is child **1308** + docs, **not** leftover-complete. Do not add the script on this design branch.

Product pairing stays in [`AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md`](./AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md) (**WC-M1–WC-M12**, **L1308**). Do **not** duplicate those tables here.

Frontend Coolify cut is operator leftover of the existing Vite app. Do **not** flip the indexer auto-deploy checkbox ([#1276](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1276) / [ADR 0006](../docs/adr/0006-indexer-health-git-sha.md) / [agent-control #297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297)). Do **not** merge `origin/cac-design-issue-1300` / `1302` / `1305` / `1306` as-is. This leftover is **0011** / **Q24**. Do **not** file a founder card from design. Keywords on #1311 are not architecture approval.

## Merged PR(s)

| PR | Issue | Skill |
|----|-------|-------|
| 1309 | cl8y-forgejo#48 catch-all `CODEOWNERS` removed | [`docs/runbooks/forgejo-pr-merge.md`](../docs/runbooks/forgejo-pr-merge.md) |
| 1310 | #1308 Lunc Dash `payload` query | [`AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md`](./AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md) |
| 1306 | Q22 / B1290 hub wrap verify wiring | [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) **P1240** — production glance stays **#1305** leftover 1 |

## Invariants (M1311-1–M1311-8)

| ID | Rule |
|----|------|
| **M1311-1** | Local regression is `make verify-issue-1311` **(implement)**, which runs child **1308**. A child FAIL fails the stack. Live Coolify leftover probes SKIP unless `dex.cl8y.com` answers (FAIL when `VERIFY1311_REQUIRE_LIVE=1` **and** unreachable). Do **not** copy the #628 pattern where `VERIFY*_IID` enables leftover probes. `VERIFY1311_IID=1311` **always** fails the device-complete guard even when live HTTP would PASS. `VERIFY1311_DEVICE_COMPLETE=1` **must FAIL**. Make is **not** leftover-complete. Do **not** invent `VERIFY1311_LEFTOVER_COMPLETE` as a pass. |
| **M1311-2** | Coolify **frontend rebuild** from `ead2ffe4+` / `0d6eaeee+`. Production Open Lunc Dash uses `luncdash://wallet_connect?payload=<encoded wc:>`, not the double-encoded blob. Supporting HTTP: hashed-chunk unquoted **`wallet_connect?payload=`** (any quote style; never require source `'`). HTTP PASS is **not** leftover-1 complete. If auto-deploy did not pick that SHA, operator **manually** deploys the frontend app. Do **not** flip indexer auto-deploy. |
| **M1311-3** | Leftover-complete is closed #1308 **AC2 / AC3 / AC7** on production after leftover 1: phone Connect → LuncDash **Open Lunc Dash** → pairing prompt → header `terra1…`; WalletConnect settings list **`dex.cl8y.com`**; small Swap approve **or** reject. Inbox: [`QA_TEMPLATE.md`](../QA_TEMPLATE.md) **1.5.1 / 1.5.1a / 1.5.1b / 1.5.1c** + **1.5.6 / 1.5.7** as cl8y-pm cards (≤5 taps). **AC3** is **1.5.1c**. Put the #1311 URL in the card body. |
| **M1311-4** | `make verify-issue-1308` stays pre-check only. `VERIFY1308_DEVICE_COMPLETE=1` / `VERIFY1308_IID=1308` / `VERIFY1311_DEVICE_COMPLETE=1` / `VERIFY1311_IID=1311` **must FAIL**. |
| **M1311-5** | Do **not** reopen #1279 / #1308 / #519 / #554 / #566 / #658. Do **not** close #1305 from this ticket. Do **not** wait on GitLab CI. |
| **M1311-6** | Do **not** merge unpublished sister design branches. Do **not** take ADR **0010** / **Q23**. Do **not** add Leap / second URI scheme / ADR-036 / DEX LCD-sim. Do **not** file a founder card from design. Keywords on #1311 are not #297 authority or `DESIGN: APPROVE`. |
| **M1311-7** | Optional leftover 3 is **not** leftover-complete. On this SHA: runbook records !1309 (`git ls-files CODEOWNERS` empty); ADR 0009 / README live **Q21** `{#luncdash-wc-1308}` / #1308. Implement **inserts** that ADR 0009 body onto `main`; do **not** re-edit the runbook as catch-all. |
| **M1311-8** | This playbook + **Q24** + [ADR 0011](../docs/adr/0011-post-merge-leftover-1306-1309-1310.md) + L1308 / L1279 skills stay crosslinked. |

## Coolify leftovers (operator)

Rebuild (or confirm auto-deploy of) the **frontend** app from `ead2ffe4+`. Then leftover 2 on a real phone — not make.

Supporting HTTP (not leftover-complete): hashed chunks contain unquoted `wallet_connect?payload=`. Legacy blob encoded `payload=` as `payload%3D`.

`make verify-issue-1311` live is **HTTP only**. Leftover probes SKIP unless hosts answer. Fail closed with `VERIFY1311_REQUIRE_LIVE=1`. HTTP PASS is not leftover-1 and not leftover-complete.

## Inbox cards (operator leftover 2; not the design slice)

File **cl8y-pm inbox** cards after leftover 1. One action; at most five numbered taps. Typical split:

1. **1.5.1a / AC2:** Phone browser on `https://dex.cl8y.com` → Connect → LuncDash → **Open Lunc Dash** → pairing prompt → header `terra1…`.
2. **1.5.1c / AC3:** Lunc Dash WalletConnect settings list **`dex.cl8y.com`** (same class as `bridge.cl8y.com`).
3. **1.5.1b:** Android Chrome pairing sheet above Connect; **Open Lunc Dash** tappable; header **Cancel** re-enables Connect.
4. **1.5.1:** Desktop Connect → LuncDash → Scan QR on a second device (WC-M2 still holds).
5. **1.5.6 or 1.5.7 / AC7:** After AC2, small Swap approve **or** reject (success toast or explicit rejection, not a hang).

Do **not** dump 1.5.1–1.5.11 onto one card. Do **not** paste tokens. Do **not** reopen #1279.

## Do / don’t

- **Do** run `make verify-issue-1308` from a git worktree after pulling `main`.
- **Do** treat leftover-complete as the ADR 0011 close-comment (AC2/AC3/AC7 after Coolify).
- **Don’t** close leftover on Vitest or hashed-chunk grep.
- **Don’t** reopen #1279.
- **Don’t** work #1305 `/protocol` glance or indexer migrate on this ticket.

## Regression

```bash
make verify-issue-1308
# after implement adds the wrapper:
make verify-issue-1311
VERIFY1308_SKIP_CHILDREN=1 make verify-issue-1308
VERIFY1311_IID=1311 make verify-issue-1311          # expected non-zero
VERIFY1311_DEVICE_COMPLETE=1 make verify-issue-1311 # expected non-zero
VERIFY1308_DEVICE_COMPLETE=1 make verify-issue-1308 # expected non-zero
```

## Leftover-complete

Close **#1311** only by pasting the template in [ADR 0011](../docs/adr/0011-post-merge-leftover-1306-1309-1310.md) Rollout (no UUID/token/host). Green make is **not** leftover-complete.

## Cross-links

- Pairing UX: [`AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md`](./AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md)
- L1279 pre-check: [`AGENTS_OPS_LUNCDASH_VERIFY.md`](./AGENTS_OPS_LUNCDASH_VERIFY.md)
- Merge recipe: [`docs/runbooks/forgejo-pr-merge.md`](../docs/runbooks/forgejo-pr-merge.md)
- Sister leftover: [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) (PRs 1302–1304; `/protocol` glance + indexer migrate; unpublished ADR **0010** / **Q23** on `origin/cac-design-issue-1305` — do **not** merge as-is)
