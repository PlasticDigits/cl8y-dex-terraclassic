# LocalTerra — Terra Classic SDK 0.53 / terrad v4

Operator and agent reference for the **LocalTerra** stack after [GitLab **#292**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/292).

## Pinned image

| Field | Value |
| ----- | ----- |
| Registry | `ghcr.io/plasticdigits/localterra-cl8y` |
| Tag (human) | `:latest` |
| Digest (compose) | `sha256:29e2d125c123a230aac86b72e781b33db3f357f00fa44c751249a9a2c2512faf` |
| `terrad version` | **4.0.1** |
| Cosmos SDK | **0.53.6** (`cosmos_sdk_version` in `terrad version --long`) |
| Wasm | `wasmd` **0.61.8**, `wasmvm` **v3.0.3** |
| Chain ID | `localterra` (unchanged) |

Bump procedure: [`docs/local-development.md`](./local-development.md) § Docker Setup (`docker pull` + `docker inspect` → update `docker-compose.yml`).

## Invariants

| ID | Invariant | Rationale |
| -- | --------- | --------- |
| **LT1** | Compose pins **digest**, not floating `:latest` | Reproducible QA / CI agents ([#292](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/292)) |
| **LT2** | Mounted [`docker/init-chain.sh`](../docker/init-chain.sh) matches image SDK **0.53** genesis (`bond_denom`, gov `min_deposit`, gentx commission flags) | SDK 0.53 defaults `bond_denom` to `stake` without patch |
| **LT3** | Oracle slash patch (**LT3a**) runs when `app_state.oracle.params` exists | Single-validator localnet has no oracle feeder; validator jails at ~100800 without patch |
| **LT4** | Genesis test account: **1M LUNC** + stablecoins per `init-chain.sh` | CW20 / E2E funding uses deploy + `e2e-provision-dev-wallet.sh`, not 100M LUNC genesis |
| **LT5** | DEX wasm stays **cosmwasm-std 1.5.x** until a deliberate contract bump | Runtime accepts 1.5 artifacts on LocalTerra wasmvm v3 ([#292](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/292) acceptance) |
| **LT6** | After digest bump, **`make reset && make start && make wait-healthy`** before deploy | Stale `localterra-data` volumes are pre–SDK-53 state ([#202](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/202)) |
| **LT7** | `terrad query tx` wasm events live at **`.events`** (not `.logs[0].events`) | Use [`scripts/lib/terrad-tx-events.sh`](../scripts/lib/terrad-tx-events.sh) in deploy/e2e scripts |
| **LT8** | Treasury bank send uses **`DEPLOY_TREASURY_FUND_COINS`** default **2M USTC + 200k LUNC** | Genesis is **1M LUNC**; legacy 10M LUNC send fails on SDK 0.53 LocalTerra |
| **LT9** | E2E `global-setup.ts` falls back to **`docker exec` LCD** when host `:1317` fetch times out | Same userland-proxy pattern as [`scripts/lib/localterra-host-curl.sh`](../scripts/lib/localterra-host-curl.sh) |
| **LT10** | Indexer integration wiremocks for tx search use **`query` + `page` + `limit`** (not legacy `events=` / `pagination.offset`) | Matches terrad v4 `GetTxsEvent`; see `indexer/src/lcd/mod.rs` tests and `indexer/tests/indexer_ingestion_hardening.rs` |
| **LT11** | Strict E2E: **one** `make deploy-local` per chain volume, then **`bash scripts/e2e-start-indexer.sh`** before `make test-e2e` | `make deploy-local` does not restart the indexer ([#325](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/325)); a second deploy on the same volume orphans prior pair rows. **`make test-e2e-tx`** runs `deploy-dex-local.sh` again — prefer CI order below or `make reset-qa` first |
| **LT12** | Playwright browsers: install via **locked** `frontend-dapp` dependency (`./node_modules/.bin/playwright install chromium`), not bare `npx playwright` | Installs **chromium** and **chromium_headless_shell** for the pinned `@playwright/test` revision; partial install → `Executable doesn't exist at …chromium_headless_shell-1208…`. `make setup-cloud-localterra` runs this step after deploy (**LT12**) |

## Verification commands

```bash
make reset && make start && make wait-healthy
make build-optimized && make deploy-local
(cd indexer && cargo build --release)   # required before e2e-start-indexer on fresh VM
bash scripts/e2e-start-indexer.sh   # after deploy; see LT11
bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright install chromium  # LT12
make test-contracts
make test-frontend
make test-qa-verify-deploy
make test-indexer-integration
# Cloud Agent: sg docker -c 'CI=1 make test-e2e'  (docker group for global-setup scripts)
make test-e2e                     # smoke (5 workers) + e2e-tx (1 worker); CI reference job
```

**Do not** run `make test-e2e-tx` after an earlier `make deploy-local` on the same volumes without restarting the indexer (LT11) or wiping Postgres (`make reset-qa`).

### #292 acceptance status (SDK 53 scope vs separate issues)

| Layer | Command | SDK 53 / v4 scope |
| ----- | ------- | ----------------- |
| Infra + deploy | `reset` → `start` → `wait-healthy` → `deploy-local` | **In scope** — digest pin, terrad **4.0.1**, wasm upload/instantiate/execute on wasmvm v3 |
| Unit | `make test-contracts`, `make test-indexer-integration` | **In scope** |
| QA | `make test-qa-verify-deploy` | **In scope** |
| Frontend unit | `make test-frontend` | **Out of scope** when failures are tracked elsewhere (e.g. [#293](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/293) slippage display) |
| Strict E2E | `make test-e2e` | **Mixed** — on-chain tx specs exercise v4 signing/broadcast; smoke UI regressions ([#186](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/186), [#178](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/178), [#179](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/179)) are not SDK 53 blockers |

## Agent playbooks

- E2E / strict chain: [`skills/AGENTS_E2E_STRICT_CHAIN.md`](../skills/AGENTS_E2E_STRICT_CHAIN.md)
- Gas / wallets on LocalTerra: [`skills/AGENTS_TERRACLASSIC_GAS.md`](../skills/AGENTS_TERRACLASSIC_GAS.md)
- QA deploy verify: [`skills/AGENTS_QA_DEPLOY_VERIFY.md`](../skills/AGENTS_QA_DEPLOY_VERIFY.md)
- Fresh volumes: [`skills/AGENTS_QA_FRESH_VOLUMES.md`](../skills/AGENTS_QA_FRESH_VOLUMES.md)

## Related

- [Local development](./local-development.md)
- [Environment matrix](./environment-matrix.md)
- [QA invariants](./qa-invariants.md)
- [Testing](./testing.md)
