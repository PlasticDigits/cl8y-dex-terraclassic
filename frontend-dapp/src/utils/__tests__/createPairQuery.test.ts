import { describe, expect, it } from 'vitest'
import { canonicalCreatePairSearch, parseCreatePairQuery, resolveCreatePairQueryValue } from '@/utils/createPairQuery'

const CL8Y = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'
const LISTED = 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'
const CUSTOM = 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72'

const CATALOG = [LISTED, CL8Y]

describe('resolveCreatePairQueryValue (#713)', () => {
  it('accepts catalog hits and checksummed custom terra1', () => {
    expect(resolveCreatePairQueryValue(LISTED, CATALOG)).toBe(LISTED)
    expect(resolveCreatePairQueryValue(LISTED.toUpperCase(), CATALOG)).toBe(LISTED)
    expect(resolveCreatePairQueryValue(CUSTOM, CATALOG)).toBe(CUSTOM)
  })

  it('ignores natives, hostile, and bad checksum (T12 / T13 / A6)', () => {
    expect(resolveCreatePairQueryValue('uluna', CATALOG)).toBeNull()
    expect(resolveCreatePairQueryValue('uusd', CATALOG)).toBeNull()
    expect(resolveCreatePairQueryValue('LUNC', CATALOG)).toBeNull()
    expect(resolveCreatePairQueryValue('javascript:alert(1)', CATALOG)).toBeNull()
    expect(resolveCreatePairQueryValue('https://phish', CATALOG)).toBeNull()
    expect(resolveCreatePairQueryValue(`${LISTED.slice(0, -3)}289`, CATALOG)).toBeNull()
  })
})

describe('parseCreatePairQuery (#713)', () => {
  it('prefills listed a/b and tokenA/token_a aliases', () => {
    expect(parseCreatePairQuery(`a=${LISTED}&b=${CL8Y}`, CATALOG)).toEqual({ tokenA: LISTED, tokenB: CL8Y })
    expect(parseCreatePairQuery(`tokenA=${LISTED}&token_b=${CL8Y}`, CATALOG)).toEqual({
      tokenA: LISTED,
      tokenB: CL8Y,
    })
  })

  it('one-sided query is OK; same token drops B', () => {
    expect(parseCreatePairQuery(`a=${LISTED}`, CATALOG)).toEqual({ tokenA: LISTED, tokenB: null })
    expect(parseCreatePairQuery(`a=${LISTED}&b=${LISTED}`, CATALOG)).toEqual({ tokenA: LISTED, tokenB: null })
  })

  it('canonical search uses a/b only', () => {
    expect(canonicalCreatePairSearch({ tokenA: LISTED, tokenB: CL8Y }).toString()).toBe(`a=${LISTED}&b=${CL8Y}`)
    expect(canonicalCreatePairSearch({ tokenA: LISTED, tokenB: null }).toString()).toBe(`a=${LISTED}`)
    expect(canonicalCreatePairSearch({ tokenA: null, tokenB: null }).toString()).toBe('')
  })
})
