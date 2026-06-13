# Runbook: Post-swap hook registration

Governance playbook for factory `SetPairHooks` and per-hook `UpdateAllowedPairs`. Parent remediation: GitLab [#377](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/377) (**H-02**, **H-03**, **I-02**).

**Related:** [security-model.md § Hook safety](../security-model.md), [hooks README](../../smartcontracts/contracts/hooks/README.md), [launch-checklist.md](./launch-checklist.md).

## Policy summary

| Rule | Rationale |
|------|-----------|
| **Audit before register** | Hook `Err` atomically reverts the entire swap (invariant **H1**). |
| **Blocking hooks are allowed** | AML, compliance, or incident-response hooks may intentionally revert swaps. Do **not** wrap hooks in `reply_on_error` without explicit product approval. |
| **Allowlist hygiene** | Each hook’s `UpdateAllowedPairs` must list **only verified pair contracts** — never routers, EOAs, or helper contracts. |
| **LP-burn hook** | `pair` in `AfterSwap` must equal `info.sender`; hook queries pair `Pair {}` and matches `liquidity_token` to config (**H-03**). |
| **Tax / burn hooks** | Fees are deducted from swap ask-token flow during pair settlement (**I-02**); hook treasury balance is **not** required for normal fee collection. |

## Registration workflow

### 1. Wasm review checklist

Before mainnet registration:

- [ ] Source matches tagged release in this repo (optimizer build for first-party hooks).
- [ ] `GetConfig` query documented; admin/multisig identified.
- [ ] Gas bounded (no unbounded loops in `AfterSwap`).
- [ ] Failure modes documented (revert vs skip).
- [ ] For **tax-hook** / **burn-hook**: confirm pair forwards settlement (no treasury subsidy).
- [ ] For **lp-burn-hook**: confirm `target_pair` + `lp_token` match on-chain pair state.

### 2. Staging drill

1. Instantiate hook on staging.
2. `UpdateAllowedPairs { add: [<pair>] }` — **only** the pair address.
3. Factory `SetPairHooks { pair, hooks: [<hook>] }`.
4. Small swap — confirm expected attrs (`settled_by_pair` for tax/burn; LP burn only when pre-funded).
5. Optional: register a **reverting** hook on a test pair; confirm swap fails atomically (`swap_fails_atomically_when_allowlisted_hook_reverts`).

### 3. Production registration

```bash
# Verify hook config
terrad query wasm contract-state smart <hook> '{"get_config":{}}' --node <lcd>

# Governance tx: allow pair caller on hook
terrad tx wasm execute <hook> '{"update_allowed_pairs":{"add":["<pair>"],"remove":[]}}' ...

# Governance tx: register on pair via factory
terrad tx wasm execute <factory> '{"set_pair_hooks":{"pair":"<pair>","hooks":["<hook>"]}}' ...
```

### 4. Allowlist hygiene (ongoing)

- Remove decommissioned pairs from hook allowlists before removing pair hooks.
- Never add “helper” or “spoofer” contracts to `UpdateAllowedPairs`.
- After pair migration, update hook `target_pair` / allowlists to the new address.

## Intentional swap blocking

Hooks that revert (e.g. compliance deny-list) are **by design**. Operators must:

1. Document the hook’s revert conditions in the deployment record.
2. Communicate user impact (swaps fail with hook error string).
3. Plan removal path (`SetPairHooks` with empty list, or unblacklist + hook update).

Pair dispatch uses plain `WasmMsg::Execute` — no `reply_on_error`. This preserves atomic rollback (invariant **H1**).

## Migration: treasury-funded tax/burn deployments

Legacy deployments that pre-funded tax/burn hook balances should:

1. Migrate to current hook wasm (pair-settled fees).
2. Withdraw residual hook balances to treasury after cutover.
3. Re-run staging swap with **zero** hook balance to confirm fees still collect.

## Verification commands

```bash
cd smartcontracts && cargo test adversarial
cd smartcontracts && cargo test -p cl8y-dex-lp-burn-hook
cd smartcontracts && cargo test -p cl8y-dex-tax-hook -p cl8y-dex-burn-hook
make test-contracts
```
