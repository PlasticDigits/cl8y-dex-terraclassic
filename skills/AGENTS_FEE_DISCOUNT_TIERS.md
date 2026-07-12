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
- CL8Y uses **18 decimals** (`1 CL8Y = 10^18` base units). Frontend [`tokenRegistry.ts`](../frontend-dapp/src/utils/tokenRegistry.ts) must list CL8Y at **18** so `/tiers` Hold labels match `min_cl8y_balance` ([#476](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/476)).
- **LocalTerra:** `scripts/deploy-dex-local.sh` deploys **TCL8Y** (18-decimal proxy) for `cl8y_token` / `VITE_CL8Y_TOKEN_ADDRESS`; trading tokens (EMBER, etc.) are 6 decimals and must not substitute ([#383](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/383)). Verify: `make verify-issue-383`.
- **Frontend gas (GitLab #384):** `/tiers` **`register`** / **`deregister`** must use dedicated limits in [`terraGas.ts`](../frontend-dapp/src/services/terraclassic/terraGas.ts) (`REGISTER_FEE_DISCOUNT_GAS_LIMIT` **300k**, `DEREGISTER_FEE_DISCOUNT_GAS_LIMIT` **250k**). The dApp does not LCD-simulate execute gas — missing cases fell through to **`BASE_GAS_LIMIT` (200k)** and blocked FT-3 / FT-4 UI on LocalTerra. Verify: `make verify-issue-384`.
- Effective pair fee: `fee_bps * (10000 - discount_bps) / 10000` (integer division).
- Router must be on the fee-discount **trusted router** list before `trader` forwarding applies on **execute**.
- **Quotes:** pass optional `trader` (and `sender` if needed) on `HybridSimulation` / router `SimulateSwapOperations` for execute-matching discounted output ([GitLab **#238**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/238), [skills/AGENTS_HYBRID_QUOTING.md](./AGENTS_HYBRID_QUOTING.md)). **dApp + indexer** must forward the connected wallet on all quote LCD hops and route-solve calls ([#245](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/245)).
- **Pair cache ([GitLab #251](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/251)):** `execute_swap` and limit placement cache registry `GetDiscount` for **300s** per `(trader, sender)`; sim reads cache only. Do not assume quotes re-query registry every block within TTL — document **I9** in [`docs/reference/fee-discount-tiers.md`](../docs/reference/fee-discount-tiers.md). Constant: `dex_common::pair::DISCOUNT_CACHE_TTL_SECONDS`.
- **Registry outage observability ([GitLab #365](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/365)):** On-chain **fail-closed** to full pair fee when registry `GetDiscount` errors — do not change without ADR. Off-chain: indexer background LCD `config` probe → `GET /api/v1/health/fee-discount` (`fee_discount_registry_ok`, `consecutive_lcd_failures`; no per-trader errors). dApp **Swap and Pool** show a non-blocking amber warning when LCD registration/discount queries fail or indexer health is down. Utility: [`feeDiscountRegistryWarning.ts`](../frontend-dapp/src/utils/feeDiscountRegistryWarning.ts); shared hook: [`useFeeDiscountRegistryStatus.ts`](../frontend-dapp/src/hooks/useFeeDiscountRegistryStatus.ts). Invariant **I10** / **I12** in [`docs/reference/fee-discount-tiers.md`](../docs/reference/fee-discount-tiers.md). Integrator decision table: [`docs/integrators.md` § Fee-discount registry outage](../docs/integrators.md#fee-discount-registry-outage). Regression: `make verify-issue-365`.
- **Pool / Swap eligibility UX ([GitLab #476](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/476)):** Do **not** imply passive hold-only discounts. Unregistered wallets get a same-origin `/tiers` CTA on Swap **and** Pool; eligibility copy must cite the configured CL8Y CW20. Do not change the on-chain register+balance model in this issue family. Frontend docs: [`docs/frontend.md` § Pool page fee-discount UX](../docs/frontend.md#pool-page-fee-discount-ux).

## Indexer tier sync (GitLab #364)

When `FEE_DISCOUNT_ADDRESS` is set:

| Path | Behavior |
|------|----------|
| **Block parsing** | `register`, `register_wallet`, `deregister`, `deregister_wallet` wasm events on the fee-discount contract update `traders.tier_id` / `tier_name` / `registered` within the indexed block (no periodic LCD scan). |
| **First swap** | New `traders` row from swap ingestion triggers one `get_registration` LCD query (lazy hydrate). |
| **Reconcile** | Background full-table `get_registration` loop defaults to **24h** (`TIER_SYNC_RECONCILE_INTERVAL`, min 60s) for drift correction only. |

Route solver GET cache keys on resolved `discount_tier` from synced `traders.tier_id` ([#283](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/283), [#245](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/245)). Integration tests: [`indexer/tests/indexer_tier_sync.rs`](../indexer/tests/indexer_tier_sync.rs). Invariants: [`docs/indexer-invariants.md`](../docs/indexer-invariants.md).

## Verification

```bash
make check-fee-discount-tier-docs
# or
python3 scripts/check_fee_discount_tier_docs.py

# Registry outage observability (#365 / #375) — no LocalTerra
make verify-issue-365
```

Run **`make check-fee-discount-tier-docs`** (reference job `docs-fee-discount-tiers`; [docs/testing.md § CI](../docs/testing.md#ci)) when `docs/**`, `scripts/deploy-dex-local.sh`, or `tier_fixtures.rs` change.

Run **`make verify-issue-365`** after edits to P5/I10 docs, indexer `GET /api/v1/health/fee-discount`, or [`feeDiscountRegistryWarning.ts`](../frontend-dapp/src/utils/feeDiscountRegistryWarning.ts).

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

## Registry outage observability (GitLab #365 / #374 / #476)

When the fee-discount registry LCD is unreachable, on-chain swaps **fail closed to full pair fee** (`smartcontracts/contracts/pair/src/discount_cache.rs`). Traders must not confuse that with “not registered”.

| Signal | Where |
|--------|--------|
| LCD `get_registration` / `get_discount` errors | [`feeDiscountRegistryWarning.ts`](../frontend-dapp/src/utils/feeDiscountRegistryWarning.ts) + [`useFeeDiscountRegistryStatus`](../frontend-dapp/src/hooks/useFeeDiscountRegistryStatus.ts) → amber banner on **Swap** and **Pool** |
| Indexer `GET /api/v1/health/fee-discount` (`fee_discount_registry_ok: false`) | Same banner when LCD reads still succeed |
| Unregistered + healthy LCD | “Hold CL8Y & register…” CTA on Swap **and** Pool — **no** outage banner |

Frontend helpers: `resolveFeeDiscountRegistryStatus`, `shouldShowFeeDiscountRegistryWarning`, `FEE_DISCOUNT_REGISTRY_WARNING_TEXT`, [`feeDiscountUiCopy.ts`](../frontend-dapp/src/utils/feeDiscountUiCopy.ts). Indexer client: `getFeeDiscountHealth()` in `frontend-dapp/src/services/indexer/client.ts`. Handler: `indexer/src/api/fee_discount_health.rs`.

**Invariants:** warning is non-blocking (swap / pool actions stay enabled); banner copy must not include raw LCD errors or wallet addresses; CTA stays same-origin `/tiers`; do not add public per-trader registry probe APIs from the frontend; do not invent UI discounts for lookalike tokens.

**Verify:** `make test-frontend` (`feeDiscountRegistryWarning.test.ts`, `feeDiscountUiCopy.test.ts`, `PoolPage.feeDiscountRegistryBanner.test.tsx`, `SwapPage.feeDiscountRegistryBanner.test.tsx`, `SwapPage.test.tsx`); indexer `cargo test --test api_fee_discount_health`; `make verify-issue-365` still passes after Pool shares the warning util.
