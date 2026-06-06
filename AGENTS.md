# AGENTS.md

Guidance for AI coding agents working in this repository.

## Cursor Cloud specific instructions

### VM prerequisites (one-time per snapshot)

These are **not** in the startup update script; install once when provisioning a new Cloud Agent VM:

- **Docker**: Cloud Agent VMs need Docker CE with `fuse-overlayfs` storage driver and `iptables-legacy`. Start `dockerd` manually if systemd does not (see below).
- **Rust**: Use `rustup default stable` (1.96+ as of 2026). Indexer build needs `libssl-dev` and `pkg-config`.
- **Node 24**: `nvm install` from repo `.nvmrc`. Cloud VMs may ship `/exec-daemon/node` (v22) **before** nvm on `PATH` — prepend the nvm bin dir or `hash -r` after `nvm use`, or `scripts/with-node.sh` may run the wrong Node.
- **Docker access**: `sudo usermod -aG docker $USER` then use `sg docker -c '…'` in non-login shells.
- **GitLab CLI (`glab`)**: Not preinstalled on Cloud Agent VMs. After `GITLAB_TOKEN` is available, run `./scripts/setup-glab-cloud-agent.sh` once per checkout (installs `glab`, authenticates, sets `remote.origin_url`, writes `.env.glab` with `GITLAB_REPO`). Cloud Agent git remotes use `https://x-access-token:…@gitlab.com/PlasticDigits/<repo>.git`, which breaks `glab repo view` / issue commands unless `GITLAB_REPO` is set — `source .env.glab` in the shell or re-run the setup script. Verify: `glab api "projects/PlasticDigits%2F<repo>"` (encode `/` as `%2F`).
- **Chrome + Keplr**: Run `./scripts/setup-browser-cloud-agent.sh` once per VM (or when Keplr is missing). Installs **google-chrome-stable** when absent, downloads Keplr from the Chrome Web Store, and registers it under `~/.config/google-chrome/Default/Extensions/`. Idempotent — safe to re-run. Regression: `make test-setup-browser`.

### Docker daemon

If `docker info` fails, start the daemon in tmux (Cloud VMs use the portal tmux config when present):

```bash
tmux -f /exec-daemon/tmux.portal.conf new-session -d -s dockerd 'sudo dockerd > /tmp/dockerd.log 2>&1'
```

Use `sg docker -c 'docker compose …'` when the current shell is not in the `docker` group.

### LocalTerra + `.env.local` (per session)

**One command** (infra → optimized wasm build → deploy → writes env files; starts indexer in tmux `indexer-dev`):

```bash
chmod +x scripts/setup-cloud-agent-localterra.sh
make setup-cloud-localterra
# or: ./scripts/setup-cloud-agent-localterra.sh --start-frontend
```

| Output | Purpose |
|--------|---------|
| `frontend-dapp/.env.local` | Vite contract addresses, LCD/RPC, `VITE_INDEXER_URL` (required for `make dev`) |
| `frontend-dapp/.env.development` | `VITE_DEV_MNEMONIC` for **Simulated Wallet** |
| `indexer/.env` | Indexer Postgres + chain endpoints |

**Node 24:** After `nvm use`, prepend `$(nvm which node | xargs dirname)` to `PATH` so `scripts/with-node.sh` and Playwright do not pick `/exec-daemon/node` (v22).

**Frontend + manual QA:**

```bash
export PATH="$HOME/.nvm/versions/node/$(cat .nvmrc)/bin:$PATH"
make dev   # http://127.0.0.1:5173
google-chrome --no-sandbox --disable-dev-shm-usage --disable-gpu http://127.0.0.1:5173/limits
```

