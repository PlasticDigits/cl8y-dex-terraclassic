# Pool triage by liquidity (incident)

Use when implementing or verifying **SEC-G03** operator docs for ranking pools by approximate TVL or liquidity during incidents ([GitLab **#436**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/436)).

## Canonical doc

| Path | Purpose |
|------|---------|
| [`docs/runbooks/emergency-commands.md`](../docs/runbooks/emergency-commands.md) § Quick pool triage | SQL on `pair_reserves`, volume API proxy, LCD single-pair fallback |
| [`docs/templates/incident-dex-indexer.md`](../docs/templates/incident-dex-indexer.md) | Triage checklist item links to pool ranking |
| [`docs/runbooks/book-snapshot-mirror.md`](../docs/runbooks/book-snapshot-mirror.md) | How `pair_reserves` is populated and freshness expectations |

## Ranking options

| Method | Sort key | When to use |
|--------|----------|-------------|
| **Indexer SQL** | `approx_liquidity_units` (normalized reserve sum) or `quote_reserve_human` | Preferred — Postgres reachable; reflects on-chain pool reserves |
| **Indexer API** | `volume_quote_24h` via `sort=volume_24h&order=desc` | Indexer HTTP up but no DB shell access; activity proxy, not TVL |
| **LCD `pool` query** | Single pair reserves | Indexer down; you already have `$PAIR_ADDR` |

Schema: `pair_reserves` (one row per pair) joined to `pairs` + `assets`. Migration: `indexer/migrations/20260605010000_pair_reserves.sql`.

## Verification commands

```bash
# Doc invariant (no chain)
make check-pool-triage-docs
make verify-issue-436

# Optional SQL smoke (needs indexer/.env + Postgres)
source indexer/.env && psql "$DATABASE_URL" -X -c "SELECT COUNT(*) FROM pair_reserves;"
```

## Do not duplicate

- **Emergency pause/blacklist commands** live in [`AGENTS_EMERGENCY_COMMANDS.md`](./AGENTS_EMERGENCY_COMMANDS.md) — pool triage only discovers `$PAIR_ADDR`; mitigation uses that cookbook.
- **Blacklist decision tree** is separate — [`AGENTS_BLACKLIST_DECISION.md`](./AGENTS_BLACKLIST_DECISION.md).

## LocalTerra notes

- After `make setup-cloud-localterra`, `pair_reserves` is populated by the indexer book-snapshot loop (~10s).
- Use `export INDEXER_URL=http://127.0.0.1:3001` for the volume API proxy on Cloud Agent VMs.
