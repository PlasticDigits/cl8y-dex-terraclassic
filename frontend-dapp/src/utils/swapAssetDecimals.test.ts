import { describe, expect, it } from 'vitest'
import { toRawAmount } from '@/utils/formatAmount'
import {
  indexerPairLegDecimals,
  indexerTokenDecimals,
  parseTokenInfoDecimals,
  registrySwapDecimals,
  resolveSwapAssetDecimals,
} from '@/utils/swapAssetDecimals'
import { USDT_CW20_ADDRESS, USTR_CW20_ADDRESS } from '@/utils/tokenRegistry'

const UNKNOWN = 'terra1from00000000000000000000000000000001'
const UNKNOWN_B = 'terra1to00000000000000000000000000000001'

describe('parseTokenInfoDecimals (#1255)', () => {
  it('accepts integer 0…18', () => {
    expect(parseTokenInfoDecimals(0)).toBe(0)
    expect(parseTokenInfoDecimals(6)).toBe(6)
    expect(parseTokenInfoDecimals(18)).toBe(18)
    expect(parseTokenInfoDecimals('6')).toBe(6)
    expect(parseTokenInfoDecimals('18')).toBe(18)
  })

  it('rejects hostile / non-integer values', () => {
    expect(parseTokenInfoDecimals(255)).toBeNull()
    expect(parseTokenInfoDecimals(1e9)).toBeNull()
    expect(parseTokenInfoDecimals(-1)).toBeNull()
    expect(parseTokenInfoDecimals(18.5)).toBeNull()
    expect(parseTokenInfoDecimals('18e0')).toBeNull()
    expect(parseTokenInfoDecimals('18.0')).toBeNull()
    expect(parseTokenInfoDecimals('-1')).toBeNull()
    expect(parseTokenInfoDecimals('255')).toBeNull()
    expect(parseTokenInfoDecimals('')).toBeNull()
    expect(parseTokenInfoDecimals(null)).toBeNull()
    expect(parseTokenInfoDecimals(undefined)).toBeNull()
    expect(parseTokenInfoDecimals(NaN)).toBeNull()
    expect(parseTokenInfoDecimals(Infinity)).toBeNull()
  })
})

describe('registrySwapDecimals (#1255)', () => {
  it('pins listed product tokens', () => {
    expect(registrySwapDecimals(USTR_CW20_ADDRESS)).toBe(18)
    expect(registrySwapDecimals(USDT_CW20_ADDRESS)).toBe(18)
    expect(registrySwapDecimals('uluna')).toBe(6)
    expect(registrySwapDecimals('uusd')).toBe(6)
    expect(registrySwapDecimals('terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3')).toBe(18)
  })

  it('returns null for unknown CW20 and unknown bank denom (not 6)', () => {
    expect(registrySwapDecimals(UNKNOWN)).toBeNull()
    expect(registrySwapDecimals('terra1unknown')).toBeNull()
    expect(registrySwapDecimals('ibc/ABC')).toBeNull()
    expect(registrySwapDecimals('')).toBeNull()
    expect(registrySwapDecimals(null)).toBeNull()
  })
})

describe('resolveSwapAssetDecimals (#1255)', () => {
  it('uses registry first', () => {
    expect(
      resolveSwapAssetDecimals({
        registryDecimals: 18,
        indexerDecimals: 6,
        onChainDecimals: 6,
        lcdSettled: true,
        lcdHostile: false,
      })
    ).toEqual({ decimals: 18, source: 'registry' })
  })

  it('uses LCD over indexer when both are valid', () => {
    expect(
      resolveSwapAssetDecimals({
        registryDecimals: null,
        indexerDecimals: 6,
        onChainDecimals: 18,
        lcdSettled: true,
        lcdHostile: false,
      })
    ).toEqual({ decimals: 18, source: 'lcd' })
  })

  it('uses indexer while LCD is in flight (T10)', () => {
    expect(
      resolveSwapAssetDecimals({
        registryDecimals: null,
        indexerDecimals: 18,
        onChainDecimals: null,
        lcdSettled: false,
        lcdHostile: false,
      })
    ).toEqual({ decimals: 18, source: 'indexer' })
  })

  it('uses indexer when LCD transport failed', () => {
    expect(
      resolveSwapAssetDecimals({
        registryDecimals: null,
        indexerDecimals: 18,
        onChainDecimals: null,
        lcdSettled: true,
        lcdHostile: false,
      })
    ).toEqual({ decimals: 18, source: 'indexer' })
  })

  it('fails closed on hostile LCD even if indexer is 18', () => {
    expect(
      resolveSwapAssetDecimals({
        registryDecimals: null,
        indexerDecimals: 18,
        onChainDecimals: null,
        lcdSettled: true,
        lcdHostile: true,
      })
    ).toEqual({ decimals: null, source: null })
  })

  it('returns null when nothing is resolved (not 6)', () => {
    expect(
      resolveSwapAssetDecimals({
        registryDecimals: null,
        indexerDecimals: null,
        onChainDecimals: null,
        lcdSettled: false,
        lcdHostile: false,
      }).decimals
    ).toBeNull()
    expect(
      resolveSwapAssetDecimals({
        registryDecimals: null,
        indexerDecimals: null,
        onChainDecimals: null,
        lcdSettled: true,
        lcdHostile: false,
      }).decimals
    ).toBeNull()
  })
})

describe('indexer identity matching (#1255)', () => {
  it('matches pair legs by contract/denom, not symbol', () => {
    const pair = {
      asset_0: { symbol: 'UST1', contract_addr: UNKNOWN, denom: null, decimals: 18 },
      asset_1: { symbol: 'UST1', contract_addr: UNKNOWN_B, denom: null, decimals: 6 },
    }
    expect(indexerPairLegDecimals(pair, UNKNOWN)).toBe(18)
    expect(indexerPairLegDecimals(pair, UNKNOWN_B)).toBe(6)
    expect(indexerPairLegDecimals(pair, 'terra1other')).toBeNull()
    expect(indexerPairLegDecimals(pair, 'uluna')).toBeNull()
  })

  it('rejects out-of-range indexer decimals', () => {
    expect(indexerTokenDecimals({ decimals: 18 })).toBe(18)
    expect(indexerTokenDecimals({ decimals: 255 })).toBeNull()
    expect(indexerTokenDecimals({ decimals: -1 })).toBeNull()
  })
})

describe('human to raw with resolved decimals (#1255)', () => {
  it('scales unlisted 18-dec typed 1 and 1.5', () => {
    expect(toRawAmount('1', 18)).toBe('1000000000000000000')
    expect(toRawAmount('1.5', 18)).toBe('1500000000000000000')
  })

  it('scales unlisted 6-dec typed 1', () => {
    expect(toRawAmount('1', 6)).toBe('1000000')
  })
})
