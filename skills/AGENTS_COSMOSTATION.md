# Agent playbook: Cosmostation / Mintscan CW20s (GitLab #640)

Use when adding Terra Classic CW20 metadata for **Cosmostation** or **Mintscan**, or when someone asks why CL8Y / UST1 / wraps show as unnamed contracts there.

Parent catalog: [#639](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/639). This is **not** Keplr ([#629](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/629)) and **not** Hexxagon ([#641](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/641)).

**Upstream blocked:** [cosmostation/chainlist](https://github.com/cosmostation/chainlist) is **archived** (read-only, 2026-07). Cosmostation Wallet apps shut down **2026-09-01**. Keep the in-repo pack; do **not** open a GitHub PR. Fork (cannot PR upstream): https://github.com/PlasticDigits/chainlist/tree/feat/terra-classic-cl8y-cw20

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
7. **C640-7 No CI PR** — export only. Do **not** open a GitHub PR while `cosmostation/chainlist` is archived. Link a replacement-registry PR on #640 if one appears.
8. **C640-8** — this skill + README + `make verify-issue-640`.

## Verification

```bash
make verify-issue-640
```

No LocalTerra. Do **not** treat an archived-repo PR as a remaining deliverable. After a replacement registry appears: Cosmostation / Mintscan search CL8Y.

## Do not

- Open a GitHub PR against archived `cosmostation/chainlist`.
- Reopen Keplr ([#629](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/629)) or DeFiLlama ([#631](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/631)).
- Invent a phoenix / Terra 2 folder.

## Cross-links

- Parent catalog: [`AGENTS_LISTINGS.md`](./AGENTS_LISTINGS.md) · [#639](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/639)
- Keplr (same pins): [`AGENTS_KEPLR_CW20_REGISTRY.md`](./AGENTS_KEPLR_CW20_REGISTRY.md)
- Hexxagon / Galaxy Station (live PR venue): [`AGENTS_HEXXAGON.md`](./AGENTS_HEXXAGON.md) · [#641](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/641)
- chain-registry / Leap: [#642](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/642)
- QA: [`docs/qa/issue-640/README.md`](../docs/qa/issue-640/README.md)
