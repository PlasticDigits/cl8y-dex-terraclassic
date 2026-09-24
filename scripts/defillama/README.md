# DeFiLlama adapter copies (GitLab #631 / completed #687)

Canonical **in-repo** copies of the three Llama adapters. Upstream PRs live in Llama’s repos; keep this tree in sync when factory pins or the gem list change.

| Copy | Upstream path | Test (in Llama clone) |
|------|---------------|------------------------|
| [`tvl/index.js`](./tvl/index.js) | `DefiLlama-Adapters/projects/cl8y-dex/index.js` | `node test.js projects/cl8y-dex/index.js` |
| [`dexs/index.ts`](./dexs/index.ts) | `dimension-adapters/dexs/cl8y-dex/index.ts` | `pnpm test dexs cl8y-dex` (version 1, UTC-day API) |
| [`fees/index.ts`](./fees/index.ts) | `dimension-adapters/fees/cl8y-dex/index.ts` | `pnpm test fees cl8y-dex` (version 1, UTC-day API) |
| [`stablecoins/index.ts`](./stablecoins/index.ts) | `peggedassets-server/src/adapters/peggedAssets/ust1/index.ts` | `npx ts-node --transpile-only test.ts ust1 peggedUSD` |

In-repo CI (no Llama SDK):

```bash
node --test scripts/defillama/tvl/tvlCore.test.js \
  scripts/defillama/dimensions/mapDaily.test.js \
  scripts/defillama/stablecoins/ust1Core.test.js
```

## Pins

| Item | Value |
|------|--------|
| Website | `https://dex.cl8y.com` |
| Indexer | `https://indexer.dex.cl8y.com` |
| Daily API | `GET /api/v1/defillama/daily?timestamp=<unix_00:00_utc>` |
| Factory | `terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea` |
| Chain | `terra` (Terra Classic) |
| Slug | `cl8y-dex` |
| Start | `1786924800` / `2026-08-17` (first UTC day GET returns 200; verified in closed #687) |

## Operator notes

- TVL never uses indexer USD or CG `liquidity_in_usd`. Named wrap map only: cLUNC → `uluna`, cUSTC → `uusd`.
- Volume/fees trust the indexer rollup. TVL stays on-chain if the indexer is wrong (A10).
- Upstream: [DefiLlama-Adapters#20676](https://github.com/DefiLlama/DefiLlama-Adapters/pull/20676) (TVL), [dimension-adapters#8987](https://github.com/DefiLlama/dimension-adapters/pull/8987) (volume + fees; merged 2026-08-27 and verified under [#687](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/687)), [peggedassets-server#903](https://github.com/DefiLlama/peggedassets-server/pull/903) (UST1 unstablecoin, draft).
- When changing the volume/fees copies, rerun `pnpm test dexs cl8y-dex` / `pnpm test fees cl8y-dex` (yesterday + start) against the upstream adapters and keep the evidence cross-linked from the closed [#687](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/687) and post-merge ops record [#688](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/688):

  `curl -sS "https://indexer.dex.cl8y.com/api/v1/defillama/daily?timestamp=$(date -u -d yesterday +%s | awk '{print int($1/86400)*86400}')"`

- Icon / metadata PR (Llama icons repo) is a follow-up if Llama asks.

See [`docs/DEFILLAMA.md`](../../docs/DEFILLAMA.md) and [`skills/AGENTS_DEFILLAMA.md`](../../skills/AGENTS_DEFILLAMA.md).
