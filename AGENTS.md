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
make verify-issue-515                    # #515 ticker-scoped external oracle (ustc/lunc/vfdusd catalog + routes)
make verify-issue-550                    # #550 /protocol global USD stats + unified oracle card
make verify-issue-522                    # #522 pair Price (USD) human scale + oracle conversion
make verify-issue-551                    # #551 portfolio/trader P&L human scale + USD totals
make verify-issue-556                    # #556 DEX hub USD (cUSTC/UST1/USTR) + Protocol DEX card
make verify-issue-560                    # #560 portfolio/trader realized P&L USD from hub prices
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
- [skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md](skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md) — WalletConnect same-device mobile pairing (deep-link + copy, not QR-only; **WC-M1–WC-M12**, [#519](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/519) / [#554](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/554) / [#566](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/566)); `make verify-issue-519` · `make verify-issue-554` · `make verify-issue-566`
- [skills/AGENTS_FRONTEND_KEPLR_LEDGER.md](skills/AGENTS_FRONTEND_KEPLR_LEDGER.md) — Keplr + Ledger Nano signing (amino, pre-sign suggest, stall UX) (**K567-1–K567-8**, [#567](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/567)); `make verify-issue-567`
- [skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md](skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md) — single execution-aligned **Route** row on Swap and `/trade` market quote (#158, #302); Trade market GET `/route/solve` default (#501)
- [skills/AGENTS_HYBRID_QUOTING.md](skills/AGENTS_HYBRID_QUOTING.md) — hybrid quote = execute; Swap + Trade market share `quoteCw20ViaRouteSolve` (#418, #501); regression `make verify-issue-501`
- [skills/AGENTS_FRONTEND_DEFAULT_SLIPPAGE.md](skills/AGENTS_FRONTEND_DEFAULT_SLIPPAGE.md) — default **Slippage protection** 5% + shared presets (#497)
- [skills/AGENTS_FRONTEND_SLIPPAGE_PRESET_ALIGN.md](skills/AGENTS_FRONTEND_SLIPPAGE_PRESET_ALIGN.md) — Trade Market + Swap Settings 0.5/1/5% chips stay one aligned group ([#528](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/528))
- [skills/AGENTS_FRONTEND_SWAP_QUOTE_REFETCH.md](skills/AGENTS_FRONTEND_SWAP_QUOTE_REFETCH.md) — sim `refetchInterval` guard + receive Calculating UX for slow multihop quotes (#484); clear/load You Receive on pay amount/token change (#496)
- [skills/AGENTS_INDEXER_ROUTE_SOLVE_PROGRESS.md](skills/AGENTS_INDEXER_ROUTE_SOLVE_PROGRESS.md) — distant-pair latency + `route/solve/progress` poll (#485)
- [skills/AGENTS_INDEXER_EXTERNAL_ORACLE.md](skills/AGENTS_INDEXER_EXTERNAL_ORACLE.md) — ticker-scoped USTC/LUNC/vFDUSD external USD feeds (**X1–X6**, [#515](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/515) / [#550](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/550)); `make verify-issue-515`
- [skills/AGENTS_FRONTEND_PROTOCOL_STATS.md](skills/AGENTS_FRONTEND_PROTOCOL_STATS.md) — `/protocol` global USD stats + unified oracle card (**P550-1–P550-12**, [#550](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/550)); `make verify-issue-550`
- [skills/AGENTS_INDEXER_PAIR_PRICE_USD.md](skills/AGENTS_INDEXER_PAIR_PRICE_USD.md) — pair tape/candles human quote-per-base + USD of 1 human base (**P522-1–P522-5**, [#522](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/522)); `make verify-issue-522`
- [skills/AGENTS_INDEXER_HUB_USD.md](skills/AGENTS_INDEXER_HUB_USD.md) — DEX hub USD for cUSTC/UST1/USTR from largest-liquidity pools (**H1–H10**, [#556](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/556)); `make verify-issue-556`
- [skills/AGENTS_FRONTEND_PORTFOLIO_PNL.md](skills/AGENTS_FRONTEND_PORTFOLIO_PNL.md) — `/portfolio` + `/trader` human-scale P&amp;L / cost / avg entry; mixed totals omitted or USD (**P551-1–P551-6**, [#551](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/551)); `make verify-issue-551`
- [skills/AGENTS_FRONTEND_HUB_PNL.md](skills/AGENTS_FRONTEND_HUB_PNL.md) — `/portfolio` + `/trader` realized P&amp;L USD from `GET /api/v1/hub-prices` (**P560-1–P560-6**, [#560](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/560)); `make verify-issue-560`
- [`AGENTS_FRONTEND_CHARTS_OVERVIEW.md`](skills/AGENTS_FRONTEND_CHARTS_OVERVIEW.md) — `/charts` overview 24h volume USD-only + catalog `volume_usd` ingest (**C1–C9**, [#548](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/548)); `make verify-issue-548`
- [skills/AGENTS_FRONTEND_TRADER_VOLUME_USD.md](skills/AGENTS_FRONTEND_TRADER_VOLUME_USD.md) — `/charts` trader leaderboard + profile **Total Volume (USD)** from `total_volume_usd` (**T553-1–T553-6**, [#553](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/553)); `make verify-issue-553`
- [skills/AGENTS_FRONTEND_TRADE_PAIR_INVERT.md](skills/AGENTS_FRONTEND_TRADE_PAIR_INVERT.md) — `/trade` + `/charts` UI invert for UST1-as-base pairs (other-side Price USD, pill, convert-on-submit) (**T524-1–T524-11**, [#524](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/524)); `make verify-issue-524`
- [skills/AGENTS_FRONTEND_USD_CANDLE_INVERT.md](skills/AGENTS_FRONTEND_USD_CANDLE_INVERT.md) — Price (USD) candles must use `invertUsd` (not `1/x`) + adaptive Y-axis (**C543-1–C543-9**, [#543](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/543)); `make verify-issue-543`
- [skills/AGENTS_FRONTEND_TOKEN_IDENTITY.md](skills/AGENTS_FRONTEND_TOKEN_IDENTITY.md) — compact copy + explorer for pair legs + pair contract on `/pool`, `/trade`, `/charts` (**T541-1–T541-8**, [#541](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/541)); `make verify-issue-541`
- [skills/AGENTS_LIMIT_PRICE_DECIMALS.md](skills/AGENTS_LIMIT_PRICE_DECIMALS.md) — limit-order price band is **human-scale** `raw × 10^(dec0 − dec1)` so UST1/USTR can place (**L20** / [#529](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/529)); `make verify-issue-529`
- [skills/AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md](skills/AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md) — `/trade` Place limit / Market CTA docks to ticket bottom; Chrome sticky mid-form float (**T527-1–T527-10**, [#527](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/527)); `make verify-issue-527`
- [skills/AGENTS_FRONTEND_LIMIT_CANCEL_OPEN.md](skills/AGENTS_FRONTEND_LIMIT_CANCEL_OPEN.md) — My open limits Cancel vs stale `●` row / fill lifecycle / `/trade` reachability (**F530-1–F530-8**, [#530](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/530)); `make verify-issue-530`
- [skills/AGENTS_FRONTEND_POOL_TABLE.md](skills/AGENTS_FRONTEND_POOL_TABLE.md) — `/pool` sortable table, UST1-first catalog default, Charts `/charts/:pairAddr` (**P547-1–P547-10**, [#547](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/547)); `make verify-issue-547`
- [skills/AGENTS_FRONTEND_POOL_LP_HOWTO.md](skills/AGENTS_FRONTEND_POOL_LP_HOWTO.md) — retail how-to for LUNC v2 LP + maker-limit disambiguation; no incentive program ([#531](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/531))
- [skills/AGENTS_FRONTEND_POOL_ONE_SIDED.md](skills/AGENTS_FRONTEND_POOL_ONE_SIDED.md) — one-sided pool add/withdraw (auto zap + wrap; **Z533-1–Z533-10**, [#533](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/533)); `make verify-issue-533`
- [skills/AGENTS_FRONTEND_POOL_ZAP_FLOORS.md](skills/AGENTS_FRONTEND_POOL_ZAP_FLOORS.md) — zap-in/out execution follows floors, not optimistic quotes (**Z559-1–Z559-4**, [#559](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/559)); `make verify-issue-559`
- [skills/AGENTS_FRONTEND_PAIR_CATALOG_RANK.md](skills/AGENTS_FRONTEND_PAIR_CATALOG_RANK.md) — pair/token pickers list economic markets first, gems last; human quote volume badges (**P534-1–P534-8**, [#534](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/534)); `make verify-issue-534`
- [skills/AGENTS_FRONTEND_CREATE_PAIR_PICKER.md](skills/AGENTS_FRONTEND_CREATE_PAIR_PICKER.md) — `/create` listed-CW20 picker + custom paste; not Swap’s factory universe (**C542-1–C542-11**, [#542](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/542)); `make verify-issue-542`
- [skills/AGENTS_LIMIT_ORDER_BATCH_LADDER.md](skills/AGENTS_LIMIT_ORDER_BATCH_LADDER.md) — batch/ladder place UI + gas gates; Ladder create must render when disconnected (#494)
- [skills/AGENTS_UST1_SECONDARY_AMM.md](skills/AGENTS_UST1_SECONDARY_AMM.md) — UST1 secondary AMM create/seed or Path B waiver ([#508](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/508), invariants **U1–U7**); `make verify-issue-508`
- [skills/AGENTS_FEE_DISCOUNT_TIERS.md](skills/AGENTS_FEE_DISCOUNT_TIERS.md) — CL8Y tier ladder + **I13** limit-placement discount shift (tier 9 place = 0; swap/take unchanged) + [`scripts/upgrade-514-limit-discount.sh`](scripts/upgrade-514-limit-discount.sh) ([#514](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/514)); `make verify-issue-514`
- [skills/AGENTS_FACTORY_DISCOUNT_REGISTRY.md](skills/AGENTS_FACTORY_DISCOUNT_REGISTRY.md) — factory `config.discount_registry` snapshot on `CreatePair` so new pairs are wired (**F5** / **I14**, [#536](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/536)); LocalTerra inherit + dApp `GetDiscountRegistry` first (**F538-1–F538-3**, [#538](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/538)); `make verify-issue-536` / `make verify-issue-538`
- [skills/AGENTS_FRONTEND_PAIR_FEE_DISCOUNT.md](skills/AGENTS_FRONTEND_PAIR_FEE_DISCOUNT.md) — dApp fee-tier chrome gated on pair `DISCOUNT_REGISTRY` (**I14**, [#537](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/537)); `make verify-issue-537`
- [skills/AGENTS_LP_SYMBOL_DIGITS.md](skills/AGENTS_LP_SYMBOL_DIGITS.md) — LP ticker keeps `0-9`, strips non-alnum; factory `UpdateConfig` code IDs + [`scripts/upgrade-518-lp-symbol.sh`](scripts/upgrade-518-lp-symbol.sh) (**F3**, [#518](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/518)); `make verify-issue-518`
- [skills/AGENTS_ROTATE_FEE_TREASURY.md](skills/AGENTS_ROTATE_FEE_TREASURY.md) — swap/book fees to ustr-cmm CMM (`terra16j5u6…`); factory `SetPairTreasury*` + [`scripts/rotate-fee-treasury.sh`](scripts/rotate-fee-treasury.sh) (**F4**)
- [skills/AGENTS_REBALANCE_MINT_UST1_LP.md](skills/AGENTS_REBALANCE_MINT_UST1_LP.md) — UST1/cUSTC oracle rebalance + $1k LP on UST1/cUSTC and UST1/USTR to CMM ([`scripts/rebalance-mint-ust1-lp.sh`](scripts/rebalance-mint-ust1-lp.sh))
- [skills/AGENTS_EXPIRED_LIMIT_PARK_REASON.md](skills/AGENTS_EXPIRED_LIMIT_PARK_REASON.md) — park `reason` discriminator so bots do not treat dust-filled parks as unfilled expiry ([#504](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/504), invariant **L22**)
- [skills/AGENTS_ORDER_STATUS_QUERY.md](skills/AGENTS_ORDER_STATUS_QUERY.md) — on-chain typed `OrderStatus` for vaults/bots ([#505](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/505), invariant **L21**); `ParkedRefund` ≠ park reason
- [skills/AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md](skills/AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md) — maker UX for indexer `parked_expired` + `ClaimExpiredLimitOrder` ([#141](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/141); pause blocks claim per [#120](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/120) / **L6**)
- [skills/AGENTS_UST1_WINDOW_UI.md](skills/AGENTS_UST1_WINDOW_UI.md) — `/ust1` oracle vFDUSD↔UST1 mint/redeem (CW20 Send + `effective_swap`); not faucet Mint ([#506](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/506)); `make verify-issue-506`
- [skills/AGENTS_MAINNET_WRAP_ENABLEMENT.md](skills/AGENTS_MAINNET_WRAP_ENABLEMENT.md) — post-SL5 Coolify wrap env + cLUNC/cUSTC fee UX ([#507](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/507))
- [skills/AGENTS_WRAP_UNWRAP_BURN_TAX.md](skills/AGENTS_WRAP_UNWRAP_BURN_TAX.md) — unwrap InstantWithdraw burn tax quotes + exchange-deposit warning (**W8–W11**, [#512](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/512)); `make verify-issue-512`
- [skills/AGENTS_WRAP_MAPPER_SPLIT_FEES.md](skills/AGENTS_WRAP_MAPPER_SPLIT_FEES.md) — wrap-mapper `fee_wrap_bps` / `fee_unwrap_bps` + unwrap ≈2% all-in retune (**W12–W15**, [#516](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/516)); `make verify-issue-516`. Router `unwrap_output` dual-read ([#523](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/523)); `make verify-issue-523`. LocalTerra wrap-mapper instantiate + #533 P4–P8 ([#539](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/539)); `make verify-issue-539`
- [skills/AGENTS_UST1_WRAP_PRODUCTION_OPS.md](skills/AGENTS_UST1_WRAP_PRODUCTION_OPS.md) — Phase 5 UST1/wrap ops hardening: registry, health probes, pause playbooks (invariants **O1–O8**, [#503](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/503)); `make verify-issue-503`
