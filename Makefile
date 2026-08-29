.PHONY: front send start stop restart reset build-contracts build-artifacts-cargo build-optimized deploy-local deploy-local-no-build deploy-testnet deploy-mainnet deploy-mainnet-soft-launch deploy-soft-launch-faucet test-mainnet-soft-launch-defaults dev dev-full indexer-dev build-indexer-release fetch-qa-ci-artifacts test-contracts coverage-contracts test-frontend test-frontend-charts test-e2e test-e2e-tx test-e2e-indexer-outage test-charts-integration tests-charts-integration lint check-fee-discount-tier-docs setup-hooks test-commit-msg-hook wait-localterra wait-healthy has-localterra help compose-ps start-qa qa-start stop-qa reset-qa test-qa-fresh-volumes test-qa-verify-deploy test-qa-redeploy-decision test-localterra-host-curl test-has-localterra test-indexer-target-ownership test-setup-postgres test-setup-browser qa-tunnel-help qa-verify-deploy verify-issue-238 verify-issue-245 verify-issue-274 verify-issue-276 verify-issue-285 verify-issue-293 verify-issue-309 verify-issue-313 verify-issue-295 verify-issue-324 verify-issue-503 check-ust1-wrap-ops-health verify-issue-504 verify-issue-514 verify-issue-518 verify-issue-533 verify-issue-559 verify-issue-539 verify-issue-534 verify-issue-562 verify-issue-542 verify-issue-536 verify-issue-582 verify-issue-584 verify-issue-585 verify-issue-587 verify-issue-679 verify-issue-599 verify-issue-600 verify-issue-589 verify-issue-581 verify-issue-590 verify-issue-537 verify-issue-538 verify-issue-541 verify-issue-664 verify-issue-547 verify-issue-485 verify-issue-515 verify-issue-550 verify-issue-522 verify-issue-551 verify-issue-557 verify-issue-556 verify-issue-560 verify-issue-675 verify-issue-524 verify-issue-543 verify-issue-548 verify-issue-553 verify-issue-656 verify-issue-564 verify-issue-527 verify-issue-561 verify-issue-563 verify-issue-528 verify-issue-529 verify-issue-530 verify-issue-517 verify-issue-519 verify-issue-554 verify-issue-566 verify-issue-658 verify-issue-567 verify-issue-573 verify-issue-531 verify-issue-501 verify-issue-596 verify-issue-506 verify-issue-678 verify-issue-512 verify-issue-516 verify-issue-523 verify-issue-508 verify-issue-384 verify-issue-475 swarm-local swarm-launch swarm-stop test-swarm-liquidity swarm-bootstrap-liquidity setup-cloud-localterra setup-indexer-postgres test-indexer-integration rebalance-mint-ust1-lp verify-issue-565 verify-issue-666 verify-issue-576 verify-issue-578 verify-issue-695 verify-issue-579 verify-issue-577 verify-issue-568 verify-issue-569 verify-issue-586 verify-issue-683 verify-issue-613 verify-issue-614 verify-issue-580 verify-issue-571 verify-issue-570 verify-issue-595 verify-issue-592 verify-issue-601 verify-issue-593 verify-issue-594 verify-issue-602 verify-issue-612 verify-issue-616 verify-issue-606 verify-issue-607 verify-issue-615 verify-issue-608 verify-issue-609 verify-issue-610 verify-issue-620 verify-issue-622 verify-issue-623 verify-issue-653 verify-issue-665 verify-issue-671 verify-issue-668 verify-issue-689 verify-issue-677 verify-issue-661 verify-issue-672 verify-issue-663
# Infrastructure
start:
	docker compose up -d
	@chmod +x scripts/setup-postgres-dev-databases.sh scripts/lib/upsert-dotenv.sh scripts/lib/postgres-psql.sh scripts/lib/postgres-bootstrap-role.sh
	@./scripts/setup-postgres-dev-databases.sh || true

stop:
	docker compose down

restart: stop start

reset:
	docker compose down -v

status:
	@chmod +x scripts/status.sh scripts/lib/cloud-agent-docker.sh scripts/lib/localterra-host-curl.sh
	./scripts/status.sh

compose-ps:
	docker compose ps

logs:
	docker compose logs -f

logs-terra:
	docker compose logs -f localterra

swarm-local:
	@chmod +x scripts/bots/swarm.py
	python3 scripts/bots/swarm.py

# 34 processes when SWARM_TAX_WORKERS=1 (25 swap + 5 limit + 3 lp + 1 tax); see scripts/bots/launch-swarm.sh
swarm-launch:
	@chmod +x scripts/bots/launch-swarm.sh scripts/bots/bootstrap-swarm-liquidity.sh
	./scripts/bots/launch-swarm.sh

swarm-bootstrap-liquidity:
	@chmod +x scripts/bots/bootstrap-swarm-liquidity.sh
	./scripts/bots/bootstrap-swarm-liquidity.sh

test-swarm-liquidity:
	cd scripts/bots && python3 -m unittest test_swarm_liquidity.py test_swarm_tax.py -v

swarm-stop:
	@chmod +x scripts/bots/stop-swarm.sh
	./scripts/bots/stop-swarm.sh

wait-localterra:
	@chmod +x scripts/wait-localterra.sh scripts/lib/localterra-host-curl.sh
	@./scripts/wait-localterra.sh

# Agent probe: LocalTerra RPC up? (sg docker + exec fallback — safe on Cloud Agent VMs)
has-localterra:
	@chmod +x scripts/has-localterra.sh scripts/lib/localterra-host-curl.sh scripts/lib/cloud-agent-docker.sh
	@./scripts/has-localterra.sh

wait-healthy: wait-localterra
	@echo "Waiting for Postgres..."
	@chmod +x scripts/setup-postgres-dev-databases.sh scripts/lib/postgres-psql.sh scripts/lib/postgres-bootstrap-role.sh
	@for i in $$(seq 1 30); do \
		if pg_isready -h localhost -U $${POSTGRES_USER:-cl8y_legal} > /dev/null 2>&1 \
			|| docker compose exec -T postgres pg_isready -U $${POSTGRES_USER:-cl8y_legal} > /dev/null 2>&1; then \
			echo "Postgres is ready!"; \
			./scripts/setup-postgres-dev-databases.sh || true; \
			break; \
		fi; \
		if [ "$$i" -eq 30 ]; then \
			echo "ERROR: Postgres did not start in time."; \
			exit 1; \
		fi; \
		sleep 2; \
	done

# QA server: LocalTerra + deploy + indexer; see scripts/qa/README.md
start-qa:
	@chmod +x scripts/qa/start-qa.sh scripts/qa/stop-qa.sh scripts/qa/reset-qa.sh scripts/qa/lib/print-fresh-volumes-banner.sh scripts/qa/print-qa-tunnel-instructions.sh scripts/qa/write-frontend-env-local.sh
	./scripts/qa/start-qa.sh

qa-start: start-qa

# Wipe LocalTerra + Postgres volumes then run start-qa (post-contract-change QA).
reset-qa:
	@chmod +x scripts/qa/reset-qa.sh scripts/qa/start-qa.sh scripts/qa/stop-qa.sh scripts/qa/lib/print-fresh-volumes-banner.sh scripts/qa/print-qa-tunnel-instructions.sh scripts/qa/write-frontend-env-local.sh
	./scripts/qa/reset-qa.sh

test-qa-fresh-volumes:
	@chmod +x scripts/qa/test-qa-fresh-volumes.sh
	./scripts/qa/test-qa-fresh-volumes.sh

test-qa-verify-deploy:
	@chmod +x scripts/qa/test-verify-deploy.sh scripts/qa/test-localterra-host-curl.sh scripts/lib/lcd-smart-query.sh scripts/lib/localterra-host-curl.sh
	./scripts/qa/test-verify-deploy.sh

test-qa-verify-deploy-config:
	@chmod +x scripts/qa/test-verify-deploy-config.sh scripts/lib/lcd-smart-query.sh
	./scripts/qa/test-verify-deploy-config.sh

test-qa-verify-env-addresses:
	@chmod +x scripts/qa/test-verify-env-addresses.sh scripts/lib/lcd-smart-query.sh
	./scripts/qa/test-verify-env-addresses.sh

test-qa-redeploy-decision:
	@chmod +x scripts/qa/test-qa-redeploy-decision.sh
	./scripts/qa/test-qa-redeploy-decision.sh

test-localterra-host-curl:
	@chmod +x scripts/qa/test-localterra-host-curl.sh scripts/lib/localterra-host-curl.sh
	./scripts/qa/test-localterra-host-curl.sh

test-has-localterra:
	@chmod +x scripts/test-has-localterra.sh scripts/has-localterra.sh scripts/lib/localterra-host-curl.sh
	./scripts/test-has-localterra.sh

test-indexer-target-ownership:
	@chmod +x scripts/test-indexer-target-ownership.sh scripts/lib/docker-indexer-bind-mount.sh scripts/test-charts-integration.sh
	./scripts/test-indexer-target-ownership.sh

test-setup-postgres:
	@chmod +x scripts/test-setup-postgres-dev-databases.sh scripts/lib/postgres-bootstrap-role.sh scripts/lib/postgres-psql.sh scripts/setup-postgres-dev-databases.sh scripts/setup-cloud-agent-indexer-postgres.sh
	./scripts/test-setup-postgres-dev-databases.sh

test-setup-browser:
	@chmod +x scripts/test-setup-browser-cloud-agent.sh scripts/setup-browser-cloud-agent.sh scripts/lib/keplr-chrome-extension.sh
	./scripts/test-setup-browser-cloud-agent.sh

test-setup-cloud-agent-env:
	@chmod +x scripts/test-setup-cloud-agent-env.sh scripts/setup-cloud-agent-env.sh scripts/cloud-agent-shell-init.sh scripts/lib/cloud-agent-env.sh scripts/lib/cloud-agent-toolchain.sh scripts/lib/cloud-agent-docker.sh scripts/with-node.sh scripts/setup-glab-cloud-agent.sh
	./scripts/test-setup-cloud-agent-env.sh

setup-cloud-agent-env:
	@chmod +x scripts/setup-cloud-agent-env.sh scripts/setup-cloud-agent-toolchain.sh scripts/setup-cloud-agent-localterra.sh scripts/setup-browser-cloud-agent.sh scripts/cloud-agent-shell-init.sh scripts/lib/cloud-agent-env.sh scripts/lib/cloud-agent-toolchain.sh scripts/lib/cloud-agent-docker.sh scripts/with-node.sh scripts/setup-glab-cloud-agent.sh
	./scripts/setup-cloud-agent-env.sh

