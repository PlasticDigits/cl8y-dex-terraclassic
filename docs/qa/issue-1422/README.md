# Issue #1422: funded reverse-query gas regression

Issue: [#1422](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1422).
Invariants: [FRQ1422-1–FRQ1422-8](../../../skills/AGENTS_FUNDED_REVERSE_QUERY_BUDGET.md).
Integration: [solver](../../route-solver.md#funded-reverse-provider-budget-1422),
[rollout runbook](../../runbooks/route-local-tax-1381.md),
[rollout review](../issue-1381-rollout/README.md).
Implementation: [query-local discovery](../../../smartcontracts/contracts/router/src/route_quote.rs),
[fresh router validation](../../../smartcontracts/contracts/router/src/contract.rs),
[provider error classification](../../../indexer/src/lcd/mod.rs),
[API error handling](../../../indexer/src/api/route_solver.rs), and
[3M chain harness](../../../scripts/qa/localterra-funded-reverse-1422.py).

The regression is the ordinary CL8Y→ALPHA→BETA→USTR funded reverse quote exhausting
an unmodified 3,000,000 SDK53 smart-query meter. A node with a larger limit cannot
establish this criterion. Query success, transaction gas, source work-count bounds
and production rollout are separate evidence.

## Reproduce and review

Use a fresh git worktree and a dedicated node, pinned to the repository SDK53
image digest. Never reset shared chain volumes or stress a public LCD. Build the
optimized current wasm and the independent test-only probe with Rust 1.81.0
(the pinned optimizer toolchain; newer Rust defaults can emit unsupported bulk
memory even with `target-cpu=mvp`); preserve reviewed
before-fix optimized wasm in a separate immutable directory if available. Copy
`cw20_mintable.wasm` from the reviewed existing fixture dependency, preserving its
hash. The test-probe lockfile uses the contract workspace's compatible dependency
versions and remains locked during reproduction. The fixture creates synthetic accounts, factory/router/tokens/pools and
never touches production or shared deployed contracts.

```bash
# Isolated synthetic node; no shared ports, volumes or production endpoints.
NODE_IMAGE=$(python3 -c 'import re; from pathlib import Path; print(re.search(r"image:\s*(\S*/localterra-cl8y@sha256:[0-9a-f]+)", Path("docker-compose.yml").read_text()).group(1))')
docker run -d --name cl8y-dex-issue1422-localterra \
  --tmpfs /home/terra/.terra:rw,mode=1777,size=2g \
  --tmpfs /tmp:rw,mode=1777,size=128m \
  --mount "type=bind,source=$PWD/docker/init-chain.sh,target=/usr/local/bin/init-chain.sh,readonly" \
  "$NODE_IMAGE"
# Wait until status reports latest_block_height > 0 before fixture funding.
docker exec cl8y-dex-issue1422-localterra curl -sf http://127.0.0.1:26657/status
make build-optimized
rustup toolchain install 1.81.0 --profile minimal
rustup target add wasm32-unknown-unknown --toolchain 1.81.0
RUSTFLAGS='-C link-arg=--allow-undefined -C target-cpu=mvp' \
  cargo +1.81.0 build --locked --manifest-path scripts/qa/fixtures/reverse-query-probe/Cargo.toml \
  --release --target wasm32-unknown-unknown
python3 scripts/qa/localterra-funded-reverse-1422.py \
  --container cl8y-dex-issue1422-localterra \
  --artifacts /path/to/reviewed-current-artifacts \
  --baseline-artifacts /path/to/frozen-before-fix-artifacts \
  --probe scripts/qa/fixtures/reverse-query-probe/target/wasm32-unknown-unknown/release/reverse_query_probe.wasm \
  --code-cache /tmp/dex-1422-codes.json --evidence-file /tmp/dex-1422-first.json
VERIFY_ISSUE_1422_ARTIFACTS=/path/to/reviewed-current-artifacts \
VERIFY_ISSUE_1422_BASELINE_ARTIFACTS=/path/to/frozen-before-fix-artifacts \
make verify-issue-1422
```

The harness checks `[wasm].query_gas_limit == 3000000` in running node app.toml
before broadcasting, captures the image ID, configured image and terrad version,
then records actual stored-code checksums and each queried contract's cw2 version.
Use `--node-home` only for the node's actual config location. It runs independent
old/current deployments, preserving the frozen baseline failure as `status=FAIL`
and `baseline_regression=REPRODUCED`. Only a real reverse gas-limit failure on
the named multiple-tax route counts as reproduction. Unexpected errors or a
baseline that passes fail the requested reproduction check.

For current artifacts the six cases are zero intermediates; multiple taxed
intermediates; owned router dust and existing trader balances; taxed initial gross;
taxed final buy; minimum-return rollback. Every case captures forward, reverse
and fresh forward-validation payloads and actual smart responses before executing
its swap. It requires all 18 actual smart queries to succeed at 3M, all test-probe
queries to succeed, and all six accounting/rollback assertions to pass. Probe
transactions execute no downstream messages and report gas including transaction
and wrapper overhead. Their gas is not the LCD meter reading and can exceed 3M;
actual smart-query success establishes the fixture's 3M compatibility. Evidence
is saved before probing, so a query failure remains visible even if measurement
subsequently fails. Failed final-artifact execution remains FAIL.

`make verify-issue-1422` repeats focused router funding/reverse/guard checks,
indexer deterministic provider-error and exact-output checks, relevant receive
client tests, source checks and the 3M harness. `VERIFY_ISSUE_1422_CONTAINER`,
`VERIFY_ISSUE_1422_NODE_HOME`, `VERIFY_ISSUE_1422_PROBE`,
`VERIFY_ISSUE_1422_CODE_CACHE` and `VERIFY_ISSUE_1422_EVIDENCE` customize paths.
Leave the evidence variable unset to preserve separate first/retest files.

## Agent acceptance checklist

Record revision, UTC, command, exit code and evidence for each row. Check off only
completed criteria; replace any skip after independently reproducing it.

- [ ] **Regression fixture:** preserve frozen optimized artifact provenance,
  reproduce the ordinary→ALPHA→BETA→USTR 3M reverse failure and retain raw error.
- [ ] **Actual compatibility:** independently confirm running image/config,
  reviewed code hashes/cw2, and all 18 smart-query successes on current artifacts
  at unchanged 3M. Repeat from a new fixture. Compare probe gas receipts by query,
  documenting overhead scope rather than treating them as LCD measurements.
- [ ] **Funding/sufficiency:** inspect query-local reuse keys and run affected
  regressions. Confirm authorized gross funding, original trader exemptions,
  exact credit/hook parity, fresh target-covering forward validation, work bounds,
  zero intermediate-wallet use, dust preservation and rollback remain intact.
- [ ] **Client behavior:** run receive and indexer error tests. Confirm a provider
  gas error remains deterministic/observable, Receive intent survives, Pay and
  executable operations clear, and no automatic retry/fallback/resend occurs.
  Verify DB exact-output sizing retains fresh forward preflight.
- [ ] **Rollout reconciliation:** inspect reviewed router 1.3.1/pair 1.20.0 release
  targets and hashes against #1384/#1386; re-probe actual deployed providers and
  affected routes read-only at their real limits before rollout acceptance.
  List provider limits, route/state/book scope and raw responses. Larger-limit
  nodes and local fixtures cannot establish production quote or migration success.
- [ ] **Production deployment:** only if available and explicitly authorized,
  follow the coordinated rollout runbook and preserve actual migration/smoke
  receipts. Keep this unchecked when the task establishes source/test-chain work.

Agents may skip an item they cannot complete and still mark overall verification
**PASS with explicit skips**, after completing feasible work first. Use
`VERIFY_ISSUE_1422_SKIP_CONTRACTS_REASON`, `..._SKIP_INDEXER_REASON`,
`..._SKIP_FRONTEND_REASON`, or `..._SKIP_CHAIN_REASON` with concrete explanations.
Missing independently reviewed baseline artifacts are a separate explicit skip.
Record unavailable production/provider/device scope without making it an
implementation PR hold. Executed failures remain **FAIL**; skip permission never
waives runtime protections, identity checks or funding/signing safeguards, and
does not complete the skipped acceptance criterion.

Recorded implementation evidence: [RESULTS.md](RESULTS.md).

## Verify the public build receipt

The published receipt uses explicit `path`/`sha256` entries for its hash maps.
Its `metadata` preserves the original builder receipt fields. The build used a
recorded dirty source snapshot before the fix was committed; the verifier checks
both its base revision and source bytes. Reconstruct that exact snapshot instead
of treating a later documentation/implementation commit as the build revision:

```bash
python3 - <<'PYCODE'
import hashlib, json, shutil, subprocess, tempfile
from pathlib import Path
root = Path.cwd().resolve()
public = json.loads((root / "docs/qa/issue-1422/evidence/build-provenance.json").read_text())
receipt = dict(public["metadata"])
for field in ("artifacts", "source_inputs"):
    receipt[field] = {entry["path"]: entry["sha256"] for entry in public[field]}
parent = Path(tempfile.mkdtemp(prefix="issue1422-receipt-"))
snapshot = parent / "checkout"
subprocess.run(["git", "worktree", "add", "--detach", str(snapshot), receipt["source_commit"]], check=True)
for field in ("source_inputs", "artifacts"):
    for name, digest in receipt[field].items():
        source = (root / name).resolve()
        assert source.is_relative_to(root)
        assert hashlib.sha256(source.read_bytes()).hexdigest() == digest, name
        target = snapshot / name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, target)
record = parent / "receipt.json"
record.write_text(json.dumps(receipt))
subprocess.run(["python3", str(root / "scripts/ci/build-provenance.py"), "--root", str(snapshot), "--verify", str(record)], check=True)
# Removes only the newly created temporary snapshot after successful verification.
subprocess.run(["git", "worktree", "remove", "--force", str(snapshot)], check=True)
shutil.rmtree(parent)
PYCODE
```

Build or provide the reviewed matching artifacts first. A receipt alone does not
authenticate a compiler or establish production deployment.
