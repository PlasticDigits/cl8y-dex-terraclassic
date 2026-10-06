# Independent review: #1428

**PASS with the explicit separate-scope skips below; no implementation blockers found.**
Reviewed 2026-10-06 UTC against base `e7b54989c21fc411141c4c6c3af553c03dea1bdd`
and the final working-tree source inputs recorded in
[build provenance](evidence/build-provenance.json). Scope and repeatable agent
instructions: [README.md](README.md), [results](RESULTS.md),
[GR1428 invariants](../../../skills/AGENTS_GREEDY_REVERSE_SUPPORT.md).
The issue receipt has no comments.

## Source findings

The route-shape guard rejects any four-operation reverse route containing greedy
before capability discovery, factory/pair queries or simulations. Coverage includes
every greedy-hop position, mixed modes, both orientations, zero/maximum amounts
and trader contexts. Pool-only reverse keeps four hops; forward quoting/execution
paths retain their existing cap. Supported reverse still returns authorized gross
funding with the original trader/sender and requires fresh complete net forward
sufficiency. No fallback quote, tax waiver, new cache or raised simulation,
maker/scan/hook ceiling was introduced. Migration changes cw2 metadata and preserves
configuration, nonce and funding/settlement version 1.

The additional slippage property repair uses exact integer multiples of the reduced
reserve ratio. Its positive input ranges guarantee a nonzero GCD, and cross-product
arithmetic is within `u128`. It retains the saved failure seed and explicitly checks
proportionality without changing runtime liquidity/slippage rules.

## Verification receipt

Independently executed:

- `cargo test --locked -p cl8y-dex-router -- --test-threads=1`: **24 passed**.
- `cargo clippy --locked -p cl8y-dex-router --all-targets -- -D warnings`: **PASS**.
- `cargo test --locked -p cl8y-dex-tests prop_slippage_tolerance_boundary -- --test-threads=1`:
  **PASS**, including saved regression seeds.
- `python3 scripts/qa/test-localterra-reverse-1389.py`: **8 passed**.
- Existing #1389 documentation checker, verifier shell syntax and `git diff --check`:
  **PASS**.
- Public receipt reconstruction and native provenance verification against current
  source/outputs, requiring the router artifact: **PASS** (173 inputs, 13 artifacts).
  Matrix stored-code/on-chain hashes match reviewed
  provenance and the separately recorded CW20 test dependency.

Inspected completed verifier logs: #1428 **9/0/0**, #1422 **10/0/1**, #1398
**7/0/0**, #1389 **8/0/1**, #1381 **13/0/2**, #1390 **15/0/1**
(PASS/FAIL/SKIP). The #1398 complete contract workspace reports **567 passed**.

Both integrated baseline receipts retain **17 checks**, including the fourteen
preceding successful checks and two raw populated four-hop provider failures at
15M. Their `FAIL`/`REPRODUCED` status remains distinct from acceptance. Both final
router 1.5.0 matrices contain **22 passing expected-outcome checks** and **15 fresh
target-covering forward validations**, using different synthetic contracts. Empty
and populated four-hop rejection markers match in both directions. The first
rejection probe uses **207,757 gas**, including transaction/wrapper overhead.

Populated successes qualify only one hop, maker cap 8, one raw ask unit in both
directions and one human ask unit forward. Empty three-hop boundary cases pass
both directions. This admission ceiling does not promise universal shorter-book
gas feasibility. Shorter-route exploratory failures remain disclosed and retained.
The independent requests are not fallback executions for rejected quotes.

Both final funded fixtures confirm router 1.5.0, an unchanged **3M** node limit,
**six passing accounting scenarios** and **18 actual smart-query successes** each.
Migration and atomic invoice receipts also pass. Actual provider outcomes remain
separate from probe gas, simulation counts and production deployment.

The published receipt stores metadata separately and uses `path`/`sha256` arrays.
From the repository root with matching optimized outputs available, reconstruct
its native dictionary before invoking the verifier:

```bash
python3 - <<'PY'
import json, subprocess, tempfile
from pathlib import Path
public = json.loads(Path('docs/qa/issue-1428/evidence/build-provenance.json').read_text())
native = dict(public['metadata'])
for key in ('source_inputs', 'artifacts'):
    native[key] = {entry['path']: entry['sha256'] for entry in public[key]}
    assert len(native[key]) == len(public[key]), 'Duplicate paths in ' + key
with tempfile.NamedTemporaryFile(mode='w', suffix='.json') as receipt:
    json.dump(native, receipt)
    receipt.flush()
    subprocess.run(['python3', 'scripts/ci/build-provenance.py', '--root', '.',
                    '--verify', receipt.name, '--require-artifact',
                    'smartcontracts/artifacts/cl8y_dex_router.wasm'], check=True)
PY
```

Publication-only receipt restructuring and lowercase transaction-hash formatting
preserve hash bytes, raw provider errors and gas values. Compressed command logs
retain the original transaction-hash case. This does not change the qualified scope.

## Residual scope

Optional older #1422 pre-optimization artifacts were not supplied again; their
historical evidence remains intact. The #1389 separate shell gas invocation was
skipped because the actual #1428 first/retest matrices cover that scope. Native
wrap/unwrap remains unverified without a compatible owned wrap-stack fixture.
Production migrations/provider acceptance remain #1384/#1386/#1421 and are not
established by this review. These criteria remain unchecked; their skips do not
hide executed failures or waive runtime protections.
