import { describe, expect, it } from 'vitest'
import publishedTokenlist from '../../../../tokenlist/tokenlist.json'
import {
  buildTokenlistQueryMaps,
  executeIdToQueryToken,
  foldTokenlistSymbol,
  isAsciiTokenlistSymbol,
  overlayOrPublishedAddress,
  publishedTokenlistQueryRows,
  queryTokenToExecuteId,
} from '@/utils/tokenlistQueryCatalog'
import { MAINNET_UST1_TOKEN_ADDRESS, MAINNET_VFDUSD_TOKEN_ADDRESS } from '@/utils/ust1SecondaryMarket'

const CLUNC_MAINNET = 'terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg'
const LOCAL_CLUNC = 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'

describe('tokenlistQueryCatalog (#715)', () => {
  it('folds ASCII case only so Cyrillic cannot match UST1', () => {
    expect(foldTokenlistSymbol('UST1')).toBe('ust1')
    expect(foldTokenlistSymbol('ust1')).toBe('ust1')
    expect(foldTokenlistSymbol('Ust1')).toBe('ust1')
    expect(foldTokenlistSymbol('U\u0455t1')).not.toBe('ust1')
    expect(isAsciiTokenlistSymbol('SpaceUSD')).toBe(true)
    expect(isAsciiTokenlistSymbol('U\u0455t1')).toBe(false)
  })

  it('published tokenlist symbols and execute ids are unique (CI gate)', () => {
    const rows = publishedTokenlistQueryRows()
    const folded = new Set<string>()
    const ids = new Set<string>()
    expect(rows.length).toBeGreaterThan(0)
    for (const row of rows) {
      expect(isAsciiTokenlistSymbol(row.symbol)).toBe(true)
      const key = foldTokenlistSymbol(row.symbol)
      expect(folded.has(key)).toBe(false)
      folded.add(key)
      const exec =
        row.type === 'native' ? (row.denom ?? '').trim().toLowerCase() : (row.address ?? '').trim().toLowerCase()
      expect(exec).toBeTruthy()
      expect(ids.has(exec)).toBe(false)
      ids.add(exec)
    }
    expect((publishedTokenlist as { tokens: unknown[] }).tokens.length).toBe(rows.length)
  })

  it('maps every published symbol (any casing) to an execute id and back', () => {
    for (const row of publishedTokenlistQueryRows()) {
      const id = queryTokenToExecuteId(row.symbol)
      expect(id).toBeTruthy()
      expect(queryTokenToExecuteId(row.symbol.toLowerCase())).toBe(id)
      expect(queryTokenToExecuteId(row.symbol.toUpperCase())).toBe(id)
      expect(executeIdToQueryToken(id as string)).toBe(row.symbol)
    }
    expect(queryTokenToExecuteId('LUNC')).toBe('uluna')
    expect(queryTokenToExecuteId('USTC')).toBe('uusd')
    expect(executeIdToQueryToken('uluna')).toBe('LUNC')
    expect(executeIdToQueryToken('uusd')).toBe('USTC')
    expect(executeIdToQueryToken(MAINNET_UST1_TOKEN_ADDRESS)).toBe('UST1')
    expect(executeIdToQueryToken(MAINNET_VFDUSD_TOKEN_ADDRESS)).toBe('vFDUSD')
  })

  it('overlay wins inbound; published and overlay ids both encode as the published symbol', () => {
    const maps = buildTokenlistQueryMaps(
      [
        { symbol: 'cLUNC', address: CLUNC_MAINNET, type: 'cw20' },
        { symbol: 'UST1', address: MAINNET_UST1_TOKEN_ADDRESS, type: 'cw20' },
      ],
      { cLUNC: LOCAL_CLUNC }
    )
    expect(maps.symbolToId.get('clunc')).toBe(LOCAL_CLUNC)
    expect(maps.idToSymbol.get(LOCAL_CLUNC.toLowerCase())).toBe('cLUNC')
    expect(maps.idToSymbol.get(CLUNC_MAINNET.toLowerCase())).toBe('cLUNC')
    expect(overlayOrPublishedAddress('cLUNC', CLUNC_MAINNET, { cLUNC: LOCAL_CLUNC })).toBe(LOCAL_CLUNC)
    expect(overlayOrPublishedAddress('cLUNC', CLUNC_MAINNET, { cLUNC: '' })).toBe(CLUNC_MAINNET)
  })

  it('does not invent a ticker for an unlisted execute id', () => {
    expect(executeIdToQueryToken('terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v')).toBe(
      'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'
    )
    expect(queryTokenToExecuteId('RUBY')).toBeNull()
  })
})
