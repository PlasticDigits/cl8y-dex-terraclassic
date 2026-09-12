# ADR 0004: Terra Classic retail gas census (hint vs broadcast vs wallet)

## Status

Proposed ([#1222](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1222))

Research / decision only. This ADR does **not** retune envelopes, patch wallet adapters, migrate CosmWasm, or change indexer quoting.

## Context

Retail “gas is wrong” still presents as either **out of gas** (limit too low) or **thousands of LUNC** on a wallet confirm for a swap that lands in the low hundreds of LUNC. Closed tickets fixed known execute shapes:

| Ticket | What landed |
|--------|-------------|
| [#679](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/679) / [#681](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/681) | Mixed hybrid + pool hops: 15M is **not** applied per empty hop; 4-hop envelope **6,785,500**; Station WC auto-gas residual documented |
| [#587](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/587) | Wrap + ≥2hop combo overhead **400k**; Swap **Network fee (est.)** row |
| [#599](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/599) | Unwrap + ≥2hop combo overhead **400k** |
| [#475](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/475) / [#384](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/384) | Retail `getGasLimitForTx` inventory; `BASE_GAS_LIMIT` fallthrough guardrail |
| [#249](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/249) / [#353](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/353) / [#343](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/343) | Quote-driven hybrid gas; router hop floors; unwrap add-on |

Those fixes did not produce a **single remaining-gap memo**. Three layers can still disagree: Swap/Trade **hint**, dApp **broadcast `Fee`**, wallet **confirm**. This census re-read current `terraGas.ts` / `hybridSwapGas.ts` / `swapNetworkFee.ts` on 2026-09-11 and re-fetched the #679 columbus-5 hash from public LCD.

**Out of this census (keyword overlap on “gas” is not enough):**

- [#123](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/123) — factory `SetDiscountRegistryBatch` governance pagination, **not** retail Swap gas.
- [#546](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/546) / [#618](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/618) — V3 Grid / keeper crank, **not** this census.
- [#1209](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1209) — indexer pair-creation **protocol** fees vs instantiate-gas double-count. Accounting, **not** tx gas.

## Decision

**Stay.** Document the census. **No implement issue is spawned** from this ADR.

| ID | Gap class | Score | Why |
|----|-----------|-------|-----|
| **G0** | Status quo after #679 / #587 | **Confirm** | Hint = broadcast when indexer ops are on the wire; envelopes sit above dated `gas_used` with margin. Station WC / LuncDash atomic `post` may still rewrite. |
| **G1** | Hint ≠ submit shape | **Kill (live Swap)** | `SwapPage` passes `simData.indexerOperations` as `cw20RouterOperations` when length ≥ 2. Fallback `hopCount` is pool-only **only** when ops are omitted (not the shipped submit path). |
| **G2** | Mixed hybrid + pool hops still 15M/hop | **Kill** | `gasLimitForHybridRouterOperations` uses the 950k pool floor per hop without parseable book params; mixed path adds `MIXED_HYBRID_ROUTER_HEADROOM_GAS` once. 4-hop fixture is **6,785,500**, not 46.8M. |
| **G3** | Wallet rewrite / LCD sim ≈ block gas | **Confirm residual** | dApp does **not** LCD-simulate as the broadcast envelope. Station / LuncDash WalletConnect mobile Confirm **may** LCD-sim (~147M / ~4,158 LUNC). In-repo fixes (LCD-sim default, mainnet SEC-E08) are forbidden. Already **G-AUTO-8**. |
| **G4** | Unmapped execute → `BASE_GAS_LIMIT` | **Confirm process, not a new hole** | Standing #475 / **G-RETAIL-1–2**. No new unmapped retail shape found in this census. |
| **G5** | Native wrap+N-hop still OOG / over-fee | **Kill as remaining OOG** | Wrap+2hop **2,710,000**; unwrap+2hop **3,110,000**. Live unwrap success hash stays the existing [#600](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/600) operator rung (**unmeasured** here). Do not reopen #587 / #599. |
| **G6** | Trade / Limits / Pool drift from Swap | **By design** | Different execute families, named constants. Trade market multi-hop **shares** `estimateSwapNetworkFee`. Limits/LP do not use the Swap hint (correct). |
| **G7** | Copy only | **Confirm Swap; Trade gap is copy** | Swap already splits AMM **Fee** vs **Network fee (est.)** plus `swap-wallet-fee-note`. `/trade` Market uses the envelope for the LUNC **gate** only — no Network fee row. Not an envelope bug. |

**No impl spawn.** A later bug/feature issue would need an exact execute shape + wallet class, current vs target `gas_wanted`, files to change, and explicit non-goals. G3 does not have a safe in-repo file list under this ticket’s guardrails. Optional later **copy** (Trade details one-liner mirroring `swap-wallet-fee-note`) is not opened here.

## Invariants (G-CENSUS)

| ID | Meaning |
|----|---------|
| **G-CENSUS-1** | One ADR covers hint vs broadcast vs wallet vs measured usage — not split across Swap / Trade / wrap tickets. |
| **G-CENSUS-2** | Each retail family has a dated measurement or an explicit **unmeasured** row. #679 numbers are reused only after re-reading current code **and** re-fetching the hash. |
| **G-CENSUS-3** | G0–G7 scored; product decision is Stay vs spawn. This ADR: **Stay**. |
| **G-CENSUS-4** | #123, #546/#618, #1209 stay out of this census. |
| **G-CENSUS-5** | Research MR is docs-only: no production `terraGas.ts` / `hybridSwapGas.ts` / `swapNetworkFee.ts` / wallet-adapter / `constants.ts` gas-constant diff. |
| **G-CENSUS-6** | No follow-up implement issue unless the decision is not Stay. |
| **G-CENSUS-7** | Broadcast envelope stays named hop / hybrid math above measured `gas_used`. LCD simulate is **not** the production limiter. Official dApp keeps hybrid on (**H596**). Native wrap+multihop stays pool-only (**H596-7**). |
| **G-CENSUS-8** | Fee denom is **`uluna` only**. Floor is `effectiveGasPriceUluna()` / `MIN_GAS_PRICE_ULUNA` (28.325). Do not enable mainnet SEC-E08 (#429) from this census. |

Playbook: [`skills/AGENTS_TERRACLASSIC_GAS.md`](../../skills/AGENTS_TERRACLASSIC_GAS.md). Verify: `make verify-issue-1222`.

## How the three layers work (2026-09-11 code)

1. **Broadcast.** `executeTerraContract*` → `broadcastTerraExecuteContracts` → `estimateTerraClassicFeeForEntries` → `getGasLimitForTx` / `totalGasLimitForExecuteMsgs` → `buildTerraClassicFee`. Amount is `ceil(effectiveGasPriceUluna() × gasLimit)` in **`uluna`**. There is **no** LCD `/simulate` on this path (`terraClassicFeeEstimate.ts`).
2. **Hint.** Swap: `estimateSwapNetworkFee` (`swapNetworkFee.ts`), same math as broadcast. When `cw20RouterOperations.length ≥ 2`, hint builds the same `send` → `execute_swap_operations` hook as submit. Trade market: `estimateTradeMarketNetworkFeeUluna` = allowance `BASE_GAS_LIMIT` + that helper for multi-hop.
3. **Wallet.** Extensions get `preferNoSetFee` + (Station) amino. Split sign/broadcast when the wallet exposes it. Station / LuncDash WalletConnect uses atomic `broadcastTx` (`isAtomicWalletConnectPost`). Post-sign undershoot guard is **LocalTerra-only** (`extensionSignedFeeGuard.ts`, #429).

## Retail execute inventory vs hint vs wallet

Mapper: **`getGasLimitForTx`**. Unmapped keys fall through to **`BASE_GAS_LIMIT` (200k)** + DEV warn. Inventory fixtures: [`terraGasRetailInventory.ts`](../../frontend-dapp/src/services/terraclassic/terraGasRetailInventory.ts). Combined envelopes: `RETAIL_COMBINED_ENVELOPE_FIXTURES`.

| Family | Execute / send inner | dApp `gas_wanted` | Swap Network fee hint | Wallet class (honor dApp `Fee`?) | Last measured `gas_used` |
|--------|----------------------|-------------------|------------------------|----------------------------------|--------------------------|
| Native wrap | `wrap_deposit` | `WRAP_GAS_LIMIT` **400k** | Yes (`nativeSwapFeeExecuteMsgs`) | Keplr ext / Station ext: honor when shim works. Station WC: may rewrite | Wrap LCD ~303k (code comment). Columbus-5 wrap-only success hash: **unmeasured** |
| Direct unwrap | `send` → `{unwrap}` | `UNWRAP_GAS_LIMIT` **800k** | Yes (`isDirectUnwrap`) | same | LCD sim ~562k; historical OOG `3C3B382A…287AD` wanted 550k / used ~550559 |
| Pair pool swap | `swap` / `send`→`swap` | **840k** (`gasLimitForExecuteSwapOperations(1)`) | Yes (`cw20DirectPair`) | same | Repro floor **753,321** (#115). Columbus-5 current hash: **unmeasured** |
| Pair hybrid / greedy | `swap.hybrid` / `swap.greedy` | Quote-driven; book=0 → 840k; unknown → **15M** once; greedy **G13** | Direct-pair hybrid yes; wrap+multihop **never** attaches hybrid | same | Book/8 makers envelope **1,785,500**. Live pair-only hash: **unmeasured** |
| Router pool N-hop | `execute_swap_operations` / send inner | N=1 **1.4M**; N=2 **1.91M**; N=4 **3.81M** | Yes (ops or `hopCount`) | same | 2-hop sat **1,810,064** vs wanted 1,810,000 (floor raised). Current hash: **unmeasured** |
| Mixed hybrid + pool router | hop1 `hybrid` + pool hops | **6,785,500** (8 makers + 3×950k + 2.15M) | Yes **iff** `cw20RouterOperations` | Station WC residual ~4,158 LUNC | **Dated 2026-09-11:** see measurement table |
| Unknown hybrid all hops | `hybrid: {}` on every hop | **15M** once (#249) | same as mixed | same | Fallback, not a live hub sample |
| Wrap + ≥2hop | wrap + router send N≥2 | **2,710,000** | Yes (native wrap path, pool-only) | same | Gem 2.31M too tight (#587). Columbus-5 success: **unmeasured** |
| Unwrap + ≥2hop | router N≥2 + `unwrap_output` | **3,110,000** | Yes | same | OOG at 2.71M hop+unwrap sum (#599). Success hash: **unmeasured** (operator [#600](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/600)) |
| Wrap+1hop | wrap + router 1 | **1,800,000** (no combo) | Yes | same | **unmeasured** |
| Router 1-hop + unwrap | | **2,200,000** (no unwrap combo) | Yes | same | **unmeasured** |
| Place limit | `place_limit_order` / batch / ladder (+ send inners) | 1.2M; batch 1M+180k×n | **No** (not Swap hint) | same | Tax place OOG at 580k (#625). Columbus-5: **unmeasured** |
| Cancel / claim / update limit | named keys | cancel **1M**; claim **450k**; update **350k**; batch 400k+80k×n | No | same | **unmeasured** |
| Provide / withdraw LP | `provide_liquidity` / `withdraw_liquidity` | **1M** / **900k** | No | same | Tax provide OOG at 650k (#625). Columbus-5: **unmeasured** |
| Create pair | `create_pair` | **1M** | No | same | Instantiate ~871k (#345) |
| Fee-tier register / deregister | `register` / `deregister` | **300k** / **250k** | No | same | LocalTerra register ≈ **204,438** (#384) |
| Faucet drip | `drip` | **400k** | No | same | Unmapped 200k OOG (#474). Live: **unmeasured** |
| Community token | `create_token` / `mint` / `skim_to_lp` / `register_listed_pair` | 1.2M / 400k / 800k / 400k | No | same | **unmeasured** |
| UST1 window | `send` → deposit / withdraw | **800k** | No | same | **unmeasured** |
| Pay invoice send | `send` → enable_feature / … | **600k**; wrap+2hop+invoice **3,310,000** | Combo fixture only | same | **unmeasured** |
| Allowance | `increase_allowance` / `decrease_allowance` | **200k** allowlist | Trade adds on multi-hop | same | **unmeasured** (cheap) |
| **Anything else** | unmapped | **200k** fallback | No | same | Process gap only (**G4** / #475) |

Wallet rows apply to every family: **Keplr extension** honors `preferNoSetFee` (split path). **Station extension** honors when amino + shim work (columbus-5 P0; **not** LocalTerra — #235). **Station WC / LuncDash** atomic `post` may LCD-sim. **Cosmostation** uses the shared amino/split path (no dedicated fee playbook). **Simulated/dev** signs the dApp `Fee` exactly.

## Dated measurements

Re-fetched **2026-09-11** from `https://terra-classic-lcd.publicnode.com/cosmos/tx/v1beta1/txs/{hash}` (public LCD only).

| Sample | Hash | Height | `gas_wanted` | `gas_used` | Fee | Notes |
|--------|------|--------|--------------|------------|-----|-------|
| Mixed 4-hop cLUNC→USTR (hop1 book/8, hops 2–4 pool) | [`AB8BE4F75E051837BB01C364DEDE6611727E47F0F857AADF04B17C39F360446D`](https://finder.terraclassic.community/columbus-5/tx/AB8BE4F75E051837BB01C364DEDE6611727E47F0F857AADF04B17C39F360446D) | **30121174** | **6,785,500** | **5,026,176** | **192,199,288 uluna** (~192.20 LUNC @ 28.325), `code=0` | Matches current mixed envelope. Margin 1,759,324 (~35%). **Not** Station’s ~147M / ~4,158 LUNC class. |
| Direct unwrap OOG (historical) | `3C3B382A…287AD` | — | 550,000 | ~550,559 | — | Ceiling below cost; current `UNWRAP_GAS_LIMIT` is 800k. Partial hash in `constants.ts` comment. |
| Wrap LCD | — | — | 400,000 ceiling | ~303k | — | Code comment; **unmeasured** on columbus-5 this census |
| Unwrap+2hop success | — | — | 3,110,000 | — | — | **Unmeasured.** Record via existing `VERIFY600_COLUMBUS_TX` (#600). Do not bump `UNWRAP_GAS_LIMIT` from this ADR. |
| Wrap+2hop success | — | — | 2,710,000 | — | — | **Unmeasured** on columbus-5 this census. Envelope already above gem 2.31M. |

Current code identity for the mixed path (must stay **> 5,026,176** and **< 15M**):

`1,785,500` (8 makers + scan/parks) + `3 × 950,000` + `2,150,000` headroom = **6,785,500**.

Naive sum without headroom is **4,635,500** < used (the #679 miss). Per-pool-hop 15M would be **46,785,500** — tests forbid that.

## Wallet classes (observe, do not patch)

| Class | Path | Honors dApp `Fee`? | LCD sim as envelope? |
|-------|------|--------------------|----------------------|
| Keplr extension | Split; `preferNoSetFee`; Ledger = amino | Yes (intended) | **No** |
| Station extension | Split + amino-always + shim defaults | Yes when honored (columbus-5) | **No** (dApp). LocalTerra: Station ignores overrides (#235) |
| Station WC / mobile | Atomic `broadcastTx` | Residual rewrite | **Wallet may**; dApp does **not** |
| LuncDash WC | Same atomic set as Station | Residual rewrite | **Wallet may** |
| Cosmostation | Amino; not in atomic WC set → split if `signAmino` exists | Code-path inferred | **No** |
| Simulated / dev | Offline sign of dApp `Fee` | Exactly | **No** |

Mainnet fee-guard (#429 / SEC-E08) stays **off**. Enabling it to reject Station mobile’s upward rewrite would brick that class.

## Attack / abuse evaluation (no exploits)

| ID | Vector | What this census shows | Fail if |
|----|--------|------------------------|---------|
| **A1** OOG grief | Under-envelope burns LUNC on failed tx | Bias stays measured+margin (6.79M > 5.03M; wrap/unwrap combos). | Recommending LCD-sim-only to “save” fees |
| **A2** Over-fee scare | Wallet asks near-block gas | Traders cancel. Residual is Station WC, not dApp. Network fee row is source of truth. | Treating Station 4k LUNC as “Classic is expensive” |
| **A3** Hybrid off | Strip hybrid to cheapen gas | Violates **H596**. Stay keeps hybrid. | Recommending pool-only as the gas fix |
| **A4** Unbounded book | Huge `max_maker_fills` | Caps: `MAX_MAKER_FILLS_HARD_CAP` 100, `MAX_SCAN_STEPS` 500, `HYBRID_SWAP_GAS_LIMIT` 15M | Unbounded gas as taker DoS |
| **A5** Fee denom confuse | Pay gas in USTC | Envelope is uluna-only; 0 USTC does not block a LUNC-funded swap | Suggesting `uusd` fees |
| **A6** Hostile simulate | Wallet simulate of unsized wasm | Why static envelopes exist; Station sim **is** the 4k LUNC class | Making simulate the production limiter |

## Test plan (research paths)

| ID | Path | Result |
|----|------|--------|
| **P1** Inventory | `RETAIL_GAS_SHAPE_FIXTURES` vs `getGasLimitForTx` | Mapped; unmapped → 200k (**G4** process) |
| **P2** Hint lockstep | `estimateSwapNetworkFee` + indexer ops | Unit **G-AUTO-2**; SwapPage passes ops |
| **P3** Hybrid mixed | Book hop + pool hops | 15M not per pool hop; 6,785,500 |
| **P4** Native wrap | LUNC → hub CW20 | 2.71M wrap+2hop; 3.11M unwrap+2hop; hybrid not attached |
| **P5** Wallet classes | Keplr vs Station ext vs Station WC | Observe-only; G3 residual documented |
| **P6** Unknown msg | Unmapped key | Still `BASE_GAS_LIMIT` + #475 |
| **P7** Copy | AMM Fee vs Network fee | Two Swap rows remain distinct |

Negative: a memo that only says “gas is high on Classic” fails. Reopening #679 without new 4k-LUNC dApp evidence fails. Implementation diff in this research MR fails **G-CENSUS-5**. Merging this with #123 or #1209 fails **G-CENSUS-4**.

## Consequences

- Agents treat remaining Station mobile ~4k LUNC confirms as **wallet residual**, not a dApp envelope bug. Point users at **Network fee (est.)**.
- New retail execute still needs a named constant + inventory fixture (#475) before ship.
- Constant retune, wallet-adapter work, LCD-sim-as-default, hybrid-off, and mainnet fee-guard stay **out of scope** until a separate implement issue names shape + wallet + targets.
- Optional later copy: `/trade` Market could show the same one-line wallet-fee note as Swap. Not spawned here.

## Links

- Issue: [git.cl8y.com #1222](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1222)
- [`docs/frontend.md` § Terra Classic gas limits](../frontend.md#terra-classic-gas-limits)
- [`skills/AGENTS_TERRACLASSIC_GAS.md`](../../skills/AGENTS_TERRACLASSIC_GAS.md) (**G-AUTO-1–G-AUTO-10**, **G-CENSUS-1–G-CENSUS-8**)
- [`skills/AGENTS_FRONTEND_STATION_SIGNING.md`](../../skills/AGENTS_FRONTEND_STATION_SIGNING.md) (**G-AUTO-8**)
- [`skills/AGENTS_EXTENSION_FEE_GUARD.md`](../../skills/AGENTS_EXTENSION_FEE_GUARD.md) (SEC-E08 / #429)
- Verify: `make verify-issue-1222` · QA [`docs/qa/issue-1222/README.md`](../qa/issue-1222/README.md)
