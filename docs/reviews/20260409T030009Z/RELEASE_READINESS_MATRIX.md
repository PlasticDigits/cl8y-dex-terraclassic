# Release readiness matrix

| Area | Status | Evidence | Issues | Blocker |
|------|--------|----------|--------|---------|
| **Contracts build (production)** | Partial | `make build-optimized` + [`smartcontracts/scripts/optimize.sh`](../../../smartcontracts/scripts/optimize.sh) | DEX-P1-009 | Launch |
| **Contracts build (CI)** | Partial | `.github/workflows/test.yml` cargo wasm only | DEX-P1-009 | Launch |
| **Contract tests** | Ready | `cargo llvm-cov test` in CI | — | — |
| **Deployment docs** | Ready | [`docs/deployment-guide.md`](../../deployment-guide.md) | DEX-P2-013 (tier drift) | Low |
| **Factory / governance ops** | Partial | Doc’d; no automated checklist in CI | DEX-P1-006 | Launch |
| **Treasury configuration** | Partial | Per-pair fee config | DEX-P1-006 | Launch |
| **Indexer env validation** | Partial | `CORS_ORIGINS`, `DATABASE_URL` required; others default [`config.rs`](../../../indexer/src/config.rs) | DEX-P2-023 | Medium |
| **Indexer migrations** | Ready | `indexer/migrations/` | Run order in deploy doc | Medium |
| **Indexer monitoring** | Missing | `tracing` only | DEX-P2-024 | Medium |
| **Frontend env** | Partial | `VITE_*` [`docs/frontend.md`](../../frontend.md) | DEX-P2-025 | Low |
| **E2E gate** | Ready | Playwright in CI with LocalTerra + deploy | DEX-P1-011 gap for hybrid | Medium |
| **Rate limit / CORS (indexer)** | Ready | Invariants + tests | — | — |
| **Rollback strategy** | Unclear | Wasm admin/migration not in-repo playbook | DEX-P2-026 | Medium |
| **Mainnet vs testnet separation** | Partial | README networks; no env templates in tree | DEX-P2-027 | Low |
| **Secrets** | N/A in OSS | Document operator secrets outside git | DEX-P2-028 | Medium |
| **Reconciliation / backfill** | Partial | Indexer dedup; no dedicated CLI | DEX-P2-016 | Medium |
| **Version pinning** | Partial | `cosmwasm/optimizer:0.16.1` in deployment guide; Docker compose for LocalTerra | DEX-P2-029 | Low |
| **Runbooks (pause, incident)** | Missing | Security model + audit; no single runbook | DEX-P2-030 | Medium |
| **Hybrid launch readiness** | Not ready | Quote mismatch, docs | DEX-P1-003, DEX-P1-005 | Hybrid launch |

### Checklist: minimum for pool-only (v2) launch

1. Optimizer wasm artifacts uploaded; code IDs registered.
2. Factory instantiated with intended governance, treasury, whitelist.
3. Router instantiated; added as trusted router on fee-discount if discounts used.
4. Hooks: none or audited + allowlisted on hook contracts.
5. Smoke: single-hop and multi-hop swap on staging network.
6. If indexer deployed: DB migrated, `FACTORY_ADDRESS`, LCD URLs, `CORS_ORIGINS`, optional `ROUTER_ADDRESS`.

### Checklist: hybrid-enabled launch (stricter)

1. All v2 items.
2. User-facing disclosure of simulation limits; support docs linked.
3. Contract integration tests for hybrid paths on staging.
4. Decision on whether indexer must expose book or clients use LCD.
