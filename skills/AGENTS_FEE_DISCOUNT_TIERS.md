# Agent skill: fee discount tier ladder (docs + deploy)

**Audience:** third-party agents editing deployment docs, localnet scripts, or fee-discount integration tests.

## Single source of truth (GitLab #198)

| Artifact | Role |
|----------|------|
| [`docs/reference/fee-discount-tiers.md`](../docs/reference/fee-discount-tiers.md) | **Canonical** tier table, `min_cl8y_balance` wei strings, `terrad` JSON, invariants |
| [`smartcontracts/tests/src/tier_fixtures.rs`](../smartcontracts/tests/src/tier_fixtures.rs) | `STANDARD_PRODUCTION_TIERS` — must match canonical doc byte-for-byte |
| [`scripts/deploy-dex-local.sh`](../scripts/deploy-dex-local.sh) | Localnet `add_tier` payloads — must match canonical doc |

Do **not** duplicate numeric tier tables in [`docs/deployment-guide.md`](../docs/deployment-guide.md), [`docs/architecture.md`](../docs/architecture.md), or [`docs/contracts-terraclassic.md`](../docs/contracts-terraclassic.md). Link to the reference doc instead.

## Wire format invariants

- JSON field is **`min_cl8y_balance`** (string integer), not `min_tokens`.
- `ExecuteMsg::AddTier` includes **`governance_only`**: tiers **0** and **255** are governance-only; **1–9** are self-register (EOA).
- CL8Y uses **18 decimals** (`1 CL8Y = 10^18` base units).
- Effective pair fee: `fee_bps * (10000 - discount_bps) / 10000` (integer division).
- Router must be on the fee-discount **trusted router** list before `trader` forwarding applies on **execute**.
- **Quotes:** pass optional `trader` (and `sender` if needed) on `HybridSimulation` / router `SimulateSwapOperations` for execute-matching discounted output ([GitLab **#238**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/238), [skills/AGENTS_HYBRID_QUOTING.md](./AGENTS_HYBRID_QUOTING.md)).

## Verification

```bash
make check-fee-discount-tier-docs
# or
python3 scripts/check_fee_discount_tier_docs.py
```

Run **`make check-fee-discount-tier-docs`** (reference job `docs-fee-discount-tiers`; [docs/testing.md § CI](../docs/testing.md#ci)) when `docs/**`, `scripts/deploy-dex-local.sh`, or `tier_fixtures.rs` change.

## Factory registry rollout (GitLab #242)

After the fee-discount contract is deployed and tiers are configured:

| `PAIR_COUNT` | Factory message |
|--------------|-----------------|
| 1 pair (targeted) | `set_discount_registry` with `pair` |
| ≤10 pairs | `set_discount_registry_all` (single tx) |
| >10 pairs | Repeat `set_discount_registry_batch` until response `has_more` is `false` (use `next_start_after` as cursor) |

Invariants and batch loop example: [docs/contracts-terraclassic.md § Factory discount registry rollout](../docs/contracts-terraclassic.md#factory-discount-registry-rollout-invariants-glab-123). Gas playbook: [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md).

## Related docs

- [docs/deployment-guide.md](../docs/deployment-guide.md) — §5a (points at reference)
- [docs/architecture.md](../docs/architecture.md) — fee discount flow (no duplicate table)
- [docs/security-model.md](../docs/security-model.md) — EOA registration, balance checks
- [docs/testing.md](../docs/testing.md) — fee-discount test coverage
