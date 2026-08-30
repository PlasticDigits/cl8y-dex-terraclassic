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
make verify-issue-589                    # #589 CW20 code-id audit harness (decomp + suite; gates #581)
make verify-issue-581                    # #581 8266 full A-lcd/B-lt suite + listing residuals
make verify-issue-590                    # #590 post-merge !394–!396 (fees + wrap gas + 8266 A-lcd/B-lt)
make verify-issue-585                    # #585 F6 freeze: route/solve exclude + dApp banners (Postgres)
make verify-issue-587                    # #587 wrap+≥2hop LUNC↔USTR gas + Swap Network fee (Vitest)
make verify-issue-679                    # #679 mixed hybrid+pool router gas + Swap Network fee (Vitest)
make verify-issue-708                    # #708 greedy book-first swap (multitest + gas + docs)
make verify-issue-709                    # #709 greedy query mutex + remainder_to_pool + pool_spot overflow
make verify-issue-710                    # #710 greedy tax / pause / blacklist / AfterSwap L7
make verify-issue-599                    # #599 unwrap+≥2hop USTR→USTC gas combo (Vitest)
make verify-issue-600                    # #600 post-merge !400 LocalTerra E9 + columbus-5 unwrap gas
make verify-issue-595                    # #595 pay-with-any-token invoice module (Vitest + docs)
make verify-issue-630                    # #630 LUNC/USTC picker labels (registry beats indexer uluna/uusd)
make verify-issue-661                    # #661 /pool Manage provide name/symbol + wrap default on
make verify-issue-651                    # #651 /tiers phone-width cards + How it works (Vitest + docs)
make verify-issue-665                    # #665 trader profile Share (Web Share + clipboard)
make verify-issue-674                    # #674 /portfolio hide test-gem Open Positions / P&L (Vitest + docs)
make verify-issue-669                    # #669 /token/create desktop density (Vitest + docs)
make verify-issue-662                    # #662 /pool Created relative age from indexer created_at
make verify-issue-593                    # #593 Create Token + manager console (Vitest + docs)
make verify-issue-626                    # #626 free listed-template adopt + Terraport/GDEX LP gate
make verify-issue-627                    # #627 columbus-5 CW20 code 3 adopt+list NO-GO (REPORT + AdoptLegacyLayout)
make verify-issue-628                    # #628 post-merge !418 community-tax migrate leftovers (P3/P7/P11 + 11626/11630)
make verify-issue-673                    # #673 post-merge !437–!458 leftover verify (children 655–672 + Coolify)
make verify-issue-686                    # #686 post-merge !459–!468 leftover verify (children 674–680, 683 + Coolify)
make verify-issue-698                    # #698 post-merge !474/!475 leftover verify (children 694, 695 + Coolify; #699/#700 dups)
make verify-issue-701                    # #701 post-merge !477 leftover verify (child 692 Vol USD + Coolify)
make verify-issue-702                    # #702 post-merge !476 leftover verify (children 693, 563, 653 + Coolify)
make verify-issue-632                    # #632 Keplr in-app / visualViewport token picker
make verify-issue-672                    # #672 Connect Wallet / Modal dismiss overlay (Vitest + docs)
make verify-issue-634                    # #634 migrate pair inventory + post-adopt CL8Y register tool (LocalTerra: localterra-634-migrate-inventory.sh)
make verify-issue-604                    # #604 identity + connected-wallet helpers (Vitest + crates)
make verify-issue-605                    # #605 SKU init + percent taxes (Vitest + crates)
make verify-issue-594                    # #594 community tax indexer catalog (Postgres + sqlx)
make verify-issue-592                    # #592 community tax CW20 (token + launcher + AutoLP; docs)
make verify-issue-608                    # #608 LaunchGuards per-wallet cooldown + provide after max_wallet (H-3/H-4)
make verify-issue-609                    # #609 ExemptionDirectory skips buy/sell/transfer tax (multitest + hint)
make verify-issue-633                    # #633 autoregister CL8Y pairs + Manage catch-up + manager tax skip (LocalTerra: localterra-633-autoregister.sh)
make verify-issue-610                    # #610 AutoLP factory-listed pair + skim floor (M-2 / M-3)
make verify-issue-601                    # #601 store + 11611 REPORT + factory list + LocalTerra smoke
# columbus-5 rotate (#611): ./scripts/upgrade-611-community-tax.sh (11619/11622/11621; launcher 11622)
make verify-issue-606                    # #606 launcher Enable Feature + SKU dedupe (T606; crates + dApp invoices)
make verify-issue-602                    # #602 post-merge !402 Coolify + 11614 launcher + LocalTerra Create Token
make verify-issue-612                    # #612 post-merge !407/!408 Enable Feature migrate + LocalTerra QA
make verify-issue-616                    # #616 post-merge !409–!413 option-2 / wrap / window / AutoLP / ranking
make verify-issue-620                    # #620 LocalTerra community-tax seed + Transfer funding + indexer env
make verify-issue-624                    # #624 post-merge !414 LocalTerra community-tax seed leftovers
make verify-issue-621                    # #621 tax-aware localnet swarm (gem exclude + tax workers)
make verify-issue-622                    # #622 Playwright e2e-tx community-tax pair (sell extra-debit / buy net)
make verify-issue-623                    # #623 named tax-on Layer B (keep generic B-lt tax-off)
make verify-issue-625                    # #625 post-merge !415–!417 tax swarm / e2e-tx / Layer B leftovers
make verify-issue-607                    # #607 community tax router hops tax the original trader (C-2 option 2)
make verify-issue-615                    # #615 tax-aware route/solve ranking + You Receive net
make verify-issue-515                    # #515 ticker-scoped external oracle (ustc/lunc/vfdusd catalog + routes)
make verify-issue-579                    # #579 CoinGecko User-Agent (403 vs 429; no live CoinGecko)
make verify-issue-580                    # #580 CEX FDUSD identity under path vfdusd (logs/API, not Terra vFDUSD)
make verify-issue-652                    # #652 /protocol inline Δ% + volume prior-window % + UTC-day series
make verify-issue-667                    # #667 /protocol Δ% grouped with headline + integer census
make verify-issue-668                    # #668 /protocol UTC volume chart USD axis + Hourly/Daily/Monthly
make verify-issue-689                    # #689 /protocol UTC chart Volume / Liquidity / Fees metric toggle
make verify-issue-703                    # #703 /protocol Monthly UTC chart phone x-axis (last 12 months + YY-MM)
make verify-issue-677                    # #677 /protocol leftovers: liquidity 24h-only Δ% + denser UTC volume x-axis
make verify-issue-550                    # #550 /protocol global USD stats + unified oracle card
make verify-issue-571                    # #571 /protocol vFDUSD: FDUSD reference + Venus 1 vFDUSD Price
make verify-issue-569                    # #569 /protocol total USD pair liquidity + 24h/30d % change
make verify-issue-655                    # #655 /pool v2 LP USD column + pair_liquidity_usd rollup
make verify-issue-692                    # #692 /pool Vol column + pair-list 24h volume in USD
make verify-issue-586                    # #586 /protocol treasury fees 24h/7d/30d + source/token mix
make verify-issue-683                    # #683 /protocol fee USD for CL8Y + factory-listed economic tokens
make verify-issue-631                    # #631 DeFiLlama UTC-day API + gem/hybrid/fee exclusions
make verify-issue-687                    # #687 DeFiLlama fees headline partial SUM + adapter start / 404
make verify-issue-639                    # #639 listing venue catalog (docs only; no Postgres)
make verify-issue-646                    # #646 GeckoTerminal /gt/ adapters (Postgres)
make verify-issue-684                    # #684 /gt/events post-event reserves (Postgres)
make verify-issue-694                    # #694 API4 per-request caps: /gt/events rows + progress/blacklist LCD-heavy
make verify-issue-685                    # #685 CG/CMC listing field truthfulness (Postgres)
make verify-issue-640                    # #640 Cosmostation CW20 pack (docs only; no Postgres)
make verify-issue-641                    # #641 Hexxagon Galaxy Station CW20 pack (docs only; no Postgres)
make verify-issue-653                    # #653 one chrome layer / anti-nesting (StatBox flat + allowlist)
make verify-issue-693                    # #693 /trade ticket Market default + flatten + token wash
make verify-issue-660                    # #660 /pool Manage four peer actions (provide, withdraw, zap add, zap withdraw)
make verify-issue-659                    # #659 Swap direction seam plate (opaque fill + static occluder)
make verify-issue-663                    # #663 footer official CL8Y Homepage + Bridge product links
make verify-issue-664                    # #664 /trade /charts identity v2 LP USD (stamp + chip)
make verify-issue-657                    # #657 /trader global leaderboard (shared Charts table)
make verify-issue-671                    # #671 connected wallet dropdown icon+label alignment (Vitest + docs)
make verify-issue-666                    # #666 /charts pair-scoped 24h stats + leaderboard
make verify-issue-680                    # #680 /charts UST1/USD hero + ?price= + page-wide invert
make verify-issue-613                    # #613 /protocol Wrap/Unwrap ingest (captured notify_deposit + fee)
make verify-issue-614                    # #614 /protocol UST1 window mint/redeem treasury fees
make verify-issue-577                    # #577 token/trader/pair/global 24h rollup decay + stale overview
make verify-issue-522                    # #522 pair Price (USD) human scale + oracle conversion
make verify-issue-551                    # #551 portfolio/trader P&L human scale + USD totals
make verify-issue-676                    # #676 /positions 18-dec trade_count + NUMERIC(78,18)
make verify-issue-557                    # #557 human tape / wallet Amount in/out/Price
make verify-issue-556                    # #556 DEX hub USD (cUSTC/UST1/USTR) + Protocol DEX card
make verify-issue-570                    # #570 Protocol hub cUSTC wrap link + LUNC/USD column
make verify-issue-568                    # #568 time-stamped candle USD + idle mark-to-market
make verify-issue-560                    # #560 portfolio/trader realized P&L USD from hub prices
make verify-issue-675                    # #675 portfolio/trader unrealized P&L (hub mark vs on-DEX cost)
make verify-issue-573                    # #573 post-merge stack !368–!377 (children 557–567)
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