qa-tunnel-help:
	@chmod +x scripts/qa/print-qa-tunnel-instructions.sh scripts/qa/write-frontend-env-local.sh
	./scripts/qa/print-qa-tunnel-instructions.sh

stop-qa:
	@chmod +x scripts/qa/stop-qa.sh
	./scripts/qa/stop-qa.sh

qa-verify-deploy:
	@chmod +x scripts/qa/verify-deploy.sh scripts/qa/verify-env-addresses.sh scripts/lib/lcd-smart-query.sh scripts/lib/localterra-host-curl.sh
	./scripts/qa/verify-deploy.sh

qa-verify-env-addresses:
	@chmod +x scripts/qa/verify-env-addresses.sh scripts/lib/lcd-smart-query.sh scripts/lib/localterra-host-curl.sh
	./scripts/qa/verify-env-addresses.sh

qa-verify-deploy-config:
	@chmod +x scripts/qa/verify-deploy-config.sh scripts/lib/lcd-smart-query.sh scripts/lib/localterra-host-curl.sh
	./scripts/qa/verify-deploy-config.sh

# Post-deploy pool LCD smoke — reads PAIR_ADDR / OFFER_TOKEN from deploy stamp (GitLab #368 / #86).
smoke-pool-swap:
	@chmod +x scripts/smoke-pool-swap.sh scripts/lib/smoke-deploy-env.sh
	@bash -c 'set -a; source scripts/lib/smoke-deploy-env.sh; set +a; ./scripts/smoke-pool-swap.sh'

# Post-deploy wrap-mapper pause/unpause on-chain smoke (GitLab #396 / SEC-B06).
smoke-wrap-mapper-pause:
	@chmod +x scripts/smoke-wrap-mapper-pause.sh scripts/lib/smoke-wrap-env.sh \
		scripts/lib/lcd-smart-query.sh scripts/lib/e2e-terrad-tx.sh scripts/lib/terrad-wait-tx.sh
	@bash -c 'set -a; source scripts/lib/smoke-wrap-env.sh; set +a; ./scripts/smoke-wrap-mapper-pause.sh'

# On-chain E2E for GitLab #238 (hybrid sim CL8Y fee-discount parity). Requires a
# fresh deploy (make deploy-local) + running indexer for the route/solve check.
verify-issue-238:
	@chmod +x scripts/qa/verify-issue-238.sh scripts/lib/lcd-smart-query.sh scripts/lib/localterra-host-curl.sh
	./scripts/qa/verify-issue-238.sh

# GitLab #383 — LocalTerra TCL8Y (18-decimal CL8Y proxy) + tier register/deregister.
verify-issue-383:
	@chmod +x scripts/qa/verify-issue-383.sh scripts/lib/lcd-smart-query.sh scripts/lib/localterra-host-curl.sh
	./scripts/qa/verify-issue-383.sh

# GitLab #384 — fee-discount register/deregister gas limits (FT-3 / FT-4 UI).
verify-issue-384:
	@chmod +x scripts/qa/verify-issue-384.sh scripts/lib/lcd-smart-query.sh scripts/lib/terrad-wait-tx.sh scripts/with-node.sh
	./scripts/qa/verify-issue-384.sh

# GitLab #475 — retail getGasLimitForTx inventory / BASE_GAS_LIMIT guardrail (+ #474 drip).
verify-issue-475:
	@chmod +x scripts/qa/verify-issue-475.sh scripts/lib/terrad-wait-tx.sh scripts/with-node.sh
	./scripts/qa/verify-issue-475.sh

# GitLab #396 — wrap-mapper pause/unpause LocalTerra smoke (SEC-B06) + SEC-A02 Vitest.
verify-issue-396:
	@chmod +x scripts/qa/verify-issue-396.sh scripts/smoke-wrap-mapper-pause.sh scripts/lib/smoke-wrap-env.sh \
		scripts/lib/lcd-smart-query.sh scripts/lib/e2e-terrad-tx.sh scripts/lib/terrad-wait-tx.sh scripts/with-node.sh
	@if docker info >/dev/null 2>&1; then \
		./scripts/qa/verify-issue-396.sh; \
	else \
		sg docker -c './scripts/qa/verify-issue-396.sh'; \
	fi

# GitLab #245 — off-chain trader forwarding (unit tests + optional live stack via #238 script).
verify-issue-245:
	@chmod +x scripts/qa/verify-issue-245.sh scripts/qa/verify-issue-238.sh scripts/lib/lcd-smart-query.sh
	./scripts/qa/verify-issue-245.sh

# GitLab #274 — live LocalTerra gas: CleanLimitBook traversal cap + resume cursor.
verify-issue-274:
	@chmod +x scripts/qa/verify-issue-274.sh scripts/lib/lcd-smart-query.sh scripts/lib/e2e-terrad-tx.sh scripts/lib/localterra-host-curl.sh scripts/e2e-provision-dev-wallet.sh
	./scripts/qa/verify-issue-274.sh

verify-issue-467:
	@chmod +x scripts/qa/verify-issue-467.sh
	./scripts/qa/verify-issue-467.sh

# GitLab #276 — pair-creation fee (contract tests + doc cross-links).
verify-issue-276:
	@chmod +x scripts/qa/verify-issue-276.sh
	./scripts/qa/verify-issue-276.sh

# GitLab #285 — lifecycle emitter scoping (_contract_address only) + live hybrid fill proof.
verify-issue-285:
	@chmod +x scripts/qa/verify-issue-285.sh scripts/lib/lcd-smart-query.sh scripts/lib/localterra-host-curl.sh scripts/lib/e2e-terrad-tx.sh scripts/e2e-start-indexer.sh scripts/e2e-provision-dev-wallet.sh scripts/e2e-seed-hybrid-book.sh
	@if docker info >/dev/null 2>&1; then \
		./scripts/qa/verify-issue-285.sh; \
	else \
		sg docker -c './scripts/qa/verify-issue-285.sh'; \
	fi

# GitLab #309 — live LocalTerra gas: MAX_EXPIRED_PARKS_PER_SWAP vs 15M envelope.
verify-issue-309:
	@chmod +x scripts/qa/verify-issue-309.sh scripts/lib/lcd-smart-query.sh scripts/lib/e2e-terrad-tx.sh scripts/lib/localterra-host-curl.sh scripts/e2e-provision-dev-wallet.sh
	./scripts/qa/verify-issue-309.sh

verify-issue-313:
	@chmod +x scripts/qa/verify-issue-313.sh
	./scripts/qa/verify-issue-313.sh

# GitLab #293 — OE-1 swap hub pairs near-inverse after swarm liquidity bootstrap.
verify-issue-293:
	@chmod +x scripts/qa/verify-issue-293.sh
	./scripts/qa/verify-issue-293.sh

# GitLab #372 — LocalTerra genesis/deploy/swarm funding headroom
verify-localterra-funding-headroom:
	@chmod +x scripts/qa/verify-localterra-funding-headroom.sh scripts/bots/preflight-test1-uluna.sh
	./scripts/qa/verify-localterra-funding-headroom.sh

# GitLab #295 — limit ladder rung count UI (Playwright against make dev on :5173).
verify-issue-295:
	@chmod +x scripts/verify-issue-295-ladder-rung-ui.sh scripts/with-node.sh
	./scripts/verify-issue-295-ladder-rung-ui.sh

# Cloud Agent: dockerd + LocalTerra + deploy + .env.local (+ optional indexer/frontend tmux).
setup-cloud-localterra:
	@chmod +x scripts/setup-cloud-agent-localterra.sh scripts/setup-cloud-agent-indexer-postgres.sh scripts/lib/cloud-agent-toolchain.sh scripts/lib/cloud-agent-docker.sh scripts/with-node.sh
	./scripts/setup-cloud-agent-localterra.sh

# Cloud Agent: Postgres + indexer/.env only — indexer integration tests without wasm deploy (#335).
setup-indexer-postgres:
	@chmod +x scripts/setup-cloud-agent-indexer-postgres.sh scripts/lib/cloud-agent-docker.sh scripts/lib/postgres-psql.sh
	./scripts/setup-cloud-agent-indexer-postgres.sh

test-indexer-integration: setup-indexer-postgres
	@export PATH="/usr/local/cargo/bin:$$HOME/.cargo/bin:$$PATH"; cd indexer && cargo test --tests -j 1 -- --test-threads=1

verify-issue-324:
	@chmod +x scripts/qa/verify-issue-324.sh scripts/setup-cloud-agent-indexer-postgres.sh
	./scripts/qa/verify-issue-324.sh

# GitLab #504 — ExpiredLimitParkReason on parked refund rows (multi-test gate; optional VERIFY504_LCD=1).
verify-issue-504:
	@chmod +x scripts/qa/verify-issue-504.sh scripts/qa/verify-issue-504-lcd.sh
	./scripts/qa/verify-issue-504.sh

# GitLab #514 — limit-order placement discount shift (swap/take unchanged).
verify-issue-514:
	@chmod +x scripts/qa/verify-issue-514.sh scripts/upgrade-514-limit-discount.sh scripts/with-node.sh
	./scripts/qa/verify-issue-514.sh

# GitLab #518 — LP ticker classic charset so UST1 / CL8Y create_pair does not revert.
verify-issue-518:
	@chmod +x scripts/qa/verify-issue-518.sh
	./scripts/qa/verify-issue-518.sh

# GitLab #536 — factory snapshots discount_registry so CreatePair wires new pairs.
verify-issue-536:
	@chmod +x scripts/qa/verify-issue-536.sh
	./scripts/qa/verify-issue-536.sh

# GitLab #582 — listed CW20 code_id pin + write-path whitelist re-check.
verify-issue-582:
	@chmod +x scripts/qa/verify-issue-582.sh
	./scripts/qa/verify-issue-582.sh

# GitLab #584 — F6 factory-first upgrade script, ContractInfo probe, freeze runbook.
verify-issue-584:
	@chmod +x scripts/qa/verify-issue-584.sh scripts/upgrade-582-code-id-pin.sh \
		scripts/qa/probe-columbus5-contract-info.sh scripts/qa/test-upgrade-582-pagination.sh \
		scripts/qa/test-upgrade-582-refresh-events.sh
	./scripts/qa/verify-issue-584.sh

# GitLab #585 — F6 freeze visibility on dApp + indexer route/solve.
verify-issue-585:
	@chmod +x scripts/qa/verify-issue-585.sh scripts/with-node.sh
	./scripts/qa/verify-issue-585.sh

