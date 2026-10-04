import { UST1_TOKEN_ADDRESS, MAINNET_UST1_TOKEN_ADDRESS } from '@/utils/constants'
import { buildTokenlistQueryMaps, publishedTokenlistQueryRows } from '@/utils/tokenlistQueryCatalog'
import { describe, expect, it } from 'vitest'
import type { PairInfo } from '@/types'
import { parseTradeTicketPrefill, resolveTradePairFromQuery } from '@/utils/tradeQueryResolve'

const PAIR = 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'
const PAIR_B = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'
const UST1 = UST1_TOKEN_ADDRESS || MAINNET_UST1_TOKEN_ADDRESS

function pair(addr: string, a: string, b: string): PairInfo {
  const infoA = a === 'uluna' || a === 'uusd' ? { native_token: { denom: a } } : { token: { contract_addr: a } }
  const infoB = b === 'uluna' || b === 'uusd' ? { native_token: { denom: b } } : { token: { contract_addr: b } }
  return {
    contract_addr: addr,
    liquidity_token: PAIR_B,
    asset_infos: [infoA, infoB],
  }
}

describe('resolveTradePairFromQuery (#713)', () => {
  const pairs = [pair(PAIR, 'uluna', UST1)]

  it('resolves unique from/to (order independent) and Uniswap aliases', () => {
    expect(resolveTradePairFromQuery(`from=uluna&to=${UST1}`, pairs)).toBe(PAIR)
    expect(resolveTradePairFromQuery(`from=${UST1}&to=uluna`, pairs)).toBe(PAIR)
    expect(resolveTradePairFromQuery(`inputCurrency=uluna&outputCurrency=${UST1}`, pairs)).toBe(PAIR)
    expect(resolveTradePairFromQuery('from=LUNC&to=UST1', pairs)).toBe(PAIR)
    expect(resolveTradePairFromQuery('from=ust1&to=lunc', pairs)).toBe(PAIR)
    const published = buildTokenlistQueryMaps(publishedTokenlistQueryRows(), {})
    expect(published.symbolToId.get('ust1')).toBe(MAINNET_UST1_TOKEN_ADDRESS)
    expect(
      resolveTradePairFromQuery(`from=uluna&to=${MAINNET_UST1_TOKEN_ADDRESS}`, [
        pair(PAIR, 'uluna', MAINNET_UST1_TOKEN_ADDRESS),
      ])
    ).toBe(PAIR)
  })

  it('ignores missing, same-token, hostile, and open-redirect values (T15 / A7)', () => {
    expect(resolveTradePairFromQuery('from=uluna&to=uluna', pairs)).toBeNull()
    expect(resolveTradePairFromQuery('from=uluna', pairs)).toBeNull()
    expect(resolveTradePairFromQuery('from=https://phish&to=uluna', pairs)).toBeNull()
    expect(resolveTradePairFromQuery(`from=uluna&to=${UST1}`, [])).toBeNull()
  })

  it('does not pick at random when two factory pairs share the same legs (A8)', () => {
    const dup = [pair(PAIR, 'uluna', UST1), pair(PAIR_B, UST1, 'uluna')]
    expect(resolveTradePairFromQuery(`from=uluna&to=${UST1}`, dup)).toBeNull()
  })
})

describe('parseTradeTicketPrefill (#713)', () => {
  it('reads amount and buy/sell without placing', () => {
    expect(parseTradeTicketPrefill('from=uluna&to=uusd&amount=1.5&side=sell')).toEqual({
      amountHuman: '1.5',
      side: 'ask',
    })
    expect(parseTradeTicketPrefill('exactAmount=2&side=buy')).toEqual({ amountHuman: '2', side: 'bid' })
    expect(parseTradeTicketPrefill('side=nope')).toEqual({ amountHuman: null, side: null })
  })
})
