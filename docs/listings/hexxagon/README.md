# Hexxagon / Galaxy Station CW20 pack (GitLab #641)

Append **permanent** CL8Y ecosystem CW20s to [hexxagon-io/chain-registry](https://github.com/hexxagon-io/chain-registry) `cw20/tokens/mainnet/terra.js`.

**USTR is already listed** as USTC Repeg. This pack submits the other five. Parent: [#639](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/639). Not Keplr (#629), not Cosmostation (#640).

Verify: `make verify-issue-641`. Playbook: [`skills/AGENTS_HEXXAGON.md`](../../../skills/AGENTS_HEXXAGON.md).

## Invariants (H641-1–H641-8)

| ID | Rule |
|----|------|
| **H641-1** | File is `cw20/tokens/mainnet/terra.js` (Classic). Not phoenix. |
| **H641-2** | Permanent six pins/decimals match Keplr **K629-2**. Submit five; **USTR already live**. No gems. |
| **H641-3** | Objects: `protocol`, `symbol`, `name`, `token`, `icon`, `decimals`. Optional `coinGeckoID` (Hexxagon spelling) only on CL8Y. Trailing commas. Append at end. |
| **H641-4** | `coinGeckoID` is `ceramicliberty-com` on CL8Y only. Do not invent a second id. |
| **H641-5** | UST1 is an **unstablecoin** (never `$1`). USTR is not a stablecoin and is verify-only here. |
| **H641-6** | Do not PR `terra-money/assets` — Hexxagon replaced that for Galaxy Station. |
| **H641-7** | Do not open the GitHub PR from CI. |
| **H641-8** | This README + fragment + `make verify-issue-641` + skill. |

Fragment: [`terra.fragment.json`](./terra.fragment.json). Icons are live GitLab `tokenlist/images/` URLs.

## Submit

1. Fork [hexxagon-io/chain-registry](https://github.com/hexxagon-io/chain-registry).
2. Append the five objects to `cw20/tokens/mainnet/terra.js` (trailing commas, end of list).
3. PR: *Add CL8Y ecosystem CW20s on Terra Classic (CL8Y, UST1, cLUNC, cUSTC, vFDUSD)*. Mention USTR is already listed.
