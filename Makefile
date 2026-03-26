.PHONY: start stop restart reset build-contracts build-optimized deploy-local deploy-testnet deploy-mainnet dev dev-full indexer-dev test-contracts coverage-contracts test-frontend test-e2e lint setup-hooks wait-healthy

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

# Smart Contracts
build-contracts:
	cd smartcontracts && cargo build --release --target wasm32-unknown-unknown

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
