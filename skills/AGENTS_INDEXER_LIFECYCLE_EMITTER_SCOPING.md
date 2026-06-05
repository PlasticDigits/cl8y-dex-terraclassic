# Agent skill: limit-order lifecycle emitter scoping (GitLab #285)

## When to use

You are changing **`indexer/src/indexer/parser.rs`** wasm contract scoping for limit-order lifecycle events (`limit_order_fill`, `place_limit_order`, `cancel_limit_order`, park/claim), or debugging forged lifecycle rows attributed to the wrong pair.

## Invariants

| ID | Invariant | Enforcement |
|----|-----------|-------------|
| **L285-1** | Only runtime-reserved **`_contract_address`** scopes lifecycle events | `wasm_contract_addr` / `is_wasm_contract_addr_key` match `_contract_address` only |
| **L285-2** | Unreserved **`contract_address`** is data, never the emitter | Forged victim pair address before `action` must not attribute rows to victim |
| **L285-3** | Genuine on-chain shape: `_contract_address` before `action`, optional `contract_address` after | `genuine_fill_with_both_contract_address_keys_attributes_to_pair` unit test |

Factory provenance (reject non-factory pairs on discovery) is **out of scope** here — track with [#279](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/279) / [#286](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/286) / [#287](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/287).

## Tests

**Unit (no Postgres):**

```bash
cd indexer && cargo test --lib \
  forged_contract_address_fill_not_attributed_to_victim_pair \
  forged_contract_address_cancel_not_attributed_to_victim_pair \
  forged_contract_address_placement_not_attributed_to_victim_pair \
  genuine_fill_with_both_contract_address_keys_attributes_to_pair
```

**Integration fixtures** must use `_contract_address`, not unreserved `contract_address` alone — see [`limit_order_parked_lifecycle.rs`](../indexer/tests/limit_order_parked_lifecycle.rs).

**Live LocalTerra + indexer (positive control):**

```bash
make start && make wait-healthy && make deploy-local
make verify-issue-285
```

Proves a real hybrid swap `limit_order_fill` is indexed under the pair's `_contract_address`.

## Cross-links

- Human doc: [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (lifecycle emitter scoping row)
- Parent: [`AGENTS_INDEXER_INGESTION_HARDENING.md`](./AGENTS_INDEXER_INGESTION_HARDENING.md)
- Hybrid fill E2E: [`AGENTS_E2E_LIMIT_ORDERS_TX.md`](./AGENTS_E2E_LIMIT_ORDERS_TX.md), [`AGENTS_E2E_HYBRID_SWAP.md`](./AGENTS_E2E_HYBRID_SWAP.md)
