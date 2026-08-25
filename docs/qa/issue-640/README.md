# QA — GitLab #640 Cosmostation CW20 pack

Verify (no chain): `make verify-issue-640`

Playbook: [`skills/AGENTS_COSMOSTATION.md`](../../../skills/AGENTS_COSMOSTATION.md) · pack [`docs/listings/cosmostation/`](../../listings/cosmostation/) · invariants **C640-1–C640-8**.

## Automated

- Fragment schema + Keplr pin lockstep + gem exclude
- Logos under `docs/listings/cosmostation/asset/`
- Export writes `chain/terra/` (not phoenix)
- Skill + AGENTS.md + verify target

## Manual after a replacement registry exists

1. Cosmostation / Mintscan → Terra Classic → search **CL8Y**, **UST1**, **USTR**, **cLUNC**, **cUSTC**, **vFDUSD**.
2. USD for CL8Y still depends on CoinGecko `ceramicliberty-com` (BSC-only today). Missing Classic price is **not** a #640 failure.

**Today (2026-08-25):** [cosmostation/chainlist](https://github.com/cosmostation/chainlist) is **archived**. Fork only: https://github.com/PlasticDigits/chainlist/tree/feat/terra-classic-cl8y-cw20. Do not open an upstream PR.

## Out of scope

- Opening/merging a GitHub PR from CI (**C640-7**).
- Opening a PR against the archived upstream.
- Keplr (#629), Hexxagon (#641), chain-registry (#642).
