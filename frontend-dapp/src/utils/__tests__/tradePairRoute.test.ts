import { describe, expect, it } from 'vitest'
import {
  getInvalidTradePairRouteParam,
  getTradePageInvalidLinkNotice,
  getTradePageTicketPrefill,
  getTradePageUnknownPairNotice,
  getUnknownTradePairRouteParam,
  isKnownFactoryTradePair,
  isPendingTradePairRouteResolution,
  isTradePairRouteParam,
  shouldAutoPickDefaultTradePair,
  shouldShowTradeWorkspace,
} from '@/utils/tradePairRoute'

const VALID = 'terra1pair0000000000000000000000000000000001'
const VALID_B = 'terra1pair0000000000000000000000000000000002'
const UNKNOWN_FORMAT = 'terra1xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx'

describe('tradePairRoute', () => {
  it('reads invalid/unknown pair notices from router location state (GitLab #358)', () => {
    expect(getTradePageInvalidLinkNotice({ invalidPair: 'lilwayne babyyy' })).toBe('lilwayne babyyy')
    expect(getTradePageUnknownPairNotice({ unknownPair: UNKNOWN_FORMAT })).toBe(UNKNOWN_FORMAT)
    expect(getTradePageInvalidLinkNotice(null)).toBeNull()
    expect(getTradePageUnknownPairNotice(undefined)).toBeNull()
    expect(getTradePageInvalidLinkNotice({ unknownPair: UNKNOWN_FORMAT })).toBeNull()
    expect(getTradePageTicketPrefill({ ticketAmount: '1.5', ticketSide: 'ask' })).toEqual({
      amountHuman: '1.5',
      side: 'ask',
    })
  })

  it('accepts valid terra1 pair addresses', () => {
    expect(isTradePairRouteParam(VALID)).toBe(true)
    expect(getInvalidTradePairRouteParam(VALID)).toBeNull()
  })

  it('rejects non-terra1 garbage deep links', () => {
    expect(isTradePairRouteParam('lilwayne babyyy')).toBe(false)
    expect(getInvalidTradePairRouteParam('lilwayne babyyy')).toBe('lilwayne babyyy')
  })

  it('rejects terra1 prefix without full address', () => {
    expect(isTradePairRouteParam('terra1')).toBe(false)
    expect(getInvalidTradePairRouteParam('terra1')).toBe('terra1')
  })

  it('returns null when route param is absent', () => {
    expect(isTradePairRouteParam(undefined)).toBe(false)
    expect(getInvalidTradePairRouteParam(undefined)).toBeNull()
  })

  it('flags valid-format addresses missing from the factory list (GitLab #175)', () => {
    const pairs = [{ contract_addr: VALID }]
    expect(isKnownFactoryTradePair(VALID, pairs)).toBe(true)
    expect(isKnownFactoryTradePair(UNKNOWN_FORMAT, pairs)).toBe(false)
    expect(getUnknownTradePairRouteParam(UNKNOWN_FORMAT, pairs, true)).toBe(UNKNOWN_FORMAT)
    expect(getUnknownTradePairRouteParam(VALID, pairs, true)).toBeNull()
    expect(getUnknownTradePairRouteParam(UNKNOWN_FORMAT, pairs, false)).toBeNull()
    expect(getUnknownTradePairRouteParam("terra1damThat'scrazy", pairs, true)).toBeNull()
  })

  it('detects pending deep-link resolution before factory list loads (GitLab #175)', () => {
    expect(isPendingTradePairRouteResolution(UNKNOWN_FORMAT, null, false)).toBe(true)
    expect(isPendingTradePairRouteResolution(UNKNOWN_FORMAT, null, true)).toBe(false)
    expect(isPendingTradePairRouteResolution('lilwayne babyyy', 'lilwayne babyyy', false)).toBe(false)
  })

  it('gates trade workspace when pair link errors are active (GitLab #175)', () => {
    expect(
      shouldShowTradeWorkspace({
        pairRouteReady: true,
        invalidLinkNotice: null,
        unknownPairNotice: null,
        pendingDeepLinkPair: false,
      })
    ).toBe(true)
    expect(
      shouldShowTradeWorkspace({
        pairRouteReady: true,
        invalidLinkNotice: null,
        unknownPairNotice: UNKNOWN_FORMAT,
        pendingDeepLinkPair: false,
      })
    ).toBe(false)
    expect(
      shouldShowTradeWorkspace({
        pairRouteReady: false,
        invalidLinkNotice: null,
        unknownPairNotice: null,
        pendingDeepLinkPair: true,
      })
    ).toBe(false)
  })

  it('auto-picks default pair only for bare /trade (GitLab #357)', () => {
    expect(
      shouldAutoPickDefaultTradePair({
        routePair: undefined,
        invalidRoutePair: null,
        unknownRoutePair: null,
        pendingDeepLinkPair: false,
      })
    ).toBe(true)
    expect(
      shouldAutoPickDefaultTradePair({
        routePair: VALID_B,
        invalidRoutePair: null,
        unknownRoutePair: null,
        pendingDeepLinkPair: false,
      })
    ).toBe(false)
    expect(
      shouldAutoPickDefaultTradePair({
        routePair: VALID_B,
        invalidRoutePair: null,
        unknownRoutePair: null,
        pendingDeepLinkPair: true,
      })
    ).toBe(false)
    expect(
      shouldAutoPickDefaultTradePair({
        routePair: UNKNOWN_FORMAT,
        invalidRoutePair: null,
        unknownRoutePair: UNKNOWN_FORMAT,
        pendingDeepLinkPair: false,
      })
    ).toBe(false)
    expect(
      shouldAutoPickDefaultTradePair({
        routePair: 'lilwayne babyyy',
        invalidRoutePair: 'lilwayne babyyy',
        unknownRoutePair: null,
        pendingDeepLinkPair: false,
      })
    ).toBe(false)
  })
})
