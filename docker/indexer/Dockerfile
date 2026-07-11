# Multi-stage build for Coolify (no docker-compose).
# Deploy at https://indexer.dex.cl8y.com — set RUN_MODE=prod and operator LCD_URLS.
#
# Build context: repository root
#   docker build -f docker/indexer/Dockerfile -t cl8y-dex-indexer .

FROM rust:1.96-bookworm AS builder

WORKDIR /build
RUN apt-get update \
  && apt-get install -y --no-install-recommends pkg-config libssl-dev \
  && rm -rf /var/lib/apt/lists/*

COPY indexer/ ./
# sqlx::migrate!() embeds ./migrations at compile time
RUN cargo build --release \
  && strip target/release/cl8y-dex-indexer

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates libssl3 curl \
  && rm -rf /var/lib/apt/lists/* \
  && useradd --system --uid 10001 --create-home indexer

COPY --from=builder /build/target/release/cl8y-dex-indexer /usr/local/bin/cl8y-dex-indexer

USER indexer
WORKDIR /app

ENV API_BIND=0.0.0.0 \
    API_PORT=3001 \
    RUST_LOG=info

EXPOSE 3001

# Required at runtime (Coolify env): DATABASE_URL, FACTORY_ADDRESS, CORS_ORIGINS,
# RUN_MODE=prod, LCD_URLS (non-default). See docs/runbooks/mainnet-soft-launch.md
HEALTHCHECK --interval=30s --timeout=5s --start-period=40s --retries=3 \
  CMD curl -fsS "http://127.0.0.1:${API_PORT}/health" >/dev/null || exit 1

CMD ["cl8y-dex-indexer"]
