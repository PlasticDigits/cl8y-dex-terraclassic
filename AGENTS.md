# AGENTS.md

Guidance for AI coding agents working in this repository.

## Git commits

### Commit identity (who signs the commit)

**Do not commit as an AI or automation product account** (Cursor, Claude, Codex, Copilot, or similar agent/bot identities). Commits must use a **human dev identity**:

| Environment | Use |
|-------------|-----|
| **Local dev machine** | The developer’s normal `git config user.name` / `user.email` for that checkout |
| **Cloud Agent VM** | **`GIT_USERNAME` / `GIT_EMAIL`** (Cursor Cloud Agent secrets) — applied automatically by `scripts/setup-cloud-agent-env.sh` on VM startup (see `.cursor/environment.json`). Do **not** commit as the GitLab project-bot clone identity (`project_*_bot_*@noreply.gitlab.com`) or any AI vendor login. |

Before committing, verify:

```bash
git config user.name
git config user.email
```

If either shows an AI vendor name, `noreply` agent email, or a Cursor/Claude/Codex-style identity, **stop** and set the correct identity for this repo (local dev config, or the token owner on Cloud Agent). Do not amend history to add AI co-attribution.

### Commit message content

**Do not put emails, co-authors, or author attribution in commit messages.** The message body must not contain:

- Email addresses (including `name@domain` in trailers)
- `Co-authored-by`, `Signed-off-by: Author`, or similar attribution lines
- The word **author** (e.g. “original author”, “Co-authored-by”)

Write only the subject and a short technical description of the change. Hooks in `.githooks/` enforce this:

- **`prepare-commit-msg`** — strips agent-injected `Co-authored-by` / email lines from the body before the commit is recorded.
- **`commit-msg`** — rejects any remaining violations.
- **`pre-push`** — blocks pushes that include commits whose bodies still violate the policy (catches `git commit --no-verify`).

**Never use `git commit --no-verify` or `git push --no-verify`** to skip these hooks. Cursor may append `Co-authored-by` trailers automatically; the hooks remove them — do not re-add them manually.

Enable hooks locally: `make setup-hooks` (Cloud Agent: `scripts/setup-cloud-agent-env.sh` sets `core.hooksPath` on VM startup).

## Cursor Cloud specific instructions

### Cloud Agent startup (GitLab + git identity)

**`.cursor/environment.json`** runs `scripts/setup-cloud-agent-env.sh` on **every VM boot** (after git pull). This script is idempotent and **must succeed** — it:

1. Requires **`GITLAB_TOKEN`**, **`GIT_USERNAME`**, and **`GIT_EMAIL`** (configure in Cursor Cloud Agent secrets).
2. Validates **`GIT_USERNAME` / `GIT_EMAIL`** are set and not a bot/service identity, then sets **`git config user.name` / `user.email`** from them (global + repo), overwriting the default GitLab project-bot clone identity. Writes **`.env.git`** for shell sessions.
3. Runs **`scripts/setup-glab-cloud-agent.sh`** (installs `glab`, authenticates, writes `.env.glab` with `GITLAB_REPO`).
4. Installs a **`~/.bashrc` hook** that sources `scripts/cloud-agent-shell-init.sh` so new shells keep the identity and `GITLAB_REPO`.
5. Runs **`scripts/setup-cloud-agent-toolchain.sh`** (Docker CE, Node from `.nvmrc`, `libssl-dev`).
6. Runs **`scripts/setup-browser-cloud-agent.sh`** (Chrome + Keplr).
7. Runs **`scripts/setup-cloud-agent-localterra.sh`** (LocalTerra, deploy, Playwright chromium, indexer tmux).

**Verify after startup:**

```bash
test -n "$GIT_USERNAME" && test -n "$GIT_EMAIL"
git config user.name    # matches $GIT_USERNAME
git config user.email   # matches $GIT_EMAIL
command -v glab
test -n "$GITLAB_TOKEN"
source .env.glab && echo "$GITLAB_REPO"
glab api "projects/PlasticDigits%2Fcl8y-dex-terraclassic" >/dev/null && echo OK
```

**Manual re-run** (if startup failed or identity drifted): `./scripts/setup-cloud-agent-env.sh`

Regression: `make test-setup-cloud-agent-env`

### LocalTerra — do not skip chain work

**Cloud Agent VMs can run LocalTerra in Docker.** Do **not** report `SKIP (no LocalTerra in agent VM)` without provisioning first.

| Mistake | Why it fails | Fix |
|---------|--------------|-----|
| `docker compose ps` without `sg docker` | Permission denied on docker.sock | `sg docker -c 'docker compose ps'` or `make status` |
| `curl http://127.0.0.1:1317` hangs | Host userland-proxy quirk on Linux | `make has-localterra` or `make wait-localterra` (exec fallback) |
| Only ran `make setup-indexer-postgres` | Postgres-only path — **no chain deploy** | `make setup-cloud-localterra` for swap/E2E/frontend |

