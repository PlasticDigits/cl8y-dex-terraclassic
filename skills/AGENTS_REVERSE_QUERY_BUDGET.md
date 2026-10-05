# Agent protocol: reverse-query availability (#1389)

Apply when changing pair/router reverse sizing, exact-output admission, Observe,
or receive-quote failure handling. Issue: [#1389](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1389).
Invariants and limits: [route solver](../docs/route-solver.md#reverse-query-availability-1389).
Review procedure and evidence: [QA](../docs/qa/issue-1389/README.md).

1. Meter before invoking nested simulations. Router accounting reserves opaque
   declared reverse work at the pair's full 32-call ceiling. Cached greedy probes
   cost zero; failed calls consume budget; fresh final validation has reserved
   capacity. Preserve the four-hop cap and bounded book-maker/scan caps.
   A future pair release needs reviewed accounting and an explicit supported
   version test before receiving the one-call pool cost; do not infer it from
   SemVer. These bounds assume compatible factory-selected CL8Y pair code;
   cw2 metadata is not a security attestation for arbitrary malicious contracts.
2. Checked pool inverses and interpolation only propose probes. Require actual
   forward sufficiency on the same route and trader context. Pair minimum claims
   require an adjacent insufficient offer. Do not increase ceilings to hide a
   deterministic exhaustion, claim universal hybrid support, or loosen spread,
   tax, minimum_receive, pause, blacklist or balance checks.
3. Exact-output `/route/solve` and `/route/solve/best` share four global slots and
   one active request per socket-peer IP. Ignore client-supplied forwarding
   headers, trader and sender for admission. Proxies/NAT share one bucket;
   missing socket metadata shares a conservative bucket. Never trust arbitrary
   forwarding headers to bypass that restriction. Cancellation/error/timeout
   must drop capacity; retain only active client keys and emit aggregate metrics.
4. Validate Observe horizons using checked subtraction. Invalid ranges return
   errors before storage traversal; never panic on a public query.
5. Preserve Receive and invalidate executable Pay/operations on failures. No
   automatic deterministic retry, weaker fallback, wallet action or resend.
   Existing [receive sizing](AGENTS_SWAP_EXACT_OUTPUT.md) and
   [route-local tax](AGENTS_ROUTE_LOCAL_TAX.md) protocols still apply.
6. Run `make verify-issue-1389`, `make verify-issue-1376`, affected workspace
   tests/lint and the isolated LocalTerra gas procedure in QA. Use synthetic
   mocks/test-chain fixtures only; never stress a public LCD. Record revision,
   UTC, command, exit code and evidence scope.

Agents may report **PASS with explicit skips** when all completed checks pass
and a check cannot be completed. Record each unavailable check as **SKIP** with
its concrete reason and remaining scope. An executed failure is **FAIL** until
fixed and retested. Skips do not establish deployment, migration, gas evidence,
or acceptance of the skipped criterion. Unavailable workflow holds/gates do not
block implementation review; runtime protections and credential boundaries
remain enforced. Agents own verification and followups.

Pair pool reverse cost depends on exact raw cw2 identity: `cl8y-dex-pair`
versions `1.19.0` and `1.20.0` permit cost one. Legacy/missing/malformed/failed
or unknown identities reserve the shared 32-call maximum, including greedy
seeds. Never infer safety from a larger version number. Metadata adds at most
one raw pair query per hop. Legacy greedy seeds leave no remaining search
slot after final validation reservation, so they return a budget error before
any greedy forward call. Tests cover legacy 1.18.0 and unknown future versions.
