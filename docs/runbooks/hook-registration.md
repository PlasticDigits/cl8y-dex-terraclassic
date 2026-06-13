# Runbook: post-swap hook registration

Governance-only operation. Hooks run **after** swap settlement in the same transaction; a reverting hook **blocks the entire swap** (invariant H1). See [security model § Hook safety](../security-model.md).

**Related:** [hooks README](../../smartcontracts/contracts/hooks/README.md), [contracts security audit § H1/H2](../contracts-security-audit.md), [launch checklist](./launch-checklist.md).

---

## Policy

| Rule | Rationale |
|------|-----------|
| **Audit before registration** | Hooks receive swap amounts and can revert or mis-account fees. Only deploy wasm that passed code review and testnet soak. |
| **Allowlist hygiene** | Each hook's `ALLOWED_PAIRS` must list **only real pair contract addresses** for that deployment. Never allowlist EOAs, routers, or helper contracts ([#377](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/377) H-03). |
| **LP-burn: `pair == caller`** | LP-burn hook requires `AfterSwap.pair == info.sender` and validates pair `liquidity_token` on-chain. |
| **Tax/burn: swap-flow fees** | Tax and burn hooks charge via pair-forwarded ask-token output during settlement (invariant I-02). Do not rely on pre-funded hook treasuries for normal fee collection. |
| **Blocking hooks are allowed** | AML, sanctions, or incident-response hooks may intentionally `Err` to halt trading on a pair. Do **not** wrap hooks in `reply_on_error` unless product policy explicitly changes (H-02). |

---

## Pre-registration checklist

- [ ] Wasm built from **`make build-optimized`** artifacts (not dev `cargo wasm` alone).
- [ ] Source matches tagged release; checksum recorded in deploy log.
- [ ] Unit + integration tests green (`cargo test` burn/tax/lp-burn hooks; `make test-contracts`).
- [ ] Gas bounded — no unbounded loops; no unbounded external queries.
- [ ] `UpdateAllowedPairs` lists **only** intended pair addresses (query `Pair {}` on each; confirm `liquidity_token` for LP-burn).
- [ ] Tax/burn hook `OutputFee` query matches intended bps and token.
- [ ] Staging swap with hook registered: receiver net, treasury commission, and hook fee/burn amounts reconcile.

---

## Registration steps

1. Upload hook wasm (if not already on chain) and instantiate with governance as admin.
2. `UpdateAllowedPairs { add: [<pair_addr>], remove: [] }` on the hook — **pair addresses only**.
3. Factory `SetPairHooks { pair, hooks: [<hook_addr>] }` (governance only).
4. Verify: small staging swap; inspect wasm events and recipient balances.

```bash
terrad query wasm contract-state smart <pair> '{"get_hooks":{}}' --node <lcd>
terrad query wasm contract-state smart <hook> '{"get_config":{}}' --node <lcd>
```

---

## Wasm review checklist (minimum)

- [ ] `Hook` execute gated by `assert_allowed_pair` (caller allowlist).
- [ ] LP-burn: rejects `pair != info.sender`; queries pair `Pair {}` for `liquidity_token`.
- [ ] Tax/burn: implements `OutputFee` query; does not subsidize from pre-funded balance on normal swaps.
- [ ] No `SubMsg::reply_on_error` on the pair→hook dispatch (pair uses plain `WasmMsg::Execute`).
- [ ] No arbitrary `Transfer`/`Send` to addresses from unverified user-supplied fields.

---

## Incident / rollback

- Remove hook: factory `SetPairHooks { pair, hooks: [] }`.
- Pausing the pair (factory pause) stops swaps including hook dispatch.
- Document any intentional blocking hook in ops runbooks so on-call knows swap failures are expected.

---

## Migration (treasury-funded tax/burn deployments)

Legacy deployments that pre-funded hook contracts:

1. Deploy new hook wasm with I-02 swap-flow semantics.
2. Register new hook; remove old hook from pair.
3. Sweep remaining pre-funded balances from old hook via governance (if applicable).
4. Update integrators: receiver net output is **gross return minus hook `OutputFee`** (pair deducts before transfer).
