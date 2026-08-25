# Cosmostation / Mintscan CW20 pack (GitLab #640)

Submit **permanent** CL8Y ecosystem CW20s to [cosmostation/chainlist](https://github.com/cosmostation/chainlist) so Cosmostation and Mintscan show a name, symbol, and logo.

Parent catalog: [#639](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/639). This is **not** Keplr ([#629](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/629)) and **not** DeFiLlama ([#631](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/631)).

Agent playbook: [`skills/AGENTS_COSMOSTATION.md`](../../../skills/AGENTS_COSMOSTATION.md). Verify: `make verify-issue-640`.

## Invariants (C640-1–C640-8)

| ID | Rule |
|----|------|
| **C640-1** | Chain folder is **`chain/terra/`** (Terra Classic). Never `phoenix` / Terra 2. |
| **C640-2** | Permanent six only. Same addresses/decimals as Keplr **K629-2** / **K629-3**. No gems / ALPHA / USTRIX / SpaceUSD / tax templates. |
| **C640-3** | Schema: `type=cw20`, `contract`, `name`, `symbol`, `description`, `decimals`, `image`, `coinGeckoId`. Image URL is `…/chain/terra/asset/<file>.png`. PNG lives in [`asset/`](./asset/). |
| **C640-4** | `coinGeckoId` is `ceramicliberty-com` on CL8Y only. All others are `""` (upstream empty-string rule). Do not invent a second CG id. |
| **C640-5** | UST1 description must say **unstablecoin** and must not advertise a `$1` peg. USTR is **USTC Repeg** and **not** a stablecoin. |
| **C640-6** | Append entries to the existing 43-row `cw20_2.json`. Do not recreate the file. Do not replace LUNAX…GRDX. |
| **C640-7** | Do not open the GitHub PR from this repo’s CI. Export the pack, open the upstream PR, link it on #640. |
| **C640-8** | This README + fragment + `make verify-issue-640` + [`skills/AGENTS_COSMOSTATION.md`](../../../skills/AGENTS_COSMOSTATION.md). |

## Catalog

| Token | Decimals | coinGeckoId | Contract |
|-------|----------|-------------|----------|
| CL8Y | 18 | `ceramicliberty-com` | `terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3` |
| UST1 | 6 | `""` | `terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72` |
| USTR | 18 | `""` | `terra1vy3kc0swag2rhn7jz6n72jp0l2ns0p6r6ez5grxq5uhj2rvs97fqfsetxv` |
| cLUNC | 6 | `""` | `terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg` |
| cUSTC | 6 | `""` | `terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch` |
| vFDUSD | 6 | `""` | `terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3` |

JSON to append: [`cw20_2.fragment.json`](./cw20_2.fragment.json). Logos: [`asset/`](./asset/) (from [`tokenlist/images/`](../../../tokenlist/images/)).

## Export a drop-in upstream tree

```bash
./scripts/qa/export-cosmostation-cw20-pack.sh /tmp/cosmostation-cl8y-pack
# writes chain/terra/cw20_2.fragment.json and chain/terra/asset/*.png
```

To merge into a fork that already has `cw20_2.json`:

```bash
python3 - <<'PY'
import json
from pathlib import Path
root = Path("/path/to/chainlist")
live = json.loads((root / "chain/terra/cw20_2.json").read_text())
frag = json.loads(Path("docs/listings/cosmostation/cw20_2.fragment.json").read_text())
have = {row["contract"] for row in live}
live.extend(row for row in frag if row["contract"] not in have)
(root / "chain/terra/cw20_2.json").write_text(json.dumps(live, indent=4) + "\n")
PY
```

## Upstream status (2026-08-25)

[cosmostation/chainlist](https://github.com/cosmostation/chainlist) is **archived** (read-only). Cosmostation Wallet apps shut down **2026-09-01**. The in-repo pack + `make verify-issue-640` still lock the pins. Fork branch (cannot PR upstream): https://github.com/PlasticDigits/chainlist/tree/feat/terra-classic-cl8y-cw20

## Submit (operator, GitHub) — blocked while archived

1. Fork [cosmostation/chainlist](https://github.com/cosmostation/chainlist).
2. Append the six fragment objects to `chain/terra/cw20_2.json` (do not drop existing rows).
3. Copy PNGs into `chain/terra/asset/` (`cl8y.png`, `ust1.png`, `ustr.png`, `clunc.png`, `custc.png`, `vfdusd.png`).
4. Open a PR: *Add CL8Y ecosystem CW20s on Terra Classic (CL8Y, UST1, USTR, cLUNC, cUSTC, vFDUSD)*. Link `https://dex.cl8y.com` and #640.

## Related

- Parent catalog: [#639](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/639)
- Keplr pack (same pins): [`../keplr-contract-registry/`](../keplr-contract-registry/)
- QA: [`docs/qa/issue-640/README.md`](../../qa/issue-640/README.md)
