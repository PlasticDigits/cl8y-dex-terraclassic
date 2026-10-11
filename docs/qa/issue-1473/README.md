# Issue #1473: agent verification

Issue: [#1473](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1473).
Read [TQ1473-1–TQ1473-10](../../../skills/AGENTS_TRADING_QUOTE_OUTCOMES.md),
[local API outcomes](../../route-solver.md#trading-quote-outcomes-1473),
[frontend defaults](../../frontend.md#trading-quote-outcomes-1473),
[data waits and client expiry](WARMUP.md),
and [results/provenance](RESULTS.md). Implementation: [PR #1474](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1474).

Quote behavior is generic across supported arbitrary pairs and exact amounts.
Native USTC → CL8Y is only the product entry default. The
[shared matrix](matrix.json) includes seven pairs, both directions, six/eighteen
decimals, one raw unit, nine human amounts and 1%/5%/30% protection (420 cases).
Public asset identities in the fixture never admit a route or select symbols.

## Temporary browser latency criterion

On 2026-10-11 UTC the user explicitly relaxed the #1473 initialized browser
criterion to **≤750 ms for now**. This applies to the controlled and deployed
browser helpers, measured from the original input through every attempt to
confirmed useful render. Fresh receipts record `browser_latency_budget_ms: 750`.
The issue's AC7 now states this temporary 750 ms criterion. Prior 500 ms browser
acceptance has not been established; every historical FAIL receipt retains its
verdict. A fresh passing 750 ms trial
is not a retrospective pass or proof of the original 500 ms target.

The separate socket budget stays 500 ms. Quote protection percentages, original
expiry, economic floors and other work/resource budgets remain unchanged.
The issue remains open while its other acceptance criteria are incomplete.

## Procedure

1. Fetch origin and use an isolated worktree. Record exact SHA and any dirty patch
   fingerprint plus UTC start/end. Install dependencies through `scripts/with-node.sh`.
   Inject only required credentials using openbao-tools `bao-exec`; never emit
   credential values or source runtime/migration credentials into test processes.
2. Provision `indexer/.env.qa` with the dedicated QA control capability. Test
   helpers create/migrate only random QA-owned child databases. Provision an
   owned LocalTerra container for funded checks; never reset shared/production
   state. Reused artifacts require verified checksums and equivalent source inputs.
3. Run `make verify-issue-1473`. Inspect independently derived integer outputs,
   public rejection classes, original expiry, client validation, retry generation
   and signing guards; statuses/text existence alone are insufficient.
4. Run `make verify-issue-1452`, `verify-issue-1464`, `verify-issue-1453`,
   `verify-issue-1398`, `verify-issue-1381`, `verify-issue-1390`,
   `verify-issue-1399`, `verify-issue-1422`, `verify-issue-1394`,
   `verify-issue-1392`, `verify-issue-1396`. Inspect affected tests and explicit
   scopes rather than treating earlier receipts as final-source evidence.
5. On failure, fix the cause, restart/recreate only owned fixture infrastructure
   where state matters, and repeat the relevant first/retest suite on stable final
   source. Keep first failures and successful retests separately in results.
6. Separately attest deployed backend SHA and frontend asset/build provenance.
   Run the public socket matrix and desktop/phone browser matrix across observation
   and configuration replacement (>20 s and >180 s). Inspect bounded classified
   logs. Never infer frontend provenance from backend health or issue closure.

## Acceptance checklist

Check a whole criterion only when its issue-body and comment requirements pass.
Source subsets may pass while the acceptance criterion remains unchecked.

| Complete | Criterion | Agent instructions |
| --- | --- | --- |
| [ ] | AC1 original UST1/CL8Y transitions | Preserve baseline 1/10/100/1000 offers both ways. Pin actual economic observations and compare local/contract outputs at 1/5/30%; distinguish protection, unavailable prerequisites, accounting and client validation. Synthetic equal-price pools alone do not explain historical production causes. |
| [ ] | AC2 arbitrary priceable inputs | Execute the shared 420-case integer corpus and deterministically expand supported assets/direct/multihop/wrap paths. Verify net exact amount output and dust/liquidity/unsupported outcomes; priceable amount impact no longer rejects its own quote under quote_relative_v1; legacy price-only metadata remains non-executable and is not retail display. |
| [ ] | AC3 intent and scheduled recovery | Refresh/repeat/edit/receive/direction/pair/rapid-cancel/generation changes clear stale errors. Identical deterministic 422 has no solve timer loop. Changed scheduled generation recovers without triggering producers; retained original expiry cannot be renewed. |
| [ ] | AC4 native entry default | Test shuffled/late/empty/duplicate/spoofed catalogs, missing configured addresses, explicit aliases/deep links and deliberate selections. Bare `/trade` uses a native-capable ticket/navigation; factory/chart/limit identities remain genuine. Check actual funding/labels/orientation. |
| [ ] | AC5 public API reason contract | Test every reason/status pair plus explicit quote_relative_v1 policy isolation, independently recomputed market ceiling, exact upward-rounded UI floors, legacy compatibility and overlong/malicious/unknown/private payloads. Client HTTP 200 validation failures remain distinct and current guards authoritative. GET solve/best, POST and progress docs describe local economics. |
| [ ] | AC6 expanded matrix twice | Run first/retest with meaningful raw expected outputs. Cover both directions, ±1 raw dust/tax/reserve/spread boundaries, Uint128, receive bounds, explicit/manual/hybrid/maker budgets, state/context and user transitions. Do not substitute a smaller success for a failed large input. |
| [ ] | AC7 useful latency | Initialize desktop/phone browser independently; measure original input event through every attempt to correct useful output ≤750 ms under the explicit temporary user override above; report current AC7 separately from prior 500 ms FAIL verdicts. Report misses/hits, counts, p50/p95/p99/max; loading/error is not a successful quote. Cold/new transport separate; retain measured failures. |
| [ ] | AC8 execution/accounting/security | Tamper identities/context/amount/mode/generation/floors/tax/wrappers; malformed/overflow/huge/cyclic/dense/over-budget input fails safely. Provider counters stay zero. Fund owned LocalTerra quotes and verify exact net delivery, positive floors, unchanged reserves/LP on rejected floors and indicative cannot sign. |
| [ ] | AC9 documentation and deployment | Review source invariants/API/default/UX and skill crosslinks, exact source/patch/build/artifact/UTC receipts. Attest frontend/backend separately, inspect classified logs and repeat live matrix beyond observation/config cycles. |

## Public read-only socket matrix

`scripts/qa/probe-issue-1473.py` consumes the shared corpus for source/socket/browser
checks. Profiles: reproduction=24 requests per endpoint, census=210, full=420.
At most two cycles and two unique endpoints (1,680 quote requests), paced ≥500 ms,
10-second deadlines, 2 MiB quote and 4 KiB error bounds, no redirects/retries/actors.
The probe attests `/health.git_sha` before and after each trial. No endpoint URL,
raw body, private field, actor digest or workstation location enters the receipt.

```sh
bao-exec --only FORGEJO_PLASTICDIGTS_PAT -- python3 scripts/qa/probe-issue-1473.py \
  --revision <exact-deployed-40-character-SHA> --profile full \
  --endpoints solve best --cycles 2 --cycle-delay 181 \
  --output /tmp/issue-1473-public-matrix.json
```

This probe checks truthful classification and executable shape; it cannot prove
the economics of a live rejection. A valid blocked response counts as classified,
never an executable success or useful latency. Only positive executable samples
enter socket latency distributions. `--enforce-socket-budget` enforces 500 ms
separately; socket timings cannot establish browser input-event acceptance.

## Deployed artifact and browser receipts

After backend and frontend deployment, run the read-only
[deployment attester](../../../scripts/qa/attest-trading-quotes-1473.py) with the
exact merged revision. Supply `COOLIFY_URL` and `COOLIFY_WIRE_TOKEN` securely to that
process only. Optional `--frontend-app-uuid` / `--backend-app-uuid` select the
applications; otherwise bounded discovery matches their public domains and
rejects ambiguous identities. Private reads are bounded GETs with no redirects;
credentials, application identities, private URLs and upstream bodies never enter
the public receipt.

```sh
bao-exec --only COOLIFY_WIRE_TOKEN,COOLIFY_URL -- \
  python3 scripts/qa/attest-trading-quotes-1473.py \
  --revision <exact-merged-40-character-SHA> \
  --output /tmp/issue-1473-deployment-attestation.json

EXPECTED_SOURCE_SHA=<exact-merged-40-character-SHA> \
DEX1473_ATTESTATION_FILE=/tmp/issue-1473-deployment-attestation.json \
DEX1473_BROWSER_OUTPUT=/tmp/issue-1473-live-browser.json \
  bash scripts/with-node.sh -- node scripts/qa/browser-live-trading-quotes-1473.cjs
```

The [live browser helper](../../../scripts/qa/browser-live-trading-quotes-1473.cjs)
requires separately completed frontend/backend revision attestations and exact
served root HTML/script hashes before launching. It repeats backend revision
checks through the run and root artifact hashes afterward. The default full
matrix measures all 420 cases in both viewports twice, with a 181-second cycle
gap, at most 2,200 quote GETs and a 30-minute watchdog. Fetch reservations and
network GETs have separate counters; the same request does not consume two slots
from one budget. Smaller explicit `DEX1473_BROWSER_PROFILE=reproduction|census`
or `DEX1473_BROWSER_CYCLES=1` runs remain clearly scoped in their receipts.

Timing starts at the original input event, includes every attempt, and ends only
after two render-frame opportunities confirm the validated net receive value is
visible. Scoped error text avoids repeated full-document layout. Useful output
above exactly 750 ms remains FAIL under the temporary user criterion; receipts
name that budget. Historical 500 ms failures remain FAIL, and a fresh 750 ms pass
does not establish the prior 500 ms target. Errors/loading never count as
useful quotes, and incomplete runs cannot produce a passing receipt. The browser explicitly
disables GPU use and uses native Linux shared memory when at least 256 MiB is
available, with a safe fallback. Receipts contain mode/flags, fixed labels,
public revision/artifact hashes and timings, without host/device details.
Service workers are disabled; non-read HTTP requests and WebSocket connections
are blocked. Any unsupported connection prevents a passing receipt. No wallet,
signing, broadcasts or screenshots are used. Root asset provenance does not
attest lazy-loaded assets, settlement, historical observations or physical devices;
those remain separate acceptance criteria.

### QA helper follow-up (PR pending)

The live helper distinguishes a typed blocked outcome from useful output. A
blocked result requires a visible matching error and a blank receive field;
that blank field may be below the viewport when settings are expanded. A matching
missing/stale response may instead confirm the accessible `Loading price…`
progress surface. Neither errors nor waiting count as useful output. Executable
results still require exact independently validated net amounts, original expiry,
visibility and two render-frame opportunities measured from the first input event.
The temporary authorized 750 ms limit is unchanged. Completed failed samples now
produce a nonzero process exit, as do prerequisite/runtime failures.

Failed rows include bounded diagnostic timestamps, plan labels, counters and
Boolean state transitions: at most 12 current request records, four prior quote
records and 24 state changes. Requests are recorded before response headers arrive,
so a missing request, an in-flight request, expired cached pricing and network/body
delay can be distinguished. These fields omit raw bodies, URLs, actor/context
identifiers or digests and full-document text. Earlier 37-GET diagnosis receipts
remain failures; the original 889 ms API-path sample remains unresolved. This QA
correction does not establish a network-delay fix or deployed follow-up acceptance.

Run the [DOM/mock regression](../../../scripts/qa/test-browser-live-trading-quotes-1473.cjs)
without a browser or network:

```sh
bash scripts/with-node.sh -- node --test scripts/qa/test-browser-live-trading-quotes-1473.cjs
```

It executes the helper's actual measurement closure with DOM input events and
controlled frames, including offscreen blank receive, mismatched error, waiting,
exact/late useful amounts and expired-cache transitions. A child process also
checks the actual receipt/finalization code's success and failure exit status.
These checks cover helper behavior; production latency and provider causes require
separate deployment/browser receipts.

## Explicit skips and truthful verdicts

Agents may skip items they cannot complete and mark the overall tested scope
**PASS WITH SKIPS** when every executed check passes. Record attempted command,
missing prerequisite, exact revision/fingerprint and UTC scope; leave acceptance
unchecked. Unavailable holds/gates do not block source publication or merge.
**Executed failures remain FAIL** until fixed and rerun, including correctness,
latency, compilation and accounting. Do not use skips to hide failed tests.
Do not close #1473 unless all body/comment acceptance criteria pass.

The source verifier supports reason-bearing
`VERIFY_ISSUE_1473_SKIP_<GROUP>_REASON` for RUST, DB, FRONTEND, BROWSER, TYPES,
FORMAT. Default live/device/settlement omissions are explicit; supplement them
with separate receipts. Never claim full deployment from source-only acceptance.

### Existing-router settlement proof

Run `scripts/qa/localterra-quote-protection-1473.py` against an owned localterra
container and checksum/source-verified artifacts. It derives net outputs and
ceilings from independently read synthetic reserves, compares them to actual pair
simulation, executes direct/multihop 6/18/9-decimal offers in both directions at
1/5/30%, and checks exact wallet net delta plus rejected-floor reserve/LP rollback.
The synthetic treasury and swap recipient must be distinct: counting commission
in the recipient's delta would invalidate net-delivery evidence. Repeat on fresh
fixture contracts after corrections; no production broadcasts.
