# Agent playbook: CW20 code-ID audit harness (GitLab #589)

Use when **adding a CW20 code ID** to the factory whitelist, investigating a candidate template (including **#581 / 8266 SpaceUSD**), or extending the weird-token catalogue.

This playbook is the **intake** process. Listing-time pin **F6** is [`AGENTS_CW20_CODE_ID_PIN.md`](./AGENTS_CW20_CODE_ID_PIN.md) — do not skip F6, and do not add FoT swap math (**H-01**).

## Invariants (C589)

| ID | Rule |
|----|------|
| **C589-1** | Canonical wasm is LCD `/cosmwasm/wasm/v1/code/{id}` whose SHA-256 equals `CodeInfo.data_hash`. Mismatch → fail closed (**C1**). |
| **C589-2** | Decompile **and** tests are both required (**C2**, **C3**). Missing `wabt` fails; do not skip decomp. |
| **C589-3** | Optimizer / byte-identical rebuild is an **optional appendix**, never a go/no-go input. |
| **C589-4** | Approving an ID admits **every** instantiate of that wasm. Write that on every `REPORT.md`. |
| **C589-5** | Known-bad **8654** / FoT mutants must fail 1:1 and **P2**. Green known-bad is a harness bug (**C4**). |
| **C589-6** | 10184 analogue (`cw20-mintable`) must stay green on honest rows. Failures are harness bugs. |
| **C589-7** | Layer B-lt is never a **silent** skip (**C5**). `make verify-issue-589` prints `SKIP Layer B-lt` or, with `LAYER_B_LT=1`, **executes** pinned wasm via `layer-a-lcd.sh` + `layer-b-lt.sh`. Stub PASS is a harness bug ([#590](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/590) **M590-6**). |
| **C589-8** | No mnemonics / admin keys in `cw20-codeid-audits/codeids/` (**C6**). CertiK file hashes are not `data_hash` (**C7**). |
| **C589-9** | Do not whitelist on “decomp looks like cw20-base” alone. **#581 go** comes from [`cw20-codeid-audits/codeids/8266/REPORT.md`](../cw20-codeid-audits/codeids/8266/REPORT.md). |

## Layout

[`cw20-codeid-audits/README.md`](../cw20-codeid-audits/README.md) — `PROCEDURE.md`, [`CATALOG.md`](../cw20-codeid-audits/CATALOG.md) (A–CH), fetch/decomp/fingerprint scripts, `codeids/<id>/`.

Harness: [`cw20-codeid-audits/harness/README.md`](../cw20-codeid-audits/harness/README.md). Tests: `smartcontracts/tests/src/cw20_codeid_harness.rs`, mutants `cw20_mutants.rs`.

## Operator sequence

```bash
./cw20-codeid-audits/scripts/fetch-lcd-wasm.sh <id>
./cw20-codeid-audits/scripts/fingerprint-wasm.sh <id>
./cw20-codeid-audits/scripts/decompile-wasm.sh <id>   # needs wabt
make verify-issue-589                                  # A-mt + B-mt + docs
CODE_ID=<id> make verify-issue-589                     # LCD fetch + decomp
CODE_ID=<id> LAYER_B_LT=1 make verify-issue-589        # after make has-localterra — executes token.wasm
make verify-issue-581                                  # #581 8266 full suite + listing residuals
```

Copy [`report-template.md`](../cw20-codeid-audits/report-template.md) → `codeids/<id>/REPORT.md`. Fill **every** catalogue row. Explicit **GO / NO-GO**.

8266 pin: `953AD60CF6D8C9631B99ADC84C3ABF4083815743F86FF81B2A422FDFDF5F95C0`.

Post-merge ops stack: [`AGENTS_POST_MERGE_OPS_590.md`](./AGENTS_POST_MERGE_OPS_590.md) (`make verify-issue-590`).

## Do not

- Treat a non-matching rebuild as the binary audit.
- `AddWhitelistedCodeId` while that id’s `REPORT.md` is **NO-GO**. 8266 is **GO** and listed on columbus-5; still do **not** whitelist a LocalTerra store id or ALPHA **8654**.
- Treat issuer wasm-admin, Everybody instantiate, or a minter cap as a template veto — those are documented residuals ([#581](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/581)).
- Whitelist a community-tax **LocalTerra** store id, or ALPHA **8654**. Columbus-5 **11611** REPORT is **GO** ([`codeids/11611/REPORT.md`](../cw20-codeid-audits/codeids/11611/REPORT.md)). The stub [`codeids/community-tax-token/REPORT.md`](../cw20-codeid-audits/codeids/community-tax-token/REPORT.md) stays a **NO-GO** placeholder. See [`AGENTS_COMMUNITY_TAX_CW20.md`](./AGENTS_COMMUNITY_TAX_CW20.md).
- Bind-mount `indexer/` into root Docker to run cargo.
- Store secrets under `codeids/`.
- Split catalogue + harness into a parallel issue (work stays in #589).

## Related

- [`AGENTS_CW20_CODE_ID_PIN.md`](./AGENTS_CW20_CODE_ID_PIN.md) — F6
- [`AGENTS_HOOK_CW20_OPS.md`](./AGENTS_HOOK_CW20_OPS.md) — H-01
- [`AGENTS_POST_MERGE_OPS_590.md`](./AGENTS_POST_MERGE_OPS_590.md) — #590 stacked verify
- [`docs/runbooks/cw20-whitelist-policy.md`](../docs/runbooks/cw20-whitelist-policy.md)
- [`docs/contracts-security-audit.md`](../docs/contracts-security-audit.md) **P2**
- [`docs/exploit-replay-matrix.md`](../docs/exploit-replay-matrix.md) **SEC-D06**
