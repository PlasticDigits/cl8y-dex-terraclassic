import { beforeEach, describe, expect, it, vi } from 'vitest'
import { quoteSwapReceiveViaSolver } from './swapReceiveQuote'
import { quoteCw20ExactOutputViaRouteSolve } from './cw20RouteSolveQuote'
import { wrapMappedSolvePair } from './nativeWrapRouteSolve'
import { netCw20AfterNativeWrap, netNativeAfterUnwrap, wrappedForNativeReceive } from '@/services/terraclassic/router'
import { queryWrapMapperFeeBps } from '@/services/terraclassic/wrapMapper'

vi.mock('./cw20RouteSolveQuote', () => ({ quoteCw20ExactOutputViaRouteSolve: vi.fn() }))
vi.mock('./nativeWrapRouteSolve', () => ({ wrapMappedSolvePair: vi.fn() }))
vi.mock('@/services/terraclassic/router', () => ({
  netCw20AfterNativeWrap: vi.fn(),
  netNativeAfterUnwrap: vi.fn(),
  wrappedForNativeReceive: vi.fn(),
}))
vi.mock('@/services/terraclassic/wrapMapper', async (original) => ({
  ...(await original<typeof import('@/services/terraclassic/wrapMapper')>()),
  queryWrapMapperFeeBps: vi.fn(),
}))
const request = {
  fromToken: 'uusd',
  toToken: 'terra1cl8y',
  targetRaw: '100000000000000000000',
  maxMakerFills: 8,
  slippageTolerancePercent: 5,
  maxSpreadStr: '0.05',
}

beforeEach(() => {
  vi.clearAllMocks()
  vi.mocked(wrapMappedSolvePair).mockReturnValue({
    tokenIn: 'terra1custc',
    tokenOut: request.toToken,
    needsWrapInput: true,
    needsUnwrapOutput: false,
  })
  vi.mocked(queryWrapMapperFeeBps).mockResolvedValue(200)
  vi.mocked(netCw20AfterNativeWrap).mockImplementation(async (gross) => gross - (gross * 200n) / 10000n)
  vi.mocked(quoteCw20ExactOutputViaRouteSolve).mockResolvedValue({
    offerAmount: '12649685215',
    return_amount: '100000000005328338879',
    spread_amount: '0',
    commission_amount: '0',
    indexerOperations: [],
  })
})

describe('receive native fee/tax parity (#1376)', () => {
  it('grosses up the realistic cUSTC offer with the shared wrap inverse once', async () => {
    const quote = await quoteSwapReceiveViaSolver(request)
    expect(BigInt(quote.offerAmount)).toBeGreaterThan(12649685215n)
    expect(await netCw20AfterNativeWrap(BigInt(quote.offerAmount))).toBe(12649685215n)
    expect(quoteCw20ExactOutputViaRouteSolve).toHaveBeenCalledWith(
      expect.objectContaining({ fromToken: 'terra1custc', targetRaw: request.targetRaw })
    )
  })

  it('applies native-exit burn then mapper inverse before sizing and preserves the pre-tax native floor', async () => {
    vi.mocked(wrapMappedSolvePair).mockReturnValue({
      tokenIn: 'terra1cl8y',
      tokenOut: 'terra1custc',
      needsWrapInput: false,
      needsUnwrapOutput: true,
    })
    vi.mocked(wrappedForNativeReceive).mockResolvedValue(110n)
    vi.mocked(quoteCw20ExactOutputViaRouteSolve).mockResolvedValue({
      offerAmount: '500',
      return_amount: '110',
      spread_amount: '0',
      commission_amount: '0',
      indexerOperations: [],
    })
    vi.mocked(netNativeAfterUnwrap).mockResolvedValue({ receive: 100n, routerMinReceiveBase: 105n })
    const quote = await quoteSwapReceiveViaSolver({
      ...request,
      fromToken: 'terra1cl8y',
      toToken: 'uusd',
      targetRaw: '100',
    })
    expect(quoteCw20ExactOutputViaRouteSolve).toHaveBeenCalledWith(expect.objectContaining({ targetRaw: '110' }))
    expect(quote).toMatchObject({ offerAmount: '500', return_amount: '100', routerMinReceiveBase: '105' })
  })

  it('rejects fee changes between sizing and executable gross offer validation', async () => {
    vi.mocked(netCw20AfterNativeWrap).mockResolvedValue(1n)
    await expect(quoteSwapReceiveViaSolver(request)).rejects.toThrow('Wrap fee changed')
  })

  it('sends the raw net CW20 target without client buy-tax gross-up', async () => {
    vi.mocked(wrapMappedSolvePair).mockReturnValue(null)
    const quote = await quoteSwapReceiveViaSolver({ ...request, fromToken: 'terra1custc' })
    expect(quote.offerAmount).toBe('12649685215')
    expect(quoteCw20ExactOutputViaRouteSolve).toHaveBeenCalledWith(
      expect.objectContaining({ targetRaw: request.targetRaw })
    )
    expect(queryWrapMapperFeeBps).not.toHaveBeenCalled()
  })
})