# GitLab #589 — CW20 code-id audit harness (decomp + exploit suite; gates #581).
verify-issue-589:
	@chmod +x scripts/qa/verify-issue-589.sh \
		cw20-codeid-audits/scripts/fetch-lcd-wasm.sh \
		cw20-codeid-audits/scripts/decompile-wasm.sh \
		cw20-codeid-audits/scripts/fingerprint-wasm.sh \
		cw20-codeid-audits/scripts/layer-a-lcd.sh \
		cw20-codeid-audits/scripts/layer-b-lt.sh
	./scripts/qa/verify-issue-589.sh

# GitLab #581 — 8266 full A-lcd/B-lt suite; issuer/Everybody/minter are residuals not veto.
verify-issue-581:
	@chmod +x scripts/qa/verify-issue-581.sh \
		scripts/qa/verify-issue-589.sh \
		cw20-codeid-audits/scripts/layer-a-lcd.sh \
		cw20-codeid-audits/scripts/layer-b-lt.sh \
		cw20-codeid-audits/scripts/lib-layer-lt.sh
	./scripts/qa/verify-issue-581.sh

# GitLab #590 — post-merge !394–!396 ops (fees migrate, wrap gas, 8266 A-lcd/B-lt).
verify-issue-590:
	@chmod +x scripts/qa/verify-issue-590.sh scripts/qa/verify-issue-586.sh \
		scripts/qa/verify-issue-587.sh scripts/qa/verify-issue-589.sh \
		cw20-codeid-audits/scripts/layer-a-lcd.sh \
		cw20-codeid-audits/scripts/layer-b-lt.sh \
		cw20-codeid-audits/scripts/lib-layer-lt.sh
	./scripts/qa/verify-issue-590.sh

# GitLab #587 — wrap+≥2hop LUNC↔USTR gas envelope + Swap Network fee (LUNC).
verify-issue-587:
	@chmod +x scripts/qa/verify-issue-587.sh scripts/with-node.sh
	./scripts/qa/verify-issue-587.sh

# GitLab #679 — mixed hybrid+pool router gas + Swap Network fee (hint = broadcast).
verify-issue-679:
	@chmod +x scripts/qa/verify-issue-679.sh scripts/qa/verify-issue-475.sh \
		scripts/qa/verify-issue-587.sh scripts/qa/verify-issue-596.sh scripts/with-node.sh
	./scripts/qa/verify-issue-679.sh

# GitLab #599 — unwrap+≥2hop USTR→USTC gas combo (taxed uusd InstantWithdraw).
verify-issue-599:
	@chmod +x scripts/qa/verify-issue-599.sh scripts/with-node.sh
	./scripts/qa/verify-issue-599.sh

# GitLab #600 — post-merge !400 LocalTerra E9 + columbus-5 unwrap gas.
verify-issue-600:
	@chmod +x scripts/qa/verify-issue-600.sh scripts/qa/verify-issue-599.sh \
		scripts/qa/verify-issue-587.sh scripts/with-node.sh
	./scripts/qa/verify-issue-600.sh

verify-issue-485:
	@chmod +x scripts/qa/verify-issue-485.sh scripts/setup-cloud-agent-indexer-postgres.sh scripts/with-node.sh
	./scripts/qa/verify-issue-485.sh

# GitLab #515 — ticker-scoped external oracle (ustc/lunc/vfdusd); bare /price is catalog only.
verify-issue-515:
	@chmod +x scripts/qa/verify-issue-515.sh scripts/setup-cloud-agent-indexer-postgres.sh scripts/with-node.sh
	./scripts/qa/verify-issue-515.sh

# GitLab #580 — CEX FDUSD identity under path vfdusd (logs/API; not Terra CW20 vFDUSD).
verify-issue-580:
	@chmod +x scripts/qa/verify-issue-580.sh scripts/setup-cloud-agent-indexer-postgres.sh
	./scripts/qa/verify-issue-580.sh

# GitLab #579 — CoinGecko oracle User-Agent (403 vs 429); no live CoinGecko.
verify-issue-579:
	@chmod +x scripts/qa/verify-issue-579.sh
	./scripts/qa/verify-issue-579.sh

# GitLab #550 — /protocol global USD stats + unified USTC/LUNC/vFDUSD oracle card.
verify-issue-550:
	@chmod +x scripts/qa/verify-issue-550.sh scripts/setup-cloud-agent-indexer-postgres.sh scripts/with-node.sh
	./scripts/qa/verify-issue-550.sh

# GitLab #571 — /protocol vFDUSD tab: FDUSD reference price + Venus 1 vFDUSD Price.
verify-issue-571:
	@chmod +x scripts/qa/verify-issue-571.sh scripts/setup-cloud-agent-indexer-postgres.sh scripts/with-node.sh
	./scripts/qa/verify-issue-571.sh

# GitLab #569 — /protocol total USD pair liquidity + 24h/30d % change.
verify-issue-569:
	@chmod +x scripts/qa/verify-issue-569.sh scripts/setup-cloud-agent-indexer-postgres.sh scripts/with-node.sh
	./scripts/qa/verify-issue-569.sh

# GitLab #586 — /protocol treasury fees 24h/7d/30d + source/token mix.
verify-issue-586:
	@chmod +x scripts/qa/verify-issue-586.sh scripts/setup-cloud-agent-indexer-postgres.sh scripts/with-node.sh
	./scripts/qa/verify-issue-586.sh

# GitLab #683 — /protocol fee USD for CL8Y + factory-listed economic tokens.
.PHONY: verify-issue-683
verify-issue-683:
	@chmod +x scripts/qa/verify-issue-683.sh scripts/setup-cloud-agent-indexer-postgres.sh scripts/with-node.sh
	./scripts/qa/verify-issue-683.sh

# GitLab #652 — /protocol inline Δ% + volume prior-window % + UTC-day series.
.PHONY: verify-issue-652
verify-issue-652:
	@chmod +x scripts/qa/verify-issue-652.sh scripts/setup-cloud-agent-indexer-postgres.sh scripts/with-node.sh
	./scripts/qa/verify-issue-652.sh

# GitLab #667 — /protocol Δ% grouped with headline + integer census.
.PHONY: verify-issue-667
verify-issue-667:
	@chmod +x scripts/qa/verify-issue-667.sh scripts/with-node.sh
	./scripts/qa/verify-issue-667.sh
# GitLab #668 — /protocol UTC volume chart: USD axis, tooltip, Hourly/Daily/Monthly.
.PHONY: verify-issue-668
verify-issue-668:
	@chmod +x scripts/qa/verify-issue-668.sh scripts/setup-cloud-agent-indexer-postgres.sh scripts/with-node.sh
	./scripts/qa/verify-issue-668.sh

# GitLab #689 — /protocol UTC chart Volume / Liquidity / Fees metric toggle.
.PHONY: verify-issue-689
verify-issue-689:
	@chmod +x scripts/qa/verify-issue-689.sh scripts/setup-cloud-agent-indexer-postgres.sh scripts/with-node.sh
	./scripts/qa/verify-issue-689.sh

# GitLab #677 — /protocol leftovers: liquidity 24h-only Δ% + denser UTC volume x-axis.
.PHONY: verify-issue-677
verify-issue-677:
	@chmod +x scripts/qa/verify-issue-677.sh scripts/with-node.sh
	./scripts/qa/verify-issue-677.sh

# GitLab #631 — DeFiLlama UTC-day volume/fees API + adapter copies.
.PHONY: verify-issue-631
verify-issue-631:
	@chmod +x scripts/qa/verify-issue-631.sh scripts/setup-cloud-agent-indexer-postgres.sh
	./scripts/qa/verify-issue-631.sh

# GitLab #687 — DeFiLlama fees headline partial SUM + adapter start / 404 leftover.
.PHONY: verify-issue-687
verify-issue-687:
	@chmod +x scripts/qa/verify-issue-687.sh scripts/setup-cloud-agent-indexer-postgres.sh
	./scripts/qa/verify-issue-687.sh

# GitLab #613 — /protocol Wrap/Unwrap ingest from captured wrap-mapper attrs.
verify-issue-613:
	@chmod +x scripts/qa/verify-issue-613.sh scripts/setup-cloud-agent-indexer-postgres.sh scripts/with-node.sh
	./scripts/qa/verify-issue-613.sh

# GitLab #614 — /protocol UST1 window mint/redeem treasury fees.
verify-issue-614:
	@chmod +x scripts/qa/verify-issue-614.sh scripts/qa/verify-issue-586.sh \
		scripts/setup-cloud-agent-indexer-postgres.sh scripts/with-node.sh
	./scripts/qa/verify-issue-614.sh


# GitLab #577 — token/trader/pair/global trailing-window decay + stale overview freshness.
verify-issue-577:
	@chmod +x scripts/qa/verify-issue-577.sh scripts/setup-cloud-agent-indexer-postgres.sh
	./scripts/qa/verify-issue-577.sh

# GitLab #522 — pair Price (USD): human quote-per-base + oracle USD of 1 human base.
verify-issue-522:
	@chmod +x scripts/qa/verify-issue-522.sh scripts/setup-cloud-agent-indexer-postgres.sh scripts/with-node.sh
	./scripts/qa/verify-issue-522.sh

# GitLab #551 — portfolio/trader P&L human scale + USD totals (drop mixed-unit sums).
verify-issue-551:
	@chmod +x scripts/qa/verify-issue-551.sh scripts/setup-cloud-agent-indexer-postgres.sh scripts/with-node.sh
	./scripts/qa/verify-issue-551.sh

# GitLab #676 — /positions trade_count on 18-decimal pairs (NUMERIC(78,18) + rebuild).
.PHONY: verify-issue-676
verify-issue-676:
	@chmod +x scripts/qa/verify-issue-676.sh scripts/setup-cloud-agent-indexer-postgres.sh scripts/with-node.sh
	./scripts/qa/verify-issue-676.sh

# GitLab #557 — human Amount in / Amount out / Price on Charts/Trade/Trader tape + wallet history.
verify-issue-557:
	@chmod +x scripts/qa/verify-issue-557.sh scripts/setup-cloud-agent-indexer-postgres.sh scripts/with-node.sh
	./scripts/qa/verify-issue-557.sh

# GitLab #556 — DEX hub USD (cUSTC/UST1/USTR) + Protocol DEX card.
verify-issue-556:
	@chmod +x scripts/qa/verify-issue-556.sh scripts/setup-cloud-agent-indexer-postgres.sh scripts/with-node.sh
	./scripts/qa/verify-issue-556.sh

