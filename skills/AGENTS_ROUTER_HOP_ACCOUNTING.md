# Agent playbook: router multi-hop hop accounting (R4 / H8)

**Issue:** [GitLab **#240**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/240)  
**Gap:** [`gaps/GAP_1780200149.md`](../gaps/GAP_1780200149.md) finding **H8**  
**Invariant:** **R4** — router multi-hop legs use **hop output delta**, not the router's full CW20 balance ([`docs/contracts-security-audit.md`](../docs/contracts-security-audit.md)).

## Rule

On each hop reply, the router computes:

```text
hop_output = balance_after(output_token) − pre_hop_balance
```

- `pre_hop_balance` is snapshotted in `SwapState` **before** the hop that credits `output_token`.
- Only `hop_output` is forwarded to the next pair or transferred to the recipient.
- Pre-existing dust/donations on the router stay on the router (not consumed, not sent to the user).

`SimulateSwapOperations` already chains explicit amounts; execution now matches that model when router balance ≠ hop output.

## Code

| Path | Role |
|------|------|
| [`smartcontracts/contracts/router/src/state.rs`](../smartcontracts/contracts/router/src/state.rs) | `SwapState.pre_hop_balance` |
| [`smartcontracts/contracts/router/src/contract.rs`](../smartcontracts/contracts/router/src/contract.rs) | `query_router_cw20_balance`, `hop_output_from_balance`, `reply_swap_hop` |

## Regression tests

| Test | File | Scenario |
|------|------|----------|
| `router_ignores_pre_existing_dust_on_output_token` | [`smartcontracts/tests/src/adversarial_token.rs`](../smartcontracts/tests/src/adversarial_token.rs) | Single-hop; dust on final output token |
| `router_multi_hop_ignores_dust_on_intermediate_and_final_tokens` | same | 2-hop A→B→C; dust on B and C; execute matches simulate |

**Run:**

```bash
cd smartcontracts
cargo test router_ignores_pre_existing_dust
cargo test router_multi_hop_ignores_dust
make test-contracts   # from repo root
```

## Related agent docs

- [`skills/AGENTS_TESTING_MULTIHOP_HYBRID.md`](./AGENTS_TESTING_MULTIHOP_HYBRID.md) — L8 sim vs execute on hybrid multihop ([#192](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/192))
- [`docs/contracts-security-audit.md`](../docs/contracts-security-audit.md) — invariant matrix R1–R4

## Migration note (operators)

Deployed routers must **migrate** to the wasm build that includes `pre_hop_balance` in `SwapState`. In-flight swaps cannot survive migration (atomic tx). Behavior change: dust is no longer swept into swaps — operators may optionally sweep stranded router balances via governance tooling.
