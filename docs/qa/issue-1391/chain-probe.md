# #1391 LocalTerra infrastructure attempt

UTC probe window: 2026-10-05 06:38–07:15. Scope: local development containers;
no production price manipulation or production transaction was attempted.

`make has-localterra` found the shared LocalTerra container. Its RPC responded,
but latest height 838692 carried block time `2026-10-05T06:27:28Z`; logs showed
300-second consensus timeouts. The isolated-fixture harness created only a new
local test-keyring wallet and attempted its funding transaction. Funding inclusion
timed out before any token, factory, pair, swap or withdrawal was created.
No shared deployment was reset or restarted.

A separate `cl8y-dex-issue1391-chain` container was provisioned with the repository's
pinned image digest
`sha256:309cdb1215135795ca47f2803cde6c1e7045afcfc6379dd6600bf578148ae81a`,
a private named volume, RPC port 55492 and LCD port 55493. Existing CW20/factory/pair
wasm code IDs 1/3/4 were downloaded from the local shared chain into `/tmp` and
copied to the private container. The private chain initialized, but startup took
several minutes. At 07:06 UTC its RPC still reported height 1 with block time
`2026-10-05T06:54:15.939072948Z`. Store prerequisites rejected with:

> TerraApp is not ready; please wait for first block: invalid height

A final bounded read at 07:14:59 UTC reported height 2, but block time was
`2026-10-05T07:03:53.526440046Z` (over 11 minutes old). The fixture harness
requires a live chain before mutation, so no further store/spend command ran.

Its consensus configuration used 200ms proposal/commit and 100ms prevote/precommit
timeouts, so this was independent of the shared timeout configuration. No wasm
store succeeded, and no private fixture pool manipulation or LP withdrawal ran.
Disk had 153GB available. Privileged Docker diagnostics were unavailable because
`sudo -n` required a password; no host service was restarted.

**Result: SKIP / unverified chain attack acceptance.** This does not establish
LocalTerra reserve-manipulation execution, production deployment or migration
acceptance. Canonical-pool DB and fixed-clock tests provide synthetic containment
evidence separately. Runnable test failures must still be fixed and retested.

## One bounded restart after memory pressure eased

The parent stopped only the private chain to free memory at 07:36 UTC and
restarted implementation verification with debug symbols disabled; test
assertions stayed enabled. At 07:48:49 UTC the host reported 14GiB available
memory (31GiB total, 16GiB swap used). The verifier was still compiling dependencies.
Under these improved conditions, the agent made one authorized restart attempt:

```bash
timeout 20s docker start cl8y-dex-issue1391-chain
```

The command exited **124** after 20 seconds; the Docker start request did not
complete. A separate, bounded 55-second liveness probe used `terrad status` with
five-second per-call limits. It returned RPC timeouts/unavailable responses and
expired with exit **2**, without observing fresh advancing blocks.

A bounded inspection at 07:51:28 UTC reported `Running=false`, `ExitCode=128`,
`Error="Canceled: context canceled"`, original start time 06:52:12 UTC and stop
time 07:36:25 UTC. No successful new start occurred. No additional restart,
shared-chain reset, privileged handoff, store, funding, swap or withdrawal was
performed. The actual-chain harness and processor replay remain **SKIP / unverified**.
The implementation's runnable unit/DB/mock checks are assessed independently.

## Reproduce when the private chain is usable

Use the same pinned image with `docker/init-chain.sh` mounted read-only, a private
volume and distinct ports. Confirm fresh advancing blocks and working queries
before storing wasm or funding fixtures. Store CW20/factory/pair wasm in order;
use the actual assigned code IDs rather than assuming the shared IDs.

Run the isolated mutation harness with those IDs:

```bash
python3 scripts/qa/localterra-oracle-1391.py \
  --container cl8y-dex-issue1391-chain \
  --cw20-code 1 --factory-code 2 --pair-code 3 \
  --output /tmp/oracle1391-chain-receipt.json
```

The harness authenticates `localterra` network and freshness before creating a
new test wallet, tokens, factory and three unique pools. It records actual pool
reserves before and after a canonical UST1/cUSTC swap and a USTR/cUSTC LP
withdrawal. It does not reset existing volumes or touch existing pools.

Replay a successful actual-chain receipt through the production hub processor
using an isolated Postgres database:

```bash
cd indexer
ORACLE1391_CHAIN_RECEIPT=/tmp/oracle1391-chain-receipt.json \
  cargo test --test oracle_1391_chain_receipt -- --test-threads=1
```

The replay preserves actual raw reserves and exact CW20 addresses. It advances
only the reference-confirmation fixture clock and rebases observation timestamps;
it is processor replay evidence, not an end-to-end deployed indexer receipt.
Without an actual receipt, this optional test reports SKIP and must not be counted
as executed chain acceptance. Follow [the overall review policy](README.md).
