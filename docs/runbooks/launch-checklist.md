# Runbook: pool-only (v2) DEX launch

Ordered checklist for **pool-only** swaps: direct pair and router paths with **`hybrid` unset** (no on-chain limit-book leg). For hybrid-specific launch, see [`docs/reviews/20260409T030009Z/REVIEW.md`](../reviews/20260409T030009Z/REVIEW.md) §11.

**Related docs:** [`docs/deployment-guide.md`](../deployment-guide.md), [`docs/security-model.md`](../security-model.md), [`docs/architecture.md`](../architecture.md), fee tiers [`docs/reference/fee-discount-tiers.md`](../reference/fee-discount-tiers.md). **Full executable matrix:** [GitLab **#337**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/337) (**LR-00** launch-readiness gate).

---

## Phase 0 — Preconditions

- [ ] Governance and treasury addresses are **multisigs or DAO** (not EOAs); see [Security model § Governance](../security-model.md).
- [ ] **Wasm policy:** production code uploaded from **workspace-optimizer** artifacts (`make build-optimized`; reference spec [`.github/workflows/contracts-wasm-optimizer.yml`](../../.github/workflows/contracts-wasm-optimizer.yml)), not from dev `cargo` wasm alone — [docs/testing.md § CI](../testing.md#ci).
- [ ] **Hook policy:** either **no hooks** on pairs, or only **audited** hook contracts with bounded gas (hook revert fails the whole swap — [Security model § Hook safety](../security-model.md)). Follow [hook registration runbook](./hook-registration.md).
- [ ] **Code ID whitelist** on the factory lists only intended CW20 code IDs for pair assets. **No fee-on-transfer templates** — [CW20 whitelist policy](./cw20-whitelist-policy.md); run [`scripts/verify-cw20-code-ids.sh`](../../scripts/verify-cw20-code-ids.sh) for GDEX/TerraPort IDs before whitelist.

---

## Phase 1 — Deploy contracts

Follow [`docs/deployment-guide.md`](../deployment-guide.md): optimized wasm → store → instantiate factory (governance, treasury, fees, whitelist) → router → fee-discount → tiers → **trusted router** → `set_discount_registry_all` (or per-pair) → create pairs.

**Verify (replace placeholders, chain id, node, fees):**

```bash
terrad query wasm contract-state smart <factory> '{"get_config":{}}' --node <lcd>
terrad query wasm contract-state smart <router> '{"config":{}}' --node <lcd>
```

---

## Phase 2 — Governance-sensitive settings

- [ ] **Treasury** on factory matches intended fee recipient (`get_config`).
- [ ] **Fee-discount tiers** match [`docs/reference/fee-discount-tiers.md`](../reference/fee-discount-tiers.md) (or your approved variant); `make check-fee-discount-tier-docs` passes ([GitLab #198](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/198)).
- [ ] **Router** registered as **trusted** on fee-discount (`IsTrustedRouter` = true) before relying on `trader` forwarding for discounts.
- [ ] **Discount registry** set on all pairs that should participate (`GetDiscountRegistry` per pair or factory-driven policy).

```bash
terrad query wasm contract-state smart <fee_discount> '{"get_tiers":{}}' --node <lcd>
terrad query wasm contract-state smart <fee_discount> '{"is_trusted_router":{"router":"<router_addr>"}}' --node <lcd>
```

- [ ] **Pause:** understand factory/pair pause implications for swaps and limit cancels ([`docs/limit-orders.md`](../limit-orders.md), security model).

---

## Phase 3 — Post-deploy verification (pool-only)

- [ ] **Read-only / light tx checks:** [`scripts/smoke-pool-swap.sh`](../../scripts/smoke-pool-swap.sh) — LCD pool query and optional **`hybrid_simulation`** with pool-only params (`book_input: 0`).
- [ ] **Single-hop swap** on staging with small size; confirm treasury fee and balances.
- [ ] **Multi-hop** via router (still pool-only per hop) if used in production.
- [ ] Optional: run repo E2E against staging (`frontend-dapp` Playwright) if your process includes UI gates.

**Pool-only invariant:** swap and simulation messages must **not** set hybrid / book-leg fields for this v2 launch path.

---

## Phase 4 — Off-chain stack (if applicable)

- [ ] **Indexer:** `DATABASE_URL`, migrations, `FACTORY_ADDRESS`, LCD URLs, `CORS_ORIGINS`, optional `ROUTER_ADDRESS` per [`indexer/src/config.rs`](../../indexer/src/config.rs).
- [ ] **Frontend:** `VITE_*` addresses per [`docs/frontend.md`](../frontend.md).

---

## Rollback / incident

- CosmWasm upgrades/migrations are **out of band** for this runbook; document admin keys and wasm migration policy separately.
- For live incidents: pause via factory if your governance policy allows; communicate hook/pause behavior per security model.
