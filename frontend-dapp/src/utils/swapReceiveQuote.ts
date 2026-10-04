import { withReceiveQuoteDeadline } from './receiveQuoteDeadline'
import { quoteCw20ExactOutputViaRouteSolve } from './cw20RouteSolveQuote'
import { wrapMappedSolvePair } from './nativeWrapRouteSolve'
import { netCw20AfterNativeWrap, netNativeAfterUnwrap, wrappedForNativeReceive } from '@/services/terraclassic/router'
import { queryWrapMapperFeeBps, wrapGrossForNetCw20 } from '@/services/terraclassic/wrapMapper'

/** Net receive target → selected CW20 path → gross native offer, using shared fee/tax inverses. */
export async function quoteSwapReceiveViaSolver(input: Parameters<typeof quoteCw20ExactOutputViaRouteSolve>[0]) {
  return withReceiveQuoteDeadline(input.signal, (signal) => quoteSwapReceiveOnSelectedRoute({ ...input, signal }))
}

async function quoteSwapReceiveOnSelectedRoute(input: Parameters<typeof quoteCw20ExactOutputViaRouteSolve>[0]) {
  if (!/^\d+$/.test(input.targetRaw) || BigInt(input.targetRaw) <= 0n || BigInt(input.targetRaw) > (1n << 128n) - 1n) {
    throw new Error('Receive target is outside the supported amount range')
  }
  const mapped = wrapMappedSolvePair(input.fromToken, input.toToken)
  const target = mapped?.needsUnwrapOutput
    ? (await wrappedForNativeReceive(BigInt(input.targetRaw), input.toToken)).toString()
    : input.targetRaw
  input.signal?.throwIfAborted()
  const quote = await quoteCw20ExactOutputViaRouteSolve({
    ...input,
    fromToken: mapped?.tokenIn ?? input.fromToken,
    toToken: mapped?.tokenOut ?? input.toToken,
    targetRaw: target,
  })
  const offer = mapped?.needsWrapInput
    ? wrapGrossForNetCw20(BigInt(quote.offerAmount), await queryWrapMapperFeeBps('wrap'))
    : BigInt(quote.offerAmount)
  if (offer <= 0n || offer > (1n << 128n) - 1n)
    throw new Error('Required pay amount is outside the supported amount range')
  if (mapped?.needsWrapInput && (await netCw20AfterNativeWrap(offer, input.fromToken)) !== BigInt(quote.offerAmount)) {
    throw new Error('Wrap fee changed while sizing the receive target. Retry the quote.')
  }
  let returnAmount = quote.return_amount
  let routerMinReceiveBase: string | undefined
  if (mapped?.needsUnwrapOutput) {
    const native = await netNativeAfterUnwrap(BigInt(quote.return_amount), input.toToken)
    returnAmount = native.receive.toString()
    routerMinReceiveBase = native.routerMinReceiveBase.toString()
  }
  if (BigInt(returnAmount) < BigInt(input.targetRaw)) {
    throw new Error('Fresh route output cannot satisfy the receive target. Retry the quote.')
  }
  input.signal?.throwIfAborted()
  return { ...quote, offerAmount: offer.toString(), return_amount: returnAmount, routerMinReceiveBase }
}
