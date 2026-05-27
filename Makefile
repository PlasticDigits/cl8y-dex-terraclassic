.PHONY: start stop restart reset build-contracts build-artifacts-cargo build-optimized deploy-local deploy-testnet deploy-mainnet dev dev-full indexer-dev test-contracts coverage-contracts test-frontend test-e2e test-e2e-tx test-charts-integration tests-charts-integration lint check-fee-discount-tier-docs setup-hooks wait-localterra wait-healthy help compose-ps start-qa qa-start stop-qa reset-qa test-qa-fresh-volumes qa-tunnel-help qa-verify-deploy swarm-local swarm-launch swarm-stop

# Infrastructure
start:
	docker compose up -d

stop:
	docker compose down

restart: stop start

reset:
	docker compose down -v

status:
	@chmod +x scripts/status.sh
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

# 30 processes (5 swap types × 5 replicas + 5 limit makers); see scripts/bots/launch-swarm.sh
swarm-launch:
	@chmod +x scripts/bots/launch-swarm.sh
	./scripts/bots/launch-swarm.sh

swarm-stop:
	@chmod +x scripts/bots/stop-swarm.sh
	./scripts/bots/stop-swarm.sh

wait-localterra:
	@echo "Waiting for LocalTerra..."
	@for i in $$(seq 1 60); do \
		if curl -sf http://localhost:26657/status > /dev/null 2>&1; then \
			echo "LocalTerra is ready!"; \
			exit 0; \
		fi; \
		if [ "$$i" -eq 60 ]; then \
			echo "ERROR: LocalTerra did not start in time."; \
			exit 1; \
		fi; \
		sleep 2; \
	done

wait-healthy: wait-localterra
	@echo "Waiting for Postgres..."
	@chmod +x scripts/setup-postgres-dev-databases.sh
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

qa-tunnel-help:
	@chmod +x scripts/qa/print-qa-tunnel-instructions.sh scripts/qa/write-frontend-env-local.sh
	./scripts/qa/print-qa-tunnel-instructions.sh

stop-qa:
	@chmod +x scripts/qa/stop-qa.sh
	./scripts/qa/stop-qa.sh

qa-verify-deploy:
	@chmod +x scripts/qa/verify-deploy.sh scripts/lib/lcd-smart-query.sh
	./scripts/qa/verify-deploy.sh

help:
	@echo "Infrastructure:  make start | stop | reset | status | compose-ps | wait-localterra | wait-healthy | swarm-local | swarm-launch | swarm-stop"
	@echo "QA server:       make start-qa (alias qa-start) | reset-qa | QA_FRESH_VOLUMES=1 make start-qa | stop-qa | qa-verify-deploy | qa-tunnel-help"
	@echo "Contracts:       make build-optimized | deploy-local | deploy-testnet | deploy-mainnet"
	@echo "Frontend:        make dev | build-frontend | test-frontend | test-charts-integration | test-e2e-tx | lint-frontend"
	@echo "Indexer:         make indexer-dev"
	@echo "Docs:            scripts/qa/README.md"

# Smart contracts — two different builds:
#
#   make build-optimized  →  CosmWasm workspace-optimizer (Docker). This is what deploy-local and
#                            production use: small, deterministic, optimizer-processed wasm in
#                            smartcontracts/artifacts/. Does NOT require wasm32-unknown-unknown on
#                            the host — only Docker.
#
#   CI: PRs use plain cargo wasm in .github/workflows/test.yml (fast). Release-grade wasm is built
#   in .github/workflows/contracts-wasm-optimizer.yml (same optimize.sh); do not upload PR wasm to
#   mainnet as production artifacts.
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

# Deployment
deploy-local: build-optimized
	./scripts/deploy-dex-local.sh

deploy-testnet:
	cd smartcontracts && ./scripts/deploy.sh testnet

deploy-mainnet:
	cd smartcontracts && ./scripts/deploy.sh mainnet

# Frontend (Node via nvm — scripts/with-node.sh + .nvmrc)
WITH_NODE = bash scripts/with-node.sh --cwd frontend-dapp --

dev:
	$(WITH_NODE) npm run dev

build-frontend:
	$(WITH_NODE) npm run build

test-frontend:
	$(WITH_NODE) npm run test:run

test-charts-integration tests-charts-integration:
	@chmod +x scripts/test-charts-integration.sh
	./scripts/test-charts-integration.sh

test-e2e:
	$(WITH_NODE) npm run test:e2e

# Strict on-chain Playwright (LocalTerra + deploy + global setup). Same as CI e2e job.
test-e2e-tx:
	@chmod +x scripts/deploy-dex-local.sh scripts/e2e-provision-dev-wallet.sh scripts/e2e-seed-hybrid-book.sh scripts/e2e-seed-wrap-pairs.sh scripts/with-node.sh
	docker compose up -d localterra
	$(MAKE) wait-localterra
	bash scripts/deploy-dex-local.sh
	$(WITH_NODE) npm run test:e2e:tx

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
	$(WITH_NODE) npm run dev

# Combined
test: test-contracts test-frontend

lint: lint-contracts lint-frontend check-fee-discount-tier-docs

# Git hooks
setup-hooks:
	git config core.hooksPath .githooks
	@echo "Git hooks configured to use .githooks/"
