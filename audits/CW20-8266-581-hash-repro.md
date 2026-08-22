# Plan: prove dump `d854a219` compiled to LCD wasm 8266

**Appendix only ([#589](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/589)).** A byte-identical optimizer rebuild is **not** a go/no-go input. Listing evidence is [`cw20-codeid-audits/codeids/8266/REPORT.md`](../cw20-codeid-audits/codeids/8266/REPORT.md) (LCD pin + decomp + catalogue + Layer A/B). Keep this file for operators who still want a reconstruct.

**Goal:** `sha256(optimized wasm) == 953AD60CF6D8C9631B99ADC84C3ABF4083815743F86FF81B2A422FDFDF5F95C0`

CertiK is not part of this proof. A CosmWasm optimizer rebuild is.

**Compile only.** Do not instantiate, execute, or `cargo test` the token. Compare hashes. Use `linux/amd64` (ARM optimizer images produce different bytes).

---

## Why a naive rebuild fails

| Mistake | What happens |
|---------|----------------|
| `cargo generate-lockfile` on a 2026 host | Resolves transitives that need rustc > 1.66 (`itoa` 1.0.18, edition-2024 `zeroize` / `base64ct`). Optimizer 0.12.11 cannot compile them. |
| Dump `Cargo.toml` caret versions as-is (`cosmwasm-std = "1.0.0"`) | Fresh resolve is not 1.3.3. The stored wasm embeds **1.3.3**. |
| Missing `Cargo.lock` | `rust-optimizer:0.12.11` runs `cargo build --locked` and exits. Terraport had a lockfile; the public dump omitted it. |
| Missing `classic_terraport` | Path dep `../../packages/classic_terraport` 2.5.2. `github.com/terraport/classic_terraport` is 404. |
| Substituting crates.io / git `classic_terraswap` 2.5.2 as-is | **Right family, wrong crate.** Dump is a rename of `terraswap_token` + `packages/classic_terraswap` 2.5.2, but the wasm rustc-names `classic_terraport::token::InstantiateMsg`. Public `token.rs` has **no** `marketing`; package is CosmWasm **0.16** + `terra-cosmwasm` 2.2.0. See [`CW20-8266-581-classic-terraswap.md`](CW20-8266-581-classic-terraswap.md). |
| ARM optimizer / host `cargo wasm` | Different rustc / wasm-opt → different hash. |

The earlier #581 rebuild attempt hit the host-resolve + missing lockfile + private package problem. It did not finish a wasm. This plan generates the lockfile **inside** the 1.66 container and stubs only the instantiate types that appear in the binary.

---

## What we already know about the build

From rustc panic paths and crate paths in `wasm/strings.txt`:

| Input | Value |
|-------|--------|
| rustc | 1.66.0 (`69f9c33d71c871fc16ac445211281c6e7a340943`) |
| Image | `cosmwasm/rust-optimizer:0.12.11` **or** `workspace-optimizer:0.12.11` (same rustc, same `wasm-opt -Os`, binaryen v110) |
| Registry | git crates.io index `github.com-1ecc6299db9ec823` (not sparse; rules out 0.12.13+) |
| Optimizer flags | `RUSTFLAGS='-C link-arg=-s'` + `cargo build --release --lib --target wasm32-unknown-unknown --locked` + `wasm-opt -Os` ([optimize.sh v0.12.11](https://github.com/CosmWasm/optimizer/blob/v0.12.11/optimize.sh)) |
| Direct crates in wasm | `cosmwasm-std` **1.3.3**, `serde` **1.0.188**, `serde-json-wasm` **0.5.1**, `cw-storage-plus` **0.14.0**, `base64` **0.13.1** |
| cw2 | `crates.io:terraport-token` / `0.0.0` |
| Instantiate type | `classic_terraport::token::InstantiateMsg` |

`0.12.11` is a valid reproducible builder when source + lockfile + amd64 image match. The “not reproducible from crates.io” changelog note is about **0.7.2**, not this tag.

Production code only uses `classic_terraport` for `token::InstantiateMsg` (`validate`, `get_cap`, `initial_balances`, `mint`, `marketing`). `mock_querier` is test-only. The dump already has a local copy of that struct + validate in `src/msg.rs` (unused for `instantiate`; the entry point imports the package type).

**Fork finding:** `classic_terraport` 2.5.2 is almost certainly [classic-terraswap `v2.5.2`](https://github.com/terraswap/classic-terraswap/tree/v2.5.2) `packages/classic_terraswap` renamed, with `marketing` added on `token.rs` and the token contract bumped to cw 0.14 / std 1.3.3 + snapshots. It is **not** crates.io `astroport` (no `astroport::` paths in 8266). Public TerraSwap `token.rs` (classic and phoenix) still has no `marketing`.

A minimal stub (dump `InstantiateMsg` only) already produces a wasm ~2.7 KB short of LCD. The next reconstruct should start from the **full public package tree**, not a one-file stub — see inventory **§ Plan**.

---

## Step 1 — layout (do not execute the wasm)

Work outside the DEX repo bind-mount. Suggested:

```
/tmp/cw20-8266-rebuild/
  Cargo.toml                 # package terraport-token 0.0.0 (dump, path dep adjusted)
  Cargo.lock                 # generated in step 3
  src/                       # copy of dump src/
  classic_terraport/         # stub (step 2)
    Cargo.toml
    src/lib.rs
    src/token.rs
```

Changing the path from `../../packages/classic_terraport` to `classic_terraport` does **not** change the wasm. The **crate name** must stay `classic_terraport`.

Pin dump `Cargo.toml` **exact** versions before generating the lockfile (wasm-known, not carets):

```toml
cw-utils = "=0.14.0"
cosmwasm-schema = "=1.3.3"
cw2 = "=0.14.0"
cw20 = "=0.14.0"
cw20-base = { version = "=0.14.0", features = ["library"] }
cw-storage-plus = "=0.14.0"
cosmwasm-std = "=1.3.3"
schemars = "=0.8.11"          # 0.8.1 caret; pin a 2023-era 0.8.x if lockfile drifts
serde = { version = "=1.0.188", default-features = false, features = ["derive"] }
thiserror = "=1.0.50"         # pin after first in-container resolve if needed
classic_terraport = { path = "classic_terraport", version = "2.5.2" }
```

`serde-json-wasm` 0.5.1 and `base64` 0.13.1 come in via `cosmwasm-std` 1.3.3 — confirm after lockfile generate.

Keep dump `.cargo/config` aliases or drop them; optimizer does not use `cargo wasm`.

---

## Step 1b — inventory public `classic_terraswap` 2.5.2 (do this next)

Follow [`CW20-8266-581-classic-terraswap.md`](CW20-8266-581-classic-terraswap.md): clone tag `v2.5.2`, hash every `packages/classic_terraswap/src/*` file, map which modules the dump references and which rustc paths exist in LCD 8266. Then reconstruct `classic_terraport` by **rename + `token.rs` marketing + CosmWasm 1.3 port**, not by depending on `classic_terraswap` as a crate. Drop or stub `terra-cosmwasm` 2.2.0 (0.16-only; 8266 has `requires_terra` but no `TerraQuerier` strings). Keep `stargate`.

## Step 2 — stub `classic_terraport` 2.5.2

Package name **`classic_terraport`**, version **2.5.2**, edition 2021.

`token.rs`: copy `InstantiateMarketingInfo`, `InstantiateMsg`, `get_cap`, `validate`, `has_valid_name`, `has_valid_symbol` from dump `src/msg.rs` (same field order, same error strings). Those strings are already in the wasm (`Name is not in the expected format…`, `Ticker symbol is not in expected format…`, `Decimals must not exceed 18`).

`lib.rs`: `pub mod token;` only. No `mock_querier` in the default lib (tests are not compiled into the wasm).

Do not pull in factory/pair types. Extra unused items can change monomorphization if referenced; keep the stub minimal.

---

## Step 3 — generate `Cargo.lock` inside 0.12.11 (not on the host)

```bash
# linux/amd64. Fresh registry volume — do not reuse a 2026 host cache.
docker run --rm --platform linux/amd64 \
  -v /tmp/cw20-8266-rebuild:/code \
  -w /code \
  --entrypoint cargo \
  cosmwasm/rust-optimizer:0.12.11 \
  generate-lockfile
```

That cargo is 1.66 and still uses the git crates.io index. It will not select edition-2024 crates.

Check the lockfile contains exactly:

- `cosmwasm-std` 1.3.3  
- `serde` 1.0.188  
- `serde-json-wasm` 0.5.1  
- `cw-storage-plus` 0.14.0  
- `base64` 0.13.1  

If a transitive is newer than store date (2023-11-07) or newer than the wasm strings, pin it in `Cargo.toml` and regenerate. Do not edit `Cargo.lock` by hand unless a pin is proven.

---

## Step 4 — optimize (amd64)

```bash
docker run --rm --platform linux/amd64 \
  -v /tmp/cw20-8266-rebuild:/code \
  --mount type=volume,source=cw20_8266_target,target=/code/target \
  --mount type=volume,source=cw20_8266_registry,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer:0.12.11
```

Expect `artifacts/terraport_token.wasm` (crate name with hyphens → underscores).

If rust-optimizer complains about a workspace, either flatten as above (preferred) or use `workspace-optimizer:0.12.11` with a single `contracts/token` member. Same rustc and `wasm-opt -Os`; one contract should hash the same.

```bash
sha256sum /tmp/cw20-8266-rebuild/artifacts/terraport_token.wasm
# must be 953AD60CF6D8C9631B99ADC84C3ABF4083815743F86FF81B2A422FDFDF5F95C0
cmp /tmp/cw20-8266-rebuild/artifacts/terraport_token.wasm \
    /tmp/cw20-8266-audit-581/wasm/8266.wasm
```

`cmp` identical = source matches the stored binary. That is the go condition for #581.

---

## Step 5 — if the hash misses

Do **not** whitelist. Debug in this order (still compile-only):

1. **Crate versions** — `strings` the new wasm; diff registry paths against `wasm/strings.txt`. Pin any drift and rebuild.  
2. **Image** — retry `workspace-optimizer:0.12.11` vs `rust-optimizer:0.12.11`.  
3. **schemars / thiserror / cw20-base** — unused-but-declared deps can stay out of the wasm; they still affect the lockfile. Unlikely to change bytes unless referenced.  
4. **Ask Terraport for `Cargo.lock` only** from the 2023-11-07 store. Not a CertiK zip. If instantiate validate strings differ, also ask for `packages/classic_terraport` at that lockfile’s revision.  
5. **Give up on 8266** — wrap SpaceUSD to 10184. Same as other rejected templates.

A near-miss (same strings, different hash) still means we have not proven the CFG. Do not treat “looks like the dump” as a match.

---

## After a match

The dump source review in [`CW20-8266-581.md`](CW20-8266-581.md) applies to the live 8266 wasm: 1:1 `transfer`/`send`, snapshots only, no tax/rebase surface.

## Rebuild results (2026-08-21, `/tmp/cw20-8266-rebuild`)

| Artifact | SHA-256 | Bytes |
|----------|---------|------:|
| LCD code 8266 | `953AD60CF6D8C9631B99ADC84C3ABF4083815743F86FF81B2A422FDFDF5F95C0` | 327509 |
| Rebuild (std default, no stargate) | `4F9D1D947483572382F3DFD534672ABE3C598C655FBE720FF00EDCB5C0253DD5` | 318433 |
| Rebuild (`stargate` + `staking`) | `9518E09E6D017A0388E02EDE4423B1738CE1533E071DAA9F0C15F09F64469102` | 325169 |
| Rebuild (`stargate` only) | `993665B43F7A88BEBBFF392FB20A19B90014F146C3CC1753B5D6576A3755B114` | 324806 |

None of these equal the LCD checksum. `stargate` closed most of a 9 KB gap and matched LCD exports (`requires_stargate` + stub `requires_terra`; do not enable `staking`). Remaining 2.3–2.7 KB: private `classic_terraport` vs public TerraSwap tree + lockfile. See [`CW20-8266-581-classic-terraswap.md`](CW20-8266-581-classic-terraswap.md).

Then 2-of-3 may `AddWhitelistedCodeId 8266`. Other 8266 instantiations becoming listable is accepted.

Optional (not required for this plan): a throwaway 8266 instantiate on staging for a 1:1 balance-delta probe. Do not use SpaceUSD mainnet funds for that.

---

## Safety

- Optimizer Docker **compile + wasm-opt** only.  
- Do not `wasmer` / `wasmtime` / LCD execute the rebuilt or LCD wasm for this check.  
- Do not bind-mount `indexer/` into a root cargo container.  
- New worktree under `/tmp/cw20-8266-rebuild/` — not the DEX `smartcontracts/` tree.  
