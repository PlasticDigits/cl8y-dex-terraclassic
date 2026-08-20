# Agent playbook: CosmWasm hooks and CW20 whitelist (#377)

Use when hardening or verifying post-swap hooks, factory CW20 whitelist policy, or GitLab **#377** / parent **#376**.

## Invariants

| ID | Rule | Evidence |
|----|------|----------|
| **H-01** | Never whitelist fee-on-transfer CW20 code IDs | `adversarial_token::fee_on_transfer_creates_reserve_imbalance` |
| **H-02** | Hook revert blocks swap atomically; blocking hooks OK | `swap_fails_atomically_when_allowlisted_hook_reverts` |
| **H-03** | LP-burn: `pair == info.sender`, pair `liquidity_token` check, allowlist pairs only | `lp_burn_hook_rejects_spoofed_pair_when_spoofer_allowlisted` |
| **I-02** | Tax/burn fees from pair ask-token settlement | `tax_hook_collects_from_swap_flow_with_zero_treasury_balance` |

## Ops docs

- CW20 whitelist: [`docs/runbooks/cw20-whitelist-policy.md`](../docs/runbooks/cw20-whitelist-policy.md)
- Hook registration: [`docs/runbooks/hook-registration.md`](../docs/runbooks/hook-registration.md)
- Code ID LCD check: `bash scripts/verify-cw20-code-ids.sh`

## Code map

- Pair settlement: `smartcontracts/packages/dex-common/src/hook_settlement.rs`, `pair/src/contract.rs` (`execute_swap`)
- LP-burn hardening: `smartcontracts/contracts/hooks/lp-burn-hook/src/contract.rs`
- Tax/burn hooks: `smartcontracts/contracts/hooks/tax-hook`, `burn-hook`

## Verify

```bash
cd smartcontracts && cargo test adversarial
cd smartcontracts && cargo test -p cl8y-dex-lp-burn-hook -p cl8y-dex-tax-hook -p cl8y-dex-burn-hook
make test-contracts
```

## Do not

- Add `reply_on_error` on pair hook dispatch without product approval.
- Implement balance-delta reconciliation for fee-on-transfer tokens (document prohibition only). Post-listing migrate is handled by **code_id pin + whitelist re-check** ([#582](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/582), [`AGENTS_CW20_CODE_ID_PIN.md`](./AGENTS_CW20_CODE_ID_PIN.md)) — not FoT math.
- Add non-pair addresses to hook `UpdateAllowedPairs`.
