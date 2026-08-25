# QA — GitLab #641 Hexxagon / Galaxy Station CW20 pack

Verify (no chain): `make verify-issue-641`

Playbook: [`skills/AGENTS_HEXXAGON.md`](../../../skills/AGENTS_HEXXAGON.md) · pack [`docs/listings/hexxagon/`](../../listings/hexxagon/) · invariants **H641-1–H641-8**.

## Automated

- Fragment schema + Keplr pin lockstep + gem exclude
- Five submit rows; **USTR omitted** (already live as USTC Repeg)
- `coinGeckoID` only on CL8Y (`ceramicliberty-com`)
- Skill + AGENTS.md + integrators + verify target

## Manual after upstream merge

1. Galaxy Station → Terra Classic → search **CL8Y**, **UST1**, **cLUNC**, **cUSTC**, **vFDUSD**.
2. Confirm **USTR / USTC Repeg** is unchanged.
3. Live append PR: [hexxagon-io/chain-registry#68](https://github.com/hexxagon-io/chain-registry/pull/68). Do not open a duplicate.

## Out of scope

- Opening/merging the GitHub PR from CI (**H641-7**).
- PRing `terra-money/assets` (**H641-6**).
- Keplr (#629), Cosmostation (#640 — archived), chain-registry (#642).
