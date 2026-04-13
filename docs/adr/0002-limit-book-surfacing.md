# ADR 0002: Limit book surfacing (indexer LCD proxy)

## Status

Accepted

## Context

The FIFO limit book lives on the pair contract (`OrderBookHead`, `LimitOrder` queries). Integrators and the web dApp need a consistent way to read shallow book state without each client re-implementing LCD wiring, CORS, and timeouts.

## Decision

**Primary surfacing path:** the indexer exposes read-only HTTP endpoints that **proxy CosmWasm smart queries** on LCD (`LcdClient::query_contract`), returning JSON aligned with on-chain shapes:

- `GET /api/v1/pairs/{addr}/order-book-head?side=bid|ask` — best order id on that side (or `null` if empty).
- `GET /api/v1/pairs/{addr}/limit-book-shallow?side=bid|ask&depth=N` — walk from head along `next` (depth capped server-side, default 10, max 20).

The dApp **Limits** page consumes these endpoints for visible book/head. **Governance pause** for UX honesty uses a **direct LCD** query on the pair (`IsPaused`, see `docs/limit-orders.md`) so pause state does not depend on indexer availability.

## Non-goals

- Replacing pool-only `Simulation` / indexer route solve with book-inclusive server quotes (remains per ADR 0001 and `route/solve` scope).
- Deep full-book pagination in v1 (shallow cap only).

## Consequences

- Indexer availability affects book display in the dApp; LCD errors surface as HTTP 502 from the proxy routes.
- Walking `depth` levels costs up to `depth` LCD round-trips per request; keep `depth` small.

## Links

- [`docs/limit-orders.md`](../limit-orders.md)
- [`docs/contracts-security-audit.md`](../contracts-security-audit.md) (L6 pause semantics)