- **`make build-optimized`** runs the CosmWasm optimizer in Docker and may create `smartcontracts/target` owned by root. If `make test-contracts` fails with permission denied: `sudo chown -R $(whoami) smartcontracts/target`. The optimizer overlays `/code/target` with a named volume (`smartcontracts/scripts/optimize.sh`); do not drop that overlay.
- **Never bind-mount `indexer/` (or the repo) into a root Docker container and run `cargo`.** Default uid is 0, so Cargo writes `indexer/target/debug/.cargo-build-lock` (and `deps/`) as root. Host `cargo test`, rust-analyzer, and `make verify-issue-*` then fail with permission denied. This is the usual cause of “root-owned indexer/target lock files” — not `docker/indexer/Dockerfile` (that `COPY`s; it does not write back). Host Postgres/LCD TCP hang (userland-proxy, VPN) is **not** a reason to move cargo into Docker; use `make setup-indexer-postgres`, [`scripts/lib/postgres-psql.sh`](scripts/lib/postgres-psql.sh), or [`scripts/lib/localterra-host-curl.sh`](scripts/lib/localterra-host-curl.sh). If cargo in Docker is unavoidable: `--user $(id -u):$(id -g)` and `CARGO_HOME=/tmp/cargo` + `CARGO_TARGET_DIR=/tmp/target` (or a named volume). See [`scripts/lib/docker-indexer-bind-mount.sh`](scripts/lib/docker-indexer-bind-mount.sh). One-time cleanup: `sudo chown -R $(whoami) indexer/target`. Static check: `make test-indexer-target-ownership`.
- **Indexer tmux sessions** must use `export PATH="/usr/local/cargo/bin:$PATH"` so Cargo 1.96+ is used (older system Cargo cannot parse edition-2024 deps).

### Lint and test (no chain required)

From repo root (see [README.md](README.md) and [docs/testing.md](docs/testing.md)):

| Check | Command |
|-------|---------|
| Contracts | `make test-contracts`, `make lint-contracts` |
| Frontend | `make test-frontend`, `make lint-frontend` |
| Swap/Trade acquire guidance | `make verify-issue-678` |
| One chrome layer / anti-nesting | `make verify-issue-653` |
| Trade ticket Market default + flatten | `make verify-issue-693` |
| Pool Manage four peer actions | `make verify-issue-660` |
| Swap direction seam plate | `make verify-issue-659` |
| Official CL8Y product links (footer) | `make verify-issue-663` |
| Production `VITE_DEV_MODE` reject | `make verify-issue-695` |
| Stale lazy-chunk reload (Coolify) | `make verify-issue-706` |
| Post-merge !474/!475 leftover | `make verify-issue-698` |
| Post-merge !477 leftover (`/pool` Vol USD) | `make verify-issue-701` |
| Post-merge !476 leftover (`/trade` flatten) | `make verify-issue-702` |
| Pool Manage provide labels | `make verify-issue-661` |
| Trade / Charts v2 LP USD | `make verify-issue-664` |
| Trader page global leaderboard | `make verify-issue-657` |
| Charts pair-scoped 24h + board | `make verify-issue-666` |
| Charts UST1/USD hero + `?price=` | `make verify-issue-680` |
| Keplr CW20 pack | `make verify-issue-629` |
| Listing venue catalog | `make verify-issue-639` |
| GeckoTerminal `/gt/` | `make verify-issue-646` |
| API4 per-request caps | `make verify-issue-694` |
| GeckoTerminal `/gt/events` post-event reserves | `make verify-issue-684` |
| CG/CMC listing field truth | `make verify-issue-685` |
| Cosmostation CW20 pack | `make verify-issue-640` |
| Hexxagon CW20 pack | `make verify-issue-641` |
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

