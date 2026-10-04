# Live verification and closure disposition — 2026-10-04

Issue [#1376](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1376),
implementation [PR #1378](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1378).
Verified in a separate git worktree at
`3f9310652ac83818749f5d32c4e32057b08817bc`, using OpenBao-tools `bao-exec`
with selected Forgejo/Coolify credentials; issue operations use `fj`.
No production transaction was signed or broadcast.

**Disposition: close under the user's explicit permission to skip unavailable
operator-custody work. This is not full production PASS.** The legacy pair
reverse probe is a recorded **FAIL**, not a skipped or successful test.
Its correction requires the unavailable file-keyring passphrase and 2-of-3
signing. There is no noninteractive production signer in the test backend.
The waiver releases this closure gate; it does not repair the deployed query.
No mandatory operator handoff is created for the waived work.

## Agent checks completed

| Check | Result |
| --- | --- |
| Frontend and indexer Coolify deployments | Both finished/healthy on `3f93106`; [deployment receipts](production-deployments.json). |
| Live quote-only browser | **16 PASS**: `/` and `/swap`, targets 10/100, Compact/Full, cold/warm reload in fresh disconnected contexts. Both quotes use `pool_exact_output_v1`, passed fidelity, and matching authoritative forward operations with sufficient output. [Public evidence](production-browser.json). |
| Native USTC gross Pay | 10 CL8Y → `1243.234685`; 100 → `12907.842056`. Receive remains exactly typed. These are snapshot quotes, not permanent prices. |
| Exact-output indexer | 12 library and 12 Postgres API tests per verifier pass, including candidate alternatives, fee/tax context, freshness and invalid modes. |
| Focused frontend | 192 tests per pass; lint/types pass; 13 mocked browser cases pass. |
| Full contract workspace / lint | 878 tests pass; formatting and standard Clippy pass. Real 10/100 cap fixtures included in the issue verifier. |
| Required regressions | #1256: 5/0/0; #713: 8/0; #678: 7/0; #512: exit 0; #1280: 10/0 including retest. |
| Issue verifier | `PASS=11 FAIL=0 SKIP=2` for its local invocation. Live and LocalTerra options were not selected there; both were subsequently executed separately as below. Its old default live-skip wording was corrected to avoid claiming deployment had not happened. |
| Funded LocalTerra / native guard | Both issue-specific Playwright cases pass at dedicated port 3086. CW20 transaction assertions verify selected route/send/minimum. Native dust protection passes; native broadcast is not evidenced. |
| Prepared optional migration | Shell syntax, pinned artifact checksum/cw2, unchanged contract sources and `--dry-run` pass. The live signing branch is unexecuted. |

## Live contract failure and waived remainder

[Complete production inventory](production-pairs.json) at Columbus-5 block
**30683510** enumerates **22/22** factory pairs. Factory `pair_code_id` is
**11676** and every pair is on **cw2 1.18.0**. The 100-CL8Y legacy reverse
query still returns `Hybrid reverse simulation exceeded iteration cap`;
its independent pool forward quote is sufficient. The deployed frontend
avoids this old query through exact-output solver sizing. The 10 reverse
query succeeds. Public CL8Y resolution is
`terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3`
(metadata CL8Y-cb, 18 decimals); cUSTC is
`terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch`
(6 decimals). Browser evidence records snapshot heights and public bundle paths.

Only the following scope remains unverified/deferred:

1. **Custody waiver:** migrate the factory pair pointer and 22 existing pairs to
   the reviewed **1.19.0** artifact; then agents rerun fleet/version/reverse and
   browser probes. The actual current reverse failure remains FAIL.
2. **Signing/execution scope:** production connected-wallet signing and the
   complete funded native/mapper/tax matrix are not claimed. Unit/API coverage,
   funded LocalTerra CW20 execution and native protected refusal passed.
   Unavailable production signing is waived; no safety guard is disabled.

Optional operator command (artifact already prepared and dry-run checked):

```bash
bash scripts/qa/complete-issue-1376.sh --migrate
```

The [wrapper](../../../scripts/qa/complete-issue-1376.sh) pins SHA-256
`4ad54cd3b9a5ff3d28b80bfec21800b39fee97ca9295bb524353db7590005bba`,
selects 1.19.0 explicitly and delegates custody to the existing 2-of-3 script.
`--check` is read-only and currently fails, correctly; `--dry-run` never signs.
Agents own all preparation and follow-up. Closure does not request this command
or make it a new obligation.

Merge compatibility on 2026-10-04: the wrapper's whole-contract-tree guard now
pins `ef873dcc5a1680b059d839b3578801211d8091f2`, the tested #1381 contract tree,
while retaining the identical pair checksum above. The preceding `3f93106` pin
would reject the later router/token changes before reaching a dry-run. Production
pair rollout and acceptance are tracked in [#1386](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1386).

## Invariants and system links

[E1376-1–E1376-9](../../frontend.md#solver-receive-sizing) retain offer-in execution,
matching forward quotes, bounded work, stale-state invalidation, net/pre-tax
bases and 5/30/99 protection. E1376-9 separates service deployment from complete
pair migration and distinguishes a custody waiver from PASS.

- Indexer: [invariant matrix](../../indexer-invariants.md),
  [API/search scope](../../route-solver.md#exact-output-sizing),
  [handler](../../../indexer/src/api/route_exact_output.rs).
- Contracts: [pair reverse](../../../smartcontracts/contracts/pair/src/hybrid_reverse.rs),
  [TWAP/state-preservation runbook](../../runbooks/pair-twap-uint256-columbus5.md),
  closed predecessor [#1324](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1324).
  Do not reopen its completed 1.18.0 rollout or downgrade its defaults.
- Third-party agents: [exact-output playbook](../../../skills/AGENTS_SWAP_EXACT_OUTPUT.md),
  [receive playbook](../../../skills/AGENTS_FRONTEND_SWAP_REVERSE_QUOTES.md),
  [agent-first checklist](README.md), [testing](../../testing.md#receive-sizing-1376).
- Credential boundary: [OpenBao workstation wrapper](https://git.cl8y.com/PlasticDigits/openbao-tools/src/branch/main/docs/workstation.md).
  Selected names only; no secret values, cookies, key material or wallet identities
  are published as verification evidence.
- Related issues: [#1256](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1256),
  [#713](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/713),
  [#1280](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1280).
