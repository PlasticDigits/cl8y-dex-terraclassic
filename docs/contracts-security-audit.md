# CL8Y DEX — Contracts Security Audit (Repository)

This document is the **in-repo security audit and invariant matrix** for the CosmWasm contracts under `smartcontracts/`. It complements [security-model.md](./security-model.md) (operational trust assumptions) and [testing.md](./testing.md) (how to run tests and coverage).

## Trust model (explicit)


| Assumption                  | Implication for findings                                                                                                                                                                                        |
| --------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Governance is honest**    | Multisig/DAO-controlled governance is trusted not to sabotage the protocol (malicious hooks, fee changes, pauses) on purpose. The audit still verifies **non-governance callers cannot** perform admin actions. |
| **Treasury is a sink**      | Commission tokens are transferred to `treasury`; misconfiguration of treasury address is an ops risk, not an unauthorized-access bug.                                                                           |
| **CW20 code ID whitelist**  | Only whitelisted token **code IDs** may back pairs; malicious token *logic* inside a whitelisted template remains a product risk (e.g. fee-on-transfer), mitigated by docs and tests.                           |
| **Hooks are opt-in policy** | Governance may register hooks that **revert**; by design that blocks swaps ([security-model.md](./security-model.md)).                                                                                          |
| **External git deps**       | `treasury`, `wrap-mapper`, `cw20-mintable` (see [smartcontracts/Cargo.toml](../smartcontracts/Cargo.toml)) are part of the trust boundary for wrap/treasury scenarios in tests and any integrated deployment.   |


## Contract inventory


| Crate / path                                       | Role                                                                                                              |
| -------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| `cl8y-dex-factory`                                 | Pair creation, whitelist, governance-only pair config (fees, hooks, discount registry, pause, sweep).             |
| `cl8y-dex-pair`                                    | Constant-product AMM, reserves, fees, TWAP observations, post-swap hooks, pause/sweep.                            |
| `cl8y-dex-router`                                  | Multi-hop swaps, `SwapState` + reply chain, `trader` forwarding for discounts, optional unwrap via `wrap_mapper`. |
| `cl8y-dex-fee-discount`                            | Tiered discounts, EOA registration, trusted routers, lazy deregistration.                                         |
| `cl8y-dex-burn-hook` / `tax-hook` / `lp-burn-hook` | Post-swap callbacks; gated by **allowed caller** (expected: real pair contracts).                                 |
| `dex-common`                                       | Shared types, oracle math, hook wire format.                                                                      |


## Invariant matrix

Each row states a property that should **always** hold (under the trust model). **Test** column points to the primary automated evidence in `smartcontracts/tests/`.