**Probe:** `make has-localterra` (exit 0 = chain up). **Provision:** `make setup-cloud-localterra` (~10–15 min first run).

### VM prerequisites (one-time per snapshot)

These are **not** in the startup update script; install once when provisioning a new Cloud Agent VM:

- **Docker**: Cloud Agent VMs need Docker CE with `fuse-overlayfs` storage driver and `iptables-legacy`. Start `dockerd` manually if systemd does not (see below).
- **Rust**: Use `rustup default stable` (1.96+ as of 2026). Indexer build needs `libssl-dev` and `pkg-config`.
- **Node 24**: `nvm install` from repo `.nvmrc`. Cloud VMs may ship `/exec-daemon/node` (v22) **before** nvm on `PATH` — prepend the nvm bin dir or `hash -r` after `nvm use`, or `scripts/with-node.sh` may run the wrong Node.
- **Docker access**: `sudo usermod -aG docker $USER` then use `sg docker -c '…'` in non-login shells.
- **GitLab CLI (`glab`)**: Provisioned by **`scripts/setup-cloud-agent-env.sh`** on VM startup (also `./scripts/setup-glab-cloud-agent.sh` standalone). Writes `.env.glab` with `GITLAB_REPO`. Cloud Agent git remotes use `https://x-access-token:…@gitlab.com/PlasticDigits/<repo>.git`, which breaks `glab repo view` unless `GITLAB_REPO` is set — `source .env.glab` or re-run setup. Verify: `glab api "projects/PlasticDigits%2F<repo>"` (encode `/` as `%2F`).
- **Chrome + Keplr**: Provisioned by **`scripts/setup-cloud-agent-env.sh`** on VM startup (also `./scripts/setup-browser-cloud-agent.sh` standalone). Installs **google-chrome-stable** when absent, downloads Keplr from the Chrome Web Store, and registers it under `~/.config/google-chrome/Default/Extensions/`. Idempotent — safe to re-run. Regression: `make test-setup-browser`.

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

**Node 24:** After `nvm use`, prepend `$(nvm which node | xargs dirname)` to `PATH` so `scripts/with-node.sh` and Playwright do not pick `/exec-daemon/node` (v22). **Always start Vite via `make dev` or `bash scripts/dev-frontend-local.sh`** (they wrap `with-node.sh`). A Vite process started with `/exec-daemon/node` v22 can make Chrome show **“Aw, Snap!”** on `http://127.0.0.1:5173` — this is often misreported as a Keplr/wallet bug. Fix: kill stale Vite, restart with `make dev`.

**Frontend + manual QA:**

```bash
export PATH="$HOME/.nvm/versions/node/$(cat .nvmrc)/bin:$PATH"
make dev   # http://127.0.0.1:5173
google-chrome --no-sandbox --disable-dev-shm-usage --disable-gpu http://127.0.0.1:5173/limits
```

