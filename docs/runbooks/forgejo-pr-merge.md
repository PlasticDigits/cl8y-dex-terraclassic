# Forgejo PR merge (Woodpecker status)

Source of truth is [git.cl8y.com/code/cl8y-dex-terraclassic](https://git.cl8y.com/code/cl8y-dex-terraclassic).
`main` branch protection requires status context `ci/woodpecker/pr/woodpecker`.
Do **not** use `force_merge`. Founder-required; no community autoland.

Enablement of real Woodpecker posts is [#1247](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1247).
Pipeline YAML: [`.woodpecker.yaml`](../../.woodpecker.yaml) (gitleaks-only until a follow-up ports cargo/frontend/indexer jobs).

## Merge recipe

1. **CODEOWNERS.** This repo’s [`CODEOWNERS`](../../CODEOWNERS) is `.* @code/maintainers`.
   Forgejo requests review from that team. If you are the only member, you cannot
   approve your own PR. Dismiss that self-request in the PR UI (or equivalent API)
   so the required-review rule is not a deadlock. Do not weaken branch protection.
2. **Status.** Wait for Woodpecker to post `ci/woodpecker/pr/woodpecker` on the
   head SHA (PR event). Admins must not drop that required context.
3. **Merge.** Use a normal merge (`fj pr merge` / Forgejo **Merge**). Never
   `force_merge: true`. `#1239` was marked merged while the head was not an
   ancestor of `origin/main`; reland was `#1242`.
4. **Local gitleaks** (until Woodpecker is posting, or as a belt-and-suspenders
   check): `./scripts/ci/gitleaks-scan-tracked.sh --verbose --exit-code 1`.

Do not POST a fake commit status to skip a failing Woodpecker run. Manual
`ci/woodpecker/pr/woodpecker` POSTs were a stopgap before this repo was enabled
on `ci.cl8y.com`; they are not the merge path once AC1 on #1247 holds.

## Historical (do not rewrite)

[#1302](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1302) and
[#1303](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1303) were merged
with `force_merge` before this recipe was followed. Both merge commits
(`92c84406`, `a7919548`) **are** ancestors of `origin/main`.
[#1304](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1304) used dismiss
+ normal merge (`729b097f`). Leftover ops: [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305)
([ADR 0010](../adr/0010-post-merge-leftover-1302-1304.md) leftover 5). Do not
rewrite those ancestors. Later PRs stay dismiss-then-merge.

## Do not

- `force_merge`
- Float image tags in `.woodpecker.yaml` (digest-pin only)
- Bind-mount `indexer/` into root Docker for cargo
- Paste Woodpecker / OpenBao tokens into issues or cards
