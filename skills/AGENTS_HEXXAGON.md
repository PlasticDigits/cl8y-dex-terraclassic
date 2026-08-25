# Agent playbook: Hexxagon / Galaxy Station CW20s (GitLab #641)

Use when adding Terra Classic CW20s to Galaxy Station via [hexxagon-io/chain-registry](https://github.com/hexxagon-io/chain-registry).

Parent: [#639](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/639). Not Keplr (#629). Not Cosmostation (#640 — that upstream is archived).

## Invariants (H641-1–H641-8)

1. **H641-1** — `cw20/tokens/mainnet/terra.js` (Classic).
2. **H641-2** — Same six pins as Keplr. Submit five; USTR already live as USTC Repeg.
3. **H641-3** — `protocol`, `symbol`, `name`, `token`, `icon`, `decimals`; optional `coinGeckoID`.
4. **H641-4** — `coinGeckoID` only on CL8Y (`ceramicliberty-com`).
5. **H641-5** — UST1 unstablecoin (never `$1`). USTR not a stablecoin.
6. **H641-6** — Do not PR `terra-money/assets`.
7. **H641-7** — No CI PR.
8. **H641-8** — this skill + README + `make verify-issue-641`.

```bash
make verify-issue-641
```
