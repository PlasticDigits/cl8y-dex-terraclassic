.PHONY: start stop restart reset build-contracts build-artifacts-cargo build-optimized deploy-local deploy-testnet deploy-mainnet dev dev-full indexer-dev test-contracts coverage-contracts test-frontend test-e2e lint setup-hooks wait-healthy

# Infrastructure
start:
	docker compose up -d

stop:
	docker compose down

restart: stop start

reset:
	docker compose down -v

status:
	docker compose ps

logs:
	docker compose logs -f

logs-terra:
	docker compose logs -f localterra

wait-healthy:
	@echo "Waiting for LocalTerra..."
	@for i in $$(seq 1 60); do \
		if curl -sf http://localhost:26657/status > /dev/null 2>&1; then \
			echo "LocalTerra is ready!"; \
			break; \
		fi; \
		if [ "$$i" -eq 60 ]; then \
			echo "ERROR: LocalTerra did not start in time."; \
			exit 1; \
		fi; \
		sleep 2; \
	done
	@echo "Waiting for Postgres..."
	@for i in $$(seq 1 30); do \
		if pg_isready -h localhost -U postgres > /dev/null 2>&1 \
			|| docker compose exec -T postgres pg_isready -U postgres > /dev/null 2>&1; then \
			echo "Postgres is ready!"; \
			break; \
		fi; \
		if [ "$$i" -eq 30 ]; then \
			echo "ERROR: Postgres did not start in time."; \
			exit 1; \
		fi; \
		sleep 2; \
	done

# Smart contracts — two different builds:
#
#   make build-optimized  →  CosmWasm workspace-optimizer (Docker). This is what deploy-local and
#                            production use: small, deterministic, optimizer-processed wasm in
#                            smartcontracts/artifacts/. Does NOT require wasm32-unknown-unknown on
#                            the host — only Docker.
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

# Deployment
deploy-local: build-optimized
	./scripts/deploy-dex-local.sh

deploy-testnet:
	cd smartcontracts && ./scripts/deploy.sh testnet

deploy-mainnet:
	cd smartcontracts && ./scripts/deploy.sh mainnet

# Frontend
dev:
	cd frontend-dapp && npm run dev

build-frontend:
	cd frontend-dapp && npm run build

test-frontend:
	cd frontend-dapp && npm run test:run

test-e2e:
	cd frontend-dapp && npm run test:e2e

lint-frontend:
	cd frontend-dapp && npm run lint

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
	cd frontend-dapp && npm run dev

# Combined
test: test-contracts test-frontend

lint: lint-contracts lint-frontend

# Git hooks
setup-hooks:
	git config core.hooksPath .githooks
	@echo "Git hooks configured to use .githooks/"
