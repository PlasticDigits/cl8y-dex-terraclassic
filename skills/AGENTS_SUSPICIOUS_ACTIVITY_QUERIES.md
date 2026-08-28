# Suspicious activity discovery queries

Use when implementing or verifying **SEC-G04** operator docs for incident triage discovery ([GitLab **#437**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/437)).

## Canonical doc

| Path | Purpose |
|------|---------|
| [`docs/runbooks/suspicious-activity-queries.md`](../docs/runbooks/suspicious-activity-queries.md) | Indexer API + SQL + LCD recipes for wallet, pair, and token discovery |
| [`docs/templates/incident-dex-indexer.md`](../docs/templates/incident-dex-indexer.md) | Triage section links to discovery runbook |
| [`docs/runbooks/blacklist-decision.md`](../docs/runbooks/blacklist-decision.md) | Post-discovery escalation (do not blacklist on heuristics alone) |

## Discovery surfaces (minimum SEC-G04)

| Signal | Primary path |
|--------|----------------|
| Top-volume traders (24h) | `GET /api/v1/traders/leaderboard?sort=volume_24h` |
| Failed wasm txs by sender | LCD `cosmos/tx/v1beta1/txs` + `code != 0` filter |
| Pair swap / volume spike | `GET /api/v1/pairs?sort=volume_24h` + `.../stats`; SQL on `swap_events` |
| Reserve / LP anomalies | SQL on `liquidity_events`, `pair_reserves` |
| Blacklist state probes | `GET /api/v1/compliance/blacklist-check` (no Postgres audit log) |

## Verification commands

```bash
# Doc invariant (no chain)
make check-suspicious-activity-queries-docs

# Full #437 checklist
make verify-issue-437
```

Optional API smoke (LocalTerra + indexer):

```bash
export INDEXER_URL="${INDEXER_URL:-http://127.0.0.1:3001}"
curl -sf "$INDEXER_URL/api/v1/traders/leaderboard?sort=volume_24h&limit=3" | jq 'length'
```

## Do not duplicate

- **Post-identification** factory `blacklist_check` confirm queries live in [`docs/runbooks/emergency-commands.md`](../docs/runbooks/emergency-commands.md) — link, do not fork ([`AGENTS_EMERGENCY_COMMANDS.md`](./AGENTS_EMERGENCY_COMMANDS.md)).
- **Blacklist decision criteria** live in [`docs/runbooks/blacklist-decision.md`](../docs/runbooks/blacklist-decision.md) — link discovery runbook from evidence-gathering steps only.

## Limits

- Indexer Postgres has **successful** swaps only; failed on-chain executes require LCD queries (documented in runbook § 2).
- Compliance `blacklist-check` is a live LCD proxy — no `compliance_audit` table; optional HTTP access-log grep only. Comma `tokens` ≤ **16**, `pairs` ≤ **8** ([#694](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/694)); oversize is **400**.
