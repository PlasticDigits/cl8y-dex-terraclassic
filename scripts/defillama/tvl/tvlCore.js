'use strict'

/**
 * Testable TVL helpers for the DeFiLlama adapter (GitLab #631).
 * Adapter output is raw denom / CW20 amounts via api.add — never indexer USD.
 * cLUNC / cUSTC map to uluna / uusd (named 1:1 wrap substitution).
 */

const { WRAP_TO_NATIVE } = require('../gems')

function isZeroAmount(amount) {
  if (amount == null) return true
  const s = String(amount).trim()
  return s === '' || s === '0'
}

/**
 * Extract raw pool legs from a TerraSwap-compatible `{ pool: {} }` response.
 * Skips zero amounts and never returns `total_share` / LP CW20.
 */
function poolReserveAdds(pool) {
  const adds = []
  const assets = (pool && pool.assets) || []
  for (const asset of assets) {
    const amount = asset && asset.amount
    if (isZeroAmount(amount)) continue
    const info = (asset && asset.info) || {}
    if (info.native_token && info.native_token.denom) {
      adds.push({ key: info.native_token.denom, amount: String(amount) })
      continue
    }
    if (info.token && info.token.contract_addr) {
      const addr = info.token.contract_addr
      const native = WRAP_TO_NATIVE[addr]
      adds.push({ key: native || addr, amount: String(amount) })
    }
  }
  return adds
}

function nextStartAfter(pairs) {
  if (!pairs || !pairs.length) return null
  const last = pairs[pairs.length - 1]
  return last.asset_infos || null
}

function shouldFailPoolErrors(errors, poolCount) {
  if (!poolCount) return false
  return errors.length > poolCount / 2
}

function paginateComplete(page, limit) {
  return !page || page.length < limit
}

module.exports = {
  isZeroAmount,
  poolReserveAdds,
  nextStartAfter,
  shouldFailPoolErrors,
  paginateComplete,
}