# GitLab #570 — Protocol DEX hub cUSTC wrap link + LUNC/USD column.
verify-issue-570:
	@chmod +x scripts/qa/verify-issue-570.sh scripts/setup-cloud-agent-indexer-postgres.sh scripts/with-node.sh
	./scripts/qa/verify-issue-570.sh

# GitLab #568 — time-stamped candle USD; no as-of-now hub rewrite; idle mark-to-market.
verify-issue-568:
	@chmod +x scripts/qa/verify-issue-568.sh scripts/setup-cloud-agent-indexer-postgres.sh scripts/with-node.sh
	./scripts/qa/verify-issue-568.sh

# GitLab #560 — portfolio/trader realized P&L USD from hub prices (not $1 / 2.5×).
verify-issue-560:
	@chmod +x scripts/qa/verify-issue-560.sh scripts/setup-cloud-agent-indexer-postgres.sh scripts/with-node.sh
	./scripts/qa/verify-issue-560.sh

# GitLab #675 — portfolio/trader unrealized P&L (hub mark vs on-DEX cost).
verify-issue-675:
	@chmod +x scripts/qa/verify-issue-675.sh scripts/with-node.sh
	./scripts/qa/verify-issue-675.sh

# GitLab #524 — /trade + /charts UST1 pair display invert (UI-only).
verify-issue-524:
	@chmod +x scripts/qa/verify-issue-524.sh scripts/with-node.sh
	./scripts/qa/verify-issue-524.sh

# GitLab #543 — Price (USD) candles use invertUsd (not 1/x) + adaptive axis.
verify-issue-543:
	@chmod +x scripts/qa/verify-issue-543.sh scripts/setup-cloud-agent-indexer-postgres.sh scripts/with-node.sh
	./scripts/qa/verify-issue-543.sh

# GitLab #548 — /protocol + GET /overview 24h volume USD-only + catalog volume_usd ingest.
verify-issue-548:
	@chmod +x scripts/qa/verify-issue-548.sh scripts/setup-cloud-agent-indexer-postgres.sh scripts/with-node.sh
	./scripts/qa/verify-issue-548.sh

# GitLab #553 — /charts trader leaderboard + profile Total Volume USD.
verify-issue-553:
	@chmod +x scripts/qa/verify-issue-553.sh scripts/setup-cloud-agent-indexer-postgres.sh scripts/with-node.sh
	./scripts/qa/verify-issue-553.sh

# GitLab #656 — trader 4/6 address + blockie PFP (Charts leaderboard + profile header).
verify-issue-656:
	@chmod +x scripts/qa/verify-issue-656.sh scripts/with-node.sh
	./scripts/qa/verify-issue-656.sh

# GitLab #565 — /charts pair 24h Stats Vol (USD) + human token remainder.
verify-issue-565:
	@chmod +x scripts/qa/verify-issue-565.sh scripts/with-node.sh
	./scripts/qa/verify-issue-565.sh

# GitLab #666 — /charts pair-scoped 24h stats + leaderboard (drop DEX overview).
verify-issue-666:
	@chmod +x scripts/qa/verify-issue-666.sh scripts/setup-cloud-agent-indexer-postgres.sh scripts/with-node.sh
	./scripts/qa/verify-issue-666.sh

# GitLab #680 — /charts UST1/USD hero + ?price= + page-wide invert.
verify-issue-680:
	@chmod +x scripts/qa/verify-issue-680.sh scripts/with-node.sh
	./scripts/qa/verify-issue-680.sh

.PHONY: verify-issue-680

# GitLab #576 — trailing 24h/7d/30d volume copy (not calendar-day reset).
verify-issue-576:
	@chmod +x scripts/qa/verify-issue-576.sh scripts/with-node.sh
	./scripts/qa/verify-issue-576.sh

# GitLab #564 — Charts pair 24h Stats + TWAP human scale (mixed-decimal vols).
verify-issue-564:
	@chmod +x scripts/qa/verify-issue-564.sh scripts/with-node.sh
	./scripts/qa/verify-issue-564.sh


# GitLab #527 — /trade ticket money-CTA docks to panel bottom (no Chrome sticky float).
verify-issue-527:
	@chmod +x scripts/qa/verify-issue-527.sh scripts/with-node.sh
	./scripts/qa/verify-issue-527.sh

# GitLab #561 — /trade flatten chrome, independent tape, hide book/ticket (no drag-resize).
verify-issue-561:
	@chmod +x scripts/qa/verify-issue-561.sh scripts/with-node.sh
	./scripts/qa/verify-issue-561.sh

# GitLab #563 — /trade ticket full heading, no compact wallet chip, green Buy / red Sell.
verify-issue-563:
	@chmod +x scripts/qa/verify-issue-563.sh scripts/with-node.sh
	./scripts/qa/verify-issue-563.sh

# GitLab #528 — Trade Market + Swap Settings slippage chips stay one aligned group.
verify-issue-528:
	@chmod +x scripts/qa/verify-issue-528.sh scripts/with-node.sh
	./scripts/qa/verify-issue-528.sh

# GitLab #529 — decimals-normalized limit price band (UST1/USTR 6-vs-18).
verify-issue-529:
	@chmod +x scripts/qa/verify-issue-529.sh scripts/with-node.sh
	./scripts/qa/verify-issue-529.sh

# GitLab #530 — My open limits Cancel vs stale ● / fill lifecycle / /trade reachability.
verify-issue-530:
	@chmod +x scripts/qa/verify-issue-530.sh scripts/with-node.sh
	./scripts/qa/verify-issue-530.sh

# GitLab #517 — CL8Y Legal clickwrap (TermsGate) for dex.cl8y.com.
verify-issue-517:
	@chmod +x scripts/qa/verify-issue-517.sh scripts/with-node.sh
	./scripts/qa/verify-issue-517.sh

# GitLab #519 — WalletConnect same-device mobile pairing (deep-link + copy).
verify-issue-519:
	@chmod +x scripts/qa/verify-issue-519.sh scripts/with-node.sh
	./scripts/qa/verify-issue-519.sh

# GitLab #554 — Android Chrome Connect Wallet (pairing foreground, cancel, Keplr WC).
verify-issue-554:
	@chmod +x scripts/qa/verify-issue-554.sh scripts/with-node.sh
	./scripts/qa/verify-issue-554.sh

# GitLab #566 — Station + Cosmostation WalletConnect (ustr-cmm parity, no Leap).
verify-issue-566:
	@chmod +x scripts/qa/verify-issue-566.sh scripts/with-node.sh
	./scripts/qa/verify-issue-566.sh

# GitLab #658 — Legal terms hint names DEX wallets, not Keplr-only (WC-M12 / L658).
.PHONY: verify-issue-658
verify-issue-658:
	@chmod +x scripts/qa/verify-issue-658.sh scripts/with-node.sh
	./scripts/qa/verify-issue-658.sh

# GitLab #531 — retail LUNC liquidity how-to (v2 LP + maker-limit disambiguation).
verify-issue-531:
	@chmod +x scripts/qa/verify-issue-531.sh scripts/with-node.sh
	./scripts/qa/verify-issue-531.sh

# GitLab #533 — one-sided pool add/withdraw (auto zap + wrap).
verify-issue-533:
	@chmod +x scripts/qa/verify-issue-533.sh scripts/with-node.sh
	./scripts/qa/verify-issue-533.sh

# GitLab #559 — one-sided zap execution floors (quote vs execute).
verify-issue-559:
	@chmod +x scripts/qa/verify-issue-559.sh scripts/with-node.sh
	./scripts/qa/verify-issue-559.sh

# GitLab #539 — LocalTerra wrap-mapper split-fee instantiate + #533 P4–P8.
verify-issue-539:
	@chmod +x scripts/qa/verify-issue-539.sh scripts/with-node.sh
	./scripts/qa/verify-issue-539.sh

# GitLab #534 — pair selector economic-first catalog rank + human quote volume.
verify-issue-534:
	@chmod +x scripts/qa/verify-issue-534.sh scripts/with-node.sh
	./scripts/qa/verify-issue-534.sh

# GitLab #562 — hide soft-launch gems from production retail UI.
verify-issue-562:
	@chmod +x scripts/qa/verify-issue-562.sh scripts/with-node.sh
	./scripts/qa/verify-issue-562.sh

# GitLab #542 — Create Pair listed-CW20 picker + custom paste.
verify-issue-542:
	@chmod +x scripts/qa/verify-issue-542.sh scripts/with-node.sh
	./scripts/qa/verify-issue-542.sh

# GitLab #537 — dApp fee-tier chrome gated on pair DISCOUNT_REGISTRY (I14).
verify-issue-537:
	@chmod +x scripts/qa/verify-issue-537.sh scripts/with-node.sh
	./scripts/qa/verify-issue-537.sh

# GitLab #538 — LocalTerra create_pair inherit + dApp GetDiscountRegistry first.
verify-issue-538:
	@chmod +x scripts/qa/verify-issue-538.sh scripts/qa/localterra-create-pair-inherit.sh scripts/with-node.sh
	./scripts/qa/verify-issue-538.sh

# GitLab #541 — compact token identity on Pool / Trade / Charts.
verify-issue-541:
	@chmod +x scripts/qa/verify-issue-541.sh scripts/with-node.sh
	./scripts/qa/verify-issue-541.sh

# GitLab #664 — /trade + /charts identity v2 LP USD (stamp + chip).
.PHONY: verify-issue-664
verify-issue-664:
	@chmod +x scripts/qa/verify-issue-664.sh scripts/with-node.sh
	./scripts/qa/verify-issue-664.sh

# GitLab #630 — LUNC/USTC (not uluna/uusd) in token pickers.
verify-issue-630:
	@chmod +x scripts/qa/verify-issue-630.sh scripts/with-node.sh
	./scripts/qa/verify-issue-630.sh

# GitLab #651 — phone-width /tiers cards + How it works (layout only).
verify-issue-651:
	@chmod +x scripts/qa/verify-issue-651.sh scripts/with-node.sh
	./scripts/qa/verify-issue-651.sh

# GitLab #669 — /token/create desktop density (layout only).
verify-issue-669:
	@chmod +x scripts/qa/verify-issue-669.sh scripts/with-node.sh
	./scripts/qa/verify-issue-669.sh

# GitLab #665 — trader profile Share link (Web Share + clipboard).
verify-issue-665:
	@chmod +x scripts/qa/verify-issue-665.sh scripts/with-node.sh
	./scripts/qa/verify-issue-665.sh

# GitLab #674 — hide test-gem performance on /portfolio (Show test pairs toggle).
.PHONY: verify-issue-674
verify-issue-674:
	@chmod +x scripts/qa/verify-issue-674.sh scripts/with-node.sh
	./scripts/qa/verify-issue-674.sh

