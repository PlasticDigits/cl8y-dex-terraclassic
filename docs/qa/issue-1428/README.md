# Issue #1428: populated greedy reverse support

Issue: [#1428](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1428).
Policy: [GR1428-1–GR1428-8](../../../skills/AGENTS_GREEDY_REVERSE_SUPPORT.md),
[solver](../../route-solver.md#greedy-reverse-support-1428),
[integrators](../../integrators.md#greedy-reverse-route-admission-1428).
Implementation: [admission guard](../../../smartcontracts/contracts/router/src/reverse_query.rs),
[query and regressions](../../../smartcontracts/contracts/router/src/contract.rs),
[funded greedy regression](../../../smartcontracts/contracts/router/src/issue1422_tests.rs).
Harness: [matrix](../../../scripts/qa/localterra-reverse-1389.py),
[offline harness tests](../../../scripts/qa/test-localterra-reverse-1389.py).
Results: [RESULTS.md](RESULTS.md). Independent review: [review.md](review.md).

Router 1.5.0 rejects any four-hop reverse route containing greedy before external
queries, even when books are empty or other hops are pool-only. Up to three total
hops may enter existing bounded sizing; admission does not promise universal gas
compatibility. Pool-only reverse and forward/execution keep four hops. The policy
preserves trader/gross funding, fresh forward sufficiency and net hook floors.

## Reproduction and retest

Use a separate worktree and dedicated synthetic node. Never reset shared volumes
or query production for stress tests. Freeze reviewed integrated router 1.4.0 /
pair 1.21.0 optimized artifacts and their provenance in a separate directory.
Use the existing reviewed `cw20_mintable.wasm` test dependency and locked test
query probe. Build current optimized wasm with `make build-optimized`.

The historical book matrix uses a **15,000,000** smart-query limit, distinct from
#1422's **3,000,000** funded pool fixture. Set 15M only in this dedicated node's
copied initialization script before boot, using the pinned SDK53 image digest
from `docker-compose.yml`; leave provider and shared-node limits untouched.
The harness enforces the image, SDK53, query limit, stored-code SHA-256 and cw2.
Record initialization script hash, image ID and artifact build provenance.


For example, create a private task initialization file and an isolated node with
no host ports or persistent shared volumes:

```bash
mkdir -p /tmp/dex-1428
python3 - <<'PYCODE'
from pathlib import Path
import re, subprocess
root = Path.cwd()
image = re.search(r"image:\s*(\S*/localterra-cl8y@sha256:[0-9a-f]+)",
                  (root / "docker-compose.yml").read_text()).group(1)
text = (root / "docker/init-chain.sh").read_text()
needle = 'APP_TOML="$TERRA_HOME/config/app.toml"'
assert text.count(needle) == 1
init = Path("/tmp/dex-1428/init-chain.sh")
init.write_text(text.replace(needle, needle + "\n" +
    "sed -i 's/query_gas_limit = 3000000/query_gas_limit = 15000000/' \"$APP_TOML\""))
init.chmod(0o755)
subprocess.run(["docker", "run", "-d", "--name", "cl8y-dex-issue1428-matrix",
    "--tmpfs", "/home/terra/.terra:rw,mode=1777,size=2g", "--tmpfs", "/tmp:rw,mode=1777,size=128m",
    "--mount", f"type=bind,source={init},target=/usr/local/bin/init-chain.sh,readonly", image], check=True)
PYCODE
# Wait for latest_block_height > 0 before funding synthetic fixtures.
docker exec cl8y-dex-issue1428-matrix curl -sf http://127.0.0.1:26657/status
RUSTFLAGS='-C link-arg=--allow-undefined -C target-cpu=mvp' \
  cargo +1.81.0 build --locked \
  --manifest-path scripts/qa/fixtures/reverse-query-probe/Cargo.toml \
  --release --target wasm32-unknown-unknown
```

```bash
python3 scripts/qa/localterra-reverse-1389.py \
  --container cl8y-dex-issue1428-matrix --artifacts /path/to/frozen-integrated \
  --probe /path/to/reviewed/reverse_query_probe.wasm \
  --code-cache /tmp/dex-1428-codes.json --router-version 1.4.0 \
  --expect-populated-gas-failure --evidence-file /tmp/dex-1428-baseline.json

VERIFY_ISSUE_1428_CONTAINER=cl8y-dex-issue1428-matrix \
VERIFY_ISSUE_1428_ARTIFACTS=/path/to/current-optimized \
VERIFY_ISSUE_1428_PROBE=/path/to/reviewed/reverse_query_probe.wasm \
VERIFY_ISSUE_1428_BASELINE_ARTIFACTS=/path/to/frozen-integrated \
make verify-issue-1428
```

Each fixture owns fresh synthetic contracts/signers. The baseline preserves the
fourteen preceding matrix cases and the actual populated four-hop failures in
both directions as `status=FAIL`, `baseline_regression=REPRODUCED`; successful
reproduction is separate from final acceptance. Current checks require admission
denial in empty and populated four-hop greedy cases, plus empty three-hop greedy boundary success in both directions, sufficient
explicit populated one-hop requests (maker cap 8, one raw ask unit both directions
and one human ask unit forward), and retained pool/declared behavior. These are
separate requests, not fallbacks for the rejected original quote. Actual smart errors/responses and full
payloads are saved before measurement. Probe receipts use an unchanged 12M
transaction gas cap, include transaction/wrapper overhead and can fail independently
of the 15M smart query; preserve those errors rather than treating gas as work counts.

Recheck six funded pool scenarios against router 1.5.0 on a separate unchanged
3M node using `VERIFY_ISSUE_1422_ROUTER_VERSION=1.5.0`, artifact/probe/container
variables documented in [#1422 QA](../issue-1422/README.md), then run
`make verify-issue-1422`. Also run `make verify-issue-1389`,
`make verify-issue-1398`, `make verify-issue-1381` and `make verify-issue-1390`
for affected semantics, following their own explicit skip policies. Use disposable
owned databases for indexer integration checks; never borrow shared migration history.

## Agent review and verification checklist

Agents own this checklist. Read issue/comments using openbao-tools `bao-exec`
with `fj`, inspect the final diff and follow linked invariants. Record revision,
UTC, exact command, exit code and evidence for each item. Repeat after corrections.

- [ ] **Integrated reproduction:** verify frozen hashes/cw2 and pinned SDK53
  identity; retain fourteen preceding cases and both raw populated four-hop gas
  failures at the existing 15M limit. Keep historical #1422 FAIL matrices intact.
- [ ] **Deterministic admission:** run router tests, inspect guard ordering, and
  verify zero nested queries for all/mixed greedy four-hop paths, either direction,
  zero/maximum amounts and trader contexts. Confirm empty and populated chain
  rejections carry the same stable marker. Pool reverse keeps four hops.
- [ ] **Admitted routes:** run/retest the explicitly scoped successful populated one-hop requests
  and empty three-hop boundary in both directions, and
  inspect exact operations, target, required offer and fresh forward response.
  Separate actual smart outcomes from overhead-scoped probe gas; do not infer
  universal provider support from one synthetic book.
- [ ] **Funding and protections:** run gross/trader/fresh-validation and route-tax
  regressions plus net-floor checks. Inspect ceilings and verify router 1.4→1.5
  migration preserves configuration, nonce, funding version and live-route guards.
- [ ] **Receive invalidation:** run the populated-admission failure case in
  `SwapPage.queryParams.test.tsx`. Cached executable Pay clears, Receive survives,
  signing remains disabled and no automatic retry/fallback/resend occurs.
- [ ] **3M funded pools:** execute all six #1422 scenarios and eighteen actual
  smart queries at unchanged 3M on final artifacts; retest from a fresh fixture.
- [ ] **Production acceptance:** only claim this after actual reviewed migrations
  and read-only probes at real provider limits with documented book/state scope.
  Existing rollout belongs to #1384/#1386/#1421; local source/node tests do not
  satisfy production deployment.

Agents may mark an unavailable item **SKIP** and overall verification **PASS with
explicit skips** after completing feasible work first. Use
`VERIFY_ISSUE_1428_SKIP_CONTRACTS_REASON`, `..._SKIP_FRONTEND_REASON`,
`..._SKIP_CHAIN_REASON` or `..._SKIP_BASELINE_REASON` with observed concrete reasons.
Keep each skipped criterion unchecked and state its remaining scope. Executed
final failures remain FAIL until repaired/retested; do not use skip permission to
hide them. Unavailable holds/gates do not block implementation review, and never
waive runtime protections, credentials or signing safeguards.
