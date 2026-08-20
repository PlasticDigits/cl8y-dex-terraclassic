# Agent playbook: factory discount-registry snapshot (GitLab #536)

Use when creating new pairs after the fee-discount contract exists, migrating factory/pair wasm, or debugging unwired pairs (`GetDiscountRegistry` → `null`, full pair fee).

Parent ops for **already listed** economic pairs: [#535](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/535) (out of scope here).

## Canonical references

| Artifact | Role |
|----------|------|
| Invariant **F5** | [`docs/contracts-security-audit.md`](../docs/contracts-security-audit.md) |
| Invariant **I14** | [`docs/reference/fee-discount-tiers.md`](../docs/reference/fee-discount-tiers.md) |
| Contract reference | [`docs/contracts-terraclassic.md` § Factory discount registry snapshot](../docs/contracts-terraclassic.md#factory-discount-registry-snapshot-gitlab-536) + [§ #538](../docs/contracts-terraclassic.md#gitlab-538) |
| Factory `Config.discount_registry` | [`factory/src/state.rs`](../smartcontracts/contracts/factory/src/state.rs) |
| Pair `GetDiscountRegistry` | [`dex_common::pair`](../smartcontracts/packages/dex-common/src/pair.rs) |
| Tier ladder / I10 fail-closed | [`AGENTS_FEE_DISCOUNT_TIERS.md`](./AGENTS_FEE_DISCOUNT_TIERS.md) |

## Invariants (F5 / I14)

1. **Factory pointer** — `config.discount_registry: Option<Addr>` with `#[serde(default)]` so pre-1.8.0 columbus-5 state loads as `None`.
2. **CreatePair snapshot** — factory copies `config.discount_registry` into `PairInstantiateMsg.discount_registry` (optional, default `None`). Pair instantiate stores that value; it does **not** hardcode `None`.
3. **All / Batch write the pointer** — `SetDiscountRegistryAll` / `SetDiscountRegistryBatch` with `Some(registry)` persist the factory default (so the next listing inherits). `None` clears the factory pointer **and** clears indexed pairs in that All/Batch. All that hits the pair-count cap **reverts** (pointer unchanged) — use Batch.
4. **Single-pair does not** — `SetDiscountRegistry { pair, registry }` updates that pair only.
5. **UpdateConfig** — `UpdateConfig { discount_registry: Some(addr) }` sets the factory pointer **without** touching indexed pairs. Omit the field to leave it.
6. **Query** — pair `GetDiscountRegistry` returns `{ "registry": <addr\|null> }`. Factory `Config` includes `discount_registry`. dApp/indexer must not assume every pair is wired. dApp probe is **smart-query-first** (`GetDiscountRegistry`) with LCD raw fallback ([#538](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/538), **F538-3**).
7. **I10 unchanged** — registry query errors still fail closed to full pair `fee_bps`. Do not change that here.
8. **Existing pairs** — not retroactively wired. That is #535.
9. **Post-migrate pointer (F538-1)** — after factory 1.8.0 migrate, `config.discount_registry` is `None` until All/Batch or `UpdateConfig`. Columbus-5 already set the pointer (wasm **11585**).
10. **Live inherit (F538-2)** — a new `create_pair` must match the factory pointer with no follow-up `SetDiscountRegistry`. Dedicated LocalTerra check: [`scripts/qa/localterra-create-pair-inherit.sh`](../scripts/qa/localterra-create-pair-inherit.sh).

## Versions

| Contract | cw2 |
|----------|-----|
| Factory | **1.9.0** |
| Pair | **1.15.0** |

Migrate factory then pair. After migrate, run All (≤10 pairs) or Batch until `has_more=false` **once** so the factory pointer is set; new `CreatePair`s inherit. Pre-migrate listings stay unwired until All/Batch or per-pair `SetDiscountRegistry`.

## Operator sequence (new listing)

1. Confirm factory `config.discount_registry` is the fee-discount contract (`{"config":{}}`).
2. If it is `null`, governance `update_config` **or** `set_discount_registry_all` / `_batch` (All/Batch also fan out to **indexed** pairs).
3. `create_pair` — query the new pair `{"get_discount_registry":{}}`; registry must match without a follow-up tx.
4. Optional idempotent per-pair `set_discount_registry` remains valid (local/mainnet scripts).

UST1 secondary AMM: [`AGENTS_UST1_SECONDARY_AMM.md`](./AGENTS_UST1_SECONDARY_AMM.md) — after this ships, post-create `set_discount_registry` is belt-and-suspenders if the factory pointer is already set.

## Do not

- Rely on operators remembering a post-create sweep for **new** pairs (that is how UST1/cUSTC, UST1/USTR, and cLUNC/UST1 launched unwired).
- Use single-pair `SetDiscountRegistry` to set the factory default.
- Change fail-closed fee behavior when `GetDiscount` errors (**I10** / **P5**).
- Treat this issue as wiring the three live economic pairs (#535).

## Verification

```bash
make verify-issue-536
make test-contracts   # includes the #536 inherit tests
make verify-issue-538 # LocalTerra create_pair inherit + dApp smart-query-first
```

No LocalTerra required for `make verify-issue-536`. **#538 live inherit** needs LocalTerra + `frontend-dapp/.env.local`:

```bash
make has-localterra   # exit 0 = chain up
./scripts/qa/localterra-create-pair-inherit.sh
```

That script instantiates two throwaway CW20s, `create_pair`, and asserts `GetDiscountRegistry` matches `config.discount_registry` **without** executing `SetDiscountRegistry`. `deploy-dex-local.sh` still runs an idempotent per-pair set after asserting inherit (**F538-2**).

Columbus-5 factory pointer (**F538-1**) is ops, already set on wasm **11585** — do not re-run All/Batch from this playbook unless `{"config":{}}` shows `discount_registry: null`.

## Related

- [`AGENTS_FEE_DISCOUNT_TIERS.md`](./AGENTS_FEE_DISCOUNT_TIERS.md) — I8 rollout, I10 outage, I13 placement
- [`AGENTS_FRONTEND_PAIR_FEE_DISCOUNT.md`](./AGENTS_FRONTEND_PAIR_FEE_DISCOUNT.md) — I14 chrome + **F538-3** smart-query-first
- [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md) — All cap / Batch pagination
- [`AGENTS_UST1_SECONDARY_AMM.md`](./AGENTS_UST1_SECONDARY_AMM.md)
- [`AGENTS_WASM_MIGRATION_ROLLBACK.md`](./AGENTS_WASM_MIGRATION_ROLLBACK.md)
