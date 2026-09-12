# QA — Forgejo #1222 Terra Classic retail gas census

Verify (no chain): `make verify-issue-1222`

Playbook: [`skills/AGENTS_TERRACLASSIC_GAS.md`](../../../skills/AGENTS_TERRACLASSIC_GAS.md) · ADR [`docs/adr/0004-terraclassic-retail-gas-census.md`](../../adr/0004-terraclassic-retail-gas-census.md) · invariants **G-CENSUS-1–G-CENSUS-8**.

This research MR does **not** retune gas constants, patch wallet adapters, or LCD-simulate as the broadcast envelope.

## Automated

- ADR 0004 has required headings, G0–G7 scored **Stay**, dated AB8BE4F7… re-fetch
- **G-CENSUS-1–G-CENSUS-8** present in ADR + skill
- Production `terraGas.ts` / `hybridSwapGas.ts` / `swapNetworkFee.ts` / wallet adapters / `constants.ts` unchanged vs `main`
- Mixed-hybrid + Network fee + retail inventory Vitest still match census numbers
- #123 / #546 / #618 / #1209 named as out of census

## Manual / operator (not this verify)

1. Reviewers can open the cited columbus-5 hash and see `gas_wanted` **6,785,500**, `gas_used` **5,026,176**, fee denom **`uluna`**.
2. Station mobile / WC Confirm may still show ~4k LUNC — that is **G-AUTO-8**, not a dApp envelope bug. Use **Network fee (est.)**.
3. Unwrap+2hop columbus-5 success hash remains [#600](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/600) (`VERIFY600_COLUMBUS_TX`). Do not reopen #599 from this census.
4. Do **not** open an implement issue unless the ADR decision stops being Stay.

## Out of scope here

- Envelope constant bumps, wallet-adapter patches, CosmWasm migrate.
- Factory `SetDiscountRegistryBatch` ([#123](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/123)).
- V3 Grid keeper crank ([#618](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/618)).
- Indexer protocol-fee ledger ([#1209](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1209)).
