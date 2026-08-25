# QA — GitLab #640 Cosmostation CW20 pack

Verify (no chain): `make verify-issue-640`

Playbook: [`skills/AGENTS_COSMOSTATION.md`](../../../skills/AGENTS_COSMOSTATION.md) · pack [`docs/listings/cosmostation/`](../../listings/cosmostation/) · invariants **C640-1–C640-8**.

## Automated

- Fragment schema + Keplr pin lockstep + gem exclude
- Logos under `docs/listings/cosmostation/asset/`
- Export writes `chain/terra/` (not phoenix)
- Skill + AGENTS.md + verify target

## Manual after upstream merge

1. Cosmostation / Mintscan → Terra Classic → search **CL8Y**, **UST1**, **USTR**, **cLUNC**, **cUSTC**, **vFDUSD**.
2. USD for CL8Y still depends on CoinGecko `ceramicliberty-com` (BSC-only today). Missing Classic price is **not** a #640 failure.

## Out of scope

- Opening/merging the GitHub PR from CI.
- Keplr (#629), Hexxagon (#641), chain-registry (#642).