# GitLab #547 — /pool sortable table, UST1-first catalog default, Charts deep links.
verify-issue-547:
	@chmod +x scripts/qa/verify-issue-547.sh scripts/with-node.sh
	./scripts/qa/verify-issue-547.sh

# GitLab #655 — /pool v2 LP USD column + pair_liquidity_usd rollup.
.PHONY: verify-issue-655
verify-issue-655:
	@chmod +x scripts/qa/verify-issue-655.sh scripts/setup-cloud-agent-indexer-postgres.sh scripts/with-node.sh
	./scripts/qa/verify-issue-655.sh

# GitLab #692 — /pool Vol USD + pair-list 24h volume_usd_24h stamp.
.PHONY: verify-issue-692
verify-issue-692:
	@chmod +x scripts/qa/verify-issue-692.sh scripts/setup-cloud-agent-indexer-postgres.sh scripts/with-node.sh
	./scripts/qa/verify-issue-692.sh

# GitLab #662 — /pool Created column relative age from indexer first-seen created_at.
.PHONY: verify-issue-662
verify-issue-662:
	@chmod +x scripts/qa/verify-issue-662.sh scripts/with-node.sh
	./scripts/qa/verify-issue-662.sh

# GitLab #501 — Trade market defaults to GET /route/solve (same as Swap).
verify-issue-501:
	@chmod +x scripts/qa/verify-issue-501.sh scripts/with-node.sh
	./scripts/qa/verify-issue-501.sh

# GitLab #596 — retail hybrid always-on (no opt-out on Swap / Trade market).
verify-issue-596:
	@chmod +x scripts/qa/verify-issue-596.sh scripts/with-node.sh
	./scripts/qa/verify-issue-596.sh

# GitLab #595 — pay-with-any-token invoice module (quote + card + gas).
verify-issue-595:
	@chmod +x scripts/qa/verify-issue-595.sh scripts/with-node.sh
	./scripts/qa/verify-issue-595.sh

# GitLab #592 — community tax CW20 (DEX-safe buy/sell/transfer + SKUs).
verify-issue-592:
	@chmod +x scripts/qa/verify-issue-592.sh
	./scripts/qa/verify-issue-592.sh

# GitLab #609 — ExemptionDirectory skips buy, sell, and transfer tax (M-5).
verify-issue-609:
	@chmod +x scripts/qa/verify-issue-609.sh scripts/with-node.sh
	./scripts/qa/verify-issue-609.sh

# GitLab #610 — AutoLP factory-listed pair + skim floor (M-2 / M-3).
verify-issue-610:
	@chmod +x scripts/qa/verify-issue-610.sh scripts/with-node.sh
	./scripts/qa/verify-issue-610.sh

# GitLab #593 — Create Token + manager console.
verify-issue-593:
	@chmod +x scripts/qa/verify-issue-593.sh scripts/with-node.sh
	./scripts/qa/verify-issue-593.sh

# GitLab #604 — Create Token identity + connected-wallet helpers.
verify-issue-604:
	@chmod +x scripts/qa/verify-issue-604.sh scripts/with-node.sh
	./scripts/qa/verify-issue-604.sh

# GitLab #605 — SKU init fields + retail percent taxes.
verify-issue-605:
	@chmod +x scripts/qa/verify-issue-605.sh scripts/with-node.sh
	./scripts/qa/verify-issue-605.sh

.PHONY: verify-issue-604 verify-issue-605 verify-issue-610 verify-issue-626 verify-issue-627 verify-issue-628 verify-issue-633 verify-issue-634 verify-issue-630 verify-issue-632 verify-issue-651 verify-issue-665 verify-issue-674 verify-issue-671 verify-issue-670 verify-issue-672 verify-issue-669

# GitLab #632 — Keplr in-app / visualViewport token picker clearance + browse-without-IME.
verify-issue-632:
	@chmod +x scripts/qa/verify-issue-632.sh scripts/with-node.sh
	./scripts/qa/verify-issue-632.sh

# GitLab #672 — Connect Wallet / Modal dismiss overlay (Close, backdrop, Escape, header toggle).
verify-issue-672:
	@chmod +x scripts/qa/verify-issue-672.sh scripts/with-node.sh
	./scripts/qa/verify-issue-672.sh

# GitLab #633 — listed-pair autoregister + Manage catch-up + manager role tax skip.
verify-issue-633:
	@chmod +x scripts/qa/verify-issue-633.sh scripts/qa/localterra-633-autoregister.sh \
		scripts/upgrade-635-autoregister.sh scripts/qa/test-upgrade-635-guards.sh scripts/with-node.sh
	./scripts/qa/verify-issue-633.sh

# GitLab #634 — migrate pair inventory + post-adopt CL8Y register tool.
verify-issue-634:
	@chmod +x scripts/qa/verify-issue-634.sh scripts/qa/localterra-634-migrate-inventory.sh scripts/with-node.sh
	./scripts/qa/verify-issue-634.sh

# GitLab #626 — free listed-template adopt onto 11619 + Terraport/GDEX LP gate.
verify-issue-626:
	@chmod +x scripts/qa/verify-issue-626.sh scripts/with-node.sh
	./scripts/qa/verify-issue-626.sh

# GitLab #670 — /token/migrate why-copy (Unlock {X} + examples).
verify-issue-670:
	@chmod +x scripts/qa/verify-issue-670.sh scripts/with-node.sh
	./scripts/qa/verify-issue-670.sh

# GitLab #627 — columbus-5 CW20 code 3 adopt + factory-list NO-GO.
verify-issue-627:
	@chmod +x scripts/qa/verify-issue-627.sh scripts/with-node.sh
	./scripts/qa/verify-issue-627.sh

# GitLab #628 — post-merge !418 community-tax migrate leftovers (#626).
verify-issue-628:
	@chmod +x scripts/qa/verify-issue-628.sh scripts/qa/verify-issue-626.sh \
		scripts/qa/verify-issue-592.sh scripts/qa/verify-issue-593.sh \
		scripts/qa/verify-issue-594.sh scripts/qa/localterra-628-migrate-leftover.sh \
		scripts/with-node.sh
	./scripts/qa/verify-issue-628.sh

.PHONY: verify-issue-628

# GitLab #673 — post-merge !437–!458 leftover verify (#655–#672).
.PHONY: verify-issue-673
verify-issue-673:
	@chmod +x scripts/qa/verify-issue-673.sh scripts/with-node.sh
	./scripts/qa/verify-issue-673.sh

# GitLab #686 — post-merge !459–!468 leftover verify (#674–#680, #683).
.PHONY: verify-issue-686
verify-issue-686:
	@chmod +x scripts/qa/verify-issue-686.sh scripts/with-node.sh
	./scripts/qa/verify-issue-686.sh

# GitLab #594 — community tax indexer catalog.
verify-issue-594:
	@chmod +x scripts/qa/verify-issue-594.sh
	./scripts/qa/verify-issue-594.sh

# GitLab #601 — community tax store + #589 REPORT + factory list + LocalTerra smoke.
verify-issue-601:
	@chmod +x scripts/qa/verify-issue-601.sh \
		scripts/qa/localterra-community-tax-smoke.sh \
		scripts/upgrade-611-community-tax.sh \
		scripts/qa/test-upgrade-611-guards.sh \
		scripts/qa/verify-issue-592.sh \
		scripts/qa/verify-issue-589.sh \
		cw20-codeid-audits/scripts/layer-a-lcd.sh \
		cw20-codeid-audits/scripts/layer-b-lt.sh \
		cw20-codeid-audits/scripts/fetch-lcd-wasm.sh
	./scripts/qa/verify-issue-601.sh

# GitLab #606 — launcher Enable Feature path + unique SKUs (C-1 / H-2 / L-1).
verify-issue-606:
	@chmod +x scripts/qa/verify-issue-606.sh scripts/with-node.sh
	./scripts/qa/verify-issue-606.sh

# GitLab #602 — post-merge !402 Coolify + 11614 launcher + LocalTerra Create Token.
verify-issue-602:
	@chmod +x scripts/qa/verify-issue-602.sh scripts/qa/verify-issue-593.sh \
		scripts/qa/verify-issue-594.sh scripts/qa/localterra-community-tax-smoke.sh \
		scripts/with-node.sh
	./scripts/qa/verify-issue-602.sh

# GitLab #612 — post-merge !407/!408 Enable Feature migrate + LocalTerra QA.
verify-issue-612:
	@chmod +x scripts/qa/verify-issue-612.sh \
		scripts/qa/verify-issue-601.sh \
		scripts/qa/verify-issue-606.sh \
		scripts/qa/verify-issue-607.sh \
		scripts/qa/localterra-community-tax-smoke.sh \
		scripts/with-node.sh
	./scripts/qa/verify-issue-612.sh

.PHONY: verify-issue-612

# GitLab #616 — post-merge !409–!413 option-2 / wrap / window / AutoLP / ranking.
verify-issue-616:
	@chmod +x scripts/qa/verify-issue-616.sh \
		scripts/qa/verify-issue-607.sh \
		scripts/qa/verify-issue-610.sh \
		scripts/qa/verify-issue-613.sh \
		scripts/qa/verify-issue-614.sh \
		scripts/qa/verify-issue-615.sh \
		scripts/with-node.sh
	./scripts/qa/verify-issue-616.sh

.PHONY: verify-issue-616

# GitLab #620 — LocalTerra community-tax seed + Transfer funding + indexer env.
verify-issue-620:
	@chmod +x scripts/qa/verify-issue-620.sh scripts/qa/test-cw20-funding-kind.sh \
		scripts/lib/deploy-community-tax-local.sh scripts/lib/cw20-funding-kind.sh \
		scripts/with-node.sh
	./scripts/qa/verify-issue-620.sh

.PHONY: verify-issue-620

# GitLab #624 — post-merge !414 LocalTerra community-tax seed leftovers (#620).
verify-issue-624:
	@chmod +x scripts/qa/verify-issue-624.sh scripts/qa/verify-issue-620.sh \
		scripts/qa/test-cw20-funding-kind.sh scripts/with-node.sh
	./scripts/qa/verify-issue-624.sh

.PHONY: verify-issue-624

# GitLab #621 — tax-aware localnet swarm (gem exclude + tax workers).
verify-issue-621:
	@chmod +x scripts/qa/verify-issue-621.sh scripts/with-node.sh
	./scripts/qa/verify-issue-621.sh

.PHONY: verify-issue-621

