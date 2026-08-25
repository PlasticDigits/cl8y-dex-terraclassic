'use strict'

/**
 * peggedassets-server copy: `src/adapters/peggedAssets/ust1/index.ts`
 *
 * UST1 unstablecoin on Terra Classic. Llama Stablecoins dashboard.
 * Circulating = CW20 token_info.total_supply / 1e6 (human). peggedUSD.
 * Mechanism: crypto-backed (ust1-window vs vFDUSD). Not a $1 hardcode.
 *
 * Submit upstream to https://github.com/DefiLlama/peggedassets-server
 */

const { UST1 } = require('../gems')
const { circulatingFromTokenInfo, mintedBalances } = require('./ust1Core')

module.exports = {
  chain: 'terra',
  issued: UST1,
  pegType: 'peggedUSD',
  pegMechanism: 'crypto-backed',
  product: 'unstablecoin',
  circulatingFromTokenInfo,
  mintedBalances,
}
