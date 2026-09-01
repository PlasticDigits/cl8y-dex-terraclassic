import { describe, expect, it, vi } from 'vitest'
import { COPY_BUTTON_FAILURE_MESSAGE } from '@/utils/copyButtonCopy'
import {
  buildCanonicalShareUrl,
  buildCanonicalSwapShareUrl,
  isShareAbortError,
  shareOrCopyPageLink,
  swapShareAriaLabel,
  swapShareText,
  traderShareText,
} from '@/utils/sharePageLink'
import { SHARE_LINK_TITLE, SHARE_LINK_TITLE_SWAP } from '@/utils/sharePageLinkCopy'

const TRADER = 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'
const PAIR = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'

describe('buildCanonicalShareUrl', () => {
  it('builds origin + /trader/{address} with no search or hash', () => {
    expect(
      buildCanonicalShareUrl({
        origin: 'https://dex.example.test/?utm=1#foo',
        kind: 'trader',
        id: `  ${TRADER}  `,
      })
    ).toBe(`https://dex.example.test/trader/${TRADER}`)
  })

  it('strips path, query, and hash from a location.href-like origin', () => {
    expect(
      buildCanonicalShareUrl({
        origin: `http://127.0.0.1:5173/trader/${TRADER}?mnemonic=secret#token`,
        kind: 'trader',
        id: TRADER,
      })
    ).toBe(`http://127.0.0.1:5173/trader/${TRADER}`)
  })

  it('builds trade and charts pair paths', () => {
    expect(buildCanonicalShareUrl({ origin: 'https://qa.example', kind: 'trade', id: PAIR })).toBe(
      `https://qa.example/trade/${PAIR}`
    )
    expect(buildCanonicalShareUrl({ origin: 'https://qa.example', kind: 'charts', id: PAIR })).toBe(
      `https://qa.example/charts/${PAIR}`
    )
  })

  it('rejects invalid terra ids (open redirect / xss / javascript)', () => {
    const origin = 'https://dex.example.test'
    expect(buildCanonicalShareUrl({ origin, kind: 'trader', id: '' })).toBeNull()
    expect(buildCanonicalShareUrl({ origin, kind: 'trader', id: 'not-a-wallet' })).toBeNull()
    expect(buildCanonicalShareUrl({ origin, kind: 'trader', id: 'https://evil' })).toBeNull()
    expect(buildCanonicalShareUrl({ origin, kind: 'trader', id: '//evil.com' })).toBeNull()
    expect(buildCanonicalShareUrl({ origin, kind: 'trader', id: '<script>' })).toBeNull()
    expect(buildCanonicalShareUrl({ origin, kind: 'trader', id: 'javascript:alert(1)' })).toBeNull()
    expect(buildCanonicalShareUrl({ origin, kind: 'trader', id: 'terra1' })).toBeNull()
  })

  it('rejects non-http(s) origins and credentials', () => {
    expect(buildCanonicalShareUrl({ origin: 'javascript:alert(1)', kind: 'trader', id: TRADER })).toBeNull()
    expect(buildCanonicalShareUrl({ origin: 'not a url', kind: 'trader', id: TRADER })).toBeNull()
    expect(buildCanonicalShareUrl({ origin: 'https://user:pass@evil.test', kind: 'trader', id: TRADER })).toBeNull()
  })

  it('does not hard-code production origin', () => {
    const url = buildCanonicalShareUrl({ origin: 'http://localhost:5173', kind: 'trader', id: TRADER })
    expect(url).toBe(`http://localhost:5173/trader/${TRADER}`)
    expect(url).not.toContain('dex.cl8y.com')
  })
})

describe('traderShareText', () => {
  it('is static product copy plus shortened address, not indexer stats', () => {
    const text = traderShareText(TRADER)
    expect(text.startsWith(SHARE_LINK_TITLE)).toBe(true)
    expect(text).toContain('…')
    expect(text).not.toMatch(/P&L|volume|connect wallet/i)
    expect(text).not.toContain(TRADER)
  })
})

