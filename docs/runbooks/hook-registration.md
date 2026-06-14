# Runbook: post-swap hook registration

Governance registers post-swap hooks on pairs via factory `SetPairHooks`. Hooks run **after** swap math and settlement transfers; a reverting hook **atomically rolls back the entire swap** (invariant **H1**).

**Related:** [`docs/security-model.md`](../security-model.md) (Hook safety), [`smartcontracts/contracts/hooks/README.md`](../../smartcontracts/contracts/hooks/README.md), [`docs/contracts-security-audit.md`](../contracts-security-audit.md) (H1–H2).

---

## Policy

| Rule | Rationale |
|------|-----------|
| **Audit before registration** | Hook bytecode must be reviewed for bounded gas, correct `AfterSwap` handling, and allowlist hygiene. |
| **Allowlist only real pair contracts** | Each hook's `UpdateAllowedPairs` must list **pair contract addresses only** — never routers, EOAs, or helper contracts. Non-pair allowlisting enables spoofed `AfterSwap` payloads (mitigated in lp-burn-hook by `pair == info.sender`; still forbidden). |
| **Intentional swap blocking is allowed** | Hooks that `Err` block swaps by design (AML, incident response). Do **not** add `reply_on_error` on pair hook dispatch unless product policy explicitly changes. |
| **Tax/burn hooks charge from swap output** | Pair settlement queries `ComputeSwapFee` and deducts fees from the trader's `return_amount` before `AfterSwap` — hooks must not rely on pre-funded treasury balances for normal fee collection ([#377](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/377)). |
| **LP-burn hook treasury is LP-only** | Governance may pre-fund lp-burn-hook with LP tokens; burns are proportional to verified pair output, not spoofable `pair` fields. |

---

## Pre-registration checklist

- [ ] Wasm uploaded from **optimizer** artifacts (`make build-optimized`), checksum recorded.
- [ ] Source matches deployed code ID (`terrad query wasm code-info <id>`).
- [ ] Hook implements `HookQueryMsg::ComputeSwapFee` when it deducts ask-token output (tax/burn); lp-burn returns zero.
- [ ] `UpdateAllowedPairs` will list **only** the target pair address(es).
- [ ] Gas profile reviewed on LocalTerra with representative swap sizes.
- [ ] Staging swap with hook attached: treasury fee, user net output, and hook events match expectations.
- [ ] Rollback plan documented (factory `SetPairHooks` with empty list).

---

## Wasm review checklist

1. **Caller gate** — `assert_allowed_pair` or equivalent; unauthorized callers revert.
2. **`pair == info.sender`** (lp-burn-hook) — reject forged `AfterSwap.pair` when caller is allowlisted but not the pair.
3. **No unbounded loops** over user-controlled inputs.
4. **No silent fund loss** — tax/burn use pair settlement (`ComputeSwapFee`); lp-burn uses pre-funded LP balance capped by `min(target, balance)`.
5. **Reply handling** — burn/tax may swallow CW20 submessage failures; document if swap must never fail on token quirks.
6. **Admin mutations** — config updates restricted to hook `admin` / governance.

---

## Registration steps (staging / mainnet)

Replace placeholders with deployed addresses and LCD node.

```bash
# 1. Instantiate hook (example: tax-hook)
terrad tx wasm instantiate <TAX_HOOK_CODE_ID> \
  '{"recipient":"<TREASURY_OR_RECIPIENT>","tax_percentage_bps":100,"tax_token":"<ASK_CW20>","admin":"<GOVERNANCE>"}' \
  --from <gov> --label "cl8y-tax-hook" ...

HOOK=$(terrad query wasm list-contract-by-code <TAX_HOOK_CODE_ID> --node <lcd> -o json | jq -r '.[-1]')

# 2. Allow the pair to call the hook
terrad tx wasm execute "$HOOK" \
  '{"update_allowed_pairs":{"add":["<PAIR_ADDR>"],"remove":[]}}' \
  --from <gov> ...

# 3. Register on pair via factory (governance)
terrad tx wasm execute <FACTORY> \
  '{"set_pair_hooks":{"pair":"<PAIR_ADDR>","hooks":["'"$HOOK"'"]}}' \
  --from <gov> ...
```

Verify:

```bash
terrad query wasm contract-state smart <PAIR> '{"get_hooks":{}}' --node <lcd>
terrad query wasm contract-state smart "$HOOK" '{"get_config":{}}' --node <lcd>
```

---

## When blocking hooks are acceptable

- **Compliance / AML** — hook reverts when sender or route fails policy checks.
- **Incident response** — temporary hook that fails closed while governance investigates.
- **Maintenance** — prefer factory/pair **pause** for global halt; use blocking hooks only when pair-specific logic is required.

Document every blocking hook in the ops log with owner, scope, and removal date.

---

## De-registration

```bash
terrad tx wasm execute <FACTORY> \
  '{"set_pair_hooks":{"pair":"<PAIR_ADDR>","hooks":[]}}' \
  --from <gov> ...
```

Confirm swaps succeed without hook gas overhead before closing the incident ticket.
