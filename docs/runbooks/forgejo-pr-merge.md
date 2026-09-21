# Forgejo PR merge (Woodpecker status)

Source of truth is [git.cl8y.com/code/cl8y-dex-terraclassic](https://git.cl8y.com/code/cl8y-dex-terraclassic).
`main` branch protection requires status context `ci/woodpecker/pr/woodpecker`.
Do **not** use `force_merge`. Founder-required; no community autoland.

Enablement of real Woodpecker posts is [#1247](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1247).
Pipeline YAML: [`.woodpecker.yaml`](../../.woodpecker.yaml) (gitleaks-only until a follow-up ports cargo/frontend/indexer jobs).

## Merge recipe

1. **CODEOWNERS.** Catch-all `.* @code/maintainers` was **removed** in
   [!1309](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1309)
   (`git ls-files CODEOWNERS` is empty). There is no team self-request to dismiss
   from that file. Do not restore a catch-all. Do not weaken branch protection.
   Leftover runbook nit: [#1311](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1311)
   leftover 3 (not a product gate).
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

## Do not

- `force_merge`
- Float image tags in `.woodpecker.yaml` (digest-pin only)
- Bind-mount `indexer/` into root Docker for cargo
- Paste Woodpecker / OpenBao tokens into issues or cards
