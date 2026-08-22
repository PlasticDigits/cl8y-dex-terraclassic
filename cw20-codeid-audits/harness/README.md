# Harness backends (GitLab #589)

Two layers, two execution backends. Parameterize with `CODE_ID`; do not copy-paste per ID.

## Layer A — token only

| Backend | What it runs | When |
|---------|--------------|------|
| **A-mt** | `cw-multi-test` + `cw20-mintable` (10184 analogue) + [`cw20_mutants.rs`](../../smartcontracts/tests/src/cw20_mutants.rs) | Always (CI). Fast. Proves detectors (G2) and honest CW20 behaviors. |
| **A-lcd** | Pinned LCD wasm stored on **LocalTerra** (Terra `requires_terra` / stargate / iterator capabilities) | `CODE_ID=<id>` after fetch. Script: [`scripts/layer-a-lcd.sh`](../scripts/layer-a-lcd.sh) (store + instantiate + 1:1 Transfer). Columbus-5 templates often export `requires_terra`; stock `cosmwasm-vm` without that capability is not a substitute. |

Layer A covers exact debit/credit, Send hook 1:1, zero/self/oversize, mint/burn auth, idle anti-rebase, snapshot-vs-live, decimals ≤ 18, supply conservation. See issue #589 test plan.

## Layer B — DEX + limits

| Backend | What it runs | When |
|---------|--------------|------|
| **B-mt** | Same pair/router/factory as unit tests; candidate is mintable or a mutant | Always. Re-runs P1, P2, P3, donation, flash round-trip, honeypot round-trip, limit escrow 1:1, same-asset CreatePair reject, FoT **P2** desync (known-bad must be red). |
| **B-lt** | `StoreCode` of the **pinned LCD wasm** on LocalTerra, whitelist **only in the test factory**, `CreatePair` vs EMBER, 1:1 Transfer into pair | `LAYER_B_LT=1` after `make has-localterra`. Implemented by [`scripts/layer-b-lt.sh`](../scripts/layer-b-lt.sh). Silent skip is forbidden (**C5**). Stub PASS is a harness bug ([#590](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/590)). |

Existing `cw-multi-test` invariant files stay the book-structure evidence (L5, L12–L22) unless the token can change balances mid-match.

## Mutant library (G2)

[`smartcontracts/tests/src/cw20_mutants.rs`](../../smartcontracts/tests/src/cw20_mutants.rs) is the checked-in oracle set: FoT, rebase, entrypoint-selective tax, backdoor, blocklist, pause, hidden mint, lying balance, height-activated tax, magnitude tax, cooldown, payable, ghost dust, mutable decimals, permissionless pair register. The suite is wrong if those mutants go green on 1:1 / P2 rows.

## LCD wasm vs rebuild

The suite **refuses** to treat a rebuilt wasm as the candidate unless its SHA-256 equals LCD `data_hash` (fetch pin). That is **C1**.
