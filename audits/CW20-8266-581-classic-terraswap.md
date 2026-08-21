# Inventory: public `classic_terraswap` 2.5.2 (baseline for private `classic_terraport`)

**Tag:** [terraswap/classic-terraswap `v2.5.2`](https://github.com/terraswap/classic-terraswap/tree/v2.5.2)  
**Package:** `packages/classic_terraswap` — crate name `classic_terraswap` **2.5.2**  
**Why:** Terraport dump `Cargo.toml` is a rename of `contracts/terraswap_token` + path dep `classic_terraport` **2.5.2**. LCD 8266 rustc-names `classic_terraport::token::InstantiateMsg`, not `classic_terraswap::…`. This file is the public baseline. Next work is to list every module and decide what Terraport kept, dropped, or edited.

Do **not** substitute this crate as-is into the rebuild. Crate name, CosmWasm 0.16, and missing `marketing` all disagree with 8266.

---

## Package tree (`v2.5.2`)

| Path | Role |
|------|------|
| `Cargo.toml` | `classic_terraswap` 2.5.2, edition **2018** |
| `src/lib.rs` | `asset`, `factory`, `pair`, `querier`, `router`, `token`; `mock_querier` if not wasm32; `testing` if test |
| `src/token.rs` | Token `InstantiateMsg` + `validate` / `get_cap` |
| `src/asset.rs` | `Asset` / `AssetInfo` + **`TerraQuerier` native burn-tax** (`compute_tax`, `deduct_tax`) |
| `src/factory.rs` | Factory messages / pair registry types |
| `src/pair.rs` | Pair messages / simulation types |
| `src/querier.rs` | LCD helpers (bank + CW20 + factory/pair queries) |
| `src/router.rs` | Router messages |
| `src/mock_querier.rs` | Host-only tests (dump token `testing.rs` imports this) |
| `src/testing.rs` | Package unit tests |
| `examples/schema.rs`, `schema/*.json` | JSON schema dump — not in wasm |

## `Cargo.toml` deps (this is the important delta)

```toml
cw20 = "0.8.0"
cosmwasm-storage = "0.16.0"
terra-cosmwasm = "2.2.0"          # Terra Classic tax / swap querier
cosmwasm-std = { version = "0.16.0", features = ["stargate"] }
```

- **`stargate`** on std matches LCD export `requires_stargate`.
- **`terra-cosmwasm` 2.2.0** is the usual source of `requires_terra` on Classic CosmWasm **0.16**. Official `cosmwasm-std` **1.3.3** does **not** export `requires_terra`. 8266 has that export and **no** `terra_cosmwasm` / `TerraQuerier` / `TaxRate` / `TaxCap` strings — so either they added an empty `requires_terra` stub, or they ported a thin terra helper that left no type path. `terra-cosmwasm` 2.2.0 will **not** compile against std 1.3.3 as-is.
- Native `compute_tax` in `asset.rs` is **LUNC/USTC bank tax**, not CW20 FoT. It only links if `asset` is referenced. 8266 does not contain those strings, so **`asset.rs` is not in the token wasm** even if the private package still has the file for factory/pair.

## `token.rs` vs dump vs 8266

| | `classic_terraswap` 2.5.2 `token.rs` | Dump `msg.rs` / stub | LCD 8266 |
|--|--|--|--|
| Crate path | `classic_terraswap::token::InstantiateMsg` | `classic_terraport::token::InstantiateMsg` | **`classic_terraport::…`** |
| Fields | name, symbol, decimals, initial_balances, mint | **+ `marketing`** | SpaceUSD instantiate has marketing |
| Derives | `Serialize, Deserialize, JsonSchema` | `#[cw_serde]` | — |
| Symbol charset | `[A-Za-z-]` only | same | same error string |
| Validate errors | name 3–50 / ticker `[a-zA-Z\\-]{3,12}` / decimals ≤ 18 | same strings | **same strings in wasm** |

[terraswap/terraswap](https://github.com/terraswap/terraswap) `packages/terraswap/src/token.rs` is the same no-marketing shape. Not a second source.

## `terraswap_token` contract vs dump token

`contracts/terraswap_token` is the other half of the rename:

| | `terraswap-token` 0.0.0 (`v2.5.2`) | `terraport-token` 0.0.0 (`d854a219`) |
|--|--|--|
| Path dep | `classic_terraswap` 2.5.2 | `classic_terraport` 2.5.2 |
| Repo field | `github.com/terraswap/classic_terraswap` | `github.com/terraport/classic_terraport` |
| cw / std | cw **0.8** / std **0.16** | cw **0.14** / std **1.3.3** (wasm) |
| History | none | `SnapshotMap` + `balance_at` / `total_supply_at` |

Public TerraSwap token has **no** snapshot queries. Those live in the Terraport token crate, not in this package.

## What 8266 proves is *not* linked from this package

LCD wasm has **no** `classic_terraswap`, `terraswap`, `terra_cosmwasm`, `TerraQuerier`, `astroport`. Only `classic_terraport::token::InstantiateMsg`. So factory/pair/router/asset tax code from this tree is **not** in the token binary (DCE), whether or not the private repo still contains those files.

## What we still do not know (private `classic_terraport`)

1. Exact file list after the rename (all of the above vs `token.rs` only).  
2. Whether `token.rs` is TerraSwap + `marketing` (dump) or a further edit.  
3. How they got `requires_terra` after leaving `terra-cosmwasm` 2.2.0 / std 0.16.  
4. Edition / `#[cw_serde]` vs old derives (affects bytes, not serde field names).

---

## Plan: determine exactly what is in `classic_terraswap` and what Terraport changed

Work under `/tmp` only. Compile-only.

### A — freeze the public baseline

```bash
git clone --depth 1 --branch v2.5.2 \
  https://github.com/terraswap/classic-terraswap.git \
  /tmp/cw20-8266-classic-terraswap
```

Record SHA of `v2.5.2` and SHA-256 of every file under `packages/classic_terraswap/src/`.

### B — module map (read, do not compile into 8266 yet)

For each `src/*.rs` write: public types, whether it imports `terra-cosmwasm`, whether the dump token crate references it, whether any of its rustc paths / error strings appear in `8266.wasm`.

Expected after a first pass:

| Module | Dump token uses? | In 8266 wasm? |
|--------|------------------|---------------|
| `token` | yes (`InstantiateMsg`) | **yes** (renamed crate) |
| `mock_querier` | tests only | no |
| `asset` / `pair` / `factory` / `router` / `querier` | no (prod) | **no** (no type paths) |

If any unexpected `classic_terraport::asset` (etc.) string appears in the LCD wasm, that module **is** linked — stop and read it.

### C — reconstruct `classic_terraport` from this tree

1. Copy `packages/classic_terraswap` → `/tmp/cw20-8266-rebuild/classic_terraport`.  
2. Rename package to **`classic_terraport` 2.5.2**.  
3. Edit **`token.rs` only** first: add `marketing` / `InstantiateMarketingInfo` / `Logo` (dump `msg.rs`); keep TerraSwap `validate` (letters+hyphen).  
4. Port deps 0.16 → wasm-known 1.3.3 / cw 0.14. **Drop or stub `terra-cosmwasm`** unless a 1.3-compatible crate is found; add empty `requires_terra` if needed to match LCD exports. Keep `stargate` on `cosmwasm-std`.  
5. Rebuild with `rust-optimizer:0.12.11` (existing pin/lockfile recipe). Compare SHA-256 and size to LCD `953AD60C…` / 327509.  
6. If still short ~2–3 KB: try `#[cw_serde]` vs old derives on `InstantiateMsg`; include vs omit unused modules (they should DCE); try `workspace-optimizer:0.12.11`.

### D — stop conditions

- Full-package reconstruct hashes to LCD → private package **is** this tree + `token.rs` marketing + CosmWasm bump.  
- Token-only stub and full-package reconstruct hash the **same** → unused modules are not in 8266; remaining gap is lockfile / derive / `requires_terra` helper.  
- New `classic_terraport::` paths show up in a full-package rebuild that are **absent** from LCD → do not ship those modules in the stub.  
- Still no hash: ask Terraport for `packages/classic_terraport` at the 2023-11-07 lockfile (same ask as before).

Do not whitelist on a near-miss. Native `compute_tax` in public `asset.rs` is not a CW20 transfer skim and is not in 8266.
