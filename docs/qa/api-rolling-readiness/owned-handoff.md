# Owned old/new/proxy handoff

The [fixture](../../../indexer/tests/scenarios/api_rolling_handoff.rs) uses two
independent real HTTP servers, AppStates, local LCD namespaces, startup latches
and production scheduled quote-ingestion workers. A third, permanently missing
profile is a never-ready candidate control. The local forwarding proxy returns
actual backend bytes/status once; it never fabricates health/quote output, retries
another node or hides a failed attempt. Selection reserves in-flight ownership
under the routing lock, so removal/drain cannot miss a selected request.

The old leader holds the real production advisory key. It processes a controlled
block through the real block-indexer RPC/parser/checkpoint path before warm-up,
then another while a never-ready candidate is rejected. New admission requires
both native directions from the independent cache while `/ready` is still 503 and
the old leader retains its key. Only then can the proxy add the new upstream.
An actual proxied new response must arrive before leadership moves. A real
session transfer uses the same 500-ms retry guard, then the production block
processor commits a third block and economic receipt. The proxy removes/drains
old work before retiring its HTTP listener and producer owner.

This is not an OS binary-process, full poller-supervisor or actual Coolify/Traefik
execution test. Synthetic LCD/raw reference observations are explicitly scheduled
fixture inputs; no quote/admission result or successful `health.progress` is
injected. Platform source/route eligibility and exact live deployment remain
separate requirements.

The focused profile is `paced_native_200ms_total_client_cap150_v1`: one native
request every 200ms; candidate probes every 500ms; a shared 150-call cap includes
all client-originated direct/proxied calls. Proxy forwarding is not counted as a
second logical client call. The whole owned case has a 48-second deadline and
runs in a process capped at 59 seconds. Hitting a cap fails the case; no warm-up
origin, observation expiry, producer cadence or provider limit is relaxed.

The receipt reports positive proxied request/native-success counts, HTTP
failures, maximum loopback latency, independent namespaces, standby readiness,
cold/never-ready rejection, actual block/checkpoint transfer and drain evidence.
After the positive stream ends, separate negative controls remove source data:
`/ready-api` and real quotes fail 503, while the admitted process latch and cheap
aggregate API stay routed 200. A request-only control stops producer work and
proves zero additional LCD calls without renewing original observations.

[Retained history](owned-handoff-history.json) contains the original fixture
asset-type failure, an executed stress native503 and a later stress PASS.
The native503 is **unresolved FAIL**. The passing stress run does not erase it;
changing to a focused measurement profile does not fix or reclassify it. The
paced combined-source run must execute with the missing-stage diagnostics before
new handoff acceptance is claimed. No production API was stopped or changed.
Neither this fixture nor its receipt closes #1505's sustained/live criteria.

Compile separately, then run only the owned case from the resulting executable:

```sh
CARGO_TARGET_DIR=/path/to/shared-target cargo test --manifest-path indexer/Cargo.toml \
  --test route_local_quotes_1452 --no-run
timeout 59s /path/to/shared-target/debug/deps/route_local_quotes_1452-HASH \
  --exact api_rolling_handoff::rolling_proxy_keeps_old_until_standby_native_admission_then_real_block_handoff \
  --nocapture --test-threads=1
```

The [combined frozen-source run](combined-local-results.json) now passes its
paced case in 17.877s: 106 total client calls, 44 proxied bidirectional native
successes, zero unexpected HTTP failures, maximum loopback latency5ms, three
actually parsed/checkpointed blocks and zero request-only LCD calls. All15 focused
groups passed on the same unchanged compiled-input manifest. The earlier design
history's PENDING status predates this run; its stress503 remains an unresolved
FAIL. This scoped local PASS does not certify live #1505 acceptance.
