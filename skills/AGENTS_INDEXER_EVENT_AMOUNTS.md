# Agent playbook: exact LCD event amounts

Apply for economic event parsing, fee ingestion, ledger replay or malformed-input
review in this indexer. Issue [#1395](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1395).
Contract: [A1395](../docs/indexer-invariants.md#exact-event-amounts-1395).
Review checklist and explicit skip policy: [QA](../docs/qa/issue-1395/README.md).
Evidence: [verification](../docs/qa/issue-1395/verification.md).

1. Use `indexer::event_amount::parse_raw_amount` for mandatory raw fields and
   `parse_optional_raw_amount` for optional ones. Raw aggregate sums use 58-digit integer storage; do not reintroduce saturation.
   Missing optional means `None`;
   conversion errors must propagate to rejection of the complete affected action.
2. Never use `unwrap_or_default`, permissive decimal parsing, floating conversion,
   trim, saturation or alias fallback after malformed primary input. Raw amounts
   are bounded ASCII integers; human token decimals are applied downstream.
3. Use `parse_price` for exact database-bounded prices. Preserve all column indices
   in flattened batch events; never `filter_map` one financial column independently.
4. Scope actions with reserved `_contract_address`, then retain factory verification
   or exact configured pins. Ordinary `contract_address` is untrusted data.
5. Count emission ordinals consistently through rejection. Keep ledger unique keys
   and unchanged-input replay idempotency. A malformed action must not become a
   block retry loop or discard valid siblings. Infrastructure errors still fail.
6. Use `report_amount_rejection` with static field/kind names and typed errors.
   Do not add raw attributes, tx hashes, token/contract/trader identities or
   unbounded per-event logs. Suppressed diagnostics are not persisted quarantine.
7. Run `make verify-issue-1395` against isolated Postgres; record exact revision,
   UTC, results and unavailable items. Overall PASS with explicit unavailable
   skips is permitted by the QA policy; an executed failure stays FAIL.

Cross-links: [ingestion hardening](AGENTS_INDEXER_INGESTION_HARDENING.md),
[fee hop identity](AGENTS_INDEXER_PROTOCOL_FEE_HOPS.md),
[Postgres](AGENTS_LOCAL_POSTGRES_DEV.md),
[reorg recovery](../docs/runbooks/indexer-reorg-replay-dedup.md).
