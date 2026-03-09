.PHONY: start stop reset build-contracts build-optimized deploy-local deploy-testnet deploy-mainnet dev test-contracts test-frontend test-e2e lint setup-hooks

# Infrastructure
start:
	docker compose up -d

stop:
	docker compose down

reset:
	docker compose down -v

status:
	docker compose ps

logs:
	docker compose logs -f

logs-terra:
	docker compose logs -f localterra

# Smart Contracts
build-contracts:
	cd smartcontracts && cargo build --release --target wasm32-unknown-unknown

build-optimized:
	cd smartcontracts && ./scripts/optimize.sh

test-contracts:
	cd smartcontracts && cargo test

lint-contracts:
	cd smartcontracts && cargo fmt --check && cargo clippy -- -D warnings

# Deployment
deploy-local:
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

# Combined
test: test-contracts test-frontend

lint: lint-contracts lint-frontend

# Git hooks
setup-hooks:
	git config core.hooksPath .githooks
	@echo "Git hooks configured to use .githooks/"
