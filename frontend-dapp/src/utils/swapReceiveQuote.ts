import { quoteCw20ExactOutputViaRouteSolve } from './cw20RouteSolveQuote'
import { withReceiveQuoteDeadline } from './receiveQuoteDeadline'

/** Receive sizing uses the same local pinned executable response as offer-in; no wrapper RPC fallback. */
export async function quoteSwapReceiveViaSolver(input: Parameters<typeof quoteCw20ExactOutputViaRouteSolve>[0]) {
  return withReceiveQuoteDeadline(input.signal, (signal) => quoteCw20ExactOutputViaRouteSolve({ ...input, signal }))
}