# GitLab #623 — named tax-on Layer B suite (keep generic B-lt tax-off).
verify-issue-623:
	@chmod +x scripts/qa/verify-issue-623.sh \
		cw20-codeid-audits/scripts/layer-b-tax-on.sh \
		cw20-codeid-audits/scripts/lib-tax-on.sh \
		cw20-codeid-audits/scripts/lib-layer-lt.sh \
		cw20-codeid-audits/scripts/layer-b-lt.sh
	./scripts/qa/verify-issue-623.sh

.PHONY: verify-issue-623

# GitLab #622 — Playwright e2e-tx community-tax pair (sell extra-debit / buy net).
verify-issue-622:
	@chmod +x scripts/qa/verify-issue-622.sh scripts/with-node.sh
	./scripts/qa/verify-issue-622.sh

.PHONY: verify-issue-622

# GitLab #625 — post-merge !415–!417 tax swarm / e2e-tx / Layer B leftovers.
verify-issue-625:
	@chmod +x scripts/qa/verify-issue-625.sh scripts/qa/verify-issue-621.sh \
		scripts/qa/verify-issue-622.sh scripts/qa/verify-issue-623.sh \
		scripts/qa/verify-issue-293.sh scripts/with-node.sh
	./scripts/qa/verify-issue-625.sh

.PHONY: verify-issue-625

# GitLab #607 — community tax router hops tax the original trader (C-2 option 2).
verify-issue-607:
	@chmod +x scripts/qa/verify-issue-607.sh scripts/with-node.sh
	./scripts/qa/verify-issue-607.sh

# GitLab #615 — tax-aware route/solve ranking + You Receive net.
verify-issue-615:
	@chmod +x scripts/qa/verify-issue-615.sh scripts/qa/verify-issue-607.sh scripts/with-node.sh
	./scripts/qa/verify-issue-615.sh

# GitLab #608 — LaunchGuards per-wallet cooldown + max_wallet provide liveness (H-3 / H-4).
verify-issue-608:
	@chmod +x scripts/qa/verify-issue-608.sh
	./scripts/qa/verify-issue-608.sh

verify-issue-506:
	@chmod +x scripts/qa/verify-issue-506.sh scripts/with-node.sh
	./scripts/qa/verify-issue-506.sh

verify-issue-678:
	@chmod +x scripts/qa/verify-issue-678.sh scripts/with-node.sh
	./scripts/qa/verify-issue-678.sh

# GitLab #512 — unwrap InstantWithdraw burn tax quotes + wrap mint fee-only (W8–W11).
verify-issue-512:
	@chmod +x scripts/qa/verify-issue-512.sh scripts/with-node.sh
	./scripts/qa/verify-issue-512.sh

# GitLab #516 — wrap-mapper fee_wrap_bps / fee_unwrap_bps consumer (W12–W15).
verify-issue-516:
	@chmod +x scripts/qa/verify-issue-516.sh scripts/with-node.sh
	./scripts/qa/verify-issue-516.sh

# GitLab #523 — router unwrap_output dual-reads fee_unwrap_bps (R3 / W13).
verify-issue-523:
	@chmod +x scripts/qa/verify-issue-523.sh
	./scripts/qa/verify-issue-523.sh

# GitLab #503 — UST1/wrap production ops (registry, runbooks O1–O8, FAQ, health script).
# Optional: VERIFY503_MAINNET=1 runs read-only columbus-5 LCD probe.
verify-issue-503:
	@chmod +x scripts/qa/verify-issue-503.sh scripts/check-ust1-wrap-ops-health.sh \
		scripts/lib/ust1-wrap-ops-defaults.sh
	./scripts/qa/verify-issue-503.sh

# Read-only columbus-5 UST1/wrap LCD health (#503).
check-ust1-wrap-ops-health:
	@chmod +x scripts/check-ust1-wrap-ops-health.sh scripts/lib/ust1-wrap-ops-defaults.sh
	./scripts/check-ust1-wrap-ops-health.sh

# Columbus-5: rebalance UST1/cUSTC to oracle, mint $5k LP ×2, send to CMM.
# DRY_RUN=1 skips txs. Live: TERRAD_HOST_KEYRING_PASS + UST1_LP_YES=1 (or TTY confirm).
rebalance-mint-ust1-lp:
	@chmod +x scripts/rebalance-mint-ust1-lp.sh scripts/lib/ust1-lp-rebalance-math.py
	./scripts/rebalance-mint-ust1-lp.sh

# GitLab #508 — UST1 secondary AMM create+seed tooling / Path B waiver (U1–U7).
# Optional: VERIFY508_LOCAL=1 (LocalTerra fixture) VERIFY508_MAINNET=1 (live pair presence).
verify-issue-508:
	@chmod +x scripts/qa/verify-issue-508.sh scripts/add-ust1-secondary-pair.sh \
		scripts/seed-ust1-secondary-pair-local.sh scripts/with-node.sh
	./scripts/qa/verify-issue-508.sh

verify-issue-369:
	@chmod +x scripts/qa/verify-issue-369.sh scripts/setup-cloud-agent-indexer-postgres.sh
	./scripts/qa/verify-issue-369.sh

# GitLab #365 / #375 — registry outage docs regression (contract P5 + indexer health + frontend warning).
verify-issue-365:
	@chmod +x scripts/qa/verify-issue-365.sh scripts/with-node.sh
	./scripts/qa/verify-issue-365.sh

verify-issue-391:
	@chmod +x scripts/qa/verify-issue-391.sh
	./scripts/qa/verify-issue-391.sh

verify-issue-399:
	@chmod +x scripts/qa/verify-issue-399.sh scripts/lib/lcd-smart-query.sh scripts/lib/localterra-host-curl.sh scripts/lib/terrad-wait-tx.sh
	@if docker compose ps -q localterra 2>/dev/null | grep -q .; then \
		./scripts/qa/verify-issue-399.sh; \
	else \
		sg docker -c './scripts/qa/verify-issue-399.sh'; \
	fi

# GitLab #397 — governance emergency controls multisig rehearsal (SEC-B09).
verify-issue-397:
	@chmod +x scripts/qa/verify-issue-397.sh scripts/rehearse-governance-emergency-controls.sh scripts/lib/terrad-multisig-tx.sh scripts/lib/lcd-smart-query.sh scripts/lib/terrad-wait-tx.sh
	./scripts/qa/verify-issue-397.sh

rehearse-governance-emergency:
	@chmod +x scripts/rehearse-governance-emergency-controls.sh scripts/lib/terrad-multisig-tx.sh scripts/lib/lcd-smart-query.sh scripts/lib/terrad-wait-tx.sh
	./scripts/rehearse-governance-emergency-controls.sh

# GitLab #398 — admin-key custody and signer roster (SEC-B10).
verify-issue-398:
	@chmod +x scripts/qa/verify-issue-398.sh
	./scripts/qa/verify-issue-398.sh

# GitLab #408 — governance key rotation cookbook + rehearsal (SEC-D10).
verify-issue-408:
	@chmod +x scripts/qa/verify-issue-408.sh scripts/rehearse-governance-key-rotation.sh
	./scripts/qa/verify-issue-408.sh

rehearse-governance-key-rotation:
	@chmod +x scripts/rehearse-governance-key-rotation.sh
	./scripts/rehearse-governance-key-rotation.sh

verify-issue-400:
	@chmod +x scripts/qa/verify-issue-400.sh
	./scripts/qa/verify-issue-400.sh

verify-issue-435:
	@chmod +x scripts/qa/verify-issue-435.sh
	./scripts/qa/verify-issue-435.sh

verify-issue-437:
	@chmod +x scripts/qa/verify-issue-437.sh
	./scripts/qa/verify-issue-437.sh

verify-issue-438:
	@chmod +x scripts/qa/verify-issue-438.sh
	./scripts/qa/verify-issue-438.sh

verify-issue-407:
	@chmod +x scripts/qa/verify-issue-407.sh scripts/verify-no-ibc-hooks-in-contracts.sh
	./scripts/qa/verify-issue-407.sh

verify-issue-429:
	@chmod +x scripts/qa/verify-issue-429.sh
	./scripts/qa/verify-issue-429.sh

# GitLab #567 — Keplr + Ledger Nano signing stall (amino, pre-sign suggest, sign-stall UX).
verify-issue-567:
	@chmod +x scripts/qa/verify-issue-567.sh
	./scripts/qa/verify-issue-567.sh

# GitLab #578 — Open Graph / Twitter cards (absolute https image URL + community medallion).
verify-issue-578:
	@chmod +x scripts/qa/verify-issue-578.sh
	./scripts/qa/verify-issue-578.sh

# GitLab #695 — production VITE_DEV_MODE reject (FE-01 Simulated Wallet chrome).
.PHONY: verify-issue-695
verify-issue-695:
	@chmod +x scripts/qa/verify-issue-695.sh scripts/with-node.sh
	./scripts/qa/verify-issue-695.sh

# GitLab #629 — Keplr CW20 contract-registry pack (columbus-5 name/logo).
.PHONY: verify-issue-629
verify-issue-629:
	@chmod +x scripts/qa/verify-issue-629.sh scripts/qa/export-keplr-cw20-pack.sh \
		scripts/qa/keplr_cw20_registry_validate.py scripts/with-node.sh
	./scripts/qa/verify-issue-629.sh

# GitLab #639 — listing venue catalog (wallets / CG / CMC forms; no new API).
.PHONY: verify-issue-639
verify-issue-639:
	@chmod +x scripts/qa/verify-issue-639.sh scripts/qa/listings_catalog_validate.py
	./scripts/qa/verify-issue-639.sh

.PHONY: verify-issue-646
verify-issue-646:
	@chmod +x scripts/qa/verify-issue-646.sh
	./scripts/qa/verify-issue-646.sh

.PHONY: verify-issue-684
verify-issue-684:
	@chmod +x scripts/qa/verify-issue-684.sh
	./scripts/qa/verify-issue-684.sh

.PHONY: verify-issue-694
verify-issue-694:
	@chmod +x scripts/qa/verify-issue-694.sh scripts/setup-cloud-agent-indexer-postgres.sh scripts/with-node.sh
	./scripts/qa/verify-issue-694.sh

# GitLab #685 — CG/CMC listing field truthfulness (TVL stamp, bid/ask, gems, freeze).
.PHONY: verify-issue-685
verify-issue-685:
	@chmod +x scripts/qa/verify-issue-685.sh
	./scripts/qa/verify-issue-685.sh

