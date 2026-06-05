# Factory pair-creation `BankMsg::Send` to treasury

**Issue:** [GitLab #313](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/313)  
**Introduced in:** [GitLab #276](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/276)  
**Code:** [`smartcontracts/contracts/factory/src/contract.rs`](../../smartcontracts/contracts/factory/src/contract.rs) (`execute_create_pair`, ~L226–297)  
**Sign-off:** Cloud Agent implement pass — 2026-06-05 (no contract logic changes required)

## Scope

Review of native **uluna** pair-creation fees forwarded to `config.treasury` via `BankMsg::Send`, including overpay refunds. Swap commissions remain CW20 transfers on the pair contract; this is the factory’s first in-repo bank send to treasury.

## Checklist

| # | Item | Result | Notes |
|---|------|--------|-------|
| 1 | **Sender authorization** — `CreatePair` is permissionless; fee cannot be waived by a non-governance caller | PASS | Fee read from `CONFIG.pair_creation_fee_uluna`; only `SetPairCreationFee` / `UpdateConfig` mutate treasury or fee (`ensure_governance`). |
| 2 | **Exact fee vs overpay** — `paid >= fee`; `refund = paid - fee` after underpay guard; no unsigned wrap | PASS | `Uint128` math; underpay returns `InsufficientPairCreationFee` before subtraction. |
| 3 | **Treasury address** — validated at `instantiate` and `UpdateConfig` via `addr_validate` | PASS | No user-controlled treasury redirect. |
| 4 | **Stray denoms** — only `uluna` in `info.funds` | PASS | Any `denom != "uluna"` → `UnexpectedPairCreationFunds`. |
| 5 | **Factory balance** — no uluna retained when fee disabled or overpaid | PASS | Zero-fee path refunds full attachment; fee path sends fee + refund. Covered by `create_pair_refunds_uluna_when_fee_disabled`. |
| 6 | **`OnePairCreationPerBlock`** — gate runs before `Response`; failed gate does not execute bank sends | PASS | `PAIR_CREATION_BLOCK` checked after fee math but before returning `Response` — entire tx reverts on `Err`. |
| 7 | **Atomicity with instantiate** — fee bank msgs in same tx as `SubMsg::reply_on_success` pair instantiate | PASS | Instantiate failure reverts whole tx (Cosmos atomicity); documented in [`security-model.md`](../security-model.md). |
| 8 | **Reentrancy** — refund `BankMsg::Send` to `info.sender` | PASS | Bank sends are terminal (no `reply`); CosmWasm actor model blocks cross-tx reentrancy. |
| 9 | **Governance fee change** — `SetPairCreationFee` governance-only | PASS | `create_pair_charges_fee_to_treasury_and_gov_can_set_it`. |
| 10 | **Governance treasury rotation** — subsequent `CreatePair` uses updated treasury | PASS | `create_pair_fee_bank_send_adversarial_paths` (GitLab #313). |
| 11 | **Indexer** — pair creation indexing does not rely on bank events | PASS | Indexer has no `create_pair` / pair-creation fee parser; pairs indexed via wasm instantiate / factory registry events. |

## Test plan mapping

| Path | Expected | Test |
|------|----------|------|
| Exact fee | Treasury credited, pair created | `create_pair_charges_fee_to_treasury_and_gov_can_set_it` |
| Underpay | `InsufficientPairCreationFee` | `create_pair_charges_fee_to_treasury_and_gov_can_set_it` |
| Overpay | Treasury = fee; user refund | `create_pair_fee_bank_send_adversarial_paths` |
| Stray denom | `UnexpectedPairCreationFunds` | `create_pair_fee_bank_send_adversarial_paths` |
| Zero fee | Free create; mistaken uluna refunded | `create_pair_refunds_uluna_when_fee_disabled` |
| Gov updates treasury | New sends to updated addr | `create_pair_fee_bank_send_adversarial_paths` |

## Attack vectors

| Vector | Mitigation | Verified |
|--------|------------|----------|
| Redirect treasury via user tx | `UpdateConfig` / instantiate governance-only | PASS |
| Reentrancy via refund | Terminal bank sends | PASS (design review) |
| Fee bypass | `paid < fee` rejected | PASS |
| Block fee griefing | `OnePairCreationPerBlock` + fee when > 0 | PASS (`test_create_pair_one_per_block_then_next_block_ok`) |
| Stuck uluna in factory | Refund path + zero-fee refund | PASS |

## Issues found

None requiring contract changes in this milestone. Test gaps (overpay, stray denom, treasury rotation on create) were closed in GitLab #313 adversarial test.

## Verification commands

```bash
cd smartcontracts && cargo test create_pair_fee_bank_send_adversarial_paths --quiet
cd smartcontracts && cargo test create_pair_charges_fee_to_treasury_and_gov_can_set_it create_pair_refunds_uluna_when_fee_disabled --quiet
make test-contracts
make verify-issue-313   # optional bundled check
```

## Related docs

- [`docs/security-model.md`](../security-model.md) — pair-creation fee paragraph
- [`docs/contracts-security-audit.md`](../contracts-security-audit.md) — invariant **F2**
- [`make verify-issue-276`](../../Makefile) — #276 regression bundle