- [skills/AGENTS_LOCAL_POSTGRES_DEV.md](skills/AGENTS_LOCAL_POSTGRES_DEV.md) — Postgres URLs, bootstrap, indexer integration tests; never bind-mount `indexer/` for cargo (`make test-indexer-target-ownership`)
- [skills/AGENTS_E2E_STRICT_CHAIN.md](skills/AGENTS_E2E_STRICT_CHAIN.md) — Playwright strict on-chain E2E
- [skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md](skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md) — QuickSwap-inspired blue + gold tokens/primitives (#488); spec [`docs/design-system.md`](docs/design-system.md)
- [skills/AGENTS_FRONTEND_CHROME_NESTING.md](skills/AGENTS_FRONTEND_CHROME_NESTING.md) — one chrome layer / anti-nesting: no metric-grid `card-glass` inside `shell-panel*` (**C653-1–C653-8**, [#653](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/653)); `make verify-issue-653`
- [skills/AGENTS_FRONTEND_SWAP_DIRECTION_SEAM.md](skills/AGENTS_FRONTEND_SWAP_DIRECTION_SEAM.md) — Swap Pay/Receive flip plate occludes the seam hairline (**S659-1–S659-8**, [#659](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/659)); `make verify-issue-659`
- [skills/AGENTS_FRONTEND_PRODUCT_LINKS.md](skills/AGENTS_FRONTEND_PRODUCT_LINKS.md) — footer official CL8Y Homepage + Bridge HTTPS allowlist (**P663-1–P663-8**, [#663](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/663)); `make verify-issue-663`
- [skills/AGENTS_FRONTEND_WALLET_CHIP.md](skills/AGENTS_FRONTEND_WALLET_CHIP.md) — connected wallet chip + dropdown row layout (**W671-1–W671-8**, [#671](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/671)); `make verify-issue-671`
- [skills/AGENTS_FRONTEND_OPENGRAPH.md](skills/AGENTS_FRONTEND_OPENGRAPH.md) — static Open Graph / Twitter cards, community medallion `/og-image.png`, allowlisted absolute URLs (**OG-1–OG-8**, [#578](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/578)); `make verify-issue-578`
- [skills/AGENTS_FRONTEND_DEV_MODE_GUARD.md](skills/AGENTS_FRONTEND_DEV_MODE_GUARD.md) — production `vite build` rejects `VITE_DEV_MODE=true` (**D695-1–D695-8**, [#695](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/695) / FE-01); `make verify-issue-695`
- [skills/AGENTS_FRONTEND_LAZY_CHUNK_LOAD.md](skills/AGENTS_FRONTEND_LAZY_CHUNK_LOAD.md) — offline Try Again (#172) + stale Coolify hashed-chunk one-shot reload (**L706-1–L706-8**, [#706](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/706)); `make verify-issue-706`
- [skills/AGENTS_FRONTEND_THEME_TOGGLE.md](skills/AGENTS_FRONTEND_THEME_TOGGLE.md) — dark/light header toggle + bootstrap FOUC notes (#488)
- [skills/AGENTS_FRONTEND_SOUND_MUTE.md](skills/AGENTS_FRONTEND_SOUND_MUTE.md) — UI SFX mute toggle + `cl8y-dex-sounds-enabled` persistence (#487)
- [skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md](skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) — anti-cognitive-overload retail copy + terminology (#489); glossary in [`docs/design-system.md`](docs/design-system.md)
- [skills/AGENTS_FRONTEND_RISK_DISCLAIMERS.md](skills/AGENTS_FRONTEND_RISK_DISCLAIMERS.md) — first-visit NFA / risk modal (#138)
- [skills/AGENTS_FRONTEND_CLICKWRAP.md](skills/AGENTS_FRONTEND_CLICKWRAP.md) — connected Legal TermsGate for `dex.cl8y.com` (**C1–C10**, **L658-1–L658-8**, [#517](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/517) / [#658](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/658)); `make verify-issue-517` · `make verify-issue-658`
- [skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md](skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md) — WalletConnect same-device mobile pairing (deep-link + copy, not QR-only; **WC-M1–WC-M12**, [#519](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/519) / [#554](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/554) / [#566](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/566) / [#658](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/658)); `make verify-issue-519` · `make verify-issue-554` · `make verify-issue-566` · `make verify-issue-658`
- [skills/AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md](skills/AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md) — Connect Wallet list + logos + dismiss overlay (**D1–D9**, [#672](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/672)); `make verify-issue-672`
- [skills/AGENTS_FRONTEND_CLICKWRAP.md](skills/AGENTS_FRONTEND_CLICKWRAP.md) — connected Legal TermsGate for `dex.cl8y.com` (**C1–C10**, [#517](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/517)); `make verify-issue-517`
- [skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md](skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md) — WalletConnect same-device mobile pairing (deep-link + copy, not QR-only; **WC-M1–WC-M12**, [#519](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/519) / [#554](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/554) / [#566](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/566)); `make verify-issue-519` · `make verify-issue-554` · `make verify-issue-566`
- [skills/AGENTS_FRONTEND_PORTAL_LISTBOX_VIEWPORT.md](skills/AGENTS_FRONTEND_PORTAL_LISTBOX_VIEWPORT.md) — portal listbox `visualViewport` + in-app chrome inset + coarse/narrow browse-without-IME (**V632-1–V632-8**, [#632](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/632)); `make verify-issue-632`
- [skills/AGENTS_FRONTEND_KEPLR_LEDGER.md](skills/AGENTS_FRONTEND_KEPLR_LEDGER.md) — Keplr + Ledger Nano signing (amino, pre-sign suggest, stall UX) (**K567-1–K567-8**, [#567](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/567)); `make verify-issue-567`
- [skills/AGENTS_KEPLR_CW20_REGISTRY.md](skills/AGENTS_KEPLR_CW20_REGISTRY.md) — Keplr Add Token CW20 pack on `cosmos/columbus` (**K629-1–K629-8**, [#629](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/629)); `make verify-issue-629`
- [skills/AGENTS_LISTINGS.md](skills/AGENTS_LISTINGS.md) — listing venue catalog + go/no-go for wallets / CG / CMC forms (**L639-1–L639-8**, [#639](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/639)); `make verify-issue-639`. GeckoTerminal `/gt/`: `make verify-issue-646` ([#646](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/646)). Event reserves: [`AGENTS_INDEXER_GT_EVENT_RESERVES.md`](skills/AGENTS_INDEXER_GT_EVENT_RESERVES.md) (**R684-1–R684-8**, [#684](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/684)); `make verify-issue-684`. API4 row cap: [`AGENTS_INDEXER_API4_PER_REQUEST.md`](skills/AGENTS_INDEXER_API4_PER_REQUEST.md) (**A694-1–A694-8**, [#694](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/694)); `make verify-issue-694`. CG/CMC field truth: [`AGENTS_INDEXER_CG_CMC_LISTING.md`](skills/AGENTS_INDEXER_CG_CMC_LISTING.md) (**L685-1–L685-8**, [#685](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/685)); `make verify-issue-685`
- [skills/AGENTS_COSMOSTATION.md](skills/AGENTS_COSMOSTATION.md) — Cosmostation / Mintscan CW20 pack on `chain/terra` (**C640-1–C640-8**, [#640](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/640)); `make verify-issue-640`
- [skills/AGENTS_HEXXAGON.md](skills/AGENTS_HEXXAGON.md) — Galaxy Station / Hexxagon CW20 pack (**H641-1–H641-8**, [#641](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/641)); `make verify-issue-641`
- [skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md](skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md) — single execution-aligned **Route** row on Swap and `/trade` market quote (#158, #302); Trade market GET `/route/solve` default (#501); always-on hybrid (#596)
- [skills/AGENTS_HYBRID_QUOTING.md](skills/AGENTS_HYBRID_QUOTING.md) — hybrid quote = execute; Swap + Trade market share `quoteCw20ViaRouteSolve` (#418, #501); regression `make verify-issue-501`
- [skills/AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md](skills/AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md) — official dApp never opts out of best-execution hybrid (**H596-1–H596-8**, [#596](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/596)); `make verify-issue-596`
- [skills/AGENTS_FRONTEND_PAY_INVOICE.md](skills/AGENTS_FRONTEND_PAY_INVOICE.md) — reusable pay-with-any-token invoice card (**I595-1–I595-14**, [#595](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/595)); `make verify-issue-595`
- [skills/AGENTS_COMMUNITY_TAX_CW20.md](skills/AGENTS_COMMUNITY_TAX_CW20.md) — community tax CW20 template + launcher + AutoLP (**T592-1–T592-13**, [#592](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/592); **O601-1–O601-7**, [#601](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/601)); `make verify-issue-592` · `make verify-issue-601`
- [skills/AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED.md](skills/AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED.md) — LocalTerra tax/EMBER seed + Transfer funding + indexer pins (**L620-1–L620-8**, [#620](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/620)); `make verify-issue-620`; post-merge leftovers [#624](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/624)
- [skills/AGENTS_E2E_COMMUNITY_TAX_TX.md](skills/AGENTS_E2E_COMMUNITY_TAX_TX.md) — Playwright `e2e-tx` tax/EMBER sell extra-debit / buy net / provide / limit (**E622-1–E622-8**, [#622](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/622)); `make verify-issue-622`; leftover live [#625](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/625)
- [skills/AGENTS_LOCALNET_SWARM_TAX.md](skills/AGENTS_LOCALNET_SWARM_TAX.md) — tax-aware LocalTerra swarm gem exclude + `tax_listed` / `--worker tax` (**S621-1–S621-8**, [#621](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/621)); `make verify-issue-621`; leftover live [#625](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/625)
- [skills/AGENTS_COMMUNITY_TAX_ROUTER.md](skills/AGENTS_COMMUNITY_TAX_ROUTER.md) — C-2 improved option 2: official-router hops tax the original trader (**T592-13** / **R607-1–R607-8**, [#607](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/607)); `make verify-issue-607`
- [skills/AGENTS_INDEXER_TAX_AWARE_ROUTING.md](skills/AGENTS_INDEXER_TAX_AWARE_ROUTING.md) — catalog net rank + 11611 pin + You Receive net (**R615-1–R615-8**, [#615](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/615)); `make verify-issue-615`
- [skills/AGENTS_COMMUNITY_TAX_LAUNCH_GUARDS.md](skills/AGENTS_COMMUNITY_TAX_LAUNCH_GUARDS.md) — LaunchGuards per-wallet cooldown + `max_wallet` protocol skip (**H608-1–H608-8**, [#608](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/608)); `make verify-issue-608`
- [skills/AGENTS_COMMUNITY_TAX_ENABLE_FEATURE.md](skills/AGENTS_COMMUNITY_TAX_ENABLE_FEATURE.md) — official launcher Enable Feature + unique SKUs (**T606-1–T606-8**, [#606](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/606)); `make verify-issue-606`; post-merge LocalTerra [#612](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/612)
- [skills/AGENTS_COMMUNITY_TAX_EXEMPT.md](skills/AGENTS_COMMUNITY_TAX_EXEMPT.md) — ExemptionDirectory skips buy/sell/transfer tax; launch guards stay on (**E609-1–E609-7**, [#609](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/609)); `make verify-issue-609`
- [skills/AGENTS_COMMUNITY_TAX_AUTOREGISTER.md](skills/AGENTS_COMMUNITY_TAX_AUTOREGISTER.md) — factory/dApp/AutoLP listed-pair register + manager role tax skip (**R633-1–R633-8**, [#633](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/633)); `make verify-issue-633`
- [skills/AGENTS_COMMUNITY_TAX_AUTOLP.md](skills/AGENTS_COMMUNITY_TAX_AUTOLP.md) — AutoLP factory-listed pair + skim floor (**M610-1–M610-8**, [#610](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/610)); `make verify-issue-610`
- [skills/AGENTS_FRONTEND_CREATE_TOKEN.md](skills/AGENTS_FRONTEND_CREATE_TOKEN.md) — Create Token + manager console (**C593-1–C593-14**, [#593](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/593); **C604-1–C604-3**, [#604](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/604); **C605-1–C605-4**, [#605](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/605)); `make verify-issue-593` · `make verify-issue-604` · `make verify-issue-605`; post-merge Coolify/LocalTerra [#602](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/602)
- [skills/AGENTS_FRONTEND_CREATE_TOKEN_LAYOUT.md](skills/AGENTS_FRONTEND_CREATE_TOKEN_LAYOUT.md) — `/token/create` desktop density (**C669-1–C669-8**, [#669](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/669)); `make verify-issue-669`
- [skills/AGENTS_FRONTEND_TOKEN_MIGRATE.md](skills/AGENTS_FRONTEND_TOKEN_MIGRATE.md) — free `/token/migrate` listed-template adopt + Terraport/GDEX LP gate (**M626-1–M626-12**, [#626](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/626)) and migrate pair inventory (**M634-1–M634-8**, [#634](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/634)); `make verify-issue-626` · `make verify-issue-634`. Post-merge leftovers: [`AGENTS_POST_MERGE_OPS_628.md`](skills/AGENTS_POST_MERGE_OPS_628.md) (**M628-1–M628-8**, [#628](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/628)); `make verify-issue-628`
- [skills/AGENTS_FRONTEND_TOKEN_MIGRATE.md](skills/AGENTS_FRONTEND_TOKEN_MIGRATE.md) — free `/token/migrate` listed-template adopt + Terraport/GDEX LP gate (**M626-1–M626-12**, [#626](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/626)), migrate pair inventory (**M634-1–M634-8**, [#634](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/634)), and why-copy Unlock {X} (**M670-1–M670-8**, [#670](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/670)); `make verify-issue-626` · `make verify-issue-634` · `make verify-issue-670`. Post-merge leftovers: [`AGENTS_POST_MERGE_OPS_628.md`](skills/AGENTS_POST_MERGE_OPS_628.md) (**M628-1–M628-8**, [#628](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/628)); `make verify-issue-628`
- [skills/AGENTS_CW20_CODE_ID_3.md](skills/AGENTS_CW20_CODE_ID_3.md) — columbus-5 **code 3** adopt + factory-list **NO-GO** (**C627-1–C627-8**, [#627](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/627)); `make verify-issue-627`
- [skills/AGENTS_INDEXER_COMMUNITY_TOKENS.md](skills/AGENTS_INDEXER_COMMUNITY_TOKENS.md) — community tax catalog API (**I594-1–I594-10**, [#594](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/594)); `make verify-issue-594`; post-merge Coolify [#602](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/602)
- [skills/AGENTS_FRONTEND_DEFAULT_SLIPPAGE.md](skills/AGENTS_FRONTEND_DEFAULT_SLIPPAGE.md) — default **Slippage protection** 5% + shared presets (#497)
- [skills/AGENTS_FRONTEND_SLIPPAGE_PRESET_ALIGN.md](skills/AGENTS_FRONTEND_SLIPPAGE_PRESET_ALIGN.md) — Trade Market + Swap Settings 0.5/1/5% chips stay one aligned group ([#528](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/528))
- [skills/AGENTS_FRONTEND_SWAP_QUOTE_REFETCH.md](skills/AGENTS_FRONTEND_SWAP_QUOTE_REFETCH.md) — sim `refetchInterval` guard + receive Calculating UX for slow multihop quotes (#484); clear/load You Receive on pay amount/token change (#496)
- [skills/AGENTS_INDEXER_ROUTE_SOLVE_PROGRESS.md](skills/AGENTS_INDEXER_ROUTE_SOLVE_PROGRESS.md) — distant-pair latency + `route/solve/progress` poll (#485); LCD-heavy + discount cache ([#694](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/694))
- [skills/AGENTS_INDEXER_API4_PER_REQUEST.md](skills/AGENTS_INDEXER_API4_PER_REQUEST.md) — OWASP API4 per-request caps on `/gt/events`, progress, blacklist-check (**A694-1–A694-8**, [#694](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/694)); `make verify-issue-694`
- [skills/AGENTS_INDEXER_EXTERNAL_ORACLE.md](skills/AGENTS_INDEXER_EXTERNAL_ORACLE.md) — ticker-scoped USTC/LUNC/CEX-FDUSD (path `vfdusd`) external USD feeds (**X1–X7**, [#515](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/515) / [#550](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/550) / [#579](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/579) CoinGecko User-Agent / [#580](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/580)); `make verify-issue-515` · `make verify-issue-579` · `make verify-issue-580`
- [skills/AGENTS_FRONTEND_PROTOCOL_STATS.md](skills/AGENTS_FRONTEND_PROTOCOL_STATS.md) — `/protocol` global USD stats + unified oracle card + pool TVL + treasury fees + inline Δ% / UTC grain volume + Δ% grouped with headline + liquidity 24h-only / denser x-axis + Volume/Liquidity/Fees metric toggle + phone Monthly `YY-MM` (**P550-1–P550-12**, **P569-1–P569-8**, **PFee-1–PFee-13**, **P652-1–P652-7**, **P667-1–P667-4**, **P668-1–P668-9**, **P689-1–P689-8**, **P703-1–P703-8**, [#550](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/550) / [#569](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/569) / [#586](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/586) / [#614](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/614) / [#652](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/652) / [#667](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/667) / [#668](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/668) / [#677](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/677) / [#689](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/689) / [#703](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/703)); `make verify-issue-550` · `make verify-issue-569` · `make verify-issue-586` · `make verify-issue-614` · `make verify-issue-652` · `make verify-issue-667` · `make verify-issue-668` · `make verify-issue-689` · `make verify-issue-677` · `make verify-issue-703`
- [skills/AGENTS_INDEXER_UST1_WINDOW_FEES.md](skills/AGENTS_INDEXER_UST1_WINDOW_FEES.md) — `/protocol` UST1 window mint/redeem treasury fees from pinned `UST1_WINDOW_ADDRESS` `fee_amount` (**I614-1–I614-8**, [#614](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/614)); `make verify-issue-614`
- [skills/AGENTS_DEFILLAMA.md](skills/AGENTS_DEFILLAMA.md) — DeFiLlama TVL/volume/fees listing + UTC-day indexer API (**L631-1–L631-11**, **L687-1–L687-8**, [#631](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/631) / [#687](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/687)); `make verify-issue-631` · `make verify-issue-687`
- [skills/AGENTS_INDEXER_WRAP_FEE_INGEST.md](skills/AGENTS_INDEXER_WRAP_FEE_INGEST.md) — wrap/unwrap protocol-fee ingest from captured mapper `notify_deposit` / `unwrap` `fee` (**I613-1–I613-8**, [#613](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/613)); `make verify-issue-613`
- [skills/AGENTS_INDEXER_VENUS_VFDUSD.md](skills/AGENTS_INDEXER_VENUS_VFDUSD.md) — `/protocol` vFDUSD **FDUSD reference price** + Venus **1 vFDUSD Price** (**V571-1–V571-10**, [#571](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/571)); `make verify-issue-571`
- [skills/AGENTS_INDEXER_PAIR_PRICE_USD.md](skills/AGENTS_INDEXER_PAIR_PRICE_USD.md) — pair tape/candles human quote-per-base + USD of 1 human base (**P522-1–P522-5**, [#522](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/522)); `make verify-issue-522`
- [skills/AGENTS_INDEXER_HUB_USD.md](skills/AGENTS_INDEXER_HUB_USD.md) — DEX hub USD for cUSTC/UST1/USTR from largest-liquidity pools (**H1–H10**, [#556](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/556)); `make verify-issue-556`
- [skills/AGENTS_INDEXER_ECONOMIC_FEE_USD.md](skills/AGENTS_INDEXER_ECONOMIC_FEE_USD.md) — factory economic fee USD (CL8Y + listed non-gems; hub card stays four cells) (**EFee-1–EFee-8**, [#683](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/683)); `make verify-issue-683`
- [skills/AGENTS_FRONTEND_PROTOCOL_HUB.md](skills/AGENTS_FRONTEND_PROTOCOL_HUB.md) — Protocol hub wrap CW20 identity + LUNC/USD column (**H11–H16**, [#570](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/570)); `make verify-issue-570`
- [skills/AGENTS_INDEXER_CANDLE_USD_MARK.md](skills/AGENTS_INDEXER_CANDLE_USD_MARK.md) — time-stamped candle USD; no as-of-now hub rewrite; idle mark-to-market bars (**C568-1–C568-8**, [#568](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/568)); `make verify-issue-568`
- [skills/AGENTS_FRONTEND_PORTFOLIO_PNL.md](skills/AGENTS_FRONTEND_PORTFOLIO_PNL.md) — `/portfolio` + `/trader` human-scale P&amp;L / cost / avg entry; mixed totals omitted or USD (**P551-1–P551-6**, [#551](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/551)); `make verify-issue-551`
- [skills/AGENTS_INDEXER_TRADER_POSITIONS_DECIMALS.md](skills/AGENTS_INDEXER_TRADER_POSITIONS_DECIMALS.md) — `/positions` 18-dec `NUMERIC(78, 18)` + `trade_count` vs `/trades` (**P676-1–P676-8**, [#676](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/676)); `make verify-issue-676`
- [skills/AGENTS_FRONTEND_PORTFOLIO_TEST_PAIRS.md](skills/AGENTS_FRONTEND_PORTFOLIO_TEST_PAIRS.md) — `/portfolio` hides test-gem Open Positions / P&amp;L / recent activity by default (**P674-1–P674-8**, [#674](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/674)); `make verify-issue-674`
- [skills/AGENTS_FRONTEND_HUB_PNL.md](skills/AGENTS_FRONTEND_HUB_PNL.md) — `/portfolio` + `/trader` realized P&amp;L USD from `GET /api/v1/hub-prices` (**P560-1–P560-6**, [#560](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/560)); `make verify-issue-560`
- [skills/AGENTS_FRONTEND_PORTFOLIO_UNREALIZED.md](skills/AGENTS_FRONTEND_PORTFOLIO_UNREALIZED.md) — `/portfolio` + `/trader` hub mark + unrealized vs on-DEX cost (**P675-1–P675-8**, [#675](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/675)); `make verify-issue-675`
- [skills/AGENTS_FRONTEND_TAPE_AMOUNTS.md](skills/AGENTS_FRONTEND_TAPE_AMOUNTS.md) — tape + wallet Amount in/out/Price human scale (**T557-1–T557-11**, [#557](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/557)); `make verify-issue-557`
- [`AGENTS_FRONTEND_CHARTS_PAIR_SCOPED.md`](skills/AGENTS_FRONTEND_CHARTS_PAIR_SCOPED.md) — `/charts` pair-scoped 24h stats + leaderboard (**CS-1–CS-15**, [#666](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/666)); `make verify-issue-666`
- [`AGENTS_FRONTEND_CHARTS_UST1_HERO.md`](skills/AGENTS_FRONTEND_CHARTS_UST1_HERO.md) — `/charts` UST1/cUSTC hero + `?price=` + page-wide invert (**C680-1–C680-9**, [#680](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/680)); `make verify-issue-680`
- [`AGENTS_FRONTEND_CHARTS_OVERVIEW.md`](skills/AGENTS_FRONTEND_CHARTS_OVERVIEW.md) — `/protocol` + `GET /overview` 24h volume USD-only + catalog `volume_usd` ingest (**C1–C9**, [#548](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/548)); not a Charts census strip; `make verify-issue-548`
- [skills/AGENTS_FRONTEND_TRAILING_WINDOW.md](skills/AGENTS_FRONTEND_TRAILING_WINDOW.md) — Charts/Protocol/Pool **24h volume** is a trailing window, not a midnight reset (**W1–W5**, [#576](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/576)); `make verify-issue-576`
- [skills/AGENTS_INDEXER_VOLUME_WINDOW_DECAY.md](skills/AGENTS_INDEXER_VOLUME_WINDOW_DECAY.md) — token/trader/pair/global trailing windows **zero** when swaps leave the cutoff; stale `global_stats_24h.updated_at` is log-only (**D1–D7**, [#577](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/577)); `make verify-issue-577`
- [skills/AGENTS_FRONTEND_CHARTS_PAIR_STATS.md](skills/AGENTS_FRONTEND_CHARTS_PAIR_STATS.md) — `/charts` pair 24h Stats **Vol (USD)** + human token remainder + TWAP/histogram human scale (**P565-1–P565-7**, **S564-1–S564-11**, [#565](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/565) / [#564](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/564)); `make verify-issue-565` · `make verify-issue-564`
- [skills/AGENTS_FRONTEND_TRADER_VOLUME_USD.md](skills/AGENTS_FRONTEND_TRADER_VOLUME_USD.md) — `/charts` trader leaderboard + profile **Total Volume (USD)** from `total_volume_usd` (**T553-1–T553-6**, [#553](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/553)); `make verify-issue-553`
- [skills/AGENTS_FRONTEND_TRADER_LEADERBOARD.md](skills/AGENTS_FRONTEND_TRADER_LEADERBOARD.md) — `/trader` + `/trader/:address` global **Leaderboard** last in page content (**TL-1–TL-10**, [#657](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/657)); `make verify-issue-657`
- [skills/AGENTS_FRONTEND_TRADER_IDENTITY.md](skills/AGENTS_FRONTEND_TRADER_IDENTITY.md) — trader 4/6 address + blockie PFP on Charts leaderboard + `/trader` / `/portfolio` (**T-ID-1–T-ID-10**, [#656](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/656)); `make verify-issue-656`
- [skills/AGENTS_FRONTEND_TRADER_VOLUME_USD.md](skills/AGENTS_FRONTEND_TRADER_VOLUME_USD.md) — `/charts` pair-scoped leaderboard + profile **Total Volume (USD)** from `total_volume_usd` (**T553-1–T553-6**, [#553](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/553) / [#666](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/666)); unscoped API stays global; `make verify-issue-553`
- [skills/AGENTS_FRONTEND_TRADE_PAIR_INVERT.md](skills/AGENTS_FRONTEND_TRADE_PAIR_INVERT.md) — `/trade` + `/charts` UI invert for UST1-as-base pairs (other-side Price USD, pill, convert-on-submit) (**T524-1–T524-11**, [#524](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/524)); `make verify-issue-524`
- [skills/AGENTS_FRONTEND_USD_CANDLE_INVERT.md](skills/AGENTS_FRONTEND_USD_CANDLE_INVERT.md) — Price (USD) candles must use `invertUsd` (not `1/x`) + adaptive Y-axis (**C543-1–C543-9**, [#543](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/543)); `make verify-issue-543`
- [skills/AGENTS_FRONTEND_TOKEN_IDENTITY.md](skills/AGENTS_FRONTEND_TOKEN_IDENTITY.md) — compact copy + explorer for pair legs + pair contract on `/pool`, `/trade`, `/charts` (**T541-1–T541-8**, [#541](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/541)); `make verify-issue-541`
- [skills/AGENTS_FRONTEND_TRADE_IDENTITY_LP.md](skills/AGENTS_FRONTEND_TRADE_IDENTITY_LP.md) — Trade / Charts identity **v2 LP** USD from single-pair `liquidity_usd` (**T664-1–T664-8**, [#664](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/664)); `make verify-issue-664`
- [skills/AGENTS_FRONTEND_NATIVE_TICKERS.md](skills/AGENTS_FRONTEND_NATIVE_TICKERS.md) — Swap/Pool/Pay/Mint show **LUNC** / **USTC** not `uluna` / `uusd` (**N630-1–N630-8**, [#630](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/630)); `make verify-issue-630`
- [skills/AGENTS_FRONTEND_POOL_PROVIDE_LABELS.md](skills/AGENTS_FRONTEND_POOL_PROVIDE_LABELS.md) — `/pool` Manage provide **name (symbol)** + wrap default on (**P661-1–P661-12**, [#661](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/661)); `make verify-issue-661`
- [skills/AGENTS_FRONTEND_TIERS_PHONE.md](skills/AGENTS_FRONTEND_TIERS_PHONE.md) — `/tiers` phone-width cards + How it works stack (**T651-1–T651-8** / **I15**, [#651](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/651)); `make verify-issue-651`
- [skills/AGENTS_FRONTEND_SHARE_LINK.md](skills/AGENTS_FRONTEND_SHARE_LINK.md) — `/trader/:address` Share (Web Share + clipboard) (**TS-1–TS-13**, [#665](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/665)); `make verify-issue-665`
- [skills/AGENTS_LIMIT_PRICE_DECIMALS.md](skills/AGENTS_LIMIT_PRICE_DECIMALS.md) — limit-order price band is **human-scale** `raw × 10^(dec0 − dec1)` so UST1/USTR can place (**L20** / [#529](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/529)); `make verify-issue-529`
- [skills/AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md](skills/AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md) — `/trade` Place limit / Market CTA docks to ticket bottom; Chrome sticky mid-form float (**T527-1–T527-10**, [#527](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/527)); `make verify-issue-527`
- [skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md](skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md) — `/trade` desktop CSS grid (no drag-resize), independent tape row, hide book/ticket (**L561-1–L561-12**, [#561](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/561)); `make verify-issue-561`
- [skills/AGENTS_FRONTEND_TRADE_TICKET_HEADING.md](skills/AGENTS_FRONTEND_TRADE_TICKET_HEADING.md) — `/trade` full **Buy {base}** heading, no compact wallet chip, green Buy / red Sell side control (**T563-1–T563-8**, [#563](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/563)); `make verify-issue-563`
- [skills/AGENTS_FRONTEND_TRADE_TICKET_FLATTEN.md](skills/AGENTS_FRONTEND_TRADE_TICKET_FLATTEN.md) — `/trade` ticket **Market** default, compact text tabs, heading logo + clamped wash, flatten Side/mode cards, slippage in Advanced (**T693-1–T693-8**, [#693](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/693)); `make verify-issue-693`
- [skills/AGENTS_FRONTEND_LIMIT_CANCEL_OPEN.md](skills/AGENTS_FRONTEND_LIMIT_CANCEL_OPEN.md) — My open limits Cancel vs stale `●` row / fill lifecycle / `/trade` reachability (**F530-1–F530-8**, [#530](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/530)); `make verify-issue-530`
- [skills/AGENTS_FRONTEND_POOL_TABLE.md](skills/AGENTS_FRONTEND_POOL_TABLE.md) — `/pool` sortable table, UST1-first catalog default, Charts `/charts/:pairAddr` (**P547-1–P547-10**, [#547](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/547)); `make verify-issue-547`
- [skills/AGENTS_FRONTEND_POOL_MANAGE_IA.md](skills/AGENTS_FRONTEND_POOL_MANAGE_IA.md) — `/pool` Manage four peer actions: provide, withdraw, zap add, zap withdraw (**M660-1–M660-8**, [#660](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/660)); `make verify-issue-660`
- [skills/AGENTS_INDEXER_PAIR_LIQUIDITY_USD.md](skills/AGENTS_INDEXER_PAIR_LIQUIDITY_USD.md) — `/pool` **v2 LP USD** + `GET /pairs` `liquidity_usd` rollup (**P655-1–P655-8**, [#655](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/655)); `make verify-issue-655`
- [skills/AGENTS_INDEXER_PAIR_VOLUME_USD.md](skills/AGENTS_INDEXER_PAIR_VOLUME_USD.md) — `/pool` **Vol** USD + `GET /pairs` `volume_usd_24h` rollup (**PVol-1–PVol-8**, [#692](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/692)); `make verify-issue-692`
- [skills/AGENTS_FRONTEND_POOL_CREATED.md](skills/AGENTS_FRONTEND_POOL_CREATED.md) — `/pool` Created relative age from indexer first-seen `created_at` (**P662-1–P662-8**, [#662](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/662)); `make verify-issue-662`
- [skills/AGENTS_FRONTEND_POOL_LP_HOWTO.md](skills/AGENTS_FRONTEND_POOL_LP_HOWTO.md) — retail how-to for LUNC v2 LP + maker-limit disambiguation; no incentive program ([#531](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/531))
- [skills/AGENTS_FRONTEND_POOL_ONE_SIDED.md](skills/AGENTS_FRONTEND_POOL_ONE_SIDED.md) — one-sided pool add/withdraw (auto zap + wrap; **Z533-1–Z533-10**, [#533](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/533)); `make verify-issue-533`
- [skills/AGENTS_FRONTEND_POOL_ZAP_FLOORS.md](skills/AGENTS_FRONTEND_POOL_ZAP_FLOORS.md) — zap-in/out execution follows floors, not optimistic quotes (**Z559-1–Z559-4**, [#559](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/559)); `make verify-issue-559`
- [skills/AGENTS_FRONTEND_PAIR_CATALOG_RANK.md](skills/AGENTS_FRONTEND_PAIR_CATALOG_RANK.md) — pair/token pickers list economic markets first, gems last; USD then human-quote volume badges (**P534-1–P534-8**, [#534](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/534) / [#692](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/692)); `make verify-issue-534`
- [skills/AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md](skills/AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md) — production hides soft-launch gems from Swap/Trade/Pool/Charts/Create + rejects gem-bridge quotes (**P562-1–P562-8**, [#562](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/562)); `make verify-issue-562`
- [skills/AGENTS_POST_MERGE_STACK.md](skills/AGENTS_POST_MERGE_STACK.md) — post-merge Coolify + indexer stack for !368–!377 (**M573-1–M573-8**, [#573](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/573)); `make verify-issue-573`
- [skills/AGENTS_FRONTEND_CREATE_PAIR_PICKER.md](skills/AGENTS_FRONTEND_CREATE_PAIR_PICKER.md) — `/create` listed-CW20 picker + custom paste; not Swap’s factory universe (**C542-1–C542-11**, [#542](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/542)); `make verify-issue-542`
- [skills/AGENTS_LIMIT_ORDER_BATCH_LADDER.md](skills/AGENTS_LIMIT_ORDER_BATCH_LADDER.md) — batch/ladder place UI + gas gates; Ladder create must render when disconnected (#494)
- [skills/AGENTS_UST1_SECONDARY_AMM.md](skills/AGENTS_UST1_SECONDARY_AMM.md) — UST1 secondary AMM create/seed or Path B waiver ([#508](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/508), invariants **U1–U7**); `make verify-issue-508`
- [skills/AGENTS_FEE_DISCOUNT_TIERS.md](skills/AGENTS_FEE_DISCOUNT_TIERS.md) — CL8Y tier ladder + **I13** limit-placement discount shift (tier 9 place = 0; swap/take unchanged) + [`scripts/upgrade-514-limit-discount.sh`](scripts/upgrade-514-limit-discount.sh) ([#514](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/514)); `make verify-issue-514`
- [skills/AGENTS_FACTORY_DISCOUNT_REGISTRY.md](skills/AGENTS_FACTORY_DISCOUNT_REGISTRY.md) — factory `config.discount_registry` snapshot on `CreatePair` so new pairs are wired (**F5** / **I14**, [#536](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/536)); LocalTerra inherit + dApp `GetDiscountRegistry` first (**F538-1–F538-3**, [#538](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/538)); `make verify-issue-536` / `make verify-issue-538`
- [skills/AGENTS_CW20_CODE_ID_PIN.md](skills/AGENTS_CW20_CODE_ID_PIN.md) — listed CW20 `code_id` pin + write-path whitelist re-check so `MsgMigrateContract` cannot leave the listing template (**F6**, [#582](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/582)); factory-first migrate [`scripts/upgrade-582-code-id-pin.sh`](scripts/upgrade-582-code-id-pin.sh) ([#584](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/584)) including `UpdateConfig { pair_code_id }` + LCD whitelist retries; `make verify-issue-582` · `make verify-issue-584`
- [skills/AGENTS_CW20_CODE_ID_AUDIT.md](skills/AGENTS_CW20_CODE_ID_AUDIT.md) — generalized CW20 code-id audit harness: LCD pin + decomp + catalogue + Layer A/B (**C589-1–C589-9**, [#589](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/589)); gates [#581](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/581) 8266 go; `make verify-issue-589`
- [skills/AGENTS_CW20_CODE_ID_TAX_ON.md](skills/AGENTS_CW20_CODE_ID_TAX_ON.md) — named tax-on Layer B (**C623-1–C623-8**, [#623](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/623)); do **not** merge into B-lt; `make verify-issue-623`; leftover live [#625](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/625)
- [skills/AGENTS_POST_MERGE_OPS_590.md](skills/AGENTS_POST_MERGE_OPS_590.md) — post-merge Coolify + LocalTerra ops for !394–!396 (**M590-1–M590-8**, [#590](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/590)); `make verify-issue-590`
- [skills/AGENTS_POST_MERGE_OPS_600.md](skills/AGENTS_POST_MERGE_OPS_600.md) — post-merge LocalTerra E9 + columbus-5 USTR→USTC unwrap gas for !400 (**M600-1–M600-8**, [#600](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/600)); `make verify-issue-600`
- [skills/AGENTS_POST_MERGE_OPS_602.md](skills/AGENTS_POST_MERGE_OPS_602.md) — post-merge !402 Coolify + 11614 launcher + LocalTerra Create Token (**M602-1–M602-8**, [#602](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/602)); `make verify-issue-602`
- [skills/AGENTS_POST_MERGE_OPS_612.md](skills/AGENTS_POST_MERGE_OPS_612.md) — post-merge !407/!408 Enable Feature migrate + LocalTerra QA (**M612-1–M612-8**, [#612](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/612)); `make verify-issue-612`
- [skills/AGENTS_POST_MERGE_OPS_616.md](skills/AGENTS_POST_MERGE_OPS_616.md) — post-merge !409–!413 option-2 wasm, wrap/window fees, AutoLP, tax ranking (**M616-1–M616-8**, [#616](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/616)); `make verify-issue-616`
- [skills/AGENTS_POST_MERGE_OPS_624.md](skills/AGENTS_POST_MERGE_OPS_624.md) — post-merge !414 LocalTerra community-tax seed leftovers (**M624-1–M624-8**, [#624](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/624)); `make verify-issue-624`
- [skills/AGENTS_POST_MERGE_OPS_625.md](skills/AGENTS_POST_MERGE_OPS_625.md) — post-merge !415–!417 tax swarm / e2e-tx / Layer B leftovers (**M625-1–M625-8**, [#625](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/625)); `make verify-issue-625`
- [skills/AGENTS_POST_MERGE_OPS_628.md](skills/AGENTS_POST_MERGE_OPS_628.md) — post-merge !418 community-tax migrate leftovers (**M628-1–M628-8**, [#628](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/628)); `make verify-issue-628`
- [skills/AGENTS_POST_MERGE_OPS_673.md](skills/AGENTS_POST_MERGE_OPS_673.md) — post-merge !437–!458 leftover verify (**M673-1–M673-8**, [#673](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/673)); `make verify-issue-673`
- [skills/AGENTS_POST_MERGE_OPS_686.md](skills/AGENTS_POST_MERGE_OPS_686.md) — post-merge !459–!468 leftover verify (**M686-1–M686-8**, [#686](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/686)); `make verify-issue-686`
- [skills/AGENTS_POST_MERGE_OPS_698.md](skills/AGENTS_POST_MERGE_OPS_698.md) — post-merge !474/!475 leftover verify (**M698-1–M698-8**, [#698](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/698); #699/#700 dups); `make verify-issue-698`
- [skills/AGENTS_POST_MERGE_OPS_701.md](skills/AGENTS_POST_MERGE_OPS_701.md) — post-merge !477 leftover verify (**M701-1–M701-8**, [#701](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/701)); `make verify-issue-701`
- [skills/AGENTS_POST_MERGE_OPS_702.md](skills/AGENTS_POST_MERGE_OPS_702.md) — post-merge !476 leftover verify (**M702-1–M702-8**, [#702](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/702)); `make verify-issue-702`
- [skills/AGENTS_FRONTEND_CODE_ID_FREEZE.md](skills/AGENTS_FRONTEND_CODE_ID_FREEZE.md) — dApp + indexer F6 freeze visibility: `route/solve` excludes frozen hops, pair `code_id_frozen`, humanized execute errors (**F585-1–F585-8**, [#585](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/585)); not a substitute for on-chain pin; `make verify-issue-585`
- [skills/AGENTS_TERRACLASSIC_GAS.md](skills/AGENTS_TERRACLASSIC_GAS.md) — Terra Classic fee envelopes; wrap+≥2hop LUNC↔USTR combo ([#587](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/587)); mixed hybrid+pool router hops ([#679](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/679)); unwrap+≥2hop USTR→USTC combo ([#599](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/599)); post-merge E9/columbus-5 ([#600](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/600)); greedy book-first gas (**G13**, [#708](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/708)); Swap **Network fee (est.) ~X LUNC**; `make verify-issue-587` · `make verify-issue-679` · `make verify-issue-599` · `make verify-issue-600` · `make verify-issue-708` · `make verify-issue-709` · `make verify-issue-710`
- [skills/AGENTS_GREEDY_BOOK_FIRST.md](skills/AGENTS_GREEDY_BOOK_FIRST.md) — opt-in greedy book-first pair/router swap (**G1–G14**, [#708](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/708)); query mutex + `remainder_to_pool` + A7 overflow ([#709](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/709)); tax/pause/blacklist/L7 ([#710](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/710)); `make verify-issue-708` · `make verify-issue-709` · `make verify-issue-710`
- [skills/AGENTS_FRONTEND_PAIR_FEE_DISCOUNT.md](skills/AGENTS_FRONTEND_PAIR_FEE_DISCOUNT.md) — dApp fee-tier chrome gated on pair `DISCOUNT_REGISTRY` (**I14**, [#537](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/537)); `make verify-issue-537`
- [skills/AGENTS_LP_SYMBOL_DIGITS.md](skills/AGENTS_LP_SYMBOL_DIGITS.md) — LP ticker keeps `0-9`, strips non-alnum; factory `UpdateConfig` code IDs + [`scripts/upgrade-518-lp-symbol.sh`](scripts/upgrade-518-lp-symbol.sh) (**F3**, [#518](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/518)); `make verify-issue-518`
- [skills/AGENTS_ROTATE_FEE_TREASURY.md](skills/AGENTS_ROTATE_FEE_TREASURY.md) — swap/book fees to ustr-cmm CMM (`terra16j5u6…`); factory `SetPairTreasury*` + [`scripts/rotate-fee-treasury.sh`](scripts/rotate-fee-treasury.sh) (**F4**)
- [skills/AGENTS_REBALANCE_MINT_UST1_LP.md](skills/AGENTS_REBALANCE_MINT_UST1_LP.md) — UST1/cUSTC oracle rebalance + $5k LP on UST1/cUSTC and UST1/USTR to CMM ([`scripts/rebalance-mint-ust1-lp.sh`](scripts/rebalance-mint-ust1-lp.sh))
- [skills/AGENTS_EXPIRED_LIMIT_PARK_REASON.md](skills/AGENTS_EXPIRED_LIMIT_PARK_REASON.md) — park `reason` discriminator so bots do not treat dust-filled parks as unfilled expiry ([#504](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/504), invariant **L22**)
- [skills/AGENTS_ORDER_STATUS_QUERY.md](skills/AGENTS_ORDER_STATUS_QUERY.md) — on-chain typed `OrderStatus` for vaults/bots ([#505](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/505), invariant **L21**); `ParkedRefund` ≠ park reason
- [skills/AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md](skills/AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md) — maker UX for indexer `parked_expired` + `ClaimExpiredLimitOrder` ([#141](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/141); pause blocks claim per [#120](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/120) / **L6**)
- [skills/AGENTS_UST1_WINDOW_UI.md](skills/AGENTS_UST1_WINDOW_UI.md) — `/ust1` oracle vFDUSD↔UST1 mint/redeem (CW20 Send + `effective_swap`); not faucet Mint ([#506](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/506)); protocol mint/redeem fees **PFee-13** / **I614** ([#614](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/614)); `make verify-issue-506` · `make verify-issue-614`
- [skills/AGENTS_FRONTEND_SWAP_ACQUIRE_GUIDANCE.md](skills/AGENTS_FRONTEND_SWAP_ACQUIRE_GUIDANCE.md) — Swap/Trade unfunded pay + UST1 `/ust1` Guide + quote-only (**A678-1–A678-11**, [#678](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/678)); `make verify-issue-678`
- [skills/AGENTS_MAINNET_WRAP_ENABLEMENT.md](skills/AGENTS_MAINNET_WRAP_ENABLEMENT.md) — post-SL5 Coolify wrap env + cLUNC/cUSTC fee UX ([#507](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/507))
- [skills/AGENTS_WRAP_UNWRAP_BURN_TAX.md](skills/AGENTS_WRAP_UNWRAP_BURN_TAX.md) — unwrap InstantWithdraw burn tax quotes + exchange-deposit warning (**W8–W11**, [#512](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/512)); `make verify-issue-512`
- [skills/AGENTS_WRAP_MAPPER_SPLIT_FEES.md](skills/AGENTS_WRAP_MAPPER_SPLIT_FEES.md) — wrap-mapper `fee_wrap_bps` / `fee_unwrap_bps` + unwrap ≈2% all-in retune (**W12–W15**, [#516](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/516)); `make verify-issue-516`. Router `unwrap_output` dual-read ([#523](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/523)); `make verify-issue-523`. LocalTerra wrap-mapper instantiate + #533 P4–P8 ([#539](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/539)); `make verify-issue-539`
- [skills/AGENTS_UST1_WRAP_PRODUCTION_OPS.md](skills/AGENTS_UST1_WRAP_PRODUCTION_OPS.md) — Phase 5 UST1/wrap ops hardening: registry, health probes, pause playbooks (invariants **O1–O8**, [#503](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/503)); `make verify-issue-503`
