# QA — GitLab #629 Keplr CW20 recognition

Verify (no chain): `make verify-issue-629`

Playbook: [`skills/AGENTS_KEPLR_CW20_REGISTRY.md`](../../../skills/AGENTS_KEPLR_CW20_REGISTRY.md) · pack [`docs/listings/keplr-contract-registry/`](../../listings/keplr-contract-registry/) · invariants **K629-1–K629-8**.

## Automated

- Pack JSON schema + pins + tokenlist CL8Y decimals **18**
- Vitest `keplrCw20Registry.test.ts` (catalog ↔ on-disk files)
- Export script omits already-registered USTR

## Manual after upstream merge

1. Keplr extension → Terra Classic (columbus-5) → hamburger **Add Token**.
2. Search **CL8Y**, **UST1**, **cLUNC**, **cUSTC**, **vFDUSD** — name + logo, not a bare address.
3. **USTR** already listed as **USTC Repeg** (do not treat a missing rename as a regression).
4. USD figure for CL8Y is **optional** and depends on CoinGecko `ceramicliberty-com` (BSC platform today). Missing price is **not** a Job 1 failure.

## Out of scope here

- Opening/merging the GitHub PR (operator).
- CoinGecko adding the Terra Classic CW20 platform (parent catalog [#639](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/639)).
- Keplr Ledger signing ([#567](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/567)).
