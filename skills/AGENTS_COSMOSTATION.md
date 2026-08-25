# Agent playbook: Cosmostation / Mintscan CW20s (GitLab #640)

Use when adding Terra Classic CW20 metadata for **Cosmostation** or **Mintscan**, or when someone asks why CL8Y / UST1 / wraps show as unnamed contracts there.

Parent catalog: [#639](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/639). This is **not** Keplr ([#629](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/629)) and **not** Hexxagon ([#641](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/641)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/listings/cosmostation/README.md](../docs/listings/cosmostation/README.md) | Invariants **C640-1–C640-8** |
| [`cw20_2.fragment.json`](../docs/listings/cosmostation/cw20_2.fragment.json) | Rows to append |
| [`export-cosmostation-cw20-pack.sh`](../scripts/qa/export-cosmostation-cw20-pack.sh) | Drop-in `chain/terra/` tree |
| Upstream | [cosmostation/chainlist](https://github.com/cosmostation/chainlist) `chain/terra/cw20_2.json` |

## Invariants (C640-1–C640-8)

1. **C640-1 Terra folder** — `chain/terra/`. Never phoenix / Terra 2.
2. **C640-2 Permanent six** — same pins/decimals as **K629-2**. No gems.
3. **C640-3 Schema** — `type`, `contract`, `name`, `symbol`, `description`, `decimals`, `image`, `coinGeckoId`. PNG under `chain/terra/asset/`.
4. **C640-4 coinGeckoId** — `ceramicliberty-com` on CL8Y only; `""` elsewhere. No second id.
5. **C640-5 Pegs** — UST1 is an unstablecoin (never `$1`). USTR is USTC Repeg, not a stablecoin.
6. **C640-6 Append** — do not replace the live 43-row file.
7. **C640-7 No CI PR** — export, open GitHub PR, link #640.
8. **C640-8** — this skill + README + `make verify-issue-640`.

## Verification

```bash
make verify-issue-640
```

No LocalTerra. After upstream merge: Cosmostation / Mintscan search CL8Y.

## Cross-links

- Parent catalog: [#639](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/639)
- Keplr: [`AGENTS_KEPLR_CW20_REGISTRY.md`](./AGENTS_KEPLR_CW20_REGISTRY.md)
- Hexxagon: [#641](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/641)
- chain-registry / Leap: [#642](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/642)
