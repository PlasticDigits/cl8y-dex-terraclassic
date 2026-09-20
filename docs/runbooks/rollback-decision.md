# Runbook: Rollback and forward-fix decision tree (SEC-H09)

Operator playbook for **when** to rollback vs hotfix forward during live incidents across the four deployment surfaces: **frontend**, **indexer**, **CosmWasm contracts**, and **chain dependencies**. Parent remediation: GitLab [#445](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/445) (**SEC-H09**).

**Related:** [launch-checklist.md § Rollback / incident](./launch-checklist.md#rollback--incident), [wasm admin migration](./wasm-admin-migration.md), [emergency commands](./emergency-commands.md) (on-chain pause/blacklist), [blacklist decision](./blacklist-decision.md) (exploit/compliance — distinct from deploy rollback), [indexer reorg / replay](./indexer-reorg-replay-dedup.md), [incident template](../templates/incident-dex-indexer.md), [user incident FAQ](../user-incident-faq.md). Agent playbook: [`skills/AGENTS_ROLLBACK_DECISION.md`](../../skills/AGENTS_ROLLBACK_DECISION.md).

## Policy summary

| Surface | Typical symptom | First control | Rollback available? |
|---------|-----------------|---------------|---------------------|
| **Frontend** | Broken UI, wrong `VITE_*` addresses, CSP/connect-src failure | Hotfix build or redeploy prior static artifact | Yes — prior `dist/` or Render deploy rollback |
| **Indexer** | Crash loop, wrong API data, failed migration | Restart process; Coolify restore or hotfix; optional `down.sql`; attest via `/health` `git_sha` ([#1276](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1276)) | Partial — schema rollback only when paired `.down.sql` exists **for the ahead version**; **2(b)** keep schema + hotfix that still ships N (even if a paired down exists); **2(c)** **iff** restoring the prior image is required **and** `indexer/migrations/revert/<ahead-version>_*.down.sql` exists → auto-deploy off + snapshot + that `down.sql` + `DELETE` that `_sqlx_migrations` row + restore |
| **Contract** | Logic bug post-migrate | Emergency **pause** / blacklist; forward-fix **migrate** | Partial — migrate to prior `code_id` only if still on chain and state compatible |
| **Chain dependency** | Chain upgrade incompatibility, IBC-hooks patch, LCD/RPC outage | Pause pairs; switch LCD provider; wait for validator upgrade | No on-chain rollback — coordinate with network |

**On-chain user txs during a bad window cannot be rolled back.** Rollback paths restore **operator-controlled** surfaces (static site, indexer mirror, contract code pointer). Record all steps in the [incident timeline](../templates/incident-dex-indexer.md#incident-timeline).

## Top-level decision tree

Classify the incident **before** choosing a path. Many incidents combine surfaces (e.g. contract bug + stale indexer); treat **on-chain risk first**, then off-chain mirrors.

```mermaid
flowchart TD
  start([Live incident]) --> classify{Primary blast radius?}
  classify -->|UI / quotes wrong<br/>swaps still safe on-chain| fe[Frontend-only]
  classify -->|API/charts stale or wrong<br/>chain state OK| idx[Indexer]
  classify -->|Swap/LP/limit logic wrong<br/>or exploit active| ctr[Contract]
  classify -->|RPC/LCD/chain binary<br/>or IBC-hooks exposure| chain[Chain dependency]
  fe --> fe_dec{Fix in &lt;30 min<br/>without env risk?}
  fe_dec -->|Yes| fe_hot[Forward-fix: patch + rebuild]
  fe_dec -->|No| fe_roll[Rollback: prior static build]
  idx --> idx_dec{Process crash only?}
  idx_dec -->|Yes| idx_restart[Restart indexer]
  idx_dec -->|No — bad data or migration| idx_schema{Ledger vs prior Coolify SHA?}
  idx_schema -->|Dirty success=false| idx_dirty["DELETE failed row only; never UPDATE success; then restore or 2(b)"]
  idx_schema -->|Unchanged vs prior latest *.sql| idx_cool["2(a) restore; auto-deploy off or land fix before next webhook"]
  idx_schema -->|Ahead| idx_ahead{Restore prior image required?}
  idx_ahead -->|No — next image still ships N| idx_fwd[2(b) keep schema; hotfix]
  idx_ahead -->|Yes — schema is the bug| idx_pair{Paired down.sql for ahead version?}
  idx_pair -->|No| idx_fwd
  idx_pair -->|Yes — that version only| idx_down["2(c) auto-deploy off; snapshot; that down.sql; DELETE that ledger row; restore"]
  idx_fwd --> idx_attest[Attest /health git_sha EXPECT_SHA]
  idx_cool --> idx_attest
  idx_down --> idx_attest
  idx_dirty --> idx_attest
  ctr --> ctr_risk{Funds at risk<br/>or exploit active?}
  ctr_risk -->|Yes| ctr_pause[Emergency pause / blacklist]
  ctr_risk -->|No — contained bug| ctr_fix{State-compatible<br/>forward migrate?}
  ctr_pause --> ctr_plan[Governance: forward-fix or migrate-back plan]
  ctr_fix -->|Yes| ctr_migrate[Forward-fix migrate]
  ctr_fix -->|No| ctr_wait[Pause affected pairs; plan migration]
  chain --> chain_dec{Trading safe on<br/>alternate LCD/RPC?}
  chain_dec -->|Yes| chain_lcd[Fail over LCD/RPC; monitor]
  chain_dec -->|No| chain_pause[Pause all affected pairs; coordinate upgrade]
```

Indexer `idx_schema` / `idx_attest` → [§ Auto-deploy era Coolify incident checklist](#auto-deploy-era-1276) (**three-way:** **2(a)** unchanged vs prior Coolify Deploys SHA → restore; **2(b)** keep schema + hotfix that still ships N, **even if** a paired `down.sql` exists; **2(c)** **iff** restoring the prior image is required **and** `indexer/migrations/revert/<ahead-version>_*.down.sql` exists for version(s) **newer than that baseline** — historical `revert/` files do **not** select 2(c); no paired file for the ahead version → 2(b)). Inspect `_sqlx_migrations` (`version`, `success`; `installed_on` inspect-only) via Coolify DB / indexer `DATABASE_URL` (not `postgres-psql.sh`). Baseline: `git ls-tree --name-only <prior-coolify-deploys-sha> indexer/migrations/`. Dirty: `DELETE FROM _sqlx_migrations WHERE version = <v> AND success = false` only; never `UPDATE success`. Revert files do **not** touch the ledger. Attest `git_sha` with `EXPECT_SHA`. After 2(c), re-enable auto-deploy only when `main` **no longer ships N**, or re-applying N is explicit intent.

---

## 1. Frontend-only incident

### Symptoms / triggers

- Broken production build (blank page, JS error, failed asset load).
- Wrong inlined contract addresses (`VITE_FACTORY_ADDRESS`, `VITE_ROUTER_ADDRESS`, pair env vars).
- Mixed-content or CSP `connect-src` blocks indexer/LCD (swaps fail in browser only; chain txs from CLI still work).
- WalletConnect / Keplr connect failures caused by frontend config (not chain halt).

### Decision criteria

| Choose | When |
|--------|------|
| **Forward-fix (hotfix)** | Root cause is a **small, verified** config or code fix; you can ship a new static build in minutes; **no** wrong-address risk remains in the prior artifact; on-chain trading is safe. |
| **Rollback (revert artifact)** | Bad build is live; fix is uncertain or needs review; wrong `VITE_*` addresses were inlined; or hotfix pipeline is slower than restoring the last known-good deploy. |
| **Pause on-chain (escalate)** | Frontend bug **misroutes** swaps to attacker-controlled contracts or indexer URL — treat as **contract/security** incident; pause affected pairs per [emergency commands](./emergency-commands.md) while rolling back frontend. |

### Rollback path (commands)

Production static hosting (Render example — adapt to your CDN):

```bash
# 1. Identify last known-good git tag or deploy ID from launch tracking issue / deploy trace
git fetch --tags
export GOOD_SHA="<prior-release-sha>"

# 2. Rebuild from that commit with production env (never reuse a dev .env.local)
git checkout "$GOOD_SHA"
cd frontend-dapp
# Load production VITE_* from your secret store — not from repo
npm ci && NODE_OPTIONS=--max-old-space-size=4096 npm run build

# 3. Publish dist/ (Render: trigger manual deploy from GOOD_SHA branch or upload artifact)
# Render dashboard: Service → Deploys → Rollback to previous deploy (if available)

# 4. Invalidate CDN cache if fronted by Cloudflare/etc.
```

**Local verification before promote:**

```bash
make lint-frontend
make test-frontend
# Optional smoke against staging indexer:
VITE_INDEXER_URL=https://<staging-indexer> npm run build
```

### Limitations

- Static rollback does **not** undo user transactions broadcast during the bad window.
- `VITE_*` are **baked in at build time** — rolling back without matching env reproduces the old bug if env was the root cause; verify env file against [deploy trace](../templates/deploy-trace.md).
- Browser cache may serve stale `index.html` — HTML is `Cache-Control: no-cache, must-revalidate` ([`docker/frontend/nginx.conf`](../../docker/frontend/nginx.conf)). If a CDN ignores that, purge `/` and `/index.html` (not hashed JS). Long-lived tabs after a Coolify roll recover via one-shot document reload ([GitLab **#706**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/706), [`AGENTS_FRONTEND_LAZY_CHUNK_LOAD.md`](../../skills/AGENTS_FRONTEND_LAZY_CHUNK_LOAD.md)); a 404 of a **new** hash must not be cached as `immutable`.

### Recovery verification

- [ ] `/protocol` shows expected factory/router addresses ([security model § Off-chain trust](../security-model.md#off-chain-trust-boundaries-frontend)).
- [ ] Swap page loads quotes from indexer (`VITE_INDEXER_URL` HTTPS, no mixed content).
- [ ] Route row visible at confirmation ([`AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md`](../../skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md)).
- [ ] Record deploy SHA + UTC in [incident timeline](../templates/incident-dex-indexer.md#incident-timeline).

---

## 2. Indexer incident

### Symptoms / triggers

- Process exit / OOM / panic loop.
- API returns stale or incorrect data (charts, routes, pair list) while LCD shows correct on-chain state.
- Failed `sqlx migrate` on startup after a release.
- Reorg halt (`INDEXER_REORG_HALT`) — see [indexer reorg runbook](./indexer-reorg-replay-dedup.md) (may be chain + indexer combined).

### Decision criteria

| Choose | When |
|--------|------|
| **Restart only** | Crash from transient LCD 429, OOM, or host reboot; **no** schema change in the failing release; data spot-checks match LCD. |
| **Forward-fix** | Bug is in indexer logic but schema is compatible; patch release ready; safe to redeploy binary and catch up from `last_indexed_height`. |
| **Rollback binary** | New release introduced bad parsing, wrong migrations, or data corruption; prior release binary is known-good. |
| **Rollback schema (`down.sql`)** | A migration in the bad release must be reversed **and** a paired `.down.sql` exists **for that ahead version** under [`indexer/migrations/revert/`](../../indexer/migrations/revert/) ([docs/testing.md § Manual rollback SQL](../testing.md#frontend-integration-tests-charts--indexer)). After `down.sql`, `DELETE` **that** `_sqlx_migrations` row (revert files undo schema only; they do **not** touch the ledger). Historical files in `revert/` for versions already in the prior Coolify baseline do **not** select this path. |

### Auto-deploy era (#1276)

Protected-branch auto-deploy on the indexer Coolify app (operator leftover; [ADR 0006](../adr/0006-indexer-health-git-sha.md)) means every indexer-touching `main` land rebuilds [`docker/indexer/Dockerfile`](../../docker/indexer/Dockerfile) and runs `sqlx::migrate!()` before bind. Production `sqlx::migrate!()` in [`indexer/src/main.rs`](../../indexer/src/main.rs) does **not** call `set_ignore_missing` — the default migrator **rejects applied versions missing from the binary**. Integration tests set `set_ignore_missing(true)` only for worktree skew ([`indexer/tests/common/mod.rs`](../../indexer/tests/common/mod.rs)); do **not** paper over a leftover ledger row that way in production. Frontend Vite may already be at a newer tip (**dual-app skew** — expected). Production rollback is the **three-way** Coolify path below, not `git checkout` + `cargo run` on the host. Image binary is `cl8y-dex-indexer`.

Classify **unchanged** vs **ahead** by comparing `_sqlx_migrations` (`version`, `success`; `installed_on` is inspect-only) to the **prior successful Coolify deploy’s** latest `indexer/migrations/*.sql`. Baseline command: `git ls-tree --name-only <prior-coolify-deploys-sha> indexer/migrations/` (or equivalent); that tree’s latest `*.sql` filename prefix (not `revert/`). Coolify Deploys SHA is allowed; still **no** SOURCE SHA log scrape. Do **not** treat `SELECT version … LIMIT 5` or “no new row” as the baseline.

| Choose | When |
|--------|------|
| **Restore previous image (2(a))** | Ledger is **unchanged** vs that prior SHA’s latest `*.sql` (max successful `version` matches; no successful row newer than the baseline). Includes a transactional failure of the **first** new migration that left **no** row. Restore the prior Coolify indexer deploy. While auto-deploy is on and `main` is still the bad SHA, that image is sticky until the next webhook: turn auto-deploy **off** **or** land revert/hotfix before the next `main` land (same retrigger class as 2(c), without re-applying N). Confirm `GET /health` `git_sha` matches that commit via `VERIFY1276_EXPECT_SHA` (prefix OK). **Do not** scrape Coolify SOURCE SHA logs. |
| **Forward-fix (2(b))** | Ledger is **ahead** **and** the next image will still ship N (keep schema; do not restore the previous binary) — **even when** a paired `down.sql` exists. Typical: logic bug with an additive migration. Also required when there is **no** `indexer/migrations/revert/<ahead-version>_*.down.sql` for the version(s) newer than the baseline (historical files in `revert/` do **not** count). Success of `N` then failure of `N+1` is ahead (`N` applied). Snapshot Postgres; ship a hotfix image. Do not invent a revert file. |
| **Documented revert then restore (2(c))** | Ledger is **ahead**, restoring the prior image is required (schema itself is the bug, or no hotfix that still embeds N can ship), **and** `indexer/migrations/revert/<ahead-version>_*.down.sql` exists for the version(s) **newer than the prior Coolify Deploys SHA baseline**. Historical revert files for versions already in that baseline **do not** select 2(c). **Turn auto-deploy off first** ([#297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297)). Then snapshot Postgres → apply **that** `down.sql` (several ahead versions: **descending**) → `DELETE FROM _sqlx_migrations WHERE version = <that version>` (**each** matching row) → restore the prior Coolify indexer image → attest. Files under `revert/` undo schema only; they do **not** `DELETE` `_sqlx_migrations`. Do not invent a revert file. After 2(c), re-enable auto-deploy only when `main` **no longer ships version N** (migration file reverted / not in the binary), **or** when re-applying N is the explicit intent. A logic hotfix that still embeds N is **2(b)** — do not 2(c) then ship that hotfix. |
| **Repair dirty ledger** | Any `_sqlx_migrations` row with `success = false` is dirty. `DELETE FROM _sqlx_migrations WHERE version = <v> AND success = false;` (**only that failed row**). Never `UPDATE success`. Then restore (if now unchanged vs baseline) or forward-fix (if a successful newer version remains). Restore of the prior image still fails while a dirty row remains. This tree’s migrations do not disable transactions; a failed first new migrate often leaves **no** row — that is **unchanged**, not dirty. |
| **Disable auto-deploy temporarily** | Breaking / rewrite migrations, **and** as the **first** step of documented revert (2(c)), **and** before a 2(a) restore while `main` is still the bad SHA. Operator Coolify checkbox off ([#297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297)). After 2(c), re-enable only when `main` no longer ships N (or re-applying N is explicit intent). Expand-only migrations may ride auto-deploy. |

#### Coolify incident checklist (production indexer)

Use this table from the mermaid `idx_schema` branch. Do not publish Coolify UUIDs, tokens, or hosts.

1. **Inspect `_sqlx_migrations` against the prior successful Coolify deploy.** Query `SELECT version, success, installed_on FROM _sqlx_migrations ORDER BY version DESC;` via the **Coolify Postgres shell** or `psql` using the **indexer app** `DATABASE_URL` secret (the DB the running Coolify indexer uses). `installed_on` is inspect-only — classify on `version` / `success`. Do **not** use `scripts/lib/postgres-psql.sh` here — that helper is host/`docker compose exec` for **LocalTerra/dev** Postgres, not Coolify-provisioned production ([`mainnet-soft-launch.md`](./mainnet-soft-launch.md): Postgres is provisioned in Coolify separately). Baseline: Coolify Deploys SHA of the **prior successful** indexer deploy (allowed). `git ls-tree --name-only <prior-coolify-deploys-sha> indexer/migrations/` (or equivalent); that tree’s latest `*.sql` filename prefix is “unchanged.” Still **do not** scrape Coolify SOURCE SHA logs. Classify: **dirty** = any `success = false` (`DELETE FROM _sqlx_migrations WHERE version = <v> AND success = false;` only — never `UPDATE success`; then restore or 2(b); restore of the prior image **fails** while a dirty row remains); **unchanged** = max successful `version` equals the prior SHA’s latest `*.sql` and no successful row is newer (a transactional failure of the **first** new migration that left **no** row is unchanged, not dirty); **ahead** = a successful `version` newer than that baseline (success of `N` then failure of `N+1` is ahead — `N` applied).
2. **Three-way action.** (a) Schema **unchanged** vs that baseline → **restore the previous Coolify indexer deploy**. While auto-deploy is on and `main` is still the bad SHA, that image is sticky until the next webhook: turn auto-deploy **off** **or** land revert/hotfix before the next `main` land. (b) Schema **ahead** and the next image will still ship N (keep schema) **or** there is **no** `indexer/migrations/revert/<ahead-version>_*.down.sql` for the version(s) newer than the baseline → snapshot Postgres and ship a **hotfix Coolify image** (do not restore the previous binary). Historical files in `revert/` do **not** select 2(c). (c) Schema **ahead**, restoring the prior image is required (schema itself is the bug, or no hotfix that still embeds N can ship), **and** a paired revert file exists for **that ahead version** → **turn auto-deploy off first** ([#297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297)) → snapshot Postgres → apply **that** `down.sql` (several ahead versions: **descending**) → `DELETE FROM _sqlx_migrations WHERE version = <that version>` (**each** matching row) → restore the prior Coolify indexer image → attest. Files under `revert/` undo schema only; they do **not** `DELETE` `_sqlx_migrations`. Do not invent a revert file. Do **not** set `set_ignore_missing(true)` on production `sqlx::migrate!()`. Local `git checkout` + `cargo run` / systemd is a **dev** path only.
3. **Attest `/health` `git_sha`.** `VERIFY1276_REQUIRE_LIVE=1 VERIFY1276_EXPECT_SHA=<restored-or-hotfix-sha> make verify-issue-1276` (or `curl` + jq). Prefix-match. Do not scrape Coolify SOURCE SHA logs. Do not infer the auto-deploy checkbox from HTTP.
4. **Re-enable auto-deploy** after 2(c) only when `main` **no longer ships version N** (migration file reverted / not in the binary), **or** when re-applying N is the explicit intent (#297). A typical hotfix still embeds N — that is the **2(b)** path; do not 2(c) then ship that hotfix. Leaving the flag on after 2(c) retriggers the bad tip and re-applies `N`. After 2(a), same: auto-deploy off or a good tip before the next `main` webhook.
5. **Record** the restored/hotfix SHA and UTC in the [incident timeline](../templates/incident-dex-indexer.md#incident-timeline). Close leftover #1276 still needs the ADR slice 3 comment template (not this incident path).

CAC drain (one UUID per Forgejo path) is **not** the indexer rollback/redeploy path.

### Rollback path (commands)

**Production (Coolify — leftover path):** follow **Auto-deploy era → Coolify incident checklist** above (**three-way**). Do not treat `git checkout` + `cargo run` as the production rollback.

**Local / systemd (dev):**

```bash
# 1. Stop indexer (systemd, k8s, or tmux)
# systemctl stop cl8y-indexer

# 2. Note current migration ledger (compare to prior SHA latest *.sql — not LIMIT 5 alone)
#    Baseline: git ls-tree --name-only <prior-sha> indexer/migrations/
#    installed_on is inspect-only
source indexer/.env
psql "$DATABASE_URL" -X -c "SELECT version, success, installed_on FROM _sqlx_migrations ORDER BY version DESC;"

# 3. If the bad release ran a new migration, apply manual down.sql ONLY when documented
# Example (adjust filename / version to the migration being reverted):
psql "$DATABASE_URL" -X -v ON_ERROR_STOP=1 \
  -f indexer/migrations/revert/20260509160000_limit_order_placement_lifecycle.down.sql
# Revert files undo schema only — they do NOT DELETE _sqlx_migrations.
# Production sqlx::migrate!() rejects applied versions missing from the binary.
psql "$DATABASE_URL" -X -v ON_ERROR_STOP=1 \
  -c "DELETE FROM _sqlx_migrations WHERE version = 20260509160000;"
# Dirty failed row only (never UPDATE success):
# psql "$DATABASE_URL" -X -v ON_ERROR_STOP=1 \
#   -c "DELETE FROM _sqlx_migrations WHERE version = <v> AND success = false;"
# Several ahead versions: apply down.sql descending; DELETE each matching ledger row.

# 4. Deploy prior release binary (dev only — production uses Coolify restore).
# Image / cargo binary is cl8y-dex-indexer (docker/indexer/Dockerfile), not cl8y-indexer.
export PATH="/usr/local/cargo/bin:$PATH"
git checkout "<prior-release-sha>"
cd indexer && cargo build --release
# Install binary to service path, e.g. cp target/release/cl8y-dex-indexer /usr/local/bin/

# 5. Restart and watch logs
cd indexer && cargo run --release
# Or: systemctl start cl8y-indexer
```

**Reorg-specific recovery** (not a version rollback — cursor reset):

```bash
./scripts/indexer-reorg-recover.sh --height <FORK_HEIGHT> --cleanup-derived --apply
```

See [indexer reorg runbook § Shallow reorg recovery](./indexer-reorg-replay-dedup.md#shallow-reorg-recovery-1–few-blocks).

### Limitations

- Most migrations have **no** automatic down path — rolling back binary without `down.sql` for the **ahead** version leaves schema ahead of code (startup may fail). Historical files in `revert/` are not a 2(c) selector.
- Derived tables (swaps, charts) may need rebuild from chain replay after a bad ingest window.
- `down.sql` may **drop data** — take a Postgres snapshot before applying.
- Files under `indexer/migrations/revert/` do **not** `DELETE` `_sqlx_migrations`. After `down.sql`, delete **each** matching ledger row (`DELETE FROM _sqlx_migrations WHERE version = <that version>`) or the restored prior image exits on migrate validation (applied version missing from the binary). Several ahead versions: revert **descending**. Do **not** set `set_ignore_missing(true)` in production.
- A dirty `success = false` row is not “unchanged.” `DELETE FROM _sqlx_migrations WHERE version = <v> AND success = false;` only. Never `UPDATE success`. A failed first new migrate with **no** row is unchanged, not dirty.
- After 2(c), re-enable Coolify auto-deploy only when `main` **no longer ships N** (or re-applying N is explicit intent). A hotfix that still embeds N is **2(b)** — do not 2(c) then ship that hotfix.
- After 2(a), a prior Coolify image is sticky until the next webhook while auto-deploy is on and `main` is still the bad SHA. Turn auto-deploy off **or** land revert/hotfix before the next `main` land.
- Indexer rollback does **not** fix on-chain state; pair pause may still be required if users acted on bad off-chain quotes.

### Recovery verification

```bash
curl -sS "${INDEXER_URL:-http://127.0.0.1:3001}/health" | jq .
curl -sS "${INDEXER_URL}/api/v1/pairs?limit=3" | jq '.items[0].pair_address'
# Compare reserves to LCD for a sample pair:
terrad query wasm contract-state smart "<pair_addr>" '{"pool":{}}' --node "$LCD_URL" | jq '.data'
```

- [ ] `/health` returns OK (`status=ok`); when the image bakes a commit, `git_sha` is lowercase hex 7–40 matching the restored/hotfix commit (prefix OK; compare with `VERIFY1276_EXPECT_SHA`) — omit means unset/rejected env, not a substitute for Coolify log scrape ([#1276](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1276)). `VERIFY1276_REQUIRE_LIVE=1` without IID/`EXPECT_SHA` is bake presence. Sibling leftover IID is unreachable-fail. `VERIFY1276_IID=1276` without `EXPECT_SHA` **FAIL**s before curl (intentional leftover-complete gate). Leftover-complete still needs checkbox evidence **and** `VERIFY1276_EXPECT_SHA` tip-match ([ADR 0006](../adr/0006-indexer-health-git-sha.md)).
- [ ] Block lag acceptable vs chain head.
- [ ] Spot-check pair reserves and recent swaps against LCD.
- [ ] No `INDEXER_REORG_HALT` in logs after recovery.
- [ ] Record binary SHA, migration actions, and UTC in incident timeline.

---

## 3. Contract incident

### Symptoms / triggers

- Logic bug discovered **after** wasm migrate (incorrect fees, broken limits, hook dispatch).
- Exploit or abnormal drain on a pair (may also need [blacklist decision](./blacklist-decision.md)).
- Migration left contract in inconsistent state (failed mid-governance).

### Decision criteria

| Choose | When |
|--------|------|
| **Emergency pause** | Active loss, exploit in progress, or unknown scope — [pause pair](./emergency-commands.md#1-pause-a-pair) or blacklist per [blacklist decision](./blacklist-decision.md). |
| **Forward-fix migrate** | Bug is fixable in new wasm; `Migrate` preserves state; governance can execute quickly; **no** irreversible state corruption. |
| **Migrate back to prior `code_id`** | Prior wasm is still **stored on chain**; migration path is reversible; post-migrate state is compatible with old code (verify in `cw-multi-test` / staging). |
| **Pause-and-wait** | Fix requires audit or multisig delay; funds are contained by pause; communicate per [incident comms](../templates/incident-dex-indexer.md#appendix-communications-templates-sec-g05). |

### Rollback path (commands)

**Pause first when in doubt:**

```bash
# See emergency-commands.md — export FACTORY_ADDR, PAIR_ADDR, GOVERNANCE_KEY, etc.
terrad tx wasm execute "$FACTORY_ADDR" "$(jq -nc \
  --arg pair "$PAIR_ADDR" \
  '{set_pair_paused:{pair:$pair,paused:true}}')" \
  --from "$GOVERNANCE_KEY" --chain-id "$CHAIN_ID" --node "$NODE" \
  --gas auto --gas-adjustment 1.4 --fees 500000uluna -y
```

**Forward-fix migrate** (preferred when state-compatible):

```bash
# Store optimized wasm (workspace-optimizer only — see wasm-admin-migration.md)
terrad tx wasm store artifacts/cl8y_dex_pair.wasm \
  --from <wallet> --gas auto --gas-adjustment 1.4 \
  --fees 500000uluna --chain-id <chain-id> --node <rpc-url>

terrad tx wasm migrate <pair_addr> <new_code_id> '{}' \
  --from <admin> --gas auto --gas-adjustment 1.4 \
  --fees 500000uluna --chain-id <chain-id> --node <rpc-url>
```

**Migrate back** to prior `code_id` (only when limitations below are satisfied):

```bash
# Prior code_id must still exist on chain:
terrad query wasm list-code --node <lcd> | jq '.code_infos[] | select(.code_id=="<prior_code_id>")'

terrad tx wasm migrate <pair_addr> <prior_code_id> '{}' \
  --from <admin> --gas auto --gas-adjustment 1.4 \
  --fees 500000uluna --chain-id <chain-id> --node <rpc-url>
```

Full checklist: [wasm admin migration](./wasm-admin-migration.md). Regression: `make test-contracts` / `migration_tests`.

### Limitations

- **CosmWasm cannot delete** uploaded code; old `code_id` remains only if still on chain and admin retained.
- **Admin key loss** or admin transfer without backup blocks migrate-back.
- State written by a **new** migration may be **incompatible** with older wasm — migrate-back can brick the contract; prefer pause + forward-fix after staging proof.
- Factory/router/pair upgrades are **governance-gated** — rollback is not a single-button k8s deploy.
- User funds in limit-order escrow and LP positions remain on-chain during pause — rollback does not automatically return them.

### Recovery verification

```bash
terrad query wasm contract-state smart "$PAIR_ADDR" '{"is_paused":{}}' --node "$LCD" | jq '.data'
terrad query wasm contract <pair_addr> --node <lcd> | jq '.contract_info.code_id'
make test-contracts   # or staging smoke against migrated addr
```

- [ ] Paused pairs unpause only after [unpause prerequisite checklist](./emergency-commands.md#2-unpause-a-pair) (SEC-G07).
- [ ] Post-migrate queries match expected state (pool, fees, hooks).
- [ ] Update [deploy trace](../templates/deploy-trace.md) with new `code_id` and git SHA.
- [ ] Record governance tx hashes in incident timeline.

---

## 4. Chain dependency incident

### Symptoms / triggers

- Terra Classic **chain upgrade** breaks LCD/RPC queries or wasm execution semantics.
- **IBC-hooks** or SDK patch required on the network ([SEC-D02](../security-model.md#ibc-hooks-chain-dependency-sec-d02), [#407](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/407)).
- Primary **LCD/RPC provider** outage or rate-limit storm (indexer and frontend degraded).
- Validator halt or consensus failure (all txs fail).

### Decision criteria

| Choose | When |
|--------|------|
| **Fail over LCD/RPC** | Alternate healthy endpoints exist; contracts unchanged; trading is safe once clients reconnect. |
| **Wait for chain upgrade** | Validators scheduled a patch; no protocol workaround; **pause** if trading on vulnerable chain build is unsafe. |
| **Pause all affected pairs** | IBC-hooks or SDK bug exposes bridged assets used in pools; cannot mitigate off-chain; until network patch is live. |
| **Update indexer `LCD_URLS` + frontend `VITE_TERRA_*`** | Provider-specific outage; chain consensus healthy on another endpoint. |

### Coordination path

1. **Monitor** validator / Terra Classic community channels for upgrade height and binary version.
2. **Record** chain version on launch issue — re-run [launch-checklist Phase 0 IBC-hooks gate](./launch-checklist.md#phase-0--preconditions):

   ```bash
   terrad version --long --node <lcd>
   make verify-no-ibc-hooks-in-contracts
   make verify-issue-407
   ```

3. **Pause** high-TVL pairs first — [quick pool triage](./emergency-commands.md#quick-pool-triage-sec-g03).
4. **Coordinate** with other operators if shared infrastructure (public RPC) is attacked — switch to dedicated nodes.
5. **Communicate** user impact via [incident comms templates](../templates/incident-dex-indexer.md#appendix-communications-templates-sec-g05).

### Rollback path (commands)

There is **no operator rollback of chain state**. Mitigations are **failover** and **trading halt**:

```bash
# Fail over indexer LCD (indexer/.env or host env)
export LCD_URLS="https://<backup-lcd-1>,https://<backup-lcd-2>"
# Restart indexer after env change

# Fail over frontend build-time endpoints (requires rebuild + redeploy)
# VITE_TERRA_LCD_URL / VITE_TERRA_RPC_URL in production secret store

# Trading halt — pause top pools (repeat per pair or use governance playbook)
terrad tx wasm execute "$FACTORY_ADDR" "$(jq -nc \
  --arg pair "$PAIR_ADDR" \
  '{set_pair_paused:{pair:$pair,paused:true}}')" \
  --from "$GOVERNANCE_KEY" --chain-id "$CHAIN_ID" --node "$NODE" \
  --gas auto --gas-adjustment 1.4 --fees 500000uluna -y
```

### Limitations

- Chain upgrades are **validator-driven** — DEX operators cannot revert a passed governance proposal.
- Pausing pairs does **not** protect funds already bridged via a vulnerable IBC-hooks path on **other** protocols.
- LCD failover may serve **lagging** or **forked** nodes — verify block height and hash against a second source before unpause.

### Recovery verification

- [ ] `terrad status --node <rpc>` shows syncing `false` and advancing height.
- [ ] `make verify-no-ibc-hooks-in-contracts` still passes for app wasm posture.
- [ ] Indexer `last_indexed_height` within acceptable lag of chain head.
- [ ] Frontend and indexer use healthy LCD/RPC; sample swap simulation succeeds.
- [ ] Unpause only per [SEC-G07](./emergency-commands.md#2-unpause-a-pair) checklist after chain patch confirmed.

---

## Doc invariant (SEC-H09)

```bash
make check-rollback-decision-docs
make verify-issue-445
```
