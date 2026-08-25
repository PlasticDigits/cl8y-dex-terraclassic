# Smart Contract Reference

All message types are defined in `smartcontracts/packages/dex-common/src/`.

Message names follow TerraSwap/Terraport conventions for Vyntrex compatibility.

## Shared Types

### AssetInfo

```json
{ "token": { "contract_addr": "terra1..." } }
// or
{ "native_token": { "denom": "uluna" } }
```

> **Note:** `NativeToken` is accepted in the type system for wire compatibility with TerraSwap but is rejected at runtime. Only CW20 tokens (`Token` variant) are supported.

### Asset

```json
{ "info": <AssetInfo>, "amount": "1000000" }
```

### PairInfo (query response)

| Field             | Type             | Description                      |
|-------------------|------------------|----------------------------------|
| `asset_infos`     | `[AssetInfo; 2]` | The two assets in the pair       |
| `contract_addr`   | `Addr`           | Pair contract address            |
| `liquidity_token` | `Addr`           | CW20 LP token address            |

---

## Factory

### InstantiateMsg

| Field                  | Type       | Description                          |
|------------------------|------------|--------------------------------------|
| `governance`           | `String`   | Address with admin privileges        |
| `treasury`             | `String`   | Address that receives swap fees      |
| `default_fee_bps`      | `u16`      | Default fee in basis points (≤10000) |
| `pair_code_id`         | `u64`      | Stored code ID for Pair contract     |
| `lp_token_code_id`     | `u64`      | Stored code ID for CW20 LP token     |
| `whitelisted_code_ids` | `Vec<u64>` | Initial CW20 code IDs allowed        |
| `pair_creation_fee_uluna` | `Uint128` | uluna required on `CreatePair`, forwarded to treasury (default **100 LUNC**; GitLab [#276](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/276)) |

### ExecuteMsg

| Variant                    | Fields                                             | Auth        |
|----------------------------|----------------------------------------------------|-------------|
| `CreatePair`               | `asset_infos: [AssetInfo; 2]`                      | Anyone — attach **uluna** ≥ `pair_creation_fee_uluna` ([one create flow per block](./security-model.md#createpair-rate-limit-and-pending-state); [#121](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/121), [#276](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/276)) |
| `AddWhitelistedCodeId`     | `code_id: u64`                                     | Governance  |
| `RemoveWhitelistedCodeId`  | `code_id: u64`                                     | Governance  |
| `SetPairFee`               | `pair: String`, `fee_bps: u16`                     | Governance  |
| `SetPairTreasury`          | `pair: String`, `treasury: String`                 | Governance — rotate one pair’s commission recipient |
| `SetPairTreasuryAll`       | `treasury: String`                                 | Governance — **≤10 pairs** only; else `PairTreasuryAllTooManyPairs` |
| `SetPairTreasuryBatch`     | `treasury: String`, `start_after?: u64`, `limit?: u32` | Governance — paginated pair treasury rollout |
| `SetPairCreationFee`       | `fee_uluna: Uint128`                               | Governance — set uluna fee for `CreatePair` ([#276](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/276)) |
| `SetPairHooks`             | `pair: String`, `hooks: Vec<String>`               | Governance  |
| `SetDiscountRegistry`      | `pair: String`, `registry: Option<String>`         | Governance  |
| `SetDiscountRegistryAll` | `registry: Option<String>`                         | Governance — **≤10 pairs** only ([rollout invariants](#factory-discount-registry-rollout-invariants-glab-123), [#242](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/242)) |
| `SetDiscountRegistryBatch` | `registry: Option<String>`, `start_after?: u64`, `limit?: u32` | Governance — paginated rollout; see [invariants](#factory-discount-registry-rollout-invariants-glab-123) |
| `UpdateConfig`             | `governance?`, `treasury?`, `default_fee_bps?`, `pair_code_id?`, `lp_token_code_id?`, `discount_registry?` | Governance — factory pointer only (`treasury` applies to **new** pairs + pair-creation LUNC; live pair fees need `SetPairTreasury*`). `discount_registry` sets the default inherited by `CreatePair` without touching indexed pairs ([#536](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/536)) |
| `RefreshPairAssetCodeIds`  | `pair: String`                                     | Governance — re-pin live asset `code_id`s ([#582](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/582)) |
| `RefreshPairAssetCodeIdsBatch` | `start_after?: u64`, `limit?: u32`              | Governance — paginated pin refresh |

### QueryMsg

| Variant                  | Parameters                             | Returns            |
|--------------------------|----------------------------------------|--------------------|
| `Config`                 | —                                      | `ConfigResponse`   |
| `Pair`                   | `asset_infos: [AssetInfo; 2]`          | `PairResponse`     |
| `Pairs`                  | `start_after?: [AssetInfo; 2]`, `limit?` | `PairsResponse`  |
| `GetWhitelistedCodeIds`  | `start_after?`, `limit?`               | `CodeIdsResponse`  |
| `GetPairCount`           | —                                      | `PairCountResponse`|
| `IsCodeIdWhitelisted`    | `code_id: u64`                         | `CodeIdWhitelistedResponse` (GitLab [#582](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/582)) |

**`CreatePair`** rejects if either asset CW20 declares `decimals` greater than **`MAX_PAIR_ASSET_DECIMALS_BOOTSTRAP`** (see `dex_common::pair`, default **18**). This aligns with empty-pool `provide_liquidity` guards ([issue #124](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/124)). When `pair_creation_fee_uluna > 0`, the caller must attach at least that much **uluna** (only uluna accepted; overpay refunded) or the tx fails with `InsufficientPairCreationFee` ([#276](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/276)).

<a id="createpair-lp-ticker-gitlab-518"></a>

### `CreatePair` LP ticker (GitLab [#518](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/518))

LP tickers **keep digits `0-9`** and strip every **non-alphanumeric** character from each asset-symbol prefix. The upgraded LP CW20 (`cw20-mintable`) validates **`[a-zA-Z0-9\-]{3,12}`**. Classic Terraswap LP code (`[a-zA-Z\-]`, no digits) still rejects `UST1-CUST-LP` — factory `lp_token_code_id` must be rotated.

| Step | Behavior |
|------|----------|
| Factory | Queries each asset `TokenInfo.symbol`, truncates to 6 chars, uppercases, passes `token_symbols` into pair instantiate (also used in the pair wasm **label**). |
| Pair | [`dex_common::lp_symbol::lp_token_instantiate_meta`](../smartcontracts/packages/dex-common/src/lp_symbol.rs) — keep **name** / **label** unique; derive LP `symbol` from ASCII **alphanumeric** only (4-char prefix each) as `{a}-{b}-LP`. |
| Fallback | If the derived ticker is not length 3–12 after hyphen collapse, use **`CL8Y-LP`**. |
| Examples | UST1/cUSTC → `UST1-CUST-LP`; CL8Y/cLUNC → `CL8Y-CLUN-LP`; `FOO_BAR`/`BAZ!` → `FOOB-BAZ-LP`; cLUNC/cUSTC → `CLUN-CUST-LP`. |

**Upgrade:** store new factory + pair wasm, `wasm migrate` the factory (1.6.0 adds `UpdateConfig` code-id fields), then `UpdateConfig { pair_code_id, lp_token_code_id }` pointing LP at digit-allowing `cw20-mintable` (columbus-5 code **10184** may be reused). Script: [`scripts/upgrade-518-lp-symbol.sh`](../scripts/upgrade-518-lp-symbol.sh). Existing pairs keep their LP tokens.

Invariant **F3** in [contracts-security-audit.md](./contracts-security-audit.md). Agent playbook: [`skills/AGENTS_LP_SYMBOL_DIGITS.md`](../skills/AGENTS_LP_SYMBOL_DIGITS.md). Regression: `make verify-issue-518`. Blocks UST1 secondary AMM Path A until the new pair code is live — [`ust1-secondary-amm-pair.md`](./runbooks/ust1-secondary-amm-pair.md).

### Factory storage & upgrades

| Storage | Role |
|---------|------|
| `pairs` | Canonical asset-key → `PairInfo` |
| `pair_count` / `pair_index` | Sequential registry for paginated `Pairs` page reads (**O(limit)** range scan per page) |
| `pair_key_idx` | Canonical pair key → numeric index; **O(1)** resolve of `Pairs { start_after: [AssetInfo; 2] }` cursor ([GitLab #258](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/258)) |
| `pair_addr_reg` | Pair contract `Addr` → `true`; **O(1)** membership for governance paths that validate a single pair address |

**Invariant:** For each index `i` in `0..pair_count`, `pair_index[i].contract_addr` has a `true` entry in `pair_addr_reg`, and `pair_key_idx[pair_key(pair_index[i].asset_infos)] == i`. Maintained when pairs register in `reply_instantiate_pair`. Legacy factory instances on wasm **1.0.0** must migrate once to **1.1.0** so `pair_addr_reg` is backfilled from `pair_index` ([GitLab #122](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/122)); instances on **1.2.0** or older must migrate to **1.3.0** so `pair_key_idx` is backfilled ([GitLab #258](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/258)); instances on **1.3.x** must migrate to **1.4.0** so stored `Config` picks up `pair_creation_fee_uluna` default **100 LUNC** via `#[serde(default)]` ([#276](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/276)); instances on **1.5.x** must migrate to **1.6.0** so `UpdateConfig` can set `pair_code_id` / `lp_token_code_id` ([#518](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/518)); instances on **1.6.x** must migrate to **1.7.0** so `SetPairTreasury` / `SetPairTreasuryAll` / `SetPairTreasuryBatch` exist (pair wasm **1.11.0** adds `UpdateTreasury`); instances on **1.7.x** must migrate to **1.8.0** so `Config.discount_registry` exists (`#[serde(default)]` → `None`) and All/Batch persist that pointer for new `CreatePair` ([#536](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/536), pair wasm **1.14.0** adds instantiate copy + `GetDiscountRegistry`); instances on **1.8.x** must migrate to **1.9.0** so `IsCodeIdWhitelisted` and `RefreshPairAssetCodeIds*` exist ([#582](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/582); pair wasm **1.15.0** pins asset `code_id`s and re-checks on write paths).

**Gas / iteration:** Per-pair governance messages use **O(1)** `pair_addr_reg` where applicable ([#122](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/122)). **`Pairs` pagination** resolves `start_after` via **O(1)** `pair_key_idx` lookup, then reads at most `limit` entries from `pair_index` ([#258](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/258)). For **broadcasting** discount-registry updates across the factory, use **`SetDiscountRegistryBatch`** ([#123](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/123), [#242](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/242)) so each transaction carries a bounded Wasm message list. **`SetDiscountRegistryAll`** is a single-tx shortcut only when `PAIR_COUNT` ≤ the default pagination cap (**10**); otherwise the contract returns `DiscountRegistryAllTooManyPairs` and operators must paginate. **LP-token admin rotation** after governance change uses the same bounded model: **`SetLpAdminBatch`** ([#277](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/277)); **`SetLpAdminAll`** only when `PAIR_COUNT` ≤ **10** (`LpAdminAllTooManyPairs` otherwise). **`UpdateConfig`** no longer fans `SetLpAdmin` to every pair — run All/Batch explicitly in the rotation runbook. Pair commission recipients use the same bounded model: **`SetPairTreasuryAll`** / **`SetPairTreasuryBatch`**. Indexers or LCD clients listing pairs should paginate rather than relying on unbounded queries. Off-chain operators and automation: [Indexer invariants — Factory LCD](./indexer-invariants.md#factory-lcd-pair-enumeration-vs-governance-gas-agents).

### Factory discount registry rollout (invariants, [GitLab #123](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/123), [#242](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/242))

`SetDiscountRegistryAll` attaches **one** `WasmMsg::Execute(SetDiscountRegistry …)` **per indexed pair**, but only when `PAIR_COUNT` ≤ **`calc_limit(None)`** (default **10**). If the factory has more pairs, execution fails with **`DiscountRegistryAllTooManyPairs`** — use batch pagination instead (on-chain DoS / block-gas fix, [#242](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/242)).

**`SetDiscountRegistryBatch`** emits at most **`limit`** such messages per transaction (omit `limit` ⇒ default **`10`**; hard cap **`30`**, same as other factory queries — see `dex_common::pagination`).

| Field / attribute | Semantics |
|-------------------|-----------|
| `start_after` | Optional **exclusive** cursor on the numeric `PAIR_INDEX` key. `null` / absent means “before index `0`”. The next index scanned is `start_after + 1` (or `0` when absent). |
| Response attrs | `pairs_updated` (messages emitted this TX), `has_more` (`true` until all indices `< PAIR_COUNT` are scanned past in order), **`next_start_after`** (omit when finished — replay this value as `start_after` on the next TX when `has_more` is true), `scanned_through_index` (last numeric index inspected this TX). |

**Invariants:**

1. Indices are contiguous from `0` to `PAIR_COUNT - 1` for normally created pairs (append-only registry).
2. If `PAIR_COUNT` grows **during** a multi-step rollout, repeat batches until **`has_more` is false**, then optionally rerun from `start_after: null` once more if new tail pairs must receive the registry (those indices were not scanned in earlier steps).
3. Failed/missing loads for an index slot are skipped (same as “all”), but contiguous indices remain the enumeration order — tooling should rely on **`has_more` / `next_start_after`**, not on `PAIR_COUNT` alone.
4. **`SetDiscountRegistryAll` cap:** `PAIR_COUNT` must be ≤ default `calc_limit` (**10**) or the factory rejects the message ([#242](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/242)).

**Example — governance multisig / script (batch loop until `has_more` is false):**

```bash
REGISTRY="<fee_discount_addr>"
FACTORY="<factory_addr>"
START=""
while true; do
  if [ -z "$START" ]; then
    MSG="{\"set_discount_registry_batch\":{\"registry\":\"$REGISTRY\",\"start_after\":null,\"limit\":10}}"
  else
    MSG="{\"set_discount_registry_batch\":{\"registry\":\"$REGISTRY\",\"start_after\":$START,\"limit\":10}}"
  fi
  terrad tx wasm execute "$FACTORY" "$MSG" --from gov --gas auto --gas-adjustment 1.4 ...
  # Parse tx logs: if has_more=false, break; else set START=next_start_after
done
```

Canonical doc for agent automation: [`skills/AGENTS_TERRACLASSIC_GAS.md`](../skills/AGENTS_TERRACLASSIC_GAS.md), [`skills/AGENTS_FEE_DISCOUNT_TIERS.md`](../skills/AGENTS_FEE_DISCOUNT_TIERS.md), [`skills/AGENTS_FACTORY_DISCOUNT_REGISTRY.md`](../skills/AGENTS_FACTORY_DISCOUNT_REGISTRY.md).

<a id="factory-discount-registry-snapshot-gitlab-536"></a>

### Factory discount registry snapshot (GitLab [#536](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/536))

Do **not** rely on a post-create `SetDiscountRegistry` sweep for **new** pairs. Factory `Config` stores a canonical `discount_registry` (`Option<Addr>`, serde default `None` so columbus-5 1.7.x state migrates). `CreatePair` copies that pointer into `PairInstantiateMsg`. Pair instantiate persists `msg.discount_registry` instead of hardcoding `None`.

| Message | Factory pointer | Indexed pairs |
|---------|-----------------|---------------|
| `SetDiscountRegistryAll` / `SetDiscountRegistryBatch` with `Some(addr)` | **Set** | Fan-out `SetDiscountRegistry` (All capped at 10 pairs) |
| All / Batch with `None` | **Clear** | Fan-out clear |
| `SetDiscountRegistry` (single pair) | Unchanged | That pair only |
| `UpdateConfig { discount_registry }` | **Set** (omit field = no change) | Unchanged |

Pair query **`GetDiscountRegistry`** returns `{ "registry": <addr or null> }` (this variant is on `QueryMsg` as of pair **1.14.0** — live 1.13.x wasm returns `unknown variant`). Factory `Config` includes `discount_registry`.

**Existing** listings are not retroactively wired — that is [#535](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/535). Fail-closed registry errors stay **I10**. Invariant **F5**. Regression: `make verify-issue-536`. Live inherit + dApp query: `make verify-issue-538` ([#538](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/538)).

<a id="gitlab-538"></a>

### Post-migrate inherit + dApp `GetDiscountRegistry` (GitLab [#538](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/538))

After factory **1.8.0** + pair **1.14.0** migrate:

| ID | Rule |
|----|------|
| **F538-1** | Factory `{"config":{}}` → `discount_registry` must be the fee-discount contract (All/Batch or `UpdateConfig { discount_registry }`). Columbus-5 ops: factory `terra1ejpg…chsea` wasm **11585**, pointer `terra1wccz…cfnecz`. |
| **F538-2** | A new LocalTerra `create_pair` must answer `GetDiscountRegistry` with that pointer **without** a follow-up `SetDiscountRegistry`. `deploy-dex-local.sh` asserts inherit before its idempotent per-pair set. Dedicated check: [`scripts/qa/localterra-create-pair-inherit.sh`](../scripts/qa/localterra-create-pair-inherit.sh). |
| **F538-3** | dApp `getPairDiscountRegistry` prefers pair `GetDiscountRegistry`; LCD raw `discount_registry` is fallback for 1.13.x wasm or LCDs that reject the query. Probe failure stays fail-closed (**I14** / **F537-2**). |

I10 fail-closed fee behavior is unchanged. Wiring already-listed economic pairs is [#535](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/535).

### Factory LP admin rotation (invariants, [GitLab #277](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/277))

`UpdateConfig` with a new `governance` address updates factory config only — it does **not** propagate LP CW20 `admin` on pairs. After rotation, governance runs **`SetLpAdminAll { admin }`** (when `PAIR_COUNT` ≤ **`calc_limit(None)`**, default **10**) or paginated **`SetLpAdminBatch { admin, start_after, limit }`** until `has_more` is false. Semantics mirror **`SetDiscountRegistryAll` / `SetDiscountRegistryBatch`** (same `calc_limit`, cursor attrs, cap errors). LP tokens may retain the prior admin until the batch completes — sequence All/Batch into any governance rotation runbook.

### Factory pair-treasury rotation

`UpdateConfig { treasury }` updates factory `config.treasury` only (new `CreatePair` snapshots + pair-creation uluna). Existing pairs keep the treasury copied at instantiate. After factory **1.7.0** + pair **1.11.0** migrate, governance runs **`SetPairTreasuryAll { treasury }`** (when `PAIR_COUNT` ≤ **10**) or **`SetPairTreasuryBatch`** until `has_more` is false. Mainnet CMM sink: `terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2`. Ops: [`docs/runbooks/rotate-fee-treasury.md`](./runbooks/rotate-fee-treasury.md), [`scripts/rotate-fee-treasury.sh`](../scripts/rotate-fee-treasury.sh).

<a id="asset-cw20-code-id-pin-gitlab-582"></a>

### Asset CW20 `code_id` pin (GitLab [#582](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/582))

Factory `CreatePair` still requires each asset's live `code_id` in `WHITELISTED_CODE_IDS`. The pair then snapshots those ids (`GetAssetCodeIds`). Write paths (swap, provide, withdraw, limit place+fill, cancel/claim) abort unless live `ContractInfo.code_id` **equals the pin** **and** `IsCodeIdWhitelisted` is still true. Factory query errors fail closed.

Honest token upgrades: whitelist the new id → migrate instances → `RefreshPairAssetCodeIds` / `RefreshPairAssetCodeIdsBatch` (refuses unlisted live ids). **Migrate factory 1.9.0 before pair 1.15.0** so write-path queries exist — enforced by [`scripts/upgrade-582-code-id-pin.sh`](../scripts/upgrade-582-code-id-pin.sh) ([#584](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/584)). The script also **`UpdateConfig { pair_code_id }`** so new `CreatePair` instantiates pair 1.15.0 (existing-pair migrate does not). Pair migrate backfills missing pins from live `ContractInfo`. LCD whitelist reads retry; a dropped query is not an empty pin. Invariant **F6**. Regression: `make verify-issue-582` / `make verify-issue-584`. dApp + indexer freeze visibility (quotes still appear; execute blocked): `make verify-issue-585` ([#585](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/585), [`AGENTS_FRONTEND_CODE_ID_FREEZE.md`](../skills/AGENTS_FRONTEND_CODE_ID_FREEZE.md)). Playbook: [`skills/AGENTS_CW20_CODE_ID_PIN.md`](../skills/AGENTS_CW20_CODE_ID_PIN.md). Ops: [`docs/runbooks/cw20-code-id-ops.md`](./runbooks/cw20-code-id-ops.md).

---

## Pair

### InstantiateMsg (PairInstantiateMsg)

| Field              | Type             | Description                       |
|--------------------|------------------|-----------------------------------|
| `asset_infos`      | `[AssetInfo; 2]` | The two assets for the pair       |
| `fee_bps`          | `u16`            | Fee in basis points               |
| `treasury`         | `Addr`           | Fee recipient                     |
| `factory`          | `Addr`           | Factory address (for auth)        |
| `lp_token_code_id` | `u64`           | Code ID for LP token instantiation|
| `discount_registry`| `Option<String>` | Copied from factory `config.discount_registry` at `CreatePair` ([#536](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/536)); omitted → `None` |

Liquidity-share tokens use CW20 **`decimals = LP_TOKEN_DECIMALS`** (18). `MINIMUM_LIQUIDITY` (first-mint permanent lock) counts **LP smallest units**, not pool asset decimals. On **first** `provide_liquidity` both reserve CW20s must have **`decimals ≤ MAX_PAIR_ASSET_DECIMALS_BOOTSTRAP`** (same cap as **`CreatePair`**; [!124](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/124)).

### ExecuteMsg

| Variant              | Fields                                                                   | Auth       |
|----------------------|--------------------------------------------------------------------------|------------|
| `Receive`            | `Cw20ReceiveMsg` (wraps `Swap` or `WithdrawLiquidity`)                  | CW20 token |
| `ProvideLiquidity`   | `assets: [Asset; 2]`, `slippage_tolerance?`, `receiver?`, `deadline?`   | Anyone     |
| `Swap`               | `offer_asset`, `belief_price?`, `max_spread?`, `to?`, `deadline?`       | (rejected for CW20 -- use CW20 Send) |
| `UpdateFee`          | `fee_bps: u16`                                                          | Factory    |
| `UpdateTreasury`     | `treasury: String`                                                      | Factory    |
| `UpdateHooks`        | `hooks: Vec<String>`                                                     | Factory    |
| `SetDiscountRegistry`| `registry: Option<String>`                                              | Factory    |
| `RefreshAssetCodeIds`| —                                                                        | Factory — re-pin live asset `code_id`s ([#582](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/582)) |

### Cw20HookMsg (sent via CW20 Send)

| Variant              | Fields                                                    |
|----------------------|-----------------------------------------------------------|
| `Swap`               | `belief_price?`, `max_spread?`, `to?`, `deadline?`, `trader?` |
| `WithdrawLiquidity`  | (no fields)                                               |

### QueryMsg

| Variant              | Parameters         | Returns                      |
|----------------------|--------------------|------------------------------|
| `Pair`               | —                  | `PairInfo`                   |
| `Pool`               | —                  | `PoolResponse`               |
| `HybridSimulation`         | `offer_asset`, `hybrid`, optional `trader`, optional `sender` | `HybridSimulationResponse`         |
| `HybridReverseSimulation`  | `ask_asset`, `hybrid`, optional `trader`, optional `sender`   | `HybridReverseSimulationResponse` — minimal `offer_amount` for `ask_asset.amount`; search seeded from pool CP math, ≤ **32** full hybrid sims per query ([#257](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/257)) |
| `GetFeeConfig`       | —                  | `FeeConfigResponse`          |
| `GetHooks`           | —                  | `HooksResponse`              |
| `GetDiscountRegistry`| —                  | `DiscountRegistryResponse`   |
| `GetAssetCodeIds`    | —                  | `AssetCodeIdsResponse` `{ code_ids: [u64; 2] }` (pair **1.15.0**, GitLab [#582](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/582)) |

`GetDiscountRegistry` is implemented on pair **1.14.0** (`{"get_discount_registry":{}}` → `{ "registry": <addr or null> }`). Pre-1.14.0 wasm (1.13.x) returns `unknown variant`. JSON `null` means the pair is **unwired** and charges full `fee_bps` (GitLab [#537](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/537) / [#538](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/538), invariant **I14**). The dApp prefers the smart query and falls back to LCD raw key `discount_registry` (`/raw/ZGlzY291bnRfcmVnaXN0cnk=`) when the query is missing or blocked. It must not advertise `VITE_FEE_DISCOUNT_ADDRESS` discounts unless the stored address matches.

### Event Attributes (swap)

| Attribute           | Description                          |
|---------------------|--------------------------------------|
| `action`            | `"swap"`                             |
| `sender`            | User who initiated the swap          |
| `receiver`          | Recipient of output tokens           |
| `offer_asset`       | Input token identifier               |
| `ask_asset`         | Output token identifier              |
| `offer_amount`      | Amount of input tokens               |
| `return_amount`     | Amount of output tokens (net of fee) |
| `spread_amount`     | Price impact amount                  |
| `commission_amount` | Fee amount taken                     |

---

## Router

### InstantiateMsg

| Field     | Type     | Description            |
|-----------|----------|------------------------|
| `factory` | `String` | Factory contract address|

### SwapOperation

```json
{ "terra_swap": { "offer_asset_info": <AssetInfo>, "ask_asset_info": <AssetInfo> } }
// or (rejected at runtime)
{ "native_swap": { "offer_denom": "uluna", "ask_denom": "uusd" } }
```

### ExecuteMsg

| Variant                    | Fields                                                            | Auth       |
|----------------------------|-------------------------------------------------------------------|------------|
| `Receive`                  | `Cw20ReceiveMsg` (wraps `ExecuteSwapOperations`)                 | CW20 token |
| `ExecuteSwapOperations`    | `operations`, `minimum_receive?`, `to?`, `deadline?`             | (rejected -- use CW20 Send) |

### Cw20HookMsg

| Variant                    | Fields                                                   |
|----------------------------|----------------------------------------------------------|
| `ExecuteSwapOperations`    | `operations: Vec<SwapOperation>`, `minimum_receive?`, `to?`, `deadline?` |

> **Note:** The Router passes the original sender's address as `trader` in the Pair's `Cw20HookMsg::Swap` so the Pair can look up the correct fee discount.

### QueryMsg

| Variant                          | Parameters                          | Returns                           |
|----------------------------------|-------------------------------------|-----------------------------------|
| `Config`                         | —                                   | `ConfigResponse`                  |
| `SimulateSwapOperations`         | `offer_amount`, `operations`        | `SimulateSwapOperationsResponse`  |
| `ReverseSimulateSwapOperations`  | `ask_amount`, `operations`          | `SimulateSwapOperationsResponse`  |

---

## Fee Discount

The fee-discount contract manages tiered swap fee discounts for CL8Y token holders. Traders register for a tier by holding the required CL8Y balance. The Pair contract queries this registry on each swap to determine the effective fee.

### InstantiateMsg

| Field            | Type     | Description                                          |
|------------------|----------|------------------------------------------------------|
| `governance`     | `String` | Address with admin privileges                        |
| `cl8y_token`     | `String` | CW20 address of the CL8Y token (18 decimals)        |

### ExecuteMsg

| Variant                | Fields                                                     | Auth        |
|------------------------|------------------------------------------------------------|-------------|
| `AddTier`              | `tier_id: u8`, `min_cl8y_balance: Uint128`, `discount_bps: u16`, `limit_discount_bps?: u16`, `governance_only: bool` | Governance  |
| `UpdateTier`           | `tier_id: u8`, `min_cl8y_balance?: Uint128`, `discount_bps?: u16`, `limit_discount_bps?: u16`, `governance_only?: bool` | Governance  |
| `RemoveTier`           | `tier_id: u8`                                              | Governance  |
| `Register`             | `tier_id: u8`                                              | EOA only (self-register) |
| `RegisterWallet`       | `wallet: String`, `tier_id: u8`                            | Governance  |
| `Deregister`           | —                                                          | Self        |
| `DeregisterWallet`     | `wallet: String`                                           | Governance  |
| `AddTrustedRouter`     | `router: String`                                           | Governance  |
| `RemoveTrustedRouter`  | `router: String`                                           | Governance  |
| `UpdateConfig`         | `governance?`, `cl8y_token?`                               | Governance  |

### QueryMsg

| Variant              | Parameters                  | Returns                     |
|----------------------|-----------------------------|-----------------------------|
| `Config`             | —                           | `ConfigResponse`            |
| `GetDiscount`        | `trader: String`            | `DiscountResponse`          |
| `GetTier`            | `tier_id: u8`               | `TierResponse`              |
| `GetTiers`           | —                           | `TiersResponse`             |
| `GetRegistration`    | `wallet: String`            | `RegistrationResponse`      |
| `IsTrustedRouter`    | `router: String`            | `IsTrustedRouterResponse`   |

### Tier ladder (canonical)

Default production tiers (IDs, CL8Y minimums, `min_cl8y_balance` wei, `discount_bps`, `limit_discount_bps`, `governance_only`) live in **[`docs/reference/fee-discount-tiers.md`](reference/fee-discount-tiers.md)** only — aligned with `smartcontracts/tests/src/tier_fixtures.rs` and [`scripts/deploy-dex-local.sh`](../scripts/deploy-dex-local.sh). Do not copy numeric tables here ([GitLab #198](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/198)). Agent playbook: [`skills/AGENTS_FEE_DISCOUNT_TIERS.md`](../skills/AGENTS_FEE_DISCOUNT_TIERS.md). Invariant **I13** ([#514](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/514)): placement uses `limit_discount_bps`; swaps / book takes use `discount_bps`.

### Discount Calculation

The Pair applies the **swap / take** discount as: `effective_fee = fee_bps * (10000 - discount_bps) / 10000`. Maker **placement** uses `limit_discount_bps` the same way, then charges `floor(effective/2)`. Example: 180 bps pair, tier 9 (`discount_bps` 9500, `limit_discount_bps` 10000) → swap effective **9 bps**, placement **0 bps**. A pair with 30 bps fee and a Tier 5 trader (5000 bps swap discount) still yields 15 bps on swaps.

### Balance Verification

The `GetDiscount` query checks the trader's CL8Y token balance on every call. If the balance is below the registered tier's threshold, the contract fires a deregistration message (fire-and-forget) and returns `discount_bps: 0` for that swap.

---

## Hook Interface

Any contract implementing this interface can be registered as a post-swap hook via the Factory.

### HookExecuteMsg

| Variant     | Fields                                                                                         |
|-------------|------------------------------------------------------------------------------------------------|
| `AfterSwap` | `pair`, `sender`, `offer_asset: Asset`, `return_asset: Asset`, `commission_amount`, `spread_amount` |

---

## Community tax CW20 (GitLab #592)

In-repo **Option A** template: `cl8y-community-tax-token` + `cl8y-community-token-launcher` + `cl8y-community-tax-autolp`. Pair/router swap math is **unchanged**. Playbook: [`skills/AGENTS_COMMUNITY_TAX_CW20.md`](../skills/AGENTS_COMMUNITY_TAX_CW20.md). Router hop tax: [`skills/AGENTS_COMMUNITY_TAX_ROUTER.md`](../skills/AGENTS_COMMUNITY_TAX_ROUTER.md) (**T592-13** / [#607](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/607) improved option 2). Invariants **T592-1–T592-13**. LaunchGuards liveness (**H608-1–H608-8**, [#608](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/608)): [`skills/AGENTS_COMMUNITY_TAX_LAUNCH_GUARDS.md`](../skills/AGENTS_COMMUNITY_TAX_LAUNCH_GUARDS.md).

### Classification (T592-7)

| Path | Tax |
|------|-----|
| `Send` to a `RegisterListedPair` pair with `Cw20HookMsg::Swap` from a **non-exempt** address | **Sell** extra-debit (`debit = amount + tax`, pair credit = `amount`) |
| `Send` to a listed pair with `Cw20HookMsg::Swap` from the official `config.router` | **Sell** — router debit = `amount`; authenticated `Swap.trader` extra-debit = tax (**T592-13** / [#607](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/607) improved option 2). Missing trader fail-closes |
| `Transfer` / `Send` **from** a listed pair to a non-protocol-exempt address | **Buy** outbound split (pair debit = `amount`) — also withdraw / limit refund |
| `Transfer` / `Send` **from** the official router to a non-protocol-exempt address | **Buy** outbound split. Pair→router stays 1:1 (**T592-1**) |
| `TransferFrom` to a pair (provide) | **Honest** 1:1 |
| `Send` + `PlaceLimitOrder*` | **Honest** 1:1 |
| Wallet↔wallet with TransferTax SKU | Transfer tax (never on protocol addresses) |
| Manager-directory (`MANAGER_EXEMPT`) on either side of Sell / Buy / Transfer ([#609](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/609)) | **Honest** 0 bps. Launch guards still apply (**T592-11**). Protocol list stays unremovable (**T592-9**). |
| `config.manager` on either side of Sell / Buy / Transfer, or as official-router hop `trader` ([#633](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/633)) | **Honest** 0 bps without a `MANAGER_EXEMPT` row. Manager is **not** protocol-exempt. Extra wallets still need ExemptionDirectory. |

`TaxPreview { from, to, amount, send_msg }` matches execute. dApp max-sell must size extra-debit (`effectiveExtraDebitSellBps` is 0 when the connected wallet is manager-exempt). Playbook: [`skills/AGENTS_COMMUNITY_TAX_EXEMPT.md`](../skills/AGENTS_COMMUNITY_TAX_EXEMPT.md). Manager role skip + listed-pair autoregister: [`skills/AGENTS_COMMUNITY_TAX_AUTOREGISTER.md`](../skills/AGENTS_COMMUNITY_TAX_AUTOREGISTER.md) (**R633-1–R633-8**). Retail CL8Y factory pairs that hold a community-tax CW20 are listed-pair tax markets once registered (factory CreatePair after migrate, official dApp Create Pair follow-up, AutoLP bind, or Manage catch-up). Terraport / GDEX stay 1:1 (**M626-10**). TransferTax + provide on an **unregistered** factory pair still FoT-desyncs reserves — do not add pair FoT math (**H-01**).

### Invoices

Both **50 UST1** (`50000000`). Token/launcher accept **UST1 `Send` only** ([#595](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/595) routes any token off-chain). Official post-create SKU unlock: manager → launcher → token ([#606](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/606), **T606-1–T606-8**). Playbook: [`skills/AGENTS_COMMUNITY_TAX_ENABLE_FEATURE.md`](../skills/AGENTS_COMMUNITY_TAX_ENABLE_FEATURE.md).

- Token `EnableFeature { sku }` — not MintControl (instantiate-only). Payer is the **manager** or **this token's** `origin.launcher` (`GetLauncherOrigin`). Arbitrary contracts stay `Unauthorized`. `UpdateSettings` stays manager-only.
- Token `UpdateSettings { settings }` — one flat 50 UST1 for the whole already-activated batch. No-op / unactivated SKU / non-manager → revert, fee not kept.
- Launcher UST1 `Send` hook: `create_token` is a **newtype** (`CreateTokenMsg` fields as the object); `enable_feature` is `{ token, sku }`. JSON is `{"create_token":{…fields…}}` / `{"enable_feature":{…}}`. Launcher checks `GetConfig.manager` + `GetLauncherOrigin` before forwarding. Create **rejects** duplicate SKU names before multiplying. Instantiate stamps `admin: cmm_governance`.
- **Identity (#604):** `validate_identity` runs **before** `cw20_base` init. Name/symbol ASCII alphanumeric, name 3–50, symbol 3–12, decimals **6–18**. Errors: `DecimalsRange`, `InvalidName`, `InvalidSymbol`. Columbus-5 **11611** does not gain these checks until launcher `token_code_id` rotates (no silent mainnet store in #604/#605).
- **SKU payloads (#605):** `transfer_bps` / `sinks` / `launch_guards` / `initial_exempt` / AutoLP fields are rejected unless that SKU is in `features`. Launch guards SKU **requires** an explicit `launch_guards` object (no silent `trading_enabled: true`). `initial_exempt` (≤20) writes `MANAGER_EXEMPT`; protocol addrs rejected.
- **AutoLP create (#605 H-1):** when `auto_v2_lp` is purchased and launcher `autolp_code_id` is set, the launcher reply instantiates the sister and `BindAutolp`s it (factory is pinned immutable). Unset code id → `AutolpCodeNotSet` (invoice not kept). `SkimToLp` is still never called from token `Transfer`/`Send`.
- **AutoLP pair + skim floor (#610 / M-2 / M-3):** `pair` must be a **factory-listed** CL8Y pool that includes this tax token. Fake / wrong-token pointers revert on set. Permissionless skim always attaches `max_spread` (default 100 bps, cap 200 bps) and optional `min_return`. Playbook: [`skills/AGENTS_COMMUNITY_TAX_AUTOLP.md`](../skills/AGENTS_COMMUNITY_TAX_AUTOLP.md) (**M610-1–M610-8**). Binding `pair` also `RegisterListedPair`s that addr on the tax token ([#633](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/633) **R633-3**).
- **Listed-pair autoregister (#633):** factory `CreatePair` reply (after factory migrate) cw2-gates `register_listed_pair` on tax assets only. Official dApp `/create` follows up for the tax pin before the first provide. Manage shows one highest-LP catch-up button when a factory pair is still unregistered. Playbook: [`skills/AGENTS_COMMUNITY_TAX_AUTOREGISTER.md`](../skills/AGENTS_COMMUNITY_TAX_AUTOREGISTER.md).
- **VariableRates (#605 M-1):** without the SKU, `max_*` must equal the corresponding current rate at instantiate. Settings `buy_bps` / `sell_bps` require `variable_rates` (no free-profile bypass). Do not leave `require_variable_or_free_profile` as a no-op.
- **0-SKU free create (C593-12 / O601-3):** launcher `ExecuteMsg::CreateToken` (no UST1). CW20 `Send` of 0 is invalid, so free create cannot use `Receive`. Paid SKUs still require the invoice hook. Canonical columbus-5 launcher is `terra126pr5…ahzwze` (code **11622**; store was **11614** / **11620**). **11612** predates this execute and is unused.
- **Launcher `UpdateConfig`:** CosmWasm admin (DEX 2-of-3) may set `token_code_id` (must be factory-whitelisted) and/or `autolp_code_id`. Live **11622** has this execute; `GetConfig` is **11619** / **11621**.

### Listing

Factory `AddWhitelistedCodeId` is **ops after** `#589` REPORT **GO**. Columbus-5 tokens **11611** and **11619** are listed ([`cw20-codeid-audits/codeids/11611/REPORT.md`](../cw20-codeid-audits/codeids/11611/REPORT.md), [`codeids/11619/REPORT.md`](../cw20-codeid-audits/codeids/11619/REPORT.md)). Launcher `terra126pr5323xkhwas7y03azv48sqr2fy3fxxg0sxu8xhmjdxr8v5tzqahzwze` (code **11622**, wasm admin DEX 2-of-3) instantiates **11619**. Stub [`community-tax-token/REPORT.md`](../cw20-codeid-audits/codeids/community-tax-token/REPORT.md) remains a **NO-GO** placeholder. Do not whitelist **8654** or launcher **11612** / **11614** / **11620** / **11622** or AutoLP **11613** / **11621**.

`make verify-issue-592` (crates). `make verify-issue-608` (LaunchGuards cooldown / `max_wallet` liveness). `make verify-issue-610` (AutoLP factory pair + skim floor). `make verify-issue-601` (store + REPORT + LocalTerra smoke). Free listed-template adopt: [`#626`](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/626) / `make verify-issue-626`. Post-merge leftovers: [`#628`](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/628) / `make verify-issue-628` / [`localterra-628-migrate-leftover.sh`](../scripts/qa/localterra-628-migrate-leftover.sh). Listed-pair autoregister LocalTerra: [`#633`](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/633) / `make verify-issue-633` / [`localterra-633-autoregister.sh`](../scripts/qa/localterra-633-autoregister.sh). Migrate pair inventory: [`#634`](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/634) / `make verify-issue-634` / [`localterra-634-migrate-inventory.sh`](../scripts/qa/localterra-634-migrate-inventory.sh) ([`AGENTS_FRONTEND_TOKEN_MIGRATE.md`](../skills/AGENTS_FRONTEND_TOKEN_MIGRATE.md)).

### Migrate-adopt (GitLab #626) {#migrate-adopt-gitlab-626}

Same-crate bump is still `MigrateMsg {}` + `cw2::ensure_from_older_version` (CMM-only; **not** the retail page). Foreign adopt is `MigrateMsg { adopt: AdoptMigrateMsg }` for allowlisted cw2 ∈ `{crates.io:cw20-base, crates.io:cw20-mintable, crates.io:terraport-token, crates.io:cw20-taxed}`. Retail source gate is **`VITE_COMMUNITY_MIGRATE_CODE_IDS`** (default **6036, 10184, 8266, 8654**) — not factory `AddWhitelistedCodeId`. Append future source ids on Coolify. Leftover `tax_info` / `tax_map` / `whale_info` are wiped on any allowlisted source; zeros map 4.5%/1% → buy 450 / sell 100. Unknown cw2 or `cfg`/`feat` smash → revert. Balances / `total_supply` unchanged. Source minter is revoked. Do **not** factory-list **8654** (pair-asset H-01) — the current listed tax pin covers the adopted address (**11626** was the #628 store; live pin is **11630**). `CONFIG.launcher` is the official launcher. `GetMigrateOrigin` is written so catalog can attest without faking `launcher_tx`. Retail `/token/migrate` is **free** (no 50 UST1) and broadcasts `MsgMigrateContract` then `MsgUpdateAdmin` → CMM. After load it inventories CL8Y factory pairs vs Terraport/GDEX and offers register only for a factory-verified CL8Y pool after Refresh ([#634](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/634) **M634** / [`AGENTS_FRONTEND_TOKEN_MIGRATE.md`](../skills/AGENTS_FRONTEND_TOKEN_MIGRATE.md)). Adopt does not register.

| Source | Status | Terraport / GDEX LP (LCD 2026-08-24) |
|--------|--------|--------------------------------------|
| **10184** | **S3 go** | Honest 1:1 stay 1:1. CL8Y pairs freeze until Refresh. |
| **6036** | **S3-6036 page-go / chain-revert** | Live cw2 is `crates.io:terraswap-token` (LCD 2026-08-25, [#628](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/628)). Retail allowlist still includes 6036; adopt reverts until a follow-up crate change. Do not append that name from #628. |
| **8266** Open `terra1qz56v6p8ca3hh34wnj5yc3jykmw6jaaal0ukecscq8m9qqtgztnscs74n3` | **S3-8266 go** | Terraport Open/LUNC `terra1uxr6m55wxez5csnttz00893zur6pksn54nwlpye0c2pyuyyqp3qqknypyc` (Open `13056446286` / uluna `1733267547`, LCD 2026-08-25). Leftover `balance_at` unread. Do not RegisterListedPair. |
| **8654** ALPHA `terra1x6e64…zysuxz` | **On migrate allowlist** (not a special case; never factory-list 8654) | Terraport ALPHA/LUNC `terra12u7kh…9e7p6` (ALPHA `25732882067035` / uluna `5603001027933`); ALPHA/USTC `terra1jg2vu…wph` (uusd `54087298` / ALPHA `23466167250`). After wipe, forward 1:1; historical 4.5% skim not unwound. No GDEX factory pin in-repo. |

Design record: [#603](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/603). Wrap fallback: [#558](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/558).

### LaunchGuards (T592-11 / #608)

Cooldown is **per user wallet**. Listed pairs, router, factory, this token, and AutoLP are not cooldown subjects — one sell must not halt the pair. `max_wallet` does **not** apply when `to` is a listed pair or other protocol-exempt address (provide `TransferFrom` after organic sells; sell-to-pair exit bypass). User Buy / Transfer still hit the cap. `trading_enabled=false` still blocks both sides (H-5 residual). Same-wallet in-block batch may still cooldown (11611 **D11**).