# GitLab #640 — Cosmostation / Mintscan CW20 pack (chain/terra).
.PHONY: verify-issue-640
verify-issue-640:
	@chmod +x scripts/qa/verify-issue-640.sh scripts/qa/export-cosmostation-cw20-pack.sh \
		scripts/qa/cosmostation_cw20_validate.py
	./scripts/qa/verify-issue-640.sh

# GitLab #641 — Hexxagon / Galaxy Station CW20 pack.
.PHONY: verify-issue-641
verify-issue-641:
	@chmod +x scripts/qa/verify-issue-641.sh scripts/qa/hexxagon_cw20_validate.py
	./scripts/qa/verify-issue-641.sh

# GitLab #653 — one chrome layer / anti-nesting (StatBox flat + allowlist guard).
.PHONY: verify-issue-653
verify-issue-653:
	@chmod +x scripts/qa/verify-issue-653.sh scripts/with-node.sh
	./scripts/qa/verify-issue-653.sh

# GitLab #659 — Swap direction seam plate (opaque fill + static occluder).
.PHONY: verify-issue-659
verify-issue-659:
	@chmod +x scripts/qa/verify-issue-659.sh scripts/with-node.sh
	./scripts/qa/verify-issue-659.sh
# GitLab #663 — official CL8Y product links (footer Homepage + Bridge).
.PHONY: verify-issue-663
verify-issue-663:
	@chmod +x scripts/qa/verify-issue-663.sh scripts/with-node.sh
	./scripts/qa/verify-issue-663.sh
# GitLab #661 — /pool Manage provide name/symbol labels + wrap default on.
.PHONY: verify-issue-661
verify-issue-661:
	@chmod +x scripts/qa/verify-issue-661.sh scripts/with-node.sh
	./scripts/qa/verify-issue-661.sh
# GitLab #657 — /trader global leaderboard (shared Charts table).
.PHONY: verify-issue-657
verify-issue-657:
	@chmod +x scripts/qa/verify-issue-657.sh scripts/with-node.sh
	./scripts/qa/verify-issue-657.sh
# GitLab #671 — connected wallet dropdown icon+label alignment.
.PHONY: verify-issue-671
verify-issue-671:
	@chmod +x scripts/qa/verify-issue-671.sh scripts/with-node.sh
	./scripts/qa/verify-issue-671.sh
# GitLab #660 — /pool Manage four peer actions (provide, withdraw, zap add, zap withdraw).
.PHONY: verify-issue-660
verify-issue-660:
	@chmod +x scripts/qa/verify-issue-660.sh scripts/with-node.sh
	./scripts/qa/verify-issue-660.sh

# GitLab #573 — post-merge stack !368–!377 (child verifies 557–567 + Coolify/ops docs).
verify-issue-573:
	@chmod +x scripts/qa/verify-issue-573.sh
	./scripts/qa/verify-issue-573.sh

verify-issue-410:
	@chmod +x scripts/qa/verify-issue-410.sh
	./scripts/qa/verify-issue-410.sh

verify-issue-416:
	@chmod +x scripts/qa/verify-issue-416.sh
	./scripts/qa/verify-issue-416.sh

verify-issue-436:
	@chmod +x scripts/qa/verify-issue-436.sh
	./scripts/qa/verify-issue-436.sh

verify-issue-440:
	@chmod +x scripts/qa/verify-issue-440.sh
	./scripts/qa/verify-issue-440.sh

verify-issue-441:
	@chmod +x scripts/qa/verify-issue-441.sh scripts/qa/verify-deploy-config.sh scripts/qa/test-verify-deploy-config.sh
	./scripts/qa/verify-issue-441.sh

verify-issue-442:
	@chmod +x scripts/qa/verify-issue-442.sh scripts/qa/verify-env-addresses.sh scripts/qa/test-verify-env-addresses.sh
	./scripts/qa/verify-issue-442.sh

verify-issue-451:
	@chmod +x scripts/qa/verify-issue-451.sh
	./scripts/qa/verify-issue-451.sh

verify-issue-443:
	@chmod +x scripts/qa/verify-issue-443.sh
	./scripts/qa/verify-issue-443.sh

verify-issue-444:
	@chmod +x scripts/qa/verify-issue-444.sh
	./scripts/qa/verify-issue-444.sh

help:
	@echo "Infrastructure:  make start | stop | reset | status | compose-ps | wait-localterra | wait-healthy | swarm-local | swarm-launch | swarm-stop"
	@echo "QA server:       make start-qa | reset-qa | QA_FRESH_VOLUMES=1 make start-qa | QA_FETCH_CI_ARTIFACTS=1 make start-qa | stop-qa | qa-verify-deploy | qa-verify-env-addresses | qa-verify-deploy-config | test-qa-redeploy-decision"
	@echo "Contracts:       make build-optimized | deploy-local | deploy-local-no-build | deploy-testnet | deploy-mainnet | deploy-mainnet-soft-launch"
	@echo "QA artifacts:    make fetch-qa-ci-artifacts | make build-indexer-release (INDEXER_QA_BIN)"
	@echo "Cloud Agent:     make setup-cloud-agent-env | setup-cloud-localterra | setup-indexer-postgres | test-setup-cloud-agent-env | test-indexer-integration | verify-issue-324 | verify-issue-503 | verify-issue-504 | verify-issue-514 | verify-issue-518 | verify-issue-533 | verify-issue-559 | verify-issue-539 | verify-issue-534 | verify-issue-562 | verify-issue-542 | verify-issue-536 | verify-issue-582 | verify-issue-584 | verify-issue-585 | verify-issue-587 | verify-issue-679 | verify-issue-599 | verify-issue-600 | verify-issue-595 | verify-issue-592 | verify-issue-593 | verify-issue-594 | verify-issue-601 | verify-issue-602 | verify-issue-612 | verify-issue-616 | verify-issue-604 | verify-issue-605 | verify-issue-606 | verify-issue-607 | verify-issue-615 | verify-issue-608 | verify-issue-609 | verify-issue-610 | verify-issue-620 | verify-issue-624 | verify-issue-621 | verify-issue-622 | verify-issue-623 | verify-issue-625 | verify-issue-626 | verify-issue-627 | verify-issue-628 | verify-issue-673 | verify-issue-686 | verify-issue-633 | verify-issue-634 | verify-issue-670 | verify-issue-629 | verify-issue-639 | verify-issue-640 | verify-issue-641 | verify-issue-630 | verify-issue-632 | verify-issue-651 | verify-issue-652 | verify-issue-668 | verify-issue-689 | verify-issue-667 | verify-issue-653 | verify-issue-671 | verify-issue-659 | verify-issue-663 | verify-issue-589 | verify-issue-590 | verify-issue-537 | verify-issue-538 | verify-issue-541 | verify-issue-664 | verify-issue-547 | verify-issue-655 | verify-issue-692 | verify-issue-662 | verify-issue-485 | verify-issue-515 | verify-issue-550 | verify-issue-522 | verify-issue-551 | verify-issue-557 | verify-issue-556 | verify-issue-560 | verify-issue-524 | verify-issue-543 | verify-issue-548 | verify-issue-553 | verify-issue-656 | verify-issue-564 | verify-issue-565 | verify-issue-666 | verify-issue-576 | verify-issue-527 | verify-issue-561 | verify-issue-563 | verify-issue-528 | verify-issue-529 | verify-issue-530 | verify-issue-517 | verify-issue-519 | verify-issue-554 | verify-issue-566 | verify-issue-658 | verify-issue-531 | verify-issue-501 | verify-issue-596 | verify-issue-512 | verify-issue-516 | verify-issue-523 | verify-issue-508 | verify-issue-365 | verify-issue-369 | verify-issue-391 | verify-issue-397 | verify-issue-398 | verify-issue-408 | verify-issue-399 | verify-issue-400 | verify-issue-435 | verify-issue-437 | verify-issue-438 | verify-issue-407 | verify-issue-429 | verify-issue-567 | verify-issue-570 | verify-issue-571 | verify-issue-580 | verify-issue-550 verify-issue-569 | verify-issue-586 | verify-issue-683 | verify-issue-631 | verify-issue-687 | verify-issue-684 | verify-issue-685 | verify-issue-613 | verify-issue-614 | verify-issue-568 | verify-issue-577 | verify-issue-579 | verify-issue-295 (needs make dev)"
	@echo "Frontend:        make dev | verify-issue-678 | verify-issue-578 | verify-issue-695 | verify-issue-629 | verify-issue-639 | verify-issue-640 | verify-issue-641 | verify-issue-630 | verify-issue-632 | verify-issue-651 | verify-issue-665 | verify-issue-674 | verify-issue-669 | verify-issue-653 | verify-issue-660 | verify-issue-671 | verify-issue-670 | verify-issue-657 | verify-issue-662 | verify-issue-664 | verify-issue-661 | verify-issue-672 | verify-issue-658 | verify-issue-659 | verify-issue-663 | verify-issue-573 | verify-issue-410 | verify-issue-416 | verify-issue-436 | verify-issue-439 | verify-issue-440 | verify-issue-441 | verify-issue-442 | verify-issue-451 | verify-issue-443 | verify-issue-444 | verify-issue-445 | verify-issue-295 (needs make dev)"

	@echo "Frontend:        make dev | build-frontend | test-frontend | test-frontend-charts | test-charts-integration | test-e2e-tx | test-e2e-indexer-outage | lint-frontend"
	@echo "Indexer:         make indexer-dev | test-indexer-integration | test-indexer-target-ownership | verify-issue-676 | indexer-reorg-recover HEIGHT=<H> [APPLY=1] [CLEANUP=1]"
	@echo "Ops:             make rebalance-mint-ust1-lp (DRY_RUN=1 to plan only)"
	@echo "Docs:            scripts/qa/README.md"

# Smart contracts — two different builds:
#
#   make build-optimized  →  CosmWasm workspace-optimizer (Docker). This is what deploy-local and
#                            production use: small, deterministic, optimizer-processed wasm in
#                            smartcontracts/artifacts/. Does NOT require wasm32-unknown-unknown on
#                            the host — only Docker.
#
#   Dev wasm: plain cargo wasm (reference job contracts-terra in .github/workflows/test.yml — fast).
#   Release wasm: make build-optimized only (reference contracts-wasm-optimizer.yml); do not upload
#   cargo wasm to mainnet. See docs/testing.md § CI and docs/deployment-guide.md (GitLab #234).
#
#   make build-contracts / build-artifacts-cargo  →  plain cargo --release wasm. Useful for quick
#                            local checks / unit tests; NOT a substitute for the optimizer. Do not
#                            deploy these if you need parity with optimized artifacts.
#
# Contract crates only — workspace "tests" member is not wasm32-compatible.
WASM_CONTRACT_PKGS := -p cl8y-dex-factory -p cl8y-dex-pair -p cl8y-dex-router -p cl8y-dex-fee-discount \
	-p cl8y-dex-faucet -p cl8y-dex-burn-hook -p cl8y-dex-tax-hook -p cl8y-dex-lp-burn-hook

