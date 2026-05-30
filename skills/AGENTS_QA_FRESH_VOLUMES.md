# QA fresh volumes (agents)

Use when bringing up the **QA server** stack and chain/DB state must match **current** wasm and indexer schema.

**Tracking:** [GitLab **#202**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/202) (toggle), [#203](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/203) (stale-contract warnings). Parent context: [#120](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/120).

## Commands

```bash
# Default — keeps LocalTerra + Postgres volumes (fast)
make start-qa

# Wipe volumes then full bring-up (post-contract-change verification)
make reset-qa
# same:
QA_FRESH_VOLUMES=1 make start-qa
```

Expect a **red banner** when volumes are wiped. Do **not** assume `deploy-local` alone resets chain state.

## Invariants

See [`docs/qa-invariants.md`](../docs/qa-invariants.md). Summary:

- `stop-qa` → `docker compose down` (volumes kept).
- `reset-qa` / `QA_FRESH_VOLUMES=1` → `down -v` on `localterra-data` + `postgres-data`, then normal `start-qa` path.
- Shared-host ports: set `QA_SHARED_HOST=1` in `.env` before either command (unchanged).

## Verify (no full QA host required)

```bash
make test-qa-fresh-volumes
make test-localterra-host-curl   # when localterra container is up
make qa-verify-deploy            # schema + stamp; uses docker exec if host ports hang
```

## Manual volume names (fallback)

If compose teardown fails, same effect as reset:

```bash
docker volume rm cl8y-dex-terraclassic_localterra-data cl8y-dex-terraclassic_postgres-data
make start-qa
```

## Cross-links

- Operator README: [`scripts/qa/README.md`](../scripts/qa/README.md)
- Stale-contract detection: [`AGENTS_QA_DEPLOY_VERIFY.md`](./AGENTS_QA_DEPLOY_VERIFY.md) ([#203](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/203))
- Helpers: [`scripts/qa/lib/qa-env.sh`](../scripts/qa/lib/qa-env.sh), [`scripts/qa/reset-qa.sh`](../scripts/qa/reset-qa.sh)
