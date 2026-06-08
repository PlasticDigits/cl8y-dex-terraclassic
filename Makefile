.PHONY: start stop restart reset build-contracts build-artifacts-cargo build-optimized deploy-local deploy-local-no-build deploy-testnet deploy-mainnet dev dev-full indexer-dev build-indexer-release fetch-qa-ci-artifacts test-contracts coverage-contracts test-frontend test-frontend-charts test-e2e test-e2e-tx test-e2e-indexer-outage test-charts-integration tests-charts-integration lint check-fee-discount-tier-docs setup-hooks test-commit-msg-hook wait-localterra wait-healthy has-localterra help compose-ps start-qa qa-start stop-qa reset-qa test-qa-fresh-volumes test-qa-verify-deploy test-qa-redeploy-decision test-localterra-host-curl test-has-localterra test-setup-postgres test-setup-browser qa-tunnel-help qa-verify-deploy verify-issue-238 verify-issue-245 verify-issue-274 verify-issue-276 verify-issue-285 verify-issue-293 verify-issue-309 verify-issue-313 verify-issue-295 verify-issue-324 swarm-local swarm-launch swarm-stop test-swarm-liquidity swarm-bootstrap-liquidity setup-cloud-localterra setup-indexer-postgres test-indexer-integration

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

# 33 processes (5 swap types × 5 replicas + 5 limit + 3 lp); see scripts/bots/launch-swarm.sh
swarm-launch:
	@chmod +x scripts/bots/launch-swarm.sh scripts/bots/bootstrap-swarm-liquidity.sh
	./scripts/bots/launch-swarm.sh

swarm-bootstrap-liquidity:
	@chmod +x scripts/bots/bootstrap-swarm-liquidity.sh
	./scripts/bots/bootstrap-swarm-liquidity.sh

test-swarm-liquidity:
	cd scripts/bots && python3 -m unittest test_swarm_liquidity.py -v

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
	./scripts/qa/test-localterra-host-curl.sh

test-qa-redeploy-decision:
	@chmod +x scripts/qa/test-qa-redeploy-decision.sh
	./scripts/qa/test-qa-redeploy-decision.sh

test-localterra-host-curl:
	@chmod +x scripts/qa/test-localterra-host-curl.sh scripts/lib/localterra-host-curl.sh
	./scripts/qa/test-localterra-host-curl.sh

test-has-localterra:
	@chmod +x scripts/test-has-localterra.sh scripts/has-localterra.sh scripts/lib/localterra-host-curl.sh
	./scripts/test-has-localterra.sh

test-setup-postgres:
	@chmod +x scripts/test-setup-postgres-dev-databases.sh scripts/lib/postgres-bootstrap-role.sh scripts/lib/postgres-psql.sh scripts/setup-postgres-dev-databases.sh scripts/setup-cloud-agent-indexer-postgres.sh
	./scripts/test-setup-postgres-dev-databases.sh

test-setup-browser:
	@chmod +x scripts/test-setup-browser-cloud-agent.sh scripts/setup-browser-cloud-agent.sh scripts/lib/keplr-chrome-extension.sh
	./scripts/test-setup-browser-cloud-agent.sh

qa-tunnel-help:
	@chmod +x scripts/qa/print-qa-tunnel-instructions.sh scripts/qa/write-frontend-env-local.sh
	./scripts/qa/print-qa-tunnel-instructions.sh

stop-qa:
	@chmod +x scripts/qa/stop-qa.sh
	./scripts/qa/stop-qa.sh

qa-verify-deploy:
	@chmod +x scripts/qa/verify-deploy.sh scripts/lib/lcd-smart-query.sh
	./scripts/qa/verify-deploy.sh

# On-chain E2E for GitLab #238 (hybrid sim CL8Y fee-discount parity). Requires a
# fresh deploy (make deploy-local) + running indexer for the route/solve check.
verify-issue-238:
	@chmod +x scripts/qa/verify-issue-238.sh scripts/lib/lcd-smart-query.sh scripts/lib/localterra-host-curl.sh
	./scripts/qa/verify-issue-238.sh

# GitLab #245 — off-chain trader forwarding (unit tests + optional live stack via #238 script).
verify-issue-245:
	@chmod +x scripts/qa/verify-issue-245.sh scripts/qa/verify-issue-238.sh scripts/lib/lcd-smart-query.sh
	./scripts/qa/verify-issue-245.sh

# GitLab #274 — live LocalTerra gas: CleanLimitBook traversal cap + resume cursor.
verify-issue-274:
	@chmod +x scripts/qa/verify-issue-274.sh scripts/lib/lcd-smart-query.sh scripts/lib/e2e-terrad-tx.sh scripts/lib/localterra-host-curl.sh scripts/e2e-provision-dev-wallet.sh
	./scripts/qa/verify-issue-274.sh

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

# GitLab #295 — limit ladder rung count UI (Playwright against make dev on :5173).
verify-issue-295:
	@chmod +x scripts/verify-issue-295-ladder-rung-ui.sh scripts/with-node.sh
	./scripts/verify-issue-295-ladder-rung-ui.sh

# Cloud Agent: dockerd + LocalTerra + deploy + .env.local (+ optional indexer/frontend tmux).
setup-cloud-localterra:
	@chmod +x scripts/setup-cloud-agent-localterra.sh scripts/setup-cloud-agent-indexer-postgres.sh
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

help:
	@echo "Infrastructure:  make start | stop | reset | status | compose-ps | wait-localterra | wait-healthy | swarm-local | swarm-launch | swarm-stop"
	@echo "QA server:       make start-qa | reset-qa | QA_FRESH_VOLUMES=1 make start-qa | QA_FETCH_CI_ARTIFACTS=1 make start-qa | stop-qa | qa-verify-deploy | test-qa-redeploy-decision"
	@echo "Contracts:       make build-optimized | deploy-local | deploy-local-no-build | deploy-testnet | deploy-mainnet"
	@echo "QA artifacts:    make fetch-qa-ci-artifacts | make build-indexer-release (INDEXER_QA_BIN)"
	@echo "Cloud Agent:     make setup-cloud-localterra | setup-indexer-postgres | test-indexer-integration | verify-issue-324 | verify-issue-295 (needs make dev)"
	@echo "Frontend:        make dev | build-frontend | test-frontend | test-frontend-charts | test-charts-integration | test-e2e-tx | test-e2e-indexer-outage | lint-frontend"
	@echo "Indexer:         make indexer-dev | test-indexer-integration"
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
	-p cl8y-dex-burn-hook -p cl8y-dex-tax-hook -p cl8y-dex-lp-burn-hook

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

# Indexer
indexer-dev:
	cd indexer && cargo run

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

lint: lint-contracts lint-frontend check-fee-discount-tier-docs

# Git hooks
setup-hooks:
	git config core.hooksPath .githooks
	@echo "Git hooks configured to use .githooks/"

test-commit-msg-hook:
	@./scripts/test-commit-msg-hook.sh
