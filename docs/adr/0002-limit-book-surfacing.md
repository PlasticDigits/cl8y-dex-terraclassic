# ADR 0002: Limit book surfacing (indexer LCD proxy)

## Status

Accepted (amended: paginated `limit-book`; insert-hints + price window **#267**)

## Context

The FIFO limit book lives on the pair contract (`OrderBookHead`, `LimitOrder` queries). Integrators and the web dApp need a consistent way to read book state without each client re-implementing LCD wiring, CORS, and timeouts. Advanced UIs need **many** resting orders; a single HTTP response must stay bounded because each visible order may require one LCD `limit_order` query.

## Decision

**Primary surfacing path:** the indexer exposes read-only HTTP endpoints that **proxy CosmWasm smart queries** on LCD (`LcdClient::query_contract`), returning JSON aligned with on-chain shapes:

- `GET /api/v1/pairs/{addr}/order-book-head?side=bid|ask` — best order id on that side (or `null` if empty).
- `GET /api/v1/pairs/{addr}/limit-book?side=bid|ask&limit=L&after_order_id=OPTIONAL` — **paginated** walk from head or keyset cursor along `next` (`limit` default 50, max 100). Optional `price_from` + `price_to` return the contiguous in-band slice ([#267](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/267)). Response includes `has_more` and `next_after_order_id` for the next page.
- `GET /api/v1/pairs/{addr}/limit-book/insert-hints?side=bid|ask&prices=...` — batch insert-hint resolution (≤ **100** prices per request; **101** LCD query budget; `pagination_gap` when the walk cannot reach a price) ([#267](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/267)).
- `GET /api/v1/pairs/{addr}/limit-book-shallow?side=bid|ask&depth=N` — **legacy** small preview (default depth 10, max 20) for integrators and simple UIs; pro interfaces should prefer `limit-book`.

The dApp **Trade** and **Limits** pages consume these endpoints (and/or `order-book-head`). **Governance pause** for UX honesty uses a **direct LCD** query on the pair (`IsPaused`, see `docs/limit-orders.md`) so pause state does not depend on indexer availability.

## Non-goals

- Replacing pool-only `Simulation` / indexer route solve with book-inclusive server quotes (remains per ADR 0001 and `route/solve` scope).
- Server-side caching or coalescing of arbitrary deep book walks beyond documented per-request LCD costs (clients paginate).

## Consequences

- Indexer availability affects book display in the dApp; LCD errors surface as HTTP **502** from the proxy routes.
- Each `limit-book` page costs up to **1 + limit** LCD round-trips in the common case; clients should choose `limit` to match UI and continue with `after_order_id`.
- Invalid cursors or side mismatches return **400** with a short message (no raw LCD stack).

## Links

- [`docs/limit-orders.md`](../limit-orders.md)
- [`docs/integrators.md`](../integrators.md#on-chain-limit-book-lcd-proxy) — HTTP integrator reference
- [`docs/frontend.md`](../frontend.md#trade-page-deep-order-book) — dApp pagination invariants ([GitLab **#194**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/194))
- [`skills/AGENTS_FRONTEND_DEEP_ORDER_BOOK.md`](../skills/AGENTS_FRONTEND_DEEP_ORDER_BOOK.md) — agent playbook
- [`docs/integrators.md`](../integrators.md#insert-hints-price-window-gitlab-267) — insert-hints & price-window HTTP (**#267**)
- [`docs/contracts-security-audit.md`](../contracts-security-audit.md) (L6 pause semantics)
