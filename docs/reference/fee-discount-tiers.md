# Fee discount registry — canonical tier table

Authoritative **mainnet-style** tier ladder for the CL8Y fee-discount contract. This file is the **only** place in the docs tree with a full numeric tier table ([GitLab #198](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/198)). CL8Y uses **18 decimals** (`1 CL8Y = 10^18` smallest units).

Wire format for `add_tier` matches [`ExecuteMsg::AddTier`](../../smartcontracts/contracts/fee-discount/src/msg.rs): `min_cl8y_balance` (string integer in JSON), `discount_bps`, optional `limit_discount_bps`, `governance_only`.

`discount_bps` applies to **swaps and book takes**. `limit_discount_bps` applies only to **maker placement** (`floor(effective/2)` of that discount). When omitted, placement uses `discount_bps` ([GitLab #514](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/514)). Production self-register tiers shift placement one step down the remaining-fee ladder; tier 9 placement is **0 bps**. CL8Y minima are unchanged.

At the default **180 bps** pair fee, placement targets are 81 / 72 / 58 / 45 / 36 / 22 / 13 / 4 / **0** bps for tiers 1–9 (unregistered stays 90). Crossing the book still charges the **taker** half of the taker’s swap `discount_bps` (unregistered retail: 90 bps).

| Tier ID | CL8Y held (min) | `min_cl8y_balance` (wei) | Discount (bps) | Limit discount (bps) | Discount % | `governance_only` |
|---------|-----------------|--------------------------|----------------|----------------------|------------|-------------------|
| 0 | 0 (assigned by gov) | `0` | 10000 | 10000 | 100% | `true` |
| 1 | 1 | `1000000000000000000` | 250 | 1000 | 2.5% | `false` |
| 2 | 5 | `5000000000000000000` | 1000 | 2000 | 10% | `false` |
| 3 | 20 | `20000000000000000000` | 2000 | 3500 | 20% | `false` |
| 4 | 75 | `75000000000000000000` | 3500 | 5000 | 35% | `false` |
| 5 | 200 | `200000000000000000000` | 5000 | 6000 | 50% | `false` |
| 6 | 500 | `500000000000000000000` | 6000 | 7500 | 60% | `false` |
| 7 | 1,500 | `1500000000000000000000` | 7500 | 8500 | 75% | `false` |
| 8 | 3,500 | `3500000000000000000000` | 8500 | 9500 | 85% | `false` |
| 9 | 7,500 | `7500000000000000000000` | 9500 | 10000 | 95% | `false` |
| 255 | 0 (assigned by gov) | `0` | 0 | 0 | 0% | `true` |

## Example `terrad` execute (tier 1)

Replace `<fee_discount_addr>`, wallet flags, chain id, node, and fees.

```bash
terrad tx wasm execute <fee_discount_addr> '{
  "add_tier": {
    "tier_id": 1,
    "min_cl8y_balance": "1000000000000000000",
    "discount_bps": 250,
    "limit_discount_bps": 1000,
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
| I7 | **Drift guard:** `make check-fee-discount-tier-docs` keeps this file, `tier_fixtures.rs`, `deploy-dex-local.sh`, and `scripts/lib/mainnet-soft-launch-defaults.sh` identical. |
| I8 | **Factory rollout:** point pairs at the fee-discount contract via factory `set_discount_registry` (one pair), `set_discount_registry_all` (≤10 pairs only), or paginated `set_discount_registry_batch` — see [contracts-terraclassic.md § Factory discount registry rollout](../contracts-terraclassic.md#factory-discount-registry-rollout-invariants-glab-123) ([#242](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/242)). **All/Batch also persist factory `config.discount_registry`** so later `CreatePair` inherits it ([#536](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/536), **I14**). |
| I9 | **Pair discount cache ([GitLab #251](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/251)):** When `discount_registry` is set, the pair caches `GetDiscount` per `(trader, sender)` for **`DISCOUNT_CACHE_TTL_SECONDS` = 300** (`dex_common::pair`). Entries with `needs_deregister: true` are never cached; cache is cleared when a deregister submessage is emitted. Stale tier within TTL may still apply the prior discount (bounded 5 min). `HybridSimulation` with `trader` reads the cache; it does not write. Pairs without registry incur no cache storage. |
| I10 | **Registry query failure → full fee (fail-closed, [#365](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/365)):** When the pair's registry `GetDiscount` smart query returns `Err`, execute and hybrid simulation use **full pair `fee_bps`** (no discount attrs). Swaps are **not** reverted. Integrators distinguish **unregistered** (`get_registration.registered = false`) from **registry unreachable** (LCD/`get_discount` errors, or indexer `GET /api/v1/health/fee-discount` → `fee_discount_registry_ok: false`). Decision table: [integrators.md § Fee-discount registry outage](../integrators.md#fee-discount-registry-outage). |
| I11 | **LocalTerra CL8Y proxy ([#383](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/383)):** `scripts/deploy-dex-local.sh` deploys **TCL8Y** (18 decimals, symbol `TCL8Y`) for fee-discount `cl8y_token` and `VITE_CL8Y_TOKEN_ADDRESS`. Trading tokens (EMBER, CORAL, …) stay at **6** decimals and must **not** be used as the CL8Y proxy. Regression: `make verify-issue-383`. |
| I12 | **dApp eligibility UX ([#476](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/476)):** Swap **and** Pool must surface that fee discount requires **hold** of the configured `cl8y_token` CW20 **and** `Register` on `/tiers`. Unregistered (or wrong-token) wallets see base pair fee + a same-origin `/tiers` CTA — not a silent “feature missing” badge. Registry outage warnings stay non-blocking on both pages. Frontend CL8Y display decimals are **18**. Docs: [frontend.md § Pool page fee-discount UX](../frontend.md#pool-page-fee-discount-ux). |
| I13 | **Limit placement vs swap discount ([#514](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/514)):** Maker placement uses `limit_discount_bps` (resolved; omitted → `discount_bps`). Swaps and book **takes** use `discount_bps` only. Standard ladder `limit_discount_bps` is the next tier’s swap discount; tier 9 → `10000` (0 bps placement at any pair fee). Unregistered / tier 255 stay full pair fee on both legs. Pair emit `effective_fee_bps` on `place_limit_order` is the **limit** effective used for `floor(eff/2)`. Helper: `dex_common::fee_discount::standard_shifted_limit_discount_bps`. Mainnet: migrate fee-discount **1.1.0** (backfills standard tier IDs) + pair **1.12.0**. Verify: `make verify-issue-514`. |
| I14 | **CreatePair inherits factory registry ([#536](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/536)); dApp chrome gated on pair registry ([#537](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/537)); post-migrate inherit + smart-query-first ([#538](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/538)):** Factory `Config.discount_registry` (serde default `None`) is copied into pair instantiate. `SetDiscountRegistryAll` / `SetDiscountRegistryBatch` write or clear that pointer; single-pair `set_discount_registry` does **not**. `UpdateConfig { discount_registry }` sets the pointer without touching indexed pairs. Pair query `GetDiscountRegistry` returns stored `Option<Addr>`. Existing listings are **not** auto-wired (that is [#535](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/535)). Fail-closed registry errors stay **I10**. Factory **1.8.0** + pair **1.14.0**. After migrate, the factory pointer must be set (All/Batch or `UpdateConfig`) so the next listing inherits (**F538-1**). A new LocalTerra `create_pair` must match that pointer with **no** follow-up `SetDiscountRegistry` (**F538-2**). Advertise a CL8Y tier discount (strikethrough `fee_bps`, “your discount”, maker place bps) only when that pair’s `DISCOUNT_REGISTRY` is set **and** equals `VITE_FEE_DISCOUNT_ADDRESS`. Unwired pairs (`None`) charge full `fee_bps` / `maker_fee_bps(fee_bps)` on-chain — do not invent a client-side discount. Probe: pair `GetDiscountRegistry` first; LCD raw key `discount_registry` (`ZGlzY291bnRfcmVnaXN0cnk=`) is fallback for 1.13.x wasm (**F538-3** / **F537-4**). Probe failure is fail-closed (full fee in chrome). Hybrid `trader` quotes stay the execute-aligned source of truth. Frontend: [`pairDiscountRegistry.ts`](../../frontend-dapp/src/utils/pairDiscountRegistry.ts). Verify: `make verify-issue-536` / `make verify-issue-537` / `make verify-issue-538`. Playbook: [`skills/AGENTS_FACTORY_DISCOUNT_REGISTRY.md`](../../skills/AGENTS_FACTORY_DISCOUNT_REGISTRY.md). |
| I15 | **Phone-width `/tiers` layout ([#651](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/651)):** Display-only. Self-register cards keep **Hold {n} CL8Y** and fee phrases intact at ≤390px; empty Register slot must not reserve width; How it works stacks on `<md`. Register / Deregister msgs, gas, and on-chain queries are unchanged. Do not duplicate this file’s numeric ladder on `/tiers` docs. Frontend: [frontend.md § Tiers Page](../frontend.md#tiers-page) (**T651-1–T651-8**). Verify: `make verify-issue-651`. Playbook: [`skills/AGENTS_FRONTEND_TIERS_PHONE.md`](../../skills/AGENTS_FRONTEND_TIERS_PHONE.md). |

## LocalTerra TCL8Y (GitLab #383)

`make deploy-local` instantiates **TCL8Y** (`Terra Classic CL8Y`, 18 decimals) and points the fee-discount contract plus `frontend-dapp/.env.local` `VITE_CL8Y_TOKEN_ADDRESS` at it. EMBER and other pair tokens remain 6-decimal QA liquidity assets only.

```bash
make deploy-local
make verify-issue-383   # TCL8Y decimals, tier-1 register, deregister (FT-3 / FT-4)
make verify-issue-384   # dApp gas limits for register/deregister (FT-3 / FT-4 UI)
```

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
- [GitLab #476](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/476) — Pool/Swap eligibility UX + CL8Y 18 decimals (invariant **I12**)
- [GitLab #514](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/514) — limit-order placement discount shift; swap/take unchanged (invariant **I13**)
- [GitLab #537](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/537) — dApp fee-tier chrome gated on pair `DISCOUNT_REGISTRY` (invariant **I14**); parent wiring [#535](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/535)
- [GitLab #651](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/651) — phone-width `/tiers` card + How it works layout (invariant **I15**)
- Integration tiers: `smartcontracts/tests/src/tier_fixtures.rs` (`STANDARD_PRODUCTION_TIERS`)
- Upgrade: [`scripts/upgrade-514-limit-discount.sh`](../../scripts/upgrade-514-limit-discount.sh)
