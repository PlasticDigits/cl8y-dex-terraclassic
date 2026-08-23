# REPORT: community-tax-token (in-repo template)

**Date:** 2026-08-23  
**Operator:** contracts  
**LCD:** *not stored — no columbus-5 / LocalTerra code_id yet*  
**Procedure:** [`PROCEDURE.md`](../../PROCEDURE.md) ([#589](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/589))  
**Issue:** [#592](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/592)

The inspected binary will be a **decompilation / fingerprint of LCD wasm** after `store`. This file is the intake template so ops does not skip catalogue rows.

## Verdict

**GO / NO-GO** for factory `AddWhitelistedCodeId`:

- [ ] GO — Layer A + Layer B green on the **pinned LCD wasm**; catalogue rows pass or N/A+reason; residuals written
- [x] **NO-GO** — code_id not stored; do not whitelist; do not add FoT math

One-line reason: in-repo crate only; factory whitelist is a **separate ops step** gated on LCD pin + Layer A/B **GO**.

Approving an ID admits **every** instantiate of that wasm (including rogue `--admin`). Catalog must still filter `ContractInfo.admin == CMM` and `GetLauncherOrigin`.

## Identity

| Field | Value |
|-------|--------|
| `code_id` | *TBD after store* |
| `data_hash` (LCD) | *TBD* |
| SHA-256 of downloaded wasm | *TBD* |
| Match | pending |
| Creator / uploader | pending |
| Instantiate permission | Everybody (expected) |
| Approximate instantiate count | 0 |
| `meta.json` | not yet |

## Fetch

Not run. After store: `cw20-codeid-audits/scripts/fetch-lcd-wasm.sh <id>`.

## Fingerprint / decomp

Not run. After store: `fingerprint-wasm.sh` + `decompile-wasm.sh`.

## Catalogue

Fill every A–CH row from [`CATALOG.md`](../../CATALOG.md) against the **pinned LCD wasm**. In-repo unit tests cover inbound 1:1, extra-debit sell, outbound buy, and invoice fail-closed. They do **not** replace Layer A/B on stored bytes.

Known-bad **8654** / FoT mutants must still **FAIL** 1:1 and **P2**. This template must **not** match mutant inbound-tax.

## Layer A / B

- Layer A (multi-test): `cl8y-community-tax-token` multitest — inbound pair Transfer 1:1; sell extra-debit; buy outbound split.
- Layer B-lt: **not run** (`LAYER_B_LT=1 make verify-issue-589` after LocalTerra store).

## Residuals

- Issuer/manager keys and Everybody instantiate — expected; F6 freezes migrate-off-template.
- Pair→EOA `Transfer` (withdraw / limit refund) takes **buy tax** (**T592-7**).
- Rogue `--admin` instantiate — dApp/indexer must not promote (`GetLauncherOrigin`).

## Cross-links

- Playbook: [`skills/AGENTS_COMMUNITY_TAX_CW20.md`](../../../skills/AGENTS_COMMUNITY_TAX_CW20.md)
- Policy exception: [`docs/runbooks/cw20-whitelist-policy.md`](../../../docs/runbooks/cw20-whitelist-policy.md)
- `make verify-issue-592`
