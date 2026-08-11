# Agent playbook: router `minimum_receive` (R3 / #469)

**Issue:** [GitLab **#469**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/469)  
**Invariant:** **R3** — router `minimum_receive` bounds what the **recipient actually receives** on the final hop ([`docs/contracts-security-audit.md`](../docs/contracts-security-audit.md)).

## Rule

On the final hop reply (`reply_swap_hop`):

| Path | Amount compared to `minimum_receive` |
|------|--------------------------------------|
| CW20 transfer (`unwrap_output = false`) | `hop_output` (router CW20 balance delta on output token) |
| Unwrap (`unwrap_output = true`) | **Post–wrap-mapper net:** `hop_output − floor(hop_output × fee_bps / 10_000)` |

The router queries wrap-mapper `Config { fee_bps }` at final-hop settlement. Integrators setting `minimum_receive` on native-output swaps must subtract the mapper fee from the simulated wrapped output (or query mapper `fee_bps` off-chain).

Non-unwrap paths are unchanged — there is no mapper fee on a direct CW20 `Transfer`.

Frontend `simulateNativeSwap` returns:

- `amount` — what the user **receives** (post-fee **and** InstantWithdraw burn tax on unwrap — [#512](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/512))
- `routerMinReceiveBase` — post-fee **pre-tax** base for router `minimum_receive` (this table / **R3**)

Integrators must submit `minimum_receive` from `routerMinReceiveBase` (after slippage), not from post-tax `amount`, or the floor will be looser than R3 intends. See [`AGENTS_WRAP_UNWRAP_BURN_TAX.md`](./AGENTS_WRAP_UNWRAP_BURN_TAX.md) and [`AGENTS_MAINNET_WRAP_ENABLEMENT.md`](./AGENTS_MAINNET_WRAP_ENABLEMENT.md).

**Note:** On-chain R3 still cannot observe burn tax; true native received is `routerMinReceiveBase` after tax until ustr-cmm gross-up.

## Code

| Path | Role |
|------|------|
| [`smartcontracts/contracts/router/src/contract.rs`](../smartcontracts/contracts/router/src/contract.rs) | `net_after_wrap_mapper_unwrap_fee`, `reply_swap_hop` |
| [`smartcontracts/packages/dex-common/src/wrap_mapper.rs`](../smartcontracts/packages/dex-common/src/wrap_mapper.rs) | `QueryMsg::Config`, `SetFeeBps` |

## Regression tests

| Test | Scenario |
|------|----------|
| `test_router_minimum_receive_assertion` | CW20 output; floor above hop output → revert |
| `test_unwrap_minimum_receive_checked_on_post_unwrap_net` | Unwrap; floor at wrapped sim amount → revert (fee skims below floor) |
| `test_unwrap_minimum_receive_rejects_when_mapper_fee_skims_below_floor` | #469 repro: `fee_bps = 50`, floor = wrapped output |
| `test_unwrap_minimum_receive_succeeds_at_post_unwrap_net` | Floor at post-unwrap net → success; native delta matches |

**Run:**

```bash
cd smartcontracts
cargo test test_unwrap_minimum_receive
cargo test test_router_minimum_receive
make test-contracts   # from repo root
```

## Related agent docs

- [`skills/AGENTS_MAINNET_WRAP_ENABLEMENT.md`](./AGENTS_MAINNET_WRAP_ENABLEMENT.md) — frontend unwrap sim + Coolify env (#507)
- [`skills/AGENTS_WRAP_UNWRAP_BURN_TAX.md`](./AGENTS_WRAP_UNWRAP_BURN_TAX.md) — burn tax on InstantWithdraw + `routerMinReceiveBase` (#512)
- [`skills/AGENTS_ROUTER_HOP_ACCOUNTING.md`](./AGENTS_ROUTER_HOP_ACCOUNTING.md) — hop output delta (R4)
- [`NATIVE_TOKEN_WRAPPING.md`](../NATIVE_TOKEN_WRAPPING.md) — unwrap path wiring
