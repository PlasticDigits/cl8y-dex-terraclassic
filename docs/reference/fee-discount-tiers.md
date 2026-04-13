# Fee discount registry — canonical tier table

Authoritative **mainnet-style** tier ladder for the CL8Y fee-discount contract. CL8Y uses **18 decimals** (`1 CL8Y = 10^18` smallest units).

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

- **Self-registration** is only allowed for tiers with `governance_only: false` (see [`docs/security-model.md`](../security-model.md)).
- **Effective swap fee:** `fee_bps * (10000 - discount_bps) / 10000` on the pair (integer division).
- **Trusted router:** the router must be registered on the fee-discount contract before `trader` forwarding applies for router-originated swaps.

Related: [`docs/deployment-guide.md`](../deployment-guide.md) §5, [`docs/architecture.md`](../architecture.md) (fee discount overview), integration tiers in `smartcontracts/tests/src/tier_fixtures.rs`.
