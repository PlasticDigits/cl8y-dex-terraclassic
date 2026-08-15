# Agent playbook: LP ticker digits (GitLab #518)

Use when creating pairs whose **asset symbols contain digits** (UST1, CL8Y), debugging `create_pair` reverts with `Ticker symbol is not in expected format`, or rotating factory `pair_code_id` / `lp_token_code_id`.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [`docs/contracts-terraclassic.md` § CreatePair LP ticker](../docs/contracts-terraclassic.md#createpair-lp-ticker-gitlab-518) | Operator + integrator narrative |
| Invariant **F3** | [`docs/contracts-security-audit.md`](../docs/contracts-security-audit.md) |
| [`dex_common::lp_symbol`](../smartcontracts/packages/dex-common/src/lp_symbol.rs) | Keep `[0-9A-Za-z]`, strip the rest |
| Factory `UpdateConfig` | `pair_code_id` / `lp_token_code_id` (factory **1.6.0**) |
| Upgrade script | [`scripts/upgrade-518-lp-symbol.sh`](../scripts/upgrade-518-lp-symbol.sh) |
| UST1 secondary AMM | [`AGENTS_UST1_SECONDARY_AMM.md`](./AGENTS_UST1_SECONDARY_AMM.md) — Path A needs this upgrade live |

## Invariants (F3)

1. **Keep digits** — LP `symbol` prefixes are ASCII **alphanumeric**. `UST1` stays `UST1`, `CL8Y` stays `CL8Y`.
2. **Sanitize non-alnum only** — drop `_`, `!`, spaces, hyphens inside the source symbol, unicode, etc. Then join `{a}-{b}-LP` (hyphens we insert are allowed).
3. **Mintable charset** — derived ticker must match **`[a-zA-Z0-9\-]{3,12}`**. Fallback **`CL8Y-LP`**.
4. **Classic LP still rejects digits** — columbus-5 Terraswap LP (`[a-zA-Z\-]`) cannot instantiate `UST1-CUST-LP`. Factory `lp_token_code_id` **must** be digit-allowing `cw20-mintable` (reuse **10184** or store `cw20_mintable.wasm`).
5. **Upgrade path** — store factory + pair wasm → `wasm migrate` factory → `UpdateConfig { pair_code_id, lp_token_code_id }`. Existing pairs keep their LP tokens.
6. **Name / label stay unique** — unsanitized factory symbols remain on LP `name` and wasm labels.

## Launch pairs (after upgrade)

| Assets | Factory `token_symbols` | LP `symbol` |
|--------|-------------------------|-------------|
| UST1 / cUSTC | `UST1`, `CUSTC` | `UST1-CUST-LP` |
| UST1 / USTR | `UST1`, `USTR` | `UST1-USTR-LP` |
| CL8Y / cLUNC | `CL8Y`, `CLUNC` | `CL8Y-CLUN-LP` |
| cLUNC / cUSTC | `CLUNC`, `CUSTC` | `CLUN-CUST-LP` |

## Upgrade

```bash
DRY_RUN=1 ./scripts/upgrade-518-lp-symbol.sh
# LocalTerra (needs deploy + artifacts):
UPGRADE518_LOCAL=1 ./scripts/upgrade-518-lp-symbol.sh
# columbus-5, reuse on-chain mintable 10184:
UPGRADE518_LP_CODE_ID=10184 ./scripts/upgrade-518-lp-symbol.sh
```

Needs optimized wasm (`make build-optimized`) unless `UPGRADE518_SKIP_STORE=1`. Governance / wasm-admin key signs store + migrate + `UpdateConfig`. See [`AGENTS_WASM_MIGRATION_ROLLBACK.md`](./AGENTS_WASM_MIGRATION_ROLLBACK.md).

## Do not

- Strip digits from UST1 / CL8Y (older letter-only sanitize).
- Treat LocalTerra success against mintable LP as proof classic `lp_token_code_id` will accept UST1.
- Fold UST1 into soft-launch gemstone catalogs (**U6**).

## Verification

```bash
make verify-issue-518
```

No LocalTerra required for the default gate. After the on-chain upgrade, re-simulate `create_pair` for UST1/cUSTC and any CL8Y pair.

## Related

- [`AGENTS_UST1_SECONDARY_AMM.md`](./AGENTS_UST1_SECONDARY_AMM.md)
- [`AGENTS_LAUNCH_GO_NO_GO.md`](./AGENTS_LAUNCH_GO_NO_GO.md)
- [`AGENTS_WASM_MIGRATION_ROLLBACK.md`](./AGENTS_WASM_MIGRATION_ROLLBACK.md)
