# QA — GitLab #629 Keplr CW20 recognition

Verify (no chain): `make verify-issue-629`

Playbook: [`skills/AGENTS_KEPLR_CW20_REGISTRY.md`](../../../skills/AGENTS_KEPLR_CW20_REGISTRY.md) · pack [`docs/listings/keplr-contract-registry/`](../../listings/keplr-contract-registry/) · invariants **K629-1–K629-9**.

## Verification snapshot (2026-09-24)

- `make verify-issue-629` → **6/6** on current `origin/main` worktree. This validates the local pack, builder, exporter, and docs; it does not query GitHub or launch Keplr.
- Upstream [keplr-contract-registry PR #132](https://github.com/chainapsis/keplr-contract-registry/pull/132) merged on 2026-09-04, with three checks passing. The current upstream JSON is authoritative; see **K629-9** before using the local exporter.
- The Keplr Extension Add Token visual smoke is a wallet-app task and is queued in the [cl8y-pm inbox](https://pm.cl8y.com/inbox); it does not require a transaction or signature.
- CoinGecko `ceramicliberty-com` still shows the BNB Chain contract only; the Terra Classic platform request is tracked by [#644](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/644) and its form draft is [`docs/listings/forms/coingecko-terra-classic-platform.md`](../../listings/forms/coingecko-terra-classic-platform.md).

## Automated

- Pack JSON schema + pins + tokenlist CL8Y decimals **18**
- Vitest `keplrCw20Registry.test.ts` (catalog ↔ on-disk files)
- Export script omits already-registered USTR

## Keplr Extension display check

1. Keplr extension → Terra Classic (columbus-5) → hamburger **Add Token**.
2. Search **CL8Y**, **UST1**, **cLUNC**, **cUSTC**, **vFDUSD** — name + logo, not a bare address.
3. **USTR** already listed as **USTC Repeg** (do not treat a missing rename as a regression).
4. USD figure for CL8Y is **optional** and depends on CoinGecko `ceramicliberty-com`; the current BNB-only platform state is tracked separately by #644. Missing price is **not** a Job 1 failure.

## Out of scope here

- CoinGecko adding the Terra Classic CW20 platform ([#644](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/644), child of parent catalog [#639](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/639)).
- Keplr Ledger signing ([#567](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/567)).
