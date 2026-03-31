# QA server + laptop (Yield Omega DEX)

## On the QA server

### One-time

- **Docker** with compose v2.
- **Rust + Cargo** on the PATH for `cargo run --release` in `indexer/` (or set **`INDEXER_QA_BIN`** to a prebuilt binary path in the environment before `make start-qa`).
- Optional repo-root **`.env`**: set **`QA_SSH_HOST`** (hostname as seen from your laptop) so `make qa-tunnel-help` prints a useful `ssh` destination; **`QA_SSH_PORT`** if SSH is not 22.

### Bring up the stack

```bash
make start-qa
# alias:
make qa-start
```

This stops any prior QA indexer and runs **`docker compose down`**, then starts **localterra** + **postgres**, waits for health, runs **`make deploy-local`** (optimizer wasm + **`scripts/deploy-dex-local.sh`**), starts the **indexer** in the background (pidfile **`.indexer-qa.pid`**, log **`.indexer-qa.log`**), checks indexer **`/health`**, and prints **laptop** steps (same as **`make qa-tunnel-help`**).

### Shared host with cl8y-bridge-monorepo

Bridge uses LocalTerra on **26658/1318** by default. To avoid port clashes, run DEX QA with:

```bash
export QA_SHARED_HOST=1   # or set in repo-root .env
make start-qa
```

This merges **`docker-compose.qa-shared-host.yml`** and defaults DEX LocalTerra to **26659/1319** (and remapped gRPC ports). Override with **`DEX_TERRA_RPC_PORT`**, **`DEX_TERRA_LCD_PORT`**, etc., if needed.

### Stop

```bash
make stop-qa
```

Stops the QA indexer (from pidfile) and **`docker compose down`**.

### Status

```bash
make status
```

Checks Docker, LocalTerra RPC, Postgres, indexer **`/health`**, and the indexer pidfile.

---

## On your laptop

1. **SSH** — Run the `ssh -4 -N ... -L ...` block from **`make qa-tunnel-help`** (forwards LocalTerra RPC/LCD and indexer API to local loopback).
2. **Env** — `scp` **`frontend-dapp/.env.local`** from the server into your clone (URLs use `localhost` and match forwarded ports).
3. **Optional** — **`./scripts/qa/write-frontend-env-local.sh`** refreshes **`VITE_TERRA_*`** / **`VITE_INDEXER_URL`** from **`scripts/qa/qa-host.env`** (useful if ports differ between machines).
4. **Vite** — `cd frontend-dapp && npm ci && npm run dev` — do **not** tunnel the Vite port; run the dev server locally.

---

## Makefile reference

| Target              | Purpose                                      |
| ------------------- | -------------------------------------------- |
| `make start-qa`     | Full QA bring-up on the server               |
| `make qa-start`     | Same as `start-qa`                           |
| `make stop-qa`      | Stop indexer + compose                       |
| `make qa-tunnel-help` | Reprint SSH + laptop steps               |
| `make status`       | Health summary                               |
| `make compose-ps`   | `docker compose ps` only                     |

---

## Troubleshooting

- **Indexer health fails** — Read **`.indexer-qa.log`**; confirm Postgres is up and **`indexer/.env`** **`DATABASE_URL`** matches compose (**`postgres://postgres:postgres@127.0.0.1:5432/dex_indexer`** by default).
- **LocalTerra not ready** — `docker compose logs localterra`; on port conflicts set **`QA_SHARED_HOST=1`** or free host ports.
- **Stale wasm** — `make build-optimized` then re-run deploy ( **`make deploy-local`** ).
