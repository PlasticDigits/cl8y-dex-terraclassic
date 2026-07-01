# Agent playbook: indexer FACTORY_ADDRESS guard (SEC-I02)

Use when verifying that the indexer **refuses to start** with an empty or whitespace-only `FACTORY_ADDRESS` in **every** `RUN_MODE` ([#451](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/451)).

## Problem

When `FACTORY_ADDRESS` is empty, [`verify_factory_provenance`](../indexer/src/indexer/pair_discovery.rs) skips the factory provenance check and any contract emitting swap events can be indexed — including attacker-controlled clone pairs. Before [#451](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/451), only `RUN_MODE=prod` rejected an empty factory address; staging/QA could ship with `FACTORY_ADDRESS=""` and silently index unverified pairs.

## Guard (invariant Q5)

| ID | Invariant |
|----|-----------|
| **Q5** | Indexer config load rejects empty or whitespace-only `FACTORY_ADDRESS` in every `RUN_MODE` (`ConfigError::EmptyFactoryAddress`); post-deploy QA also asserts the env var is set before schema checks. |

## Automated check

| Step | Command |
|------|---------|
| Config guard unit test | `cd indexer && cargo test --lib empty_factory_address_rejected_in_dev` |
| Startup rejection (manual) | `FACTORY_ADDRESS='   ' DATABASE_URL=… CORS_ORIGINS=… ./target/debug/cl8y-dex-indexer` → exit 1 with `FACTORY_ADDRESS must be non-empty` |
| Post-deploy pre-flight | **`make qa-verify-deploy`** → [`scripts/qa/verify-deploy.sh`](../scripts/qa/verify-deploy.sh) (exits non-zero when `FACTORY_ADDRESS` missing from env) |
| Doc drift | **`make check-factory-address-docs`** |
| Issue acceptance | **`make verify-issue-451`** |

## Launch sign-off

- [launch checklist Phase 0](../docs/runbooks/launch-checklist.md#phase-0--preconditions) — **Indexer FACTORY_ADDRESS (SEC-I02)** checkbox before off-chain stack deploy.
- CI **`test-indexer-lib`** runs `cargo test --lib`, including `empty_factory_address_rejected_in_dev`.

## Related

- Pair provenance invariant **P1** — [`docs/indexer-invariants.md`](../docs/indexer-invariants.md)
- Env address cross-check **Q4** — [`skills/AGENTS_DEPLOY_ENV_ADDRESSES_VERIFY.md`](./AGENTS_DEPLOY_ENV_ADDRESSES_VERIFY.md) ([#442](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/442))
- Operator env reference — [`docs/operator-secrets.md`](../docs/operator-secrets.md)
