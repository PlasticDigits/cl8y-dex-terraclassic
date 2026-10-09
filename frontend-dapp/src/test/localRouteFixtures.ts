import type { IndexerPair, IndexerRouteSolveResponse, IndexerToken } from '@/types'
export const LOCAL_FROM = 'terra1from00000000000000000000000000000001'
export const LOCAL_TO = 'terra1to00000000000000000000000000000001'
export const LOCAL_PAIR = 'terra1pair00000000000000000000000000000001'
export const LOCAL_WALLET = 'terra1wallet000000000000000000000000001'
export const localPair: IndexerPair = {
  pair_address: LOCAL_PAIR,
  asset_0: { id: 1, symbol: 'AAA', name: 'Token A', contract_addr: LOCAL_FROM, denom: null, decimals: 6 },
  asset_1: { id: 2, symbol: 'BBB', name: 'Token B', contract_addr: LOCAL_TO, denom: null, decimals: 6 },
  lp_token: 'terra1lp',
  fee_bps: 30,
  is_active: true,
}
export const localTokens = [localPair.asset_0, localPair.asset_1].map((a) => ({
  ...a,
  contract_address: a.contract_addr,
  token_type: 'cw20',
})) as unknown as IndexerToken[]
export function localQuote(
  from = LOCAL_FROM,
  to = LOCAL_TO,
  amount = '1000000',
  options?: { trader?: string; sender?: string; maxSpread?: string },
  output = amount
): IndexerRouteSolveResponse {
  const spread = options?.maxSpread ?? '0.05'
  const floor = ((BigInt(output) * BigInt(Math.round((1 - Number(spread)) * 10000))) / 10000n).toString()
  const hop = {
    pair: LOCAL_PAIR,
    offer_token: from,
    ask_token: to,
    offer_amount: amount,
    return_amount: output,
    pool_return_amount: output,
    book_return_net: '0',
    pool_input_actual: amount,
    book_offer_consumed: '0',
    spread_amount: '0',
    commission_amount: '0',
    min_return: floor,
  }
  return {
    token_in: from,
    token_out: to,
    hops: [hop],
    intermediate_tokens: [from, to],
    quote_kind: 'indexer_pool_db',
    estimated_amount_out: output,
    spot_amount_out: output,
    router_operations: [
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: from } },
          ask_asset_info: { token: { contract_addr: to } },
          min_return: floor,
        },
      },
    ],
    execution_metadata: {
      version: 1,
      generation: 'fixture1',
      input_amount: amount,
      max_spread: spread,
      trader: options?.trader,
      sender: options?.sender,
      observed_at_ms: Date.now() - 1000,
      expires_at_ms: Date.now() + 10000,
      policy: 'pool_only',
      minimum_receive: floor,
      hops: [hop],
    },
  }
}
