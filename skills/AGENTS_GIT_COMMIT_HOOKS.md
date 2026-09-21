# Git commit message hooks (#1286 / #1287)

Agent playbook for `.githooks/` body policy after [#1282](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1282) (ADR 0006 land left `Co-authored-by` on `main`).

## Invariants

| ID | Rule |
|----|------|
| **G1287-1** | `commit-msg` and `prepare-commit-msg` reject or strip email / `author` keyword lines in the **body** (subject may contain `@`). |
| **G1287-2** | `pre-push` re-scans the push range for bodies that bypassed `commit-msg` (`git commit --no-verify`). Never advise `git push --no-verify`. |
| **G1287-3** | `pre-push` **skips** commits already reachable from the push remote’s `main` (`origin/main` by default) so `git merge origin/main` on a feature branch does not re-reject historical main commits ([#1286](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1286), [#1287](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1287)). |
| **G1287-4** | New commits on the feature branch with `Co-authored-by` or body emails still fail `pre-push`. |
| **G1287-5** | `scripts/verify-commit-messages.sh` explicit ranges are unchanged; only push-range semantics use the integration-branch skip. |

## Regression

```bash
make verify-issue-1287
# or: make test-commit-msg-hook
```

Shared helpers: `.githooks/lib/validate-commit-message.sh` (`resolve_integration_main_ref`, `commit_message_policy_on_integration_branch`).