describe('shareOrCopyPageLink', () => {
  const payload = {
    url: `https://dex.example.test/trader/${TRADER}`,
    title: SHARE_LINK_TITLE,
    text: traderShareText(TRADER),
  }

  it('calls share and does not copy when share resolves', async () => {
    const share = vi.fn().mockResolvedValue(undefined)
    const copy = vi.fn()
    const result = await shareOrCopyPageLink(payload, { share, copy })
    expect(result).toEqual({ outcome: 'shared' })
    expect(share).toHaveBeenCalledWith(payload)
    expect(copy).not.toHaveBeenCalled()
  })

  it('treats AbortError as cancel: no copy, no failure', async () => {
    const abort = new DOMException('Share canceled', 'AbortError')
    const share = vi.fn().mockRejectedValue(abort)
    const copy = vi.fn()
    const result = await shareOrCopyPageLink(payload, { share, copy })
    expect(result).toEqual({ outcome: 'aborted' })
    expect(copy).not.toHaveBeenCalled()
    expect(isShareAbortError(abort)).toBe(true)
  })

  it('falls back to clipboard on non-abort share failure', async () => {
    const share = vi.fn().mockRejectedValue(new TypeError('NotAllowedError'))
    const copy = vi.fn().mockResolvedValue({ ok: true })
    const result = await shareOrCopyPageLink(payload, { share, copy })
    expect(result).toEqual({ outcome: 'copied' })
    expect(copy).toHaveBeenCalledWith(payload.url)
  })

  it('copies when share is missing', async () => {
    const copy = vi.fn().mockResolvedValue({ ok: true })
    const result = await shareOrCopyPageLink(payload, { copy })
    expect(result).toEqual({ outcome: 'copied' })
    expect(copy).toHaveBeenCalledWith(payload.url)
  })

  it('copies when canShare is false', async () => {
    const share = vi.fn()
    const copy = vi.fn().mockResolvedValue({ ok: true })
    const result = await shareOrCopyPageLink(payload, { share, canShare: () => false, copy })
    expect(result).toEqual({ outcome: 'copied' })
    expect(share).not.toHaveBeenCalled()
  })

  it('returns permission-safe failure when clipboard denies', async () => {
    const copy = vi.fn().mockResolvedValue({ ok: false, message: COPY_BUTTON_FAILURE_MESSAGE })
    const result = await shareOrCopyPageLink(payload, { copy })
    expect(result).toEqual({ outcome: 'copy-failed', message: COPY_BUTTON_FAILURE_MESSAGE })
    expect(result.outcome === 'copy-failed' && result.message).not.toMatch(/DOMException|NotAllowedError/)
  })
})

describe('buildCanonicalSwapShareUrl (#713)', () => {
  const UST1 = 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72'

  it('builds origin + /?from=&to= with tokenlist symbols and drops leftover search/hash (A2 / #715)', () => {
    expect(
      buildCanonicalSwapShareUrl({
        origin: `https://dex.example.test/swap?recipient=evil&wc=1#frag`,
        payId: 'uluna',
        receiveId: UST1,
        amountHuman: '1.5',
        exactField: 'output',
      })
    ).toBe('https://dex.example.test/?from=LUNC&to=UST1&exactAmount=1.5&exactField=output')
  })

  it('rejects hostile ids and never concatenates location.href leftovers', () => {
    expect(
      buildCanonicalSwapShareUrl({
        origin: 'https://dex.example.test',
        payId: 'javascript:alert(1)',
        receiveId: 'uluna',
      })
    ).toBeNull()
    expect(buildCanonicalSwapShareUrl({ origin: 'javascript:alert(1)', payId: 'uluna', receiveId: 'uusd' })).toBeNull()
  })

  it('share title is static product copy plus resolved symbols (A11)', () => {
    expect(swapShareText('uluna', 'uusd')).toBe(`${SHARE_LINK_TITLE_SWAP} LUNC → USTC`)
    expect(swapShareText('uluna', 'uusd')).not.toMatch(/script|from=/i)
    expect(swapShareAriaLabel('uluna', UST1)).toBe('Share LUNC to UST1 swap link')
  })
})