**Strict E2E (#292):** after `make deploy-local`, `(cd indexer && cargo build --release)` if needed, then `bash scripts/e2e-start-indexer.sh`, then `sg docker -c 'CI=1 make test-e2e'` — see **LT11** / **LT12** in [`docs/localterra-sdk53.md`](docs/localterra-sdk53.md). Install Playwright via `bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright install chromium` (not bare `npx playwright`). Do not chain `make test-e2e-tx` after an earlier deploy on the same volumes without resetting the indexer DB.

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
make verify-issue-485                    # #485 graph cache + distant TTL + progress poll + frontend helpers
make verify-issue-515                    # #515 ticker-scoped external oracle (ustc/lunc catalog + routes)
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

**Keplr agent pitfalls:** (1) Extension missing until `./scripts/setup-browser-cloud-agent.sh` runs (`make test-setup-browser`). (2) **Do not use Terra Station on LocalTerra** for fee/signing QA — use Keplr or Simulated Wallet ([#235](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/235)). (3) Blank/crashed Chrome on the dApp URL is usually **wrong Node for Vite** (see Node 24 note above), not Keplr itself.

### Related playbooks

- [skills/AGENTS_LOCAL_POSTGRES_DEV.md](skills/AGENTS_LOCAL_POSTGRES_DEV.md) — Postgres URLs, bootstrap, indexer integration tests
- [skills/AGENTS_E2E_STRICT_CHAIN.md](skills/AGENTS_E2E_STRICT_CHAIN.md) — Playwright strict on-chain E2E
- [skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md](skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md) — QuickSwap-inspired blue + gold tokens/primitives (#488); spec [`docs/design-system.md`](docs/design-system.md)
- [skills/AGENTS_FRONTEND_THEME_TOGGLE.md](skills/AGENTS_FRONTEND_THEME_TOGGLE.md) — dark/light header toggle + bootstrap FOUC notes (#488)
- [skills/AGENTS_FRONTEND_SOUND_MUTE.md](skills/AGENTS_FRONTEND_SOUND_MUTE.md) — UI SFX mute toggle + `cl8y-dex-sounds-enabled` persistence (#487)
- [skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md](skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) — anti-cognitive-overload retail copy + terminology (#489); glossary in [`docs/design-system.md`](docs/design-system.md)
- [skills/AGENTS_FRONTEND_RISK_DISCLAIMERS.md](skills/AGENTS_FRONTEND_RISK_DISCLAIMERS.md) — first-visit NFA / risk modal (#138)
- [skills/AGENTS_FRONTEND_CLICKWRAP.md](skills/AGENTS_FRONTEND_CLICKWRAP.md) — connected Legal TermsGate for `dex.cl8y.com` (**C1–C10**, [#517](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/517)); `make verify-issue-517`
- [skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md](skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md) — single execution-aligned **Route** row on Swap and `/trade` market quote (#158, #302); Trade market GET `/route/solve` default (#501)
- [skills/AGENTS_HYBRID_QUOTING.md](skills/AGENTS_HYBRID_QUOTING.md) — hybrid quote = execute; Swap + Trade market share `quoteCw20ViaRouteSolve` (#418, #501); regression `make verify-issue-501`
- [skills/AGENTS_FRONTEND_DEFAULT_SLIPPAGE.md](skills/AGENTS_FRONTEND_DEFAULT_SLIPPAGE.md) — default **Slippage protection** 5% + shared presets (#497)
- [skills/AGENTS_FRONTEND_SWAP_QUOTE_REFETCH.md](skills/AGENTS_FRONTEND_SWAP_QUOTE_REFETCH.md) — sim `refetchInterval` guard + receive Calculating UX for slow multihop quotes (#484); clear/load You Receive on pay amount/token change (#496)
- [skills/AGENTS_INDEXER_ROUTE_SOLVE_PROGRESS.md](skills/AGENTS_INDEXER_ROUTE_SOLVE_PROGRESS.md) — distant-pair latency + `route/solve/progress` poll (#485)
- [skills/AGENTS_INDEXER_EXTERNAL_ORACLE.md](skills/AGENTS_INDEXER_EXTERNAL_ORACLE.md) — ticker-scoped USTC/LUNC external USD feeds (**X1–X6**, [#515](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/515)); `make verify-issue-515`
- [skills/AGENTS_LIMIT_ORDER_BATCH_LADDER.md](skills/AGENTS_LIMIT_ORDER_BATCH_LADDER.md) — batch/ladder place UI + gas gates; Ladder create must render when disconnected (#494)
- [skills/AGENTS_UST1_SECONDARY_AMM.md](skills/AGENTS_UST1_SECONDARY_AMM.md) — UST1 secondary AMM create/seed or Path B waiver ([#508](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/508), invariants **U1–U7**); `make verify-issue-508`
- [skills/AGENTS_LP_SYMBOL_DIGITS.md](skills/AGENTS_LP_SYMBOL_DIGITS.md) — LP ticker keeps `0-9`, strips non-alnum; factory `UpdateConfig` code IDs + [`scripts/upgrade-518-lp-symbol.sh`](scripts/upgrade-518-lp-symbol.sh) (**F3**, [#518](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/518)); `make verify-issue-518`
- [skills/AGENTS_EXPIRED_LIMIT_PARK_REASON.md](skills/AGENTS_EXPIRED_LIMIT_PARK_REASON.md) — park `reason` discriminator so bots do not treat dust-filled parks as unfilled expiry ([#504](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/504), invariant **L22**)
- [skills/AGENTS_ORDER_STATUS_QUERY.md](skills/AGENTS_ORDER_STATUS_QUERY.md) — on-chain typed `OrderStatus` for vaults/bots ([#505](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/505), invariant **L21**); `ParkedRefund` ≠ park reason
- [skills/AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md](skills/AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md) — maker UX for indexer `parked_expired` + `ClaimExpiredLimitOrder` ([#141](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/141); pause blocks claim per [#120](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/120) / **L6**)
- [skills/AGENTS_UST1_WINDOW_UI.md](skills/AGENTS_UST1_WINDOW_UI.md) — `/ust1` oracle vFDUSD↔UST1 mint/redeem (CW20 Send + `effective_swap`); not faucet Mint ([#506](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/506)); `make verify-issue-506`
- [skills/AGENTS_MAINNET_WRAP_ENABLEMENT.md](skills/AGENTS_MAINNET_WRAP_ENABLEMENT.md) — post-SL5 Coolify wrap env + cLUNC/cUSTC fee_bps UX ([#507](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/507))
- [skills/AGENTS_WRAP_UNWRAP_BURN_TAX.md](skills/AGENTS_WRAP_UNWRAP_BURN_TAX.md) — unwrap InstantWithdraw burn tax quotes + exchange-deposit warning (**W8–W11**, [#512](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/512)); `make verify-issue-512`
- [skills/AGENTS_UST1_WRAP_PRODUCTION_OPS.md](skills/AGENTS_UST1_WRAP_PRODUCTION_OPS.md) — Phase 5 UST1/wrap ops hardening: registry, health probes, pause playbooks (invariants **O1–O8**, [#503](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/503)); `make verify-issue-503`
