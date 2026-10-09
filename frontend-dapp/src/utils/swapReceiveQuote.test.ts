import { describe, expect, it, vi } from 'vitest'
import { quoteCw20ExactOutputViaRouteSolve } from './cw20RouteSolveQuote'
import { quoteSwapReceiveViaSolver } from './swapReceiveQuote'
vi.mock('./cw20RouteSolveQuote', () => ({ quoteCw20ExactOutputViaRouteSolve: vi.fn() }))
describe('local receive sizing #1452', () => {
  it('delegates unchanged native identity and exact net target without a browser wrap conversion', async () => {
    const request = {
      fromToken: 'uusd',
      toToken: 'terra1ustr',
      targetRaw: '1000000000000000000',
      maxMakerFills: 8,
      slippageTolerancePercent: 5,
      maxSpreadStr: '0.05',
    }
    const quote = { offerAmount: '1000000', return_amount: request.targetRaw }
    vi.mocked(quoteCw20ExactOutputViaRouteSolve).mockResolvedValue(quote as never)
    expect(await quoteSwapReceiveViaSolver(request)).toEqual(quote)
    expect(quoteCw20ExactOutputViaRouteSolve).toHaveBeenCalledWith(expect.objectContaining(request))
  })
  it('propagates unsupported cached semantics and never attempts a conversion fallback', async () => {
    vi.mocked(quoteCw20ExactOutputViaRouteSolve).mockRejectedValue(new Error('Quote unavailable'))
    await expect(
      quoteSwapReceiveViaSolver({
        fromToken: 'uluna',
        toToken: 'terra1ustr',
        targetRaw: '1',
        maxMakerFills: 8,
        slippageTolerancePercent: 5,
        maxSpreadStr: '0.05',
      })
    ).rejects.toThrow('Quote unavailable')
  })
})
