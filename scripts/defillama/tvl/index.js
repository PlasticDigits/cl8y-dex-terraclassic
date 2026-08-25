'use strict'

/**
 * DefiLlama-Adapters project copy: `projects/cl8y-dex/index.js`
 *
 * Submit upstream to https://github.com/DefiLlama/DefiLlama-Adapters
 * Test there: `node test.js projects/cl8y-dex/index.js`
 *
 * GitLab #631 — TVL is factory `Pairs` + pair `Pool {}` raw balances only.
 * Do not read indexer USD, CG `liquidity_in_usd`, or overview.total_liquidity_usd.
 */

const { COLUMBUS5_FACTORY } = require('../gems')
const { poolReserveAdds, nextStartAfter, shouldFailPoolErrors, paginateComplete } = require('./tvlCore')

const PAGE_LIMIT = 30
const POOL_CONCURRENCY = 10

async function getAllPairContracts(api) {
  const contracts = []
  let startAfter
  for (;;) {
    const query = startAfter
      ? { pairs: { limit: PAGE_LIMIT, start_after: startAfter } }
      : { pairs: { limit: PAGE_LIMIT } }
    const res = await api.call({
      target: COLUMBUS5_FACTORY,
      abi: query,
    })
    const pairs = (res && res.pairs) || res || []
    for (const p of pairs) {
      if (p.contract_addr) contracts.push(p.contract_addr)
    }
    if (paginateComplete(pairs, PAGE_LIMIT)) break
    startAfter = nextStartAfter(pairs)
    if (!startAfter) break
  }
  return contracts
}

async function queryPool(api, contract) {
  return api.call({
    target: contract,
    abi: { pool: {} },
  })
}

async function tvl(api) {
  const poolContracts = await getAllPairContracts(api)
  const errors = []
  for (let i = 0; i < poolContracts.length; i += POOL_CONCURRENCY) {
    const chunk = poolContracts.slice(i, i + POOL_CONCURRENCY)
    const results = await Promise.allSettled(chunk.map((c) => queryPool(api, c)))
    results.forEach((r, idx) => {
      if (r.status !== 'fulfilled') {
        errors.push(chunk[idx])
        return
      }
      for (const add of poolReserveAdds(r.value || {})) {
        api.add(add.key, add.amount)
      }
    })
  }
  if (shouldFailPoolErrors(errors, poolContracts.length)) {
    throw new Error(
      `cl8y-dex TVL: ${errors.length}/${poolContracts.length} pool queries failed`,
    )
  }
}

module.exports = {
  timetravel: false,
  misrepresentedTokens: false,
  methodology:
    'TVL is the sum of raw native denoms and CW20 balances returned by factory-listed pair Pool {} queries on Terra Classic. LP share tokens, limit-book escrow, wrap-mapper natives, treasury, and UST1-window inventory are omitted. Llama prices tokens; unpriced CW20s (UST1, USTR, cLUNC, cUSTC, CL8Y, community-tax) are omitted rather than pegged. Soft-launch gem pools are included as on-chain locks. Indexer USD and CoinGecko liquidity_in_usd are never used.',
  terra: { tvl },
  factory: COLUMBUS5_FACTORY,
}
