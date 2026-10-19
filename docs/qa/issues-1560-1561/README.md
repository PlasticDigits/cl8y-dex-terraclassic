# Trading risk and compact mobile Swap — #1560 / #1561

Scope: [risk policy #1560](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1560)
and [mobile presentation #1561](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1561).
Implementation uses a shared exact evaluator and scoped mobile CSS. Risk controls
and geometry are distinct: making the action visible never grants signing permission.
See the [final results](RESULTS.md), [mobile checklist](../issue-1561/README.md),
[order policy](../../../skills/AGENTS_ORDER_SLIPPAGE.md) and
[layout policy](../../../skills/AGENTS_SWAP_LAYOUT.md).

## Release invariants

- Strict original integer comparisons select ordinary consent at signed order
  slippage >5% OR symmetric total fair-reference deviation >10%. Never add fees
  to order percentages or substitute a legacy metric when evidence is unavailable.
- Symmetric total >30% requires Expert; >=99% is nonwaivable in either direction.
  Original local observations, expiry, account/operations/amount binding, all
  admission/floor/tax/fee guards and no duplicate broadcast remain enforced.
- Mobile CTA and status use normal flow. Long errors/zoom/open DETAILS may grow
  vertically; the actual route and its destination cannot be hidden to fit.
- Exactly one current actionable notice, persistent textual failures and one
  DETAILS/network-fee presentation remain. Meaningless normal-state text is silent.
- Release only the frontend application. The API, indexer and logging applications
  stay running; unavailable verification is recorded separately from PASS.

## Reproduce

Every test process must finish within 55 seconds. Split geometry by viewport and
use the already built fixture when needed; record actual selected environment
variables and commands in RESULTS. Owned browser fixtures are reset between
failures. Never reset production infrastructure to clear a test fixture.

Frontend release helper:

```sh
bao-exec --only COOLIFY_URL,COOLIFY_WIRE_TOKEN -- \
  python3 scripts/ops/trading-ui-release.py inspect --output /tmp/ui-before.json
# Explicit effect; pass the reviewed, merged main SHA:
bao-exec --only COOLIFY_URL,COOLIFY_WIRE_TOKEN -- \
  python3 scripts/ops/trading-ui-release.py deploy --revision "$(git rev-parse HEAD)" \
  --output /tmp/ui-deploy.json
bao-exec --only COOLIFY_URL,COOLIFY_WIRE_TOKEN -- \
  python3 scripts/ops/trading-ui-release.py deployment --deployment DEPLOYMENT_ID \
  --revision "$(git rev-parse HEAD)" --output /tmp/ui-status.json
```

The helper binds the exact frontend UUID/domain/repository/branch/image recipe,
reads back the source pin, and queues its normal Coolify deployment. Deployment
observations bind both the expected exact commit and unchanged application pin.
Queued/in-progress observations are PENDING; only a finished deployment with no
unexpected error-pattern log entries is PASS. Failed/cancelled/error-bearing
finished deployments are FAIL. Only an exact Docker missing-cleanup-container
line is counted separately; an embedded cleanup phrase cannot hide another error.
These deployment receipts still require independent live artifact/availability
checks. The helper neither
stops nor force-rebuilds another service. Rollback is the same explicit deploy
operation with the previously verified frontend SHA from `deployment-before.json`.
Use this only when a failed release actually displaced the healthy frontend.
Inspect raw logs only in memory; receipts contain fixed statuses, hashes and
counts, never log lines/configuration/credentials. The managed private API origin
is consumed by the existing bounded Coolify reader; the strict workstation
adapter rejects that origin's HTTP scheme and is not used to certify this report.

Observe Grafana, Loki and exact fresh Prometheus targets through the existing
[bounded monitor](../../../scripts/qa/probe-swap-monitoring-1479-1480.py).
A count of error-pattern lines is not an outage rate or a cause diagnosis.

## Acceptance record

See RESULTS for checked criteria and explicit unavailable scopes. Synthetic
browser wallets cannot sign and do not establish funded settlement, physical
assistive-device behavior, native-route acceptance or real on-chain recovery.
Physical-device and funded checks stay unchecked unless actual receipts exist;
issues may only close when their body and comment criteria have all passed.

Post-release read-only product check (no wallet injection or broadcast):

```sh
timeout 55s bash scripts/with-node.sh -- node \
  scripts/qa/probe-trading-ui-1560-1561.cjs /tmp/live-trading-ui.json
```

The physical follow-up uses the cl8y-pm
[inbox-assigner skill](https://git.cl8y.com/PlasticDigits/cl8y-pm/src/branch/main/skills/inbox-assigner/SKILL.md).
Its card requests no signing or privileged command; other unperformed verification
stays in these DEX issues. No engineering or CI queue work was assigned to the operator.
