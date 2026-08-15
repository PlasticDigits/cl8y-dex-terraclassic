# Agent playbook: LP ticker digits (GitLab #518)

Use when creating pairs whose **asset symbols contain digits** (UST1, CL8Y, future `…2` tickers), debugging `create_pair` reverts with `Ticker symbol is not in expected format`, or changing LP CW20 instantiate / factory `lp_token_code_id`.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [`docs/contracts-terraclassic.md` § CreatePair LP ticker](../docs/contracts-terraclassic.md#createpair-lp-ticker-gitlab-518) | Operator + integrator narrative |
| Invariant **F3** | [`docs/contracts-security-audit.md`](../docs/contracts-security-audit.md) |
| [`docs/security-model.md` § CreatePair](../docs/security-model.md#createpair-rate-limit-and-pending-state) | Trust-model note |
| [`dex_common::lp_symbol`](../smartcontracts/packages/dex-common/src/lp_symbol.rs) | Sanitize + fallback |
| Pair instantiate | [`pair/src/contract.rs`](../smartcontracts/contracts/pair/src/contract.rs) `lp_token_instantiate_meta` |
| UST1 secondary AMM | [`AGENTS_UST1_SECONDARY_AMM.md`](./AGENTS_UST1_SECONDARY_AMM.md) — Path A needs this pair code live |

## Invariants (F3)

1. **Classic charset** — LP CW20 `symbol` must match **`[a-zA-Z\-]{3,12}`**. columbus-5 `lp_token_code_id` still uses this (no digits). Workspace `cw20-mintable` allowing `[a-zA-Z0-9\-]` does **not** mean mainnet LP code does.
2. **Sanitize, do not widen the validator** — strip non-letters from each 4-char prefix, join `{a}-{b}-LP`, collapse hyphens. Do not change `cw20-mintable` or factory `lp_token_code_id` for this unblock (issue option 3 is a wider blast radius).
3. **Fallback is `CLY-LP`** — never `CL8Y-LP` (digit `8` fails classic validation). Used when `token_symbols` is `None` or prefixes collapse below length 3.
4. **Name / label stay unique** — full factory-truncated symbols remain on LP `name` and the pair/LP wasm labels so wallets can still tell pools apart.
5. **Instantiate-only** — existing pairs keep their LP tokens. Unblock = upload new **pair** wasm + factory `UpdateConfig.pair_code_id`. Migrating already-created pairs is unnecessary.

## Launch pairs (factory uppercases + take 6, then pair sanitizes)

| Assets | Factory `token_symbols` | LP `symbol` |
|--------|-------------------------|-------------|
| UST1 / cUSTC | `UST1`, `CUSTC` | `UST-CUST-LP` |
| UST1 / USTR | `UST1`, `USTR` | `UST-USTR-LP` |
| CL8Y / cLUNC | `CL8Y`, `CLUNC` | `CLY-CLUN-LP` |
| cLUNC / cUSTC | `CLUNC`, `CUSTC` | `CLUN-CUST-LP` |

## Do not

- Re-introduce `format!("{}-{}-LP", take(4), take(4))` without sanitizing.
- Treat a passing LocalTerra create_pair against **digit-allowing** `cw20-mintable` as proof columbus-5 will accept UST1 — use `classic_lp_cw20` tests / `make verify-issue-518`.
- Fold UST1 into soft-launch gemstone catalogs (still **U6**).

## Verification

```bash
make verify-issue-518
# or:
cd smartcontracts && cargo test -p dex-common lp_symbol -- --quiet
cd smartcontracts && cargo test -p cl8y-dex-tests --lib lp_symbol -- --quiet
```

No LocalTerra or Postgres required. After mainnet pair-code update, re-simulate `create_pair` for UST1/cUSTC and any CL8Y pair (issue reporter).

## Related

- [`AGENTS_UST1_SECONDARY_AMM.md`](./AGENTS_UST1_SECONDARY_AMM.md) — #508 Path A is blocked until this pair code is on the factory
- [`AGENTS_LAUNCH_GO_NO_GO.md`](./AGENTS_LAUNCH_GO_NO_GO.md) — launch blocker until economic pools can be created
- [`AGENTS_WASM_MIGRATION_ROLLBACK.md`](./AGENTS_WASM_MIGRATION_ROLLBACK.md) — pair `code_id` update, not an in-place LP migrate