build-contracts:
	cd smartcontracts && cargo build --release --target wasm32-unknown-unknown $(WASM_CONTRACT_PKGS)

# Cargo-only wasm copied to artifacts/ — NOT run through workspace-optimizer (see note above).
build-artifacts-cargo: build-contracts
	mkdir -p smartcontracts/artifacts
	cp smartcontracts/target/wasm32-unknown-unknown/release/cl8y_dex_*.wasm smartcontracts/artifacts/

build-optimized:
	cd smartcontracts && ./scripts/optimize.sh

test-contracts:
	cd smartcontracts && cargo test

# Requires: cargo install cargo-llvm-cov
coverage-contracts:
	cd smartcontracts && cargo llvm-cov test --workspace --lcov --output-path lcov.info && \
		echo "LCOV written to smartcontracts/lcov.info (open HTML via: cargo llvm-cov report --open)"

lint-contracts:
	cd smartcontracts && cargo fmt --check && cargo clippy -- -D warnings

check-fee-discount-tier-docs:
	python3 scripts/check_fee_discount_tier_docs.py

check-route-solver-docs:
	python3 scripts/check_route_solver_docs.py

check-user-incident-faq-docs:
	python3 scripts/check_user_incident_faq_docs.py

check-emergency-commands-docs:
	python3 scripts/check_emergency_commands_docs.py

check-launch-go-no-go-docs:
	python3 scripts/check_launch_go_no_go_docs.py

check-governance-emergency-rehearsal-docs:
	python3 scripts/check_governance_emergency_rehearsal_docs.py

check-key-custody-docs:
	python3 scripts/check_key_custody_docs.py

check-governance-multisig-docs:
	python3 scripts/check_governance_multisig_docs.py

check-governance-key-rotation-docs:
	python3 scripts/check_governance_key_rotation_docs.py

check-blacklist-decision-docs:
	python3 scripts/check_blacklist_decision_docs.py

check-suspicious-activity-queries-docs:
	python3 scripts/check_suspicious_activity_queries_docs.py

check-anomaly-signals-docs:
	python3 scripts/check_anomaly_signals_docs.py

check-incident-comms-templates-docs:
	python3 scripts/check_incident_comms_templates_docs.py

check-incident-template-docs:
	python3 scripts/check_incident_template_docs.py

check-pool-triage-docs:
	python3 scripts/check_pool_triage_docs.py

check-rollback-decision-docs:
	python3 scripts/check_rollback_decision_docs.py

verify-issue-445:
	@chmod +x scripts/qa/verify-issue-445.sh
	./scripts/qa/verify-issue-445.sh

verify-issue-439:
	@chmod +x scripts/qa/verify-issue-439.sh
	./scripts/qa/verify-issue-439.sh

check-ibc-hooks-deploy-docs:
	python3 scripts/check_ibc_hooks_deploy_docs.py

check-extension-fee-guard-docs:
	python3 scripts/check_extension_fee_guard_docs.py

verify-no-ibc-hooks-in-contracts:
	@chmod +x scripts/verify-no-ibc-hooks-in-contracts.sh
	./scripts/verify-no-ibc-hooks-in-contracts.sh

check-deploy-trace-docs:
	python3 scripts/check_deploy_trace_docs.py

check-wasm-migration-rollback-docs:
	python3 scripts/check_wasm_migration_rollback_docs.py

check-deploy-config-docs:
	python3 scripts/check_deploy_config_docs.py

check-test-evidence-gate-docs:
	python3 scripts/check_test_evidence_gate_docs.py

check-deploy-env-addresses-docs:
	python3 scripts/check_deploy_env_addresses_docs.py

check-factory-address-docs:
	python3 scripts/check_factory_address_docs.py

check-exploit-replay-matrix-docs:
	python3 scripts/check_exploit_replay_matrix_docs.py

check-design-tokens:
	python3 scripts/check_design_tokens.py
	python3 scripts/check_chrome_nesting.py

check-launch-monitoring-docs:
	python3 scripts/check_launch_monitoring_docs.py

lint-log-secrets:
	python3 scripts/check_indexer_log_secrets.py

# Deployment
deploy-local-no-build:
	@chmod +x scripts/deploy-dex-local.sh scripts/lib/terrad-wait-tx.sh scripts/lib/qa-phase-timing.sh
	./scripts/deploy-dex-local.sh

deploy-local: build-optimized
	@chmod +x scripts/deploy-dex-local.sh scripts/lib/terrad-wait-tx.sh scripts/lib/qa-phase-timing.sh
	./scripts/deploy-dex-local.sh

build-indexer-release:
	cd indexer && cargo build --release

fetch-qa-ci-artifacts:
	@chmod +x scripts/qa/fetch-qa-ci-artifacts.sh
	./scripts/qa/fetch-qa-ci-artifacts.sh

deploy-testnet:
	cd smartcontracts && ./scripts/deploy.sh testnet

deploy-mainnet:
	cd smartcontracts && ./scripts/deploy.sh mainnet

# Soft launch (non-economic CW20): single script, cl8ydeploy key — docs/runbooks/mainnet-soft-launch.md
deploy-mainnet-soft-launch:
	@chmod +x scripts/deploy-dex-mainnet-soft-launch.sh scripts/build-cw20-base-artifact.sh
	./scripts/deploy-dex-mainnet-soft-launch.sh

# Soft-launch faucet (GitLab #473): store/instantiate + AddMinter — docs/runbooks/soft-launch-faucet.md
deploy-soft-launch-faucet:
	@chmod +x scripts/deploy-soft-launch-faucet.sh
	./scripts/deploy-soft-launch-faucet.sh

test-mainnet-soft-launch-defaults:
	@chmod +x scripts/qa/test-mainnet-soft-launch-defaults.sh
	./scripts/qa/test-mainnet-soft-launch-defaults.sh

# Frontend (Node via nvm — scripts/with-node.sh + .nvmrc)
WITH_NODE = bash scripts/with-node.sh --cwd frontend-dapp --

dev:
	@chmod +x scripts/dev-frontend-local.sh scripts/with-node.sh
	./scripts/dev-frontend-local.sh

build-frontend:
	$(WITH_NODE) npm run build

test-frontend:
	$(WITH_NODE) npm run test:run

test-frontend-charts:
	$(WITH_NODE) npm run test:charts

test-charts-integration tests-charts-integration:
	@chmod +x scripts/test-charts-integration.sh
	./scripts/test-charts-integration.sh

test-e2e:
	$(WITH_NODE) npm run test:e2e

# Strict on-chain Playwright (LocalTerra + deploy + global setup). Reference job: e2e.
test-e2e-tx:
	@chmod +x scripts/deploy-dex-local.sh scripts/e2e-provision-dev-wallet.sh scripts/e2e-seed-hybrid-book.sh scripts/e2e-seed-wrap-pairs.sh scripts/with-node.sh
	docker compose up -d localterra
	$(MAKE) wait-localterra
	bash scripts/deploy-dex-local.sh
	$(WITH_NODE) npm run test:e2e:tx

# Indexer stopped after sanity check; reference job frontend-e2e-indexer-outage (GitLab #219).
test-e2e-indexer-outage:
	@chmod +x scripts/test-e2e-indexer-outage.sh scripts/lib/e2e-trade-pair-from-deploy.sh scripts/with-node.sh
	./scripts/test-e2e-indexer-outage.sh

lint-frontend:
	$(WITH_NODE) npm run lint

lint-indexer-log-secrets:
	@chmod +x scripts/lint-indexer-log-secrets.sh
	@./scripts/lint-indexer-log-secrets.sh

# Indexer
indexer-dev:
	cd indexer && cargo run

# Operator recovery after reorg halt (dry-run unless HEIGHT=… APPLY=1 CLEANUP=1)
indexer-reorg-recover:
	@test -n "$(HEIGHT)" || (echo "Usage: make indexer-reorg-recover HEIGHT=<fork_height> [APPLY=1] [CLEANUP=1]" && exit 1)
	@args="--height $(HEIGHT)"; \
	[ "$(APPLY)" = "1" ] && args="$$args --apply"; \
	[ "$(CLEANUP)" = "1" ] && args="$$args --cleanup-derived"; \
	./scripts/indexer-reorg-recover.sh $$args

# Full devnet lifecycle: start infra, build, deploy, start indexer & frontend
dev-full: start wait-healthy build-optimized deploy-local
	@echo ""
	@echo "Starting indexer in background..."
	cd indexer && cargo run &
	@sleep 5
	@echo "Starting frontend dev server..."
	@chmod +x scripts/dev-frontend-local.sh scripts/with-node.sh
	./scripts/dev-frontend-local.sh

# Combined
test: test-contracts test-frontend

lint: lint-contracts lint-frontend lint-indexer-log-secrets lint-log-secrets check-fee-discount-tier-docs check-user-incident-faq-docs check-emergency-commands-docs check-launch-go-no-go-docs check-governance-emergency-rehearsal-docs check-ibc-hooks-deploy-docs check-extension-fee-guard-docs check-blacklist-decision-docs check-suspicious-activity-queries-docs check-anomaly-signals-docs check-incident-comms-templates-docs check-incident-template-docs check-pool-triage-docs check-rollback-decision-docs check-deploy-trace-docs check-wasm-migration-rollback-docs check-deploy-config-docs check-deploy-env-addresses-docs check-factory-address-docs check-test-evidence-gate-docs check-exploit-replay-matrix-docs check-key-custody-docs check-governance-multisig-docs check-governance-key-rotation-docs check-design-tokens check-launch-monitoring-docs verify-commit-messages

# Git hooks
setup-hooks:
	git config core.hooksPath .githooks
	@echo "Git hooks configured to use .githooks/"

test-commit-msg-hook:
	@./scripts/test-commit-msg-hook.sh

send: 
	docker exec cl8y-dex-terraclassic-localterra-1 terrad tx bank send test1 terra1753zuaneacfr60rg37l8d4t0x7j4yvqgsl7cvv 50000000uluna  --chain-id localterra --keyring-backend test --fees 6000000uluna --yes 

front:
	cd frontend-dapp && bash -c '. "$$HOME/.nvm/nvm.sh" && nvm use && VITE_NETWORK=local npm run dev -- --host'
