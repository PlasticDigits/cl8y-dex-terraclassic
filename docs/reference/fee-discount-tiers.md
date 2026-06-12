# Fee discount registry — canonical tier table

Authoritative **mainnet-style** tier ladder for the CL8Y fee-discount contract. This file is the **only** place in the docs tree with a full numeric tier table ([GitLab #198](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/198)). CL8Y uses **18 decimals** (`1 CL8Y = 10^18` smallest units).

Wire format for `add_tier` matches [`ExecuteMsg::AddTier`](../../smartcontracts/contracts/fee-discount/src/msg.rs): `min_cl8y_balance` (string integer in JSON), `discount_bps`, `governance_only`.

| Tier ID | CL8Y held (min) | `min_cl8y_balance` (wei) | Discount (bps) | Discount % | `governance_only` |
|---------|-----------------|--------------------------|----------------|------------|-------------------|
| 0 | 0 (assigned by gov) | `0` | 10000 | 100% | `true` |
| 1 | 1 | `1000000000000000000` | 250 | 2.5% | `false` |
| 2 | 5 | `5000000000000000000` | 1000 | 10% | `false` |
| 3 | 20 | `20000000000000000000` | 2000 | 20% | `false` |
| 4 | 75 | `75000000000000000000` | 3500 | 35% | `false` |
| 5 | 200 | `200000000000000000000` | 5000 | 50% | `false` |
| 6 | 500 | `500000000000000000000` | 6000 | 60% | `false` |
| 7 | 1,500 | `1500000000000000000000` | 7500 | 75% | `false` |
| 8 | 3,500 | `3500000000000000000000` | 8500 | 85% | `false` |
| 9 | 7,500 | `7500000000000000000000` | 9500 | 95% | `false` |
| 255 | 0 (assigned by gov) | `0` | 0 | 0% | `true` |

## Example `terrad` execute (tier 1)

Replace `<fee_discount_addr>`, wallet flags, chain id, node, and fees.

```bash
terrad tx wasm execute <fee_discount_addr> '{
  "add_tier": {
    "tier_id": 1,
    "min_cl8y_balance": "1000000000000000000",
    "discount_bps": 250,
    "governance_only": false
  }
}' --from <wallet> --gas auto --gas-adjustment 1.4 \
  --fees 500000uluna --chain-id <chain-id> --node <rpc-url>
```

## Invariants

| ID | Rule |
|----|------|
| I1 | **Single doc table:** do not duplicate tier rows elsewhere under `docs/`; link here instead. |
| I2 | **Wire field:** `min_cl8y_balance` (string integer in JSON), never `min_tokens`. |
| I3 | **Self-registration:** only tiers with `governance_only: false` (tiers **1–9**); tiers **0** and **255** are governance-only. |
| I4 | **Effective swap fee:** `fee_bps * (10000 - discount_bps) / 10000` on the pair (integer division). |
| I5 | **Balance check:** each `GetDiscount` compares on-chain CL8Y balance to the registered tier minimum; insufficient balance → `discount_bps: 0` and lazy deregistration. |
| I6 | **Trusted router:** router must be registered before `trader` on router-originated swaps counts for discount lookup. |
| I7 | **Drift guard:** `make check-fee-discount-tier-docs` keeps this file, `tier_fixtures.rs`, and `deploy-dex-local.sh` identical. |
| I8 | **Factory rollout:** point pairs at the fee-discount contract via factory `set_discount_registry` (one pair), `set_discount_registry_all` (≤10 pairs only), or paginated `set_discount_registry_batch` — see [contracts-terraclassic.md § Factory discount registry rollout](../contracts-terraclassic.md#factory-discount-registry-rollout-invariants-glab-123) ([#242](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/242)). |
| I9 | **Pair discount cache ([GitLab #251](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/251)):** When `discount_registry` is set, the pair caches `GetDiscount` per `(trader, sender)` for **`DISCOUNT_CACHE_TTL_SECONDS` = 300** (`dex_common::pair`). Entries with `needs_deregister: true` are never cached; cache is cleared when a deregister submessage is emitted. Stale tier within TTL may still apply the prior discount (bounded 5 min). `HybridSimulation` with `trader` reads the cache; it does not write. Pairs without registry incur no cache storage. |
| I10 | **Registry query failure → full fee (fail-closed, [#365](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/365)):** When the pair's registry `GetDiscount` smart query returns `Err`, execute and hybrid simulation use **full pair `fee_bps`** (no discount attrs). Swaps are **not** reverted. Integrators distinguish **unregistered** (`get_registration.registered = false`) from **registry unreachable** (LCD/`get_discount` errors, or indexer `GET /api/v1/health/fee-discount` → `fee_discount_registry_ok: false`). |

## Drift check

```bash
make check-fee-discount-tier-docs
```

## Related

- [`docs/deployment-guide.md`](../deployment-guide.md) §5a — deploy steps (no inline tier numbers)
- [`docs/architecture.md`](../architecture.md) — fee discount flow diagram
- [`docs/contracts-terraclassic.md`](../contracts-terraclassic.md) — message reference (links here for tiers)
- [`docs/security-model.md`](../security-model.md) — EOA / trusted router rules
- [`skills/AGENTS_FEE_DISCOUNT_TIERS.md`](../../skills/AGENTS_FEE_DISCOUNT_TIERS.md) — third-party agent playbook (factory registry rollout I8)
- [GitLab #242](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/242) — cap `SetDiscountRegistryAll`; batch path for large `PAIR_COUNT`
- [GitLab #251](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/251) — on-pair discount cache TTL (invariant **I9**)
- Integration tiers: `smartcontracts/tests/src/tier_fixtures.rs` (`STANDARD_PRODUCTION_TIERS`)
