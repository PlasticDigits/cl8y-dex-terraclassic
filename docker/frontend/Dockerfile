# Multi-stage static frontend for Coolify (no docker-compose).
# Deploy at https://dex.cl8y.com — pass VITE_* as build-args (baked into the bundle).
#
# Build context: repository root
#   docker build -f docker/frontend/Dockerfile \
#     --build-arg VITE_FACTORY_ADDRESS=terra1... \
#     --build-arg VITE_INDEXER_URL=https://indexer.dex.cl8y.com \
#     --build-arg VITE_WC_PROJECT_ID=... \
#     -t cl8y-dex-frontend .

FROM node:24-bookworm-slim AS builder

WORKDIR /app
COPY frontend-dapp/package.json frontend-dapp/package-lock.json ./
RUN npm ci

COPY frontend-dapp/ ./
# Create Pair catalog (#542) imports ../../../tokenlist/tokenlist.json from src/utils
# (WORKDIR /app → /tokenlist/tokenlist.json). Coolify does not copy the repo tokenlist otherwise.
COPY tokenlist/tokenlist.json /tokenlist/tokenlist.json

ARG VITE_NETWORK=mainnet
ARG VITE_FACTORY_ADDRESS
ARG VITE_ROUTER_ADDRESS
ARG VITE_FEE_DISCOUNT_ADDRESS
ARG VITE_CL8Y_TOKEN_ADDRESS=terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3
ARG VITE_INDEXER_URL=https://indexer.dex.cl8y.com
ARG VITE_TERRA_LCD_URL=https://terra-classic-lcd.publicnode.com
ARG VITE_TERRA_RPC_URL=https://terra-classic-rpc.publicnode.com:443
ARG VITE_GAS_PRICE_ULUNA=28.325
ARG VITE_WC_PROJECT_ID
ARG VITE_TREASURY_ADDRESS=
ARG VITE_WRAP_MAPPER_ADDRESS=
ARG VITE_LUNC_C_TOKEN_ADDRESS=
ARG VITE_USTC_C_TOKEN_ADDRESS=
ARG VITE_FAUCET_ADDRESS=
ARG VITE_TOKEN_EMBER_ADDRESS=
ARG VITE_TOKEN_CORAL_ADDRESS=
ARG VITE_TOKEN_JADE_ADDRESS=
ARG VITE_TOKEN_ONYX_ADDRESS=
ARG VITE_TOKEN_RUBY_ADDRESS=
ARG VITE_TOKEN_TOPAZ_ADDRESS=
# QA-only (#562). Must stay empty on dex.cl8y.com — never a runtime query param.
ARG VITE_SHOW_TEST_TOKENS=
# UST1 oracle window (GitLab #506) — Coolify prod must use columbus-5 addresses below.
ARG VITE_UST1_WINDOW_ADDRESS=terra1zxwpzpzpleatqn39r00grau4yt29sld8pw78s7ktvjafnj5nsaxq0h3rh2
ARG VITE_UST1_TOKEN_ADDRESS=terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72
ARG VITE_VFDUSD_TOKEN_ADDRESS=terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3
ARG VITE_UST1_ORACLE_ADDRESS=terra1fmht0t6svq3n24zx03nkfja0m40zhfyyxkdcvlrkl6u7gfe6aagq4gch8n

ENV VITE_NETWORK=$VITE_NETWORK \
    VITE_FACTORY_ADDRESS=$VITE_FACTORY_ADDRESS \
    VITE_ROUTER_ADDRESS=$VITE_ROUTER_ADDRESS \
    VITE_FEE_DISCOUNT_ADDRESS=$VITE_FEE_DISCOUNT_ADDRESS \
    VITE_CL8Y_TOKEN_ADDRESS=$VITE_CL8Y_TOKEN_ADDRESS \
    VITE_INDEXER_URL=$VITE_INDEXER_URL \
    VITE_TERRA_LCD_URL=$VITE_TERRA_LCD_URL \
    VITE_TERRA_RPC_URL=$VITE_TERRA_RPC_URL \
    VITE_GAS_PRICE_ULUNA=$VITE_GAS_PRICE_ULUNA \
    VITE_WC_PROJECT_ID=$VITE_WC_PROJECT_ID \
    VITE_TREASURY_ADDRESS=$VITE_TREASURY_ADDRESS \
    VITE_WRAP_MAPPER_ADDRESS=$VITE_WRAP_MAPPER_ADDRESS \
    VITE_LUNC_C_TOKEN_ADDRESS=$VITE_LUNC_C_TOKEN_ADDRESS \
    VITE_USTC_C_TOKEN_ADDRESS=$VITE_USTC_C_TOKEN_ADDRESS \
    VITE_FAUCET_ADDRESS=$VITE_FAUCET_ADDRESS \
    VITE_TOKEN_EMBER_ADDRESS=$VITE_TOKEN_EMBER_ADDRESS \
    VITE_TOKEN_CORAL_ADDRESS=$VITE_TOKEN_CORAL_ADDRESS \
    VITE_TOKEN_JADE_ADDRESS=$VITE_TOKEN_JADE_ADDRESS \
    VITE_TOKEN_ONYX_ADDRESS=$VITE_TOKEN_ONYX_ADDRESS \
    VITE_TOKEN_RUBY_ADDRESS=$VITE_TOKEN_RUBY_ADDRESS \
    VITE_TOKEN_TOPAZ_ADDRESS=$VITE_TOKEN_TOPAZ_ADDRESS \
    VITE_SHOW_TEST_TOKENS=$VITE_SHOW_TEST_TOKENS \
    VITE_UST1_WINDOW_ADDRESS=$VITE_UST1_WINDOW_ADDRESS \
    VITE_UST1_TOKEN_ADDRESS=$VITE_UST1_TOKEN_ADDRESS \
    VITE_VFDUSD_TOKEN_ADDRESS=$VITE_VFDUSD_TOKEN_ADDRESS \
    VITE_UST1_ORACLE_ADDRESS=$VITE_UST1_ORACLE_ADDRESS \
    NODE_OPTIONS=--max-old-space-size=4096

RUN test -n "$VITE_FACTORY_ADDRESS" \
  && test -n "$VITE_ROUTER_ADDRESS" \
  && test -n "$VITE_FEE_DISCOUNT_ADDRESS" \
  && test -n "$VITE_WC_PROJECT_ID" \
  && test -n "$VITE_INDEXER_URL" \
  && case "$VITE_INDEXER_URL" in https://*) ;; *) echo "VITE_INDEXER_URL must be https://" >&2; exit 1 ;; esac \
  && npm run build

FROM nginx:1.27-alpine AS runtime

COPY docker/frontend/nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=builder /app/dist /usr/share/nginx/html

EXPOSE 80

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD wget -qO- http://127.0.0.1/ >/dev/null || exit 1
