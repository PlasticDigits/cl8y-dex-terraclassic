# REPORT: community-tax-token (in-repo template stub)

**Date:** 2026-08-23  
**Operator:** contracts  
**LCD:** superseded — columbus-5 **11611** stored  
**Procedure:** [`PROCEDURE.md`](../../PROCEDURE.md) ([#589](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/589))  
**Issue:** [#592](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/592) / [#601](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/601)

This file was the **NO-GO** intake stub until LCD store. Canonical report is [`../11611/REPORT.md`](../11611/REPORT.md) (**GO**, Layer B-lt residual).

## Verdict

**GO / NO-GO** for factory `AddWhitelistedCodeId`:

- [ ] GO — use [`../11611/REPORT.md`](../11611/REPORT.md)
- [x] **NO-GO** — this stub has no `code_id`; do not whitelist from this path; do not add FoT math

One-line reason: placeholder only; pin + catalogue live under `codeids/11611/`.

Approving an ID admits **every** instantiate of that wasm (including rogue `--admin`). Catalog must still filter `ContractInfo.admin == CMM` and `GetLauncherOrigin`.

## Identity

| Field | Value |
|-------|--------|
| `code_id` | **11611** (see sibling REPORT) |
| `data_hash` (LCD) | `9D33BF2539A9A5B2F13FD4B321CDBD0B0FD86D936D5D6BD6681955FA30210EC2` |
| Match | yes on LCD (documented in 11611 REPORT) |

## Cross-links

- Playbook: [`skills/AGENTS_COMMUNITY_TAX_CW20.md`](../../../skills/AGENTS_COMMUNITY_TAX_CW20.md)
- Policy exception: [`docs/runbooks/cw20-whitelist-policy.md`](../../../docs/runbooks/cw20-whitelist-policy.md)
- `make verify-issue-592`
