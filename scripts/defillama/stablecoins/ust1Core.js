'use strict'

/**
 * UST1 unstablecoin circulating helper for peggedassets-server.
 * Llama Stablecoins wants human units of peggedUSD, not indexer USD.
 */

const UST1_DECIMALS = 6

function circulatingFromTokenInfo(tokenInfo, decimals = UST1_DECIMALS) {
  const raw = tokenInfo && tokenInfo.total_supply
  if (raw == null || raw === '') return null
  const n = Number(raw)
  if (!Number.isFinite(n) || n < 0) return null
  return n / 10 ** decimals
}

function mintedBalances(circulating) {
  if (circulating == null) return null
  return { peggedUSD: circulating }
}

module.exports = {
  UST1_DECIMALS,
  circulatingFromTokenInfo,
  mintedBalances,
}
