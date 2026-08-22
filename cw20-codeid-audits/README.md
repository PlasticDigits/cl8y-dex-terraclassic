# CW20 code-ID audit harness

Standing intake for factory `AddWhitelistedCodeId` ([GitLab #589](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/589)). Gates [#581](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/581) (SpaceUSD / Terraport **8266**). Does **not** replace listing-time pin **F6** ([#582](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/582)).

**Policy:** a byte-identical optimizer rebuild is **not** a go/no-go input. Canonical evidence is (1) LCD wasm SHA-256 = `CodeInfo.data_hash`, (2) decompilation of **that** binary, (3) human audit against [`CATALOG.md`](CATALOG.md), (4) automated Layer A + Layer B on **that** wasm. Optional source rebuild is an appendix only.

## Add a code ID

```bash
# From repo root. Prefer download + pin over committing wasm.
./cw20-codeid-audits/scripts/fetch-lcd-wasm.sh 8266
./cw20-codeid-audits/scripts/decompile-wasm.sh 8266
./cw20-codeid-audits/scripts/fingerprint-wasm.sh 8266

# Layer A/B multi-test (mintable control + mutant oracles) — no chain:
make verify-issue-589

# Parameterized LCD candidate (needs network + wabt; wasm cached after fetch):
CODE_ID=8266 make verify-issue-589

# Layer B on LocalTerra with the pinned wasm (needs make has-localterra):
CODE_ID=8266 LAYER_B_LT=1 make verify-issue-589
```

Then copy [`report-template.md`](report-template.md) to `codeids/<id>/REPORT.md` and fill every catalogue row. Approving the ID admits **every** instantiate of that wasm.

## Layout

| Path | Role |
|------|------|
| [`PROCEDURE.md`](PROCEDURE.md) | Standard steps (identity → fetch → decomp → suite → go/no-go) |
| [`CATALOG.md`](CATALOG.md) | Exploit / weird-token rows A–CH + citations |
| [`report-template.md`](report-template.md) | Required `REPORT.md` sections |
| `scripts/` | LCD fetch (fail-closed pin), decompile (`wabt`), fingerprint, **A-lcd / B-lt** LocalTerra execution |
| [`harness/README.md`](harness/README.md) | Layer A vs B backends |
| `fixtures/` | Known-good / known-bad control notes |
| `codeids/<id>/` | `meta.json`, `wasm.sha256`, `decomp/`, `REPORT.md` |

Do **not** dump decomp into `smartcontracts/` or `frontend-dapp/`. Do **not** store mnemonics or production admin keys here. Wasm binaries are gitignored; CI re-hashes against LCD when `CODE_ID` is set.

## Controls

| ID | Role | Expected |
|----|------|----------|
| **10184** | Protocol mintable (in-process analogue: `cw20-mintable`) | Honest Layer A/B **green** |
| **6036** | Listed TerraSwap token | Green or documented exceptions |
| **8266** | First candidate (#581) | Report must exist; listing still needs #581 go |
| **8654** / FoT mutant | Known-bad ALPHA tax / fee-on-transfer | 1:1 and **P2** **red** |

## Related

- Ops: [`docs/runbooks/cw20-whitelist-policy.md`](../docs/runbooks/cw20-whitelist-policy.md)
- F6 freeze: [`docs/runbooks/cw20-code-id-ops.md`](../docs/runbooks/cw20-code-id-ops.md)
- Invariants: [`docs/contracts-security-audit.md`](../docs/contracts-security-audit.md)
- Agent playbook: [`skills/AGENTS_CW20_CODE_ID_AUDIT.md`](../skills/AGENTS_CW20_CODE_ID_AUDIT.md)
- Prior 8266 notes (superseded as intake path): [`audits/CW20-8266-581.md`](../audits/CW20-8266-581.md)
