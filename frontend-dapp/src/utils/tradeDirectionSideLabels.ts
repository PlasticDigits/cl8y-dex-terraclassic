/** Labels for `/trade` order ticket bid/ask direction buttons (GitLab #300). */
export function tradeDirectionSideLabels(
  baseSymbol: string,
  quoteSymbol: string
): { bidLabel: string; askLabel: string } {
  return {
    bidLabel: `Buy ${baseSymbol}`,
    askLabel: `Buy ${quoteSymbol}`,
  }
}
