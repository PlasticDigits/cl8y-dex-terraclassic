# Native/wrapped route attribution verification

Issue [#1483](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1483).
Implementation [PR #1484](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1484);
retail warning follow-up [PR #1485](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1485);
all-root verification correction [PR #1488](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1488);
revision and evidence: [RESULTS.md](RESULTS.md).
Read [RC1483-1–RC1483-10](../../../skills/AGENTS_ROUTE_COST_ATTRIBUTION.md),
[the API/rounding contract](../../route-solver.md#economic-attribution-1483),
[reference integrity](../../../skills/AGENTS_SMALL_MARKET_REFERENCE.md),
[offline execution budgets](../../../skills/AGENTS_OFFLINE_SWAP_BUDGET.md).

## Agent procedure

1. Read complete issue/body/comments using openbao-tools `bao-exec` and `fj`.
   Record UTC, source revision/fingerprint and artifact/tool provenance. Keep
   original reporter input/context missing; use independently selected test inputs.
2. Run `make verify-issue-1483`. Also run `verify-issue-1452`, `1473`, `1479`,
   `1480` and applicable `1464`, `1453`, `1391`, `1398`, `1381`, `1390`, `1396`.
   Integration binaries use a dedicated local QA control capability and their own
   disposable child DBs. Never migrate/truncate shared or production databases.
3. Check independently calculated exact raw candidates, direct and naturally
   eligible intermediate paths, exact-amount cache changes, solve/best/POST/receive,
   native matched-credit/equal-gross controls and the reconciled signed components.
   New fixtures must pin original reserves, precision, fees and reference quality.
4. Inspect actual Swap/Trade explanation, malformed/forged attribution, unchanged
   5/30/99 safeguards, amount/context/expiry mutation and positive execution floors.
   Diagnostic failure cannot authorize a transaction or silently alter its metric.
   Apply [the cognitive-load rules](../../../skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md):
   ordinary <=5% information belongs in existing Signing details; higher-risk
   quotes and confirmation share one notice. Check one visible operational
   blocker at a time, silence missing advisory breakdowns and speculative
   refresh warnings, and retain required risk/trust acknowledgements. Rounded
   display must not contradict the exact threshold decision.
5. Execute funded supported fixtures with current checksum/source-verified
   artifacts. Independently verify exact wallet net and rejected-floor rollback;
   measure requested/signed gas separately. Fix failures, reset only owned affected
   chain/DB state, then repeat on stable final source.
6. Attest served frontend/root, backend and contract revisions separately; run
   bounded read-only API/browser shards and classified Grafana/Loki/Prometheus
   monitoring. Keep socket 500ms/browser 750ms clocks and counts/p50/p95/p99/max.
   Continue live checks beyond both original 20-second and 180-second cycles.

## Acceptance checklist

Whole criteria remain unchecked until every body/comment requirement has evidence.
Passing subsets, explicit unavailable checks and genuine failures belong in
[RESULTS.md](RESULTS.md). A source PASS does not establish funded or live acceptance.

| Complete | Criterion | Evidence |
| --- | --- | --- |
| [ ] | AC1 classify concern | Pinned reserves/reference/fees classify size loss, fixed costs, divergence or inferior selection; original report limitations explicit. |
| [ ] | AC2 generic attribution | Signed raw components independently reconcile fair minus net, integer bounds and native/wrapped asymmetry; unavailable/forged evidence fails closed. |
| [ ] | AC3 amount-specific ranking | Same-cohort naturally eligible direct/intermediate solve/best/POST/receive winners; cache order, unusable alternatives and unchanged work caps. |
| [ ] | AC4 actual retail explanation | One truthful route percentage/cause in Swap and Trade at desktop/phone; all confirmation/Expert/refusal and signing blocks intact. |
| [ ] | AC5 execution integrity | Zero request RPC, original context/expiry/gross/floors; owned supported native/wrapped funded net and rejected-floor rollback with gas separate. |
| [ ] | AC6 repeated/full/live evidence | Stable first/retest boundary/abuse scope, deployed artifacts, useful timing, sanitized monitoring and sustained live replacement cycles; unavailable devices/SQL explicit. |
| [x] | AC7 docs/agent guidance | API/rounding/invariants/limits/checklist/issue/PR and skills crosslinks agree with the implementation; passing source/OpenAPI/crosslink evidence in RESULTS.md. |

## Execution and receipts

`scripts/qa/verify-issue-1483.sh` runs pure attribution, same-cohort socket ranking,
actual frontend validation/entry checks and types twice against a stable source
fingerprint. `VERIFY_ISSUE_1483_SKIP_<GROUP>_REASON` supports an explicit concrete
unavailable reason for RUST, DB, FRONTEND, TYPES or FORMAT after feasible checks
are attempted. Related complete verifiers run separately to retain their scope.

The funded drivers `localterra-offline-gas-1480.py`,
`localterra-quote-protection-1473.py` and `localterra-wrapper-floors-1459.py` accept
owned isolated containers/artifacts and public receipt outputs. Inspect each
driver's ownership and artifact preflight before reuse. Runtime safety controls
are enforced even when unavailable workflow verification is explicitly skipped.

`scripts/qa/localterra-route-attribution-1483.py` reuses the existing audited
atomic two-message signer and the bounded original-hash inclusion monitor. Pass
an explicitly owned `--container`, current checksum/source-verified DEX
`--artifacts`, rebuilt protected `--wrapper-artifacts`, exact companion
`--wrapper-source` and private `--output`. It checks supported native wrap plus
one pool hop and wrapped one-hop settlement at 1/5/30% floors, including actual
custody/supply/rate/wallet principal and atomic rejected-floor rollback. Its
12M synthetic gas budget is separate from production offline calibration.
The companion source/input and byte hashes identify the owned build; they are
not independent proof of deployed wrapper artifacts.

The #1473 driver now reuses the existing #1480 bounded 120-second REST monitor.
It retains the accepted original transaction hash privately before polling and
never rebuilds/resends an accepted or uncertain transaction. The original CLI
inclusion timeout stays recorded; an included transaction is reconciled before
only owned state is reset. Current artifact/source preflight also precedes funding.

For human-only wallet review, `prepare-wallet-attribution-1483.py` attests served
backend/frontend/root artifacts, performs two anonymous read-only quotes, writes
a sanitized private receipt and opens public Swap links without exposing credentials.
Use the concise cl8y-pm card command with an explicit merged revision:

```sh
bao-exec --only COOLIFY_URL,COOLIFY_WIRE_TOKEN -- python3 scripts/qa/prepare-wallet-attribution-1483.py --revision <merged-sha> --open
```

The helper tests verify secret-free outputs, explicit revision/root binding and
the bounded public review launch. Fresh actor-bound wallet quotes, selected 5%
protection and one intentional submit/no-resend remain human checks. Tests do not certify a physical wallet's signed
fields or production settlement. One human action per cl8y-pm card remains required.

## Bounded deployed explanation matrix

After independently attesting the exact merged frontend/backend/root artifacts,
run [the live explanation tool](../../../scripts/qa/browser-route-attribution-1483.cjs):

```sh
EXPECTED_SOURCE_SHA=<exact-merged-40-hex> \
DEX1483_ATTESTATION_FILE=<independent-attester-pass-json> \
DEX1483_BROWSER_OUTPUT=/tmp/browser-explanation-live.json \
bash scripts/with-node.sh -- node scripts/qa/browser-route-attribution-1483.cjs
```

The source-bound actual validator/clock helpers check 18 desktop/phone cases:
native 1/1,000/4,000 inputs, actual quoted matching wrapped credits and genuine
cLUNC/UST1 Trade inputs. Every external root script is checked against the independent
attestation before/after, including `/bootstrap/theme.js`,
`/bootstrap/trade-shell.js` and canonical `/assets/` scripts. Both bootstrap
files and at least one asset are required within the existing eight-script cap;
unknown paths, duplicates, traversal and base overrides fail before fetching. Hard caps are 100 API reads, 100 quote reads and 180,000ms;
writes, actor requests, WebSockets and chain paths are blocked. No wallets or
screenshots are captured. Original-input useful DOM timing uses the named 750ms
budget, two frames and fully visible/unclipped matching explanation; socket500
remains separate. Fixed sanitized receipts use mode0600 and omit private bodies.
Receipts include the tool byte hash, Git HEAD and dirty status independently of
the deployed product revision. Run the actual root-parser Python tests and
`node --test scripts/qa/test-browser-route-attribution-1483.cjs` first/retest;
passing helper tests cannot replace the actual deployed matrix.

## Explicit skip policy

Complete runnable checks first. **PASS WITH SKIPS** applies only when every
executed check passes. Executed failures remain FAIL until fixed and rerun;
retained failures are not reclassified as unavailable. Source/helpers, synthetic
settlement, HTTP200 or diagnostics cannot certify original reporter economics,
independent deployed contract/device behavior or production settlement. Keep
missing criteria unchecked, provide concise human-only cl8y-pm cards and leave
the issue open until every acceptance requirement passes.