| ID  | Invariant                                                                                                                                           | Code / notes                                                                                           | Tests / fuzz                                                                                                                                         |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| P1  | **k monotonicity** — after a swap, k' \geq k within documented rounding bounds                                                                      | [pair `execute_swap](../smartcontracts/contracts/pair/src/contract.rs)`                                | `audit_invariant_tests::p1_k_non_decreasing_after_swap`; `fuzz_tests` / `additional_fuzz_tests` / `security_tests` (K checks); [pair `lib.rs`](../smartcontracts/contracts/pair/src/lib.rs) |
| P2  | **Reserves vs balances** — internal `RESERVES` update only via protocol messages; direct CW20 transfers can desync balance vs reserves              | Documented in pair `lib.rs`                                                                            | `adversarial_token::fee_on_transfer_creates_reserve_imbalance`; `sweep_tests` (excess recovery)                                                                                       |
| P3  | **First-deposit LP lock** — `MINIMUM_LIQUIDITY` burned on first mint                                                                                | [pair `execute_provide_liquidity](../smartcontracts/contracts/pair/src/contract.rs)`                   | `pair_coverage_tests::test_first_deposit_minimum_liquidity`, `test_first_deposit_below_minimum_rejected`; `security_tests::test_lp_share_inflation_first_depositor_griefing`                                          |
| P4  | **Fee bounds** — `fee_bps` / effective fee in valid range; commission rounding sane                                                                 | Pair fee + discount composition                                                                        | `fee_discount_coverage_tests`, integration swap tests, `fee_math_property_tests` (proptest)                                                          |
| P5  | **Discount registry failure → full fee** — if `GetDiscount` query fails, pair uses configured pair fee                                              | [pair `execute_swap` `Err(_) => fee_config.fee_bps](../smartcontracts/contracts/pair/src/contract.rs)` | `audit_invariant_tests::swap_uses_full_fee_when_discount_registry_query_fails`                                                                       |
| P6  | **Untrusted router cannot steal discount** — `trader` only applies when sender is trusted router                                                    | [fee-discount `GetDiscount](../smartcontracts/contracts/fee-discount/src/contract.rs)`                 | `fee_discount_coverage_tests::test_query_discount_untrusted_router_falls_back_to_sender`                                                             |
| P7  | **Factory-only pair admin** — fee, hooks, registry, pause, sweep on pair only from factory                                                          | Pair `execute_*` sender checks                                                                         | `audit_invariant_tests::p7_non_factory_cannot_update_pair_fee`; `line_coverage_tests`, `pair_coverage_tests`, `factory_coverage_tests`                                                                              |
| P8  | **Governance-only factory admin**                                                                                                                   | [factory `ensure_governance](../smartcontracts/contracts/factory/src/contract.rs)`                     | `audit_invariant_tests::p8_non_governance_cannot_add_whitelisted_code_id`; `factory_coverage_tests` (unauthorized)                                                                                                   |
| R1  | **Router swap state** — no concurrent swap; state cleared after success/failure                                                                     | `SWAP_STATE`                                                                                           | `router_coverage_tests::test_router_failed_swap_then_retry_succeeds`; `reentrancy_tests`; `adversarial_token::router_two_sequential_swaps_both_succeed_state_cleared`                                                |
| R2  | **Deadline** — user deadlines enforced on pair and router                                                                                           | `assert_deadline`                                                                                      | `deadline_tests::test_swap_deadline_exceeded_rejected`, `test_provide_liquidity_deadline_exceeded_rejected`, `test_swap_deadline_not_exceeded_succeeds`, `test_router_deadline_exceeded_rejected`                                                                 |
| R3  | **minimum_receive** — enforced on final output                                                                                                      | Router reply path                                                                                      | `router_coverage_tests::test_router_minimum_receive_assertion`, unwrap variants in [lib.rs](../smartcontracts/tests/src/lib.rs)                      |
| H1  | **Hook caller allowlist** — only allowlisted addresses can invoke `Hook`; **griefing:** allowlisted hook `Err` rolls back the whole swap            | `assert_allowed_pair` in each hook                                                                     | **Allowlist:** `new_feature_tests::swap_with_reverting_hook_fails` (misnamed: disallowed caller → `Unauthorized hook caller`); `hooks_integration_tests::test_hook_unauthorized_caller_rejected`, `test_tax_hook_unauthorized_hook_caller_rejected`, `test_lp_burn_hook_unauthorized_hook_caller_rejected`. **Allowlisted + failing:** `audit_invariant_tests::swap_fails_atomically_when_allowlisted_hook_reverts` |
| H2  | **LP burn hook + forged `pair`** — if a **non-pair** address is allowlisted, spoofed `AfterSwap` can drive burns (governance misconfiguration risk) | [lp-burn-hook `execute_after_swap](../smartcontracts/contracts/hooks/lp-burn-hook/src/contract.rs)`    | `adversarial_token::lp_burn_hook_accepts_spoofed_pair_when_spoofer_allowlisted`                                                                      |
| W1  | **Treasury collateralization** — native backing ≥ CW20 wrapped supply (wrap-mapper / treasury harness)                                              | External `treasury` / `wrap-mapper`                                                                    | `wrap_security_tests::test_wrap_mapper_reentrancy` (invariant check); `wrap_security_tests::test_unwrap_exceeds_treasury_balance`; `wrap_fuzz_tests::prop_wrap_unwrap_treasury_invariant`; [NATIVE_TOKEN_WRAPPING.md](../NATIVE_TOKEN_WRAPPING.md) |


## Attack paths considered (non-governance)

- Unauthorized factory/pair/hook/discount admin calls → blocked (`Unauthorized`).
- Discount theft via fake `trader` from non-trusted router → blocked (query uses `sender`).
- Sandwich / flash-style value extraction → mitigated by fees + `max_spread` / slippage tests (`security_tests`, proptest).
- Fee-on-transfer / reserve imbalance → documented; sweep recovers excess balance without editing reserves.
- Hook griefing (reverting hook) → intentional if governance registered an allowlisted hook that errors; tested (`audit_invariant_tests::swap_fails_atomically_when_allowlisted_hook_reverts` — full tx rollback; pool unchanged). Disallowed caller → `new_feature_tests::swap_with_reverting_hook_fails` + `hooks_integration_tests::test_*_unauthorized_hook_caller_rejected` (burn / tax / lp-burn).
- Router `SWAP_STATE` after a failed tx → not stuck; `router_coverage_tests::test_router_failed_swap_then_retry_succeeds`.
- Router dust / attribution → `adversarial_token::router_absorbs_pre_existing_dust_on_output_token`.

## Residual risks (not “bugs” under trusted governance)

- **Malicious governance** can set destructive hooks, pause pairs, or point discount registry to broken contracts (users pay full fee if query fails).
- **Wasm admin / migration** on-chain is outside these crates; deployment checklist should restrict migration keys.
- **Indexer / frontend** are not authoritative for on-chain safety; oracle/TWAP consumers must follow disclaimers in `dex-common` pair query docs.

## Third-party audit

[security-model.md](./security-model.md) notes that a formal third-party audit is recommended before high-TVL mainnet. This document does **not** replace an external audit.

## Maintenance

When adding a new execute path or economic rule:

1. Add a row to the invariant matrix (or extend an existing row).
2. Add a **deterministic** regression test that encodes the business rule (not only line coverage).
3. If the property is numeric or sequence-based, add or extend a **proptest** in `cl8y-dex-tests`.
4. Run `cargo test` in `smartcontracts/` and, for coverage, see [testing.md](./testing.md) (Rust / LLVM coverage).