**Strict E2E (#292):** after `make deploy-local`, run `bash scripts/e2e-start-indexer.sh` then `sg docker -c 'CI=1 make test-e2e'` — see **LT11** in [`docs/localterra-sdk53.md`](docs/localterra-sdk53.md). Do not chain `make test-e2e-tx` after an earlier deploy on the same volumes without resetting the indexer DB.

**Automated #295 ladder rung UI check** (needs `make dev` + deploy env; first run downloads Playwright Chromium):

```bash
make verify-issue-295
```

**Manual Chrome #295 walkthrough** (same steps as issue): open `/limits` → Ladder tab → clear rung count (must stay empty) → type `3` → type `25` (inline max error) → blur (clamps to `20`).

**Flags:** `--skip-build` (artifacts already built), `--fresh` (wipe volumes before start), `--infra-only`, `--no-indexer`, `--start-frontend` (also tmux `frontend-dev`).

If deploy fails mid-run (e.g. stale chain state), rerun with `./scripts/setup-cloud-agent-localterra.sh --fresh --skip-build`.

Idempotent deploy skip also probes the factory on LCD ([Q1](skills/AGENTS_QA_DEPLOY_VERIFY.md)); after `make reset` without `--fresh`, stale `.qa-deploy-stamp` / `.env.local` alone do not skip redeploy. **Which reset after code changes:** [skills/AGENTS_QA_REDEPLOY_DECISION.md](skills/AGENTS_QA_REDEPLOY_DECISION.md) ([#325](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/325)). QA server `make start-qa` uses the same skip; optional `QA_FETCH_CI_ARTIFACTS=1` and `INDEXER_QA_BIN`.

**tmux sessions:** `indexer-dev`, `frontend-dev`, `dockerd` — attach with `tmux -f /exec-daemon/tmux.portal.conf attach -t <name>`.

### Indexer integration tests (Postgres-only)

For **indexer-only** work (no frontend, no wasm deploy), provision Postgres + `indexer/.env` in one step ([#335](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/335)):

```bash
make setup-indexer-postgres
# equivalent: ./scripts/setup-cloud-agent-indexer-postgres.sh
# or: ./scripts/setup-cloud-agent-localterra.sh --postgres-only
```

| Output | Purpose |
|--------|---------|
| `indexer/.env` | `DATABASE_URL`, `TEST_DATABASE_URL` (minimum for integration tests) |

Does **not** start LocalTerra, build optimized wasm, or run `deploy-dex-local`. Host `psql` is optional — scripts fall back to `docker compose exec` when `postgresql-client` is missing.

```bash
export PATH="/usr/local/cargo/bin:$PATH"
make test-indexer-integration          # full suite, serialized
make verify-issue-324                    # #324 lib + route_solve_get_cache integration
cd indexer && cargo test --test api_route_solve -- --test-threads=1
```

Use **`make setup-cloud-localterra`** when you need `frontend-dapp/.env.local`, chain LCD, and a running indexer. See [skills/AGENTS_LOCAL_POSTGRES_DEV.md](skills/AGENTS_LOCAL_POSTGRES_DEV.md).

### Full local stack (manual — same as setup script steps)

| Step | Command |
|------|---------|
| Infra | `make start && make wait-healthy` |
| Contracts | `make build-optimized && make deploy-local` (~15 min first run; writes `frontend-dapp/.env.local`, `indexer/.env`) |
| Indexer | `cd indexer && cargo run --release` (port **3001**; auto-migrates Postgres) |
| Frontend | `make dev` (port **5173**; requires deploy env files) |

Convenience: `make dev-full` starts infra → deploy → indexer + frontend (indexer runs in background).

**Ports:** 26657 RPC, 1317 LCD, 5432 Postgres, 3001 indexer, 5173 Vite.

### Rust / Docker gotchas

- **`make build-optimized`** runs the CosmWasm optimizer in Docker and may create `smartcontracts/target` owned by root. If `make test-contracts` fails with permission denied: `sudo chown -R $(whoami) smartcontracts/target`.
- **Indexer tmux sessions** must use `export PATH="/usr/local/cargo/bin:$PATH"` so Cargo 1.96+ is used (older system Cargo cannot parse edition-2024 deps).

### Lint and test (no chain required)

From repo root (see [README.md](README.md) and [docs/testing.md](docs/testing.md)):

| Check | Command |
|-------|---------|
| Contracts | `make test-contracts`, `make lint-contracts` |
| Frontend | `make test-frontend`, `make lint-frontend` |
| Indexer lib | `cd indexer && cargo test --lib` |
| Docs drift | `python3 scripts/check_fee_discount_tier_docs.py` |

Frontend unit tests need Node **24** on `PATH`. Indexer integration tests need Postgres + `indexer/.env` — Cloud Agent: `make setup-indexer-postgres` (Postgres-only); full stack: [skills/AGENTS_LOCAL_POSTGRES_DEV.md](skills/AGENTS_LOCAL_POSTGRES_DEV.md).

### Browser / E2E on Cloud Agent

One-time browser wallet setup (replaces manual `cp` into `~/.config/google-chrome/Default/Extensions/…`):

```bash
./scripts/setup-browser-cloud-agent.sh
```

Launch Chrome with container flags:

```bash
google-chrome --no-sandbox --disable-dev-shm-usage --disable-gpu http://127.0.0.1:5173
```

Use **Keplr (extension)** for wallet QA on LocalTerra, or **Simulated Wallet** (dev mnemonic in `frontend-dapp/.env.development` after `make deploy-local`) for on-chain swaps without an extension. Playwright E2E uses the simulated wallet, not Keplr.

### Related playbooks

- [skills/AGENTS_LOCAL_POSTGRES_DEV.md](skills/AGENTS_LOCAL_POSTGRES_DEV.md) — Postgres URLs, bootstrap, indexer integration tests
- [skills/AGENTS_E2E_STRICT_CHAIN.md](skills/AGENTS_E2E_STRICT_CHAIN.md) — Playwright strict on-chain E2E
- [skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md](skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md) — single execution-aligned **Route** row on Swap and `/trade` market quote (#158, #302)
